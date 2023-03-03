use std::time::Duration;

use picol::{spawn, read};

fn main() {
    // Run the future on the current thread.
    spawn(async {
        let task1 = async {
            println!("Task1 start");
            let f = std::fs::File::open("examples/hello_world.rs").unwrap();
            let r = read(f, 0, 10).await;
            let s = std::str::from_utf8(&r).unwrap();
            async_std::task::sleep(Duration::from_secs(5)).await;
            println!("Task1: {}", s);
        };
        let task2 = async {
            println!("Task2 start");
            let f = std::fs::File::open("examples/hello_world.rs").unwrap();
            let r = read(f, 10, 10).await;
            let s = std::str::from_utf8(&r).unwrap();
            println!("Task2: {}", s);
        };
        futures_lite::future::zip(task1, task2).await;
    });
}