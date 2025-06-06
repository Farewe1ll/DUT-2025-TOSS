use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
// use std::fs;
use std::thread;

fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; 512];
    let _ = stream.read(&mut buffer);
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
    println!("Running multi-threaded server on 127.0.0.1:7878");

    for stream in listener.incoming() {
        let stream = stream?;
        thread::spawn(|| {
            handle_connection(stream);
        });
    }
    Ok(())
}
