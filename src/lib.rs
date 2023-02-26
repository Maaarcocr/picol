use std::future::Future;

use futures_lite::{future::{block_on, race, pending}};

pub fn spawn<T: 'static>(future: impl Future<Output = T> + 'static) -> T {
    // Create a task that runs the future.
    let (s, r) = flume::unbounded();
    let (runnable, task) = async_task::spawn_local(future, move |runnable| {
        // Schedule the task by sending it to the queue.
        s.send(runnable).unwrap();
    });

    // Run the task.
    runnable.schedule();

    block_on(async {
        let readind_task = async {
            // Poll the queue for new tasks.
            while let Ok(runnable) = r.recv_async().await {
                // Run the task.
                runnable.run();
            }

            pending().await
        };
        let res = race(task, readind_task).await;

        res
    })
}
