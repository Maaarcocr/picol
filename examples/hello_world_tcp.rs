use picol::{block_on, spawn};

fn main() {
    block_on(async {
        let tcp_listener = picol::net::tcp_listener::TcpListener::bind("0.0.0.0:8080").unwrap();
        loop {
            let tcp_stream = tcp_listener.accept().await.unwrap();
            println!("tcp_stream accepted");
            spawn(async move {
                loop {
                    let buf = vec![0u8; 1024];
                    let (read, buf) = tcp_stream.read(buf).await.unwrap();
                    if read == 0 {
                        break;
                    }
                    println!("read {}", std::str::from_utf8(&buf).unwrap());
                    tcp_stream.write(buf).await.unwrap();
                }
            }).detach();
        }
    })
}