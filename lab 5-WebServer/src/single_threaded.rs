use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;
// use std::fs;

fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; 512];
    // Read request (not parsed)
    let _ = stream.read(&mut buffer);
    // Simple HTTP response
    let contents = "Hello, World!";
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
        contents.len(),
        contents
    );
    let _ = stream.write(response.as_bytes());
    let _ = stream.flush();
}

pub fn run() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:7878")?;
    println!("Running single-threaded server on 127.0.0.1:7878");

    for stream in listener.incoming() {
        let stream = stream?;
        handle_connection(stream);
    }
    Ok(())
}
