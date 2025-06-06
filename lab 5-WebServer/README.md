# WebServer

This project demonstrates three simple Rust HTTP server implementations:

1. Single-threaded server (`single_threaded.rs`)
2. Multi-threaded server (`multi_threaded.rs`)
3. Asynchronous Tokio-based server (`async_tokio.rs`)

## Requirements
- Rust toolchain (stable)
- `wrk` for benchmarking (https://github.com/wg/wrk)

## Build

```bash
cd lab\ 5-WebServer
cargo build --release
```

## Run

You can choose which version to run by passing an argument:

```bash
# Single-threaded
WebServer single

# Multi-threaded
WebServer multi

# Tokio async
WebServer async
```

## Benchmarking

A helper script `benchmark.sh` is provided to run `wrk` against each server:

```bash
chmod +x benchmark.sh
./benchmark.sh single
```

Options:
- `single`, `multi`, `async`, or `all`: which server to start
- Duration: 10s, 4 threads, 100 connections (configurable in the script)

The script will:
1. Build in release mode
2. Launch the server in background
3. Wait 1s for startup
4. Run `wrk` and report requests/sec and latency
5. Stop the server

## Initial Performance Comparison

Run benchmarks for each version:

```bash
./benchmark.sh single
./benchmark.sh multi
./benchmark.sh async
```

Collect and compare the output `Requests/sec`. You should observe:
- Multi-threaded and Tokio async versions can handle higher concurrency than single-threaded.
- Tokio async may show the best performance under high load.