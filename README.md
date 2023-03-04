# picol

picol (piccolo = small in italian) is a very small async runtime which wants to be
single threaded and built on top of io-uring. 

## state

picol (as of now) is mostly an experiment. I've been building it for learning purposes and I do not much
about how to best build an async runtime, nor about io-uring. 

# how it works

picol runs everything in a single thread, no background threads are used. When you want to for example read from a file, you would
call the `read` function. This in turns, create a new future, which on the first time it gets polled would spawn a new async task,
whose job would be to enqueue a new read submission to io-uring. After this, another task (with low priority) is enqueued which will wait on as many io-uring submission
as there have been. 

The idea around having a low priority task is that I think it's good to do as much CPU work before waiting for IO (the IO operations will be carried on by the kernel in the meantime, 
do not block on them to be done). Maybe it's a bad idea, but for now it's what I'm exploring!

## what can it do? 

```rust
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
```

as of now, I've only implemented reading from a file. But it can do that! And it can do that in an async way!

## thank you notes

This work could have not happened without the amazing crates created by the smol runtime team. Also, thank you to
Katharina Fey, from whom I've learn almost everything I needed to make this work, at the RustNation UK workshop in 2023.