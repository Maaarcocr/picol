use std::{future::Future};

use async_task::{Runnable, Task};
use futures_lite::{future::{or, self}};
use once_cell::sync::Lazy;

pub mod fs;
mod io;

thread_local! {
    static RING: io_uring::IoUring = io_uring::IoUring::new(8).unwrap();
    static NUMBER_OF_PROCESSING: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
}

static PROCESSING_QUEUE: Lazy<(flume::Sender<Runnable>, flume::Receiver<Runnable>)> = Lazy::new(|| flume::unbounded());
static LOW_PRIORITY_QUEUE: Lazy<(flume::Sender<Runnable>, flume::Receiver<Runnable>)> = Lazy::new(|| flume::unbounded());


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
