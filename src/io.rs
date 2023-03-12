use std::{task::{Poll, Context}, pin::Pin, future::Future, rc::Rc};
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
        NUMBER_OF_PROCESSING.with(|n| n.fetch_sub(1, std::sync::atomic::Ordering::Relaxed));

        ring.submit_and_wait(1).unwrap();
        
        let mut cq = unsafe { ring.completion_shared() };
        let cqe = cq.next().unwrap();
        let data = cqe.user_data() as *mut (std::task::Waker, *mut Option<i32>);
        let data = unsafe { Box::from_raw(data) };
        let (waker, result) = *data;
        unsafe { *result = Some(cqe.result()) }
        unsafe { Rc::from_raw(result) };
        waker.wake();
    });
}

pub struct UringFuture {
    entry: Option<Entry>,
    result: Rc<Option<i32>>,
}

impl UringFuture {
    pub fn new(entry: Entry) -> Self {
        Self {
            entry: Some(entry),
            result: Rc::new(None),
        }
    }
}

impl Future for UringFuture {
    type Output = i32;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(entry) = self.entry.take() {
            let waker = cx.waker().clone();
            let result_ptr = Rc::into_raw(self.result.clone()) as *mut Option<i32>;

            let waker = Box::new((waker, result_ptr));
            let waker = Box::into_raw(waker);
            let read_e = entry.user_data(waker as u64);

            
            spawn(async move {
                handle_submission(read_e).await;
            }).detach();
            Poll::Pending
        } else if let Some(result) = *self.result.as_ref() {
            if result < 0 {
                // TODO: Handle error
                panic!("Error: {}", result);
            }
            Poll::Ready(result)
        } else {
            Poll::Pending
        } 
    }
}