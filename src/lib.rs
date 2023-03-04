use std::{future::Future, os::fd::{AsRawFd, RawFd}};

use async_task::{Runnable, Task};
use futures_lite::{future::{or, self}};
use once_cell::sync::Lazy;

struct Entry {
    pub fd: i32,
    pub offset: i64,
    pub len: u32,
    pub result: *mut u8,
    pub waker: std::task::Waker,
}

thread_local! {
    static SUBMISSION_QUEUE: (flume::Sender<Entry>, flume::Receiver<Entry>) = flume::unbounded();
    static RING: io_uring::IoUring = io_uring::IoUring::new(8).unwrap();
    static NUMBER_OF_PROCESSING: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
}

static PROCESSING_QUEUE: Lazy<(flume::Sender<Runnable>, flume::Receiver<Runnable>)> = Lazy::new(|| flume::unbounded());
static LOW_PRIORITY_QUEUE: Lazy<(flume::Sender<Runnable>, flume::Receiver<Runnable>)> = Lazy::new(|| flume::unbounded());

async fn read_submissions() {
    let receiver = SUBMISSION_QUEUE.with(|(_, r)| r.clone());
    let entry = receiver.recv_async().await.unwrap();
    let waker = Box::new(entry.waker.clone());
    let waker = Box::into_raw(waker);
    let read_e = io_uring::opcode::Read::new(io_uring::types::Fd(entry.fd), entry.result, entry.len)
        .offset(entry.offset)
        .build();
    let read_e = read_e.user_data(waker as *const _ as u64);
    RING.with(|ring| {
        unsafe { ring.submission_shared().push(&read_e).expect("submission queue is full"); }
        ring.submit().unwrap();
    });
                        
    NUMBER_OF_PROCESSING.with(|n| {
        n.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    });

    spawn_low_priority(
        read_completions()
    ).detach();
}

async fn read_completions() {
    RING.with(|ring| {
        let n = NUMBER_OF_PROCESSING.with(|n| n.swap(0, std::sync::atomic::Ordering::Relaxed));
        if n == 0 {
            return;
        }

        println!("read_completions: {}", n);

        ring.submit_and_wait(n).unwrap();
        
        let mut cq = unsafe { ring.completion_shared() };
        for _ in 0..n {
            let cqe = cq.next().unwrap();
            let waker = cqe.user_data() as *mut std::task::Waker;
            let waker = unsafe { Box::from_raw(waker) };
            waker.wake();
        }
    });
}

pub struct AsynFileRead {
    file: RawFd,
    result: Vec<u8>,
    offset: i64,
    len: u32,
    enqueue: bool,
}

pub fn read(file: &std::fs::File, offset: i64, len: u32) -> AsynFileRead  {
    let result = vec![0; len as usize];
    let read = AsynFileRead {
        file: file.as_raw_fd(),
        result,
        offset,
        len,
        enqueue: true,
    };
    read
}

impl Future for AsynFileRead {
    type Output = Vec<u8>;

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        if self.enqueue {
            self.enqueue = false;
            SUBMISSION_QUEUE.with(|(s, _)| {
                s.send(Entry {
                    fd: self.file,
                    offset: self.offset,
                    len: self.len,
                    result: self.result.as_mut_ptr(),
                    waker: cx.waker().clone(),
                }).unwrap();
            });
            spawn(async {
                read_submissions().await;
            }).detach();
            std::task::Poll::Pending
        } else {
            std::task::Poll::Ready(std::mem::take(&mut self.result))
        }
    }
}

pub fn spawn<T: 'static>(future: impl Future<Output = T> + 'static) -> Task<T> {
    // Create a task that runs the future.
    let (runnable, task) = async_task::spawn_local(future, move |runnable| {
        // Schedule the task by sending it to the queue.
        PROCESSING_QUEUE.0.send(runnable).unwrap();
    });

    // Run the task.
    runnable.schedule();
    task
}

pub fn spawn_low_priority<T: 'static>(future: impl Future<Output = T> + 'static) -> Task<T> {
    // Create a task that runs the future.
    let (runnable, task) = async_task::spawn_local(future, move |runnable| {
        // Schedule the task by sending it to the queue.
        LOW_PRIORITY_QUEUE.0.send(runnable).unwrap();
    });

    // Run the task.
    runnable.schedule();
    task
}

pub fn block_on<T: 'static>(future: impl Future<Output = T> + 'static) -> T {
    future::block_on(async {
        let processing_task = async {
            let receiver = &PROCESSING_QUEUE.1;
            let low_priority_receiver = &LOW_PRIORITY_QUEUE.1;
            loop {
                let runnable = or(receiver.recv_async(), low_priority_receiver.recv_async()).await.unwrap();
                runnable.run();
            }

        };
        let res = or(spawn(future), processing_task).await;

        res
    })
}
