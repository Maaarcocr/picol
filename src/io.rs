use std::{task::{Poll, Context}, pin::Pin, future::Future};
use io_uring::squeue::Entry;

use crate::{spawn_low_priority, spawn};

thread_local! {
    static RING: io_uring::IoUring = io_uring::IoUring::new(8).unwrap();
    static NUMBER_OF_PROCESSING: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
}

pub async fn handle_submission(entry: Entry) {
    RING.with(|ring| {
        unsafe { ring.submission_shared().push(&entry).expect("submission queue is full"); }
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
            let data = cqe.user_data() as *mut (std::task::Waker, *mut i32);
            let data = unsafe { Box::from_raw(data) };
            let (waker, result) = *data;
            unsafe { *result = cqe.result() }
            waker.wake();
        }
    });
}

pub struct UringFuture {
    entry: Option<Entry>,
    result: i32,
}

impl UringFuture {
    pub fn new(entry: Entry) -> Self {
        Self {
            entry: Some(entry),
            result: -1,
        }
    }
}

impl Future for UringFuture {
    type Output = i32;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.result == -1 {
            let waker = cx.waker().clone();
            let entry = self.entry.take().unwrap();
            let result_ptr = &mut self.result as *mut i32;

            let waker = Box::new((waker, result_ptr));
            let waker = Box::into_raw(waker);
            let read_e = entry.user_data(waker as u64);

            
            spawn(async move {
                handle_submission(read_e).await;
            }).detach();
            Poll::Pending
        } else {
            Poll::Ready(self.result)
        }
    }
}