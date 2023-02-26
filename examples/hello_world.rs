use picol::spawn;

fn main() {
    // Run the future on the current thread.
    let result = spawn(async {
        println!("Hello, world!");
        123
    });
    assert_eq!(result, 123);
}