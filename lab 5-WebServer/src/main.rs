mod single_threaded;
mod multi_threaded;
mod async_tokio;

use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let usage = format!("Usage: {} [single|multi|async]", args.get(0).unwrap_or(&String::from("WebServer")));
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("single");
    let result = match mode {
        "single" => crate::single_threaded::run(),
        "multi" => crate::multi_threaded::run(),
        "async" => {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("Failed to build Tokio runtime");
            runtime.block_on(crate::async_tokio::run())
        }
        _ => {
            eprintln!("{}", usage);
            process::exit(1);
        }
    };
    if let Err(e) = result {
        eprintln!("Server error: {}", e);
        process::exit(1);
    }
}