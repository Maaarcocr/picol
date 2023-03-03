use std::{future::Future, os::fd::AsRawFd};

use futures_lite::{future::{block_on, or}};

struct Entry {
    pub fd: i32,
    pub offset: i64,
    pub len: u32,
    pub result: *mut u8,
    pub waker: std::task::Waker,
}

thread_local! {
    static SUBMISSION_QUEUE: (flume::Sender<Entry>, flume::Receiver<Entry>) = flume::unbounded();
    static COMPLETION_QUEUE: (flume::Sender<()>, flume::Receiver<()>) = flume::unbounded();
}

async fn read_submissions(ring: &io_uring::IoUring) {
    let receiver = SUBMISSION_QUEUE.with(|(_, r)| r.clone());
    let entry = receiver.recv_async().await.unwrap();
    let waker = Box::new(entry.waker.clone());
    let waker = Box::into_raw(waker);
    let read_e = io_uring::opcode::Read::new(io_uring::types::Fd(entry.fd), entry.result, entry.len)
        .offset(entry.offset)
        .build();
    let read_e = read_e.user_data(waker as *const _ as u64);
    unsafe { ring.submission_shared().push(&read_e).expect("submission queue is full"); }
    ring.submit().unwrap();
    COMPLETION_QUEUE.with(|(s, _)| {
        s.send(()).unwrap();
    });
}

async fn read_completions(ring: &io_uring::IoUring) {
    let receiver = COMPLETION_QUEUE.with(|(_, r)| r.clone());
    receiver.recv_async().await.unwrap();

    ring.submit_and_wait(1).unwrap();
    let mut cq = unsafe { ring.completion_shared() };
    let cqe = cq.next().unwrap();
    let waker = cqe.user_data() as *mut std::task::Waker;
    let waker = unsafe { Box::from_raw(waker) };
    waker.wake()
}

pub struct AsynFileRead<'a> {
    file: std::fs::File,
    result: &'a mut Vec<u8>,
    offset: i64,
    len: u32,
    enqueue: bool,
}

pub async fn read(file: std::fs::File, offset: i64, len: u32) -> Vec<u8> {
    let mut result = vec![0; len as usize];
    let read = AsynFileRead {
        file,
        result: &mut result,
        offset,
        len,
        enqueue: true,
    };
    read.await;
    result
}

impl<'a> Future for AsynFileRead<'a> {
    type Output = ();

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        if self.enqueue {
            SUBMISSION_QUEUE.with(|(s, _)| {
                s.send(Entry {
                    fd: self.file.as_raw_fd(),
                    offset: self.offset,
                    len: self.len,
                    result: self.result.as_mut_ptr(),
                    waker: cx.waker().clone(),
                }).unwrap();
            });
            self.enqueue = false;
            std::task::Poll::Pending
        } else {
            std::task::Poll::Ready(())
        }
    }
}

pub fn spawn<T: 'static>(future: impl Future<Output = T> + 'static) -> T {
    let ring = io_uring::IoUring::new(8).unwrap();

    // Create a task that runs the future.
    let (s, r) = flume::unbounded();
    let (runnable, task) = async_task::spawn_local(future, move |runnable| {
        // Schedule the task by sending it to the queue.
        s.send(runnable).unwrap();
    });

    // Run the task.
    runnable.schedule();

    block_on(async {
        let processing_task = async {
            loop {
                or(async {
                    r.recv_async().await.unwrap().run();
                }, or(read_submissions(&ring), read_completions(&ring))).await;
            }
        };
        let res = or(task, processing_task).await;

        res
    })
}
