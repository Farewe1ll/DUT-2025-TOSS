use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub async fn run() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:7878").await?;
    println!("Running Tokio async server on 127.0.0.1:7878");

    loop {
        let (mut socket, _addr) = listener.accept().await?;
        // println!("Accepted connection from {}", addr);
        tokio::spawn(async move {
            let mut buffer = [0; 512];
            let _ = socket.read(&mut buffer).await;
            let contents = "Hello, World!";
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
                contents.len(),
                contents
            );
            let _ = socket.write_all(response.as_bytes()).await;
            let _ = socket.flush().await;
        });
    }
}
