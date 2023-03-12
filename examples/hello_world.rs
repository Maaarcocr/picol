use picol::{block_on};

fn main() {
    // Run the future on the current thread.
    block_on(async {
        let task1 = async {
            println!("Task1 start");
            let mut fs = picol::fs::file::File::open("examples/hello_world.rs").await.unwrap();
            let buf = vec![0u8; 10];
            let (_, r) = fs.read(buf).await.unwrap();
            let s = std::str::from_utf8(&r).unwrap();
            println!("Task1: {}", s);
            let mut write_file = picol::fs::file::File::create("examples/hello_world.txt").await.unwrap();
            write_file.write(r).await.unwrap();
        };
        let task2 = async {
            println!("Task2 start");
            let mut fs = picol::fs::file::File::open("examples/hello_world.rs").await.unwrap();
            let buf = vec![0u8; 10];
            let (_, r) = fs.read(buf).await.unwrap();
            let s = std::str::from_utf8(&r).unwrap();
            println!("Task2: {}", s);
            let mut write_file = picol::fs::file::File::create("examples/hello_world2.txt").await.unwrap();
            write_file.write(r).await.unwrap();
        };
        futures_lite::future::zip(task1, task2).await;
    });
}