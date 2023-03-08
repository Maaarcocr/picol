use picol::{block_on};

fn main() {
    // Run the future on the current thread.
    block_on(async {
        let task1 = async {
            println!("Task1 start");
            let mut fs = picol::fs::File::open("examples/hello_world.rs").await;
            let buf = vec![0u8; 10];
            let r = fs.read(buf).await;
            let s = std::str::from_utf8(&r).unwrap();
            println!("Task1: {}", s);

        };
        let task2 = async {
            println!("Task2 start");
            let mut fs = picol::fs::File::open("examples/hello_world.rs").await;
            let buf = vec![0u8; 10];
            let r = fs.read(buf).await;
            let s = std::str::from_utf8(&r).unwrap();
            println!("Task2: {}", s);
        };
        futures_lite::future::zip(task1, task2).await;
    });
}