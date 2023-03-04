use std::time::Duration;

use picol::{read, block_on};

fn main() {
    // Run the future on the current thread.
    block_on(async {
        let task1 = async {
            println!("Task1 start");
            let f = std::fs::File::open("examples/hello_world.rs").unwrap();
            async_io::Timer::after(Duration::from_secs(1)).await;

            let r = read(&f, 0, 10).await;
            let s = std::str::from_utf8(&r).unwrap();
            println!("Task1: {}", s);

        };
        let task2 = async {
            println!("Task2 start");
            let f = std::fs::File::open("examples/hello_world.rs").unwrap();
            let r = read(&f, 10, 10).await;
            let s = std::str::from_utf8(&r).unwrap();
            println!("Task2: {}", s);
        };
        futures_lite::future::zip(task1, task2).await;
    });
}