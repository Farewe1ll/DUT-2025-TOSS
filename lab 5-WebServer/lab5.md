# 第5次实验报告 - 基于Rust的Web服务器实现

## 1 实验原理

- Web 服务器通过 TCP 监听客户端请求，根据 HTTP 协议读取并解析请求报文，然后构造 HTTP 响应并发送。
- 单线程模式：主线程循环接收并处理连接，无法并发处理多个请求。
- 多线程模式：对每个连接`thread::spawn`一个线程并处理，提高并发能力，但线程切换开销较大。
- 异步Tokio模式：基于事件驱动和协程，使用单一或少量线程处理大量连接，性能和资源利用率更优。

## 2 实验内容与步骤

### 2.1 单线程版实现

- 文件：`src/single_threaded.rs`
- 使用 `TcpListener::bind` 监听端口
- 主线程 `for stream in listener.incoming()` 循环接收连接
- 每次阻塞读取并处理请求，返回固定 `Hello, World!` 响应

### 2.2 多线程版实现

- 文件：`src/multi_threaded.rs`
- 同样监听端口，但对每个 `TcpStream` 调用 `thread::spawn` 新线程处理
- 主线程快速返回继续接收新连接

### 2.3 异步 Tokio 版实现

- 文件：`src/async_tokio.rs`
- 使用 `tokio::net::TcpListener::bind().await` 监听
- 在循环中 `listener.accept().await` 接收连接，并 `tokio::spawn` 异步任务
- 使用 `AsyncReadExt`/`AsyncWriteExt` 非阻塞读写

### 2.4 统一入口

- `src/main.rs` 根据命令行参数 (`single|multi|async`) 选择对应版本运行

## 3 性能测试

### 3.1 测试方法

使用编写的脚本 `benchmark.sh`：

```bash
chmod +x benchmark.sh
./benchmark.sh all
```

- 并发线程数：8
- 连接数：2000
- 持续时间：5s

注意到测试可能由以下几个因素影响：

1. 单一 `"Hello, World!"` 响应过小
  - 响应报文只有几十字节，网络和系统调用的开销就占了绝大部分时间。

2. 每个请求都 `flush()`
  - 在同步和异步版本里，每次写完都显式调用 `flush()`，会强制把缓冲区刷到底层套接字。频繁 `flush` 会大幅拉低吞吐。

除以上影响因素外，我加入以下优化减少其他因素影响：

1. 系统资源限制
  - macOS 默认每个进程的文件描述符数有限，并发很高时可能被内核限流，通过 `ulimit -n 10240` 提高打开 `socket` 的上限。

2. `HTTP Keep-Alive` 与管道化
  - `wrk` 调用增加 `Connection: keep-alive` 头，减少握手开销。

### 5.2 测试结果

|   模式   | Requests/sec | Latency (med ± stdev) |
| :----: | :----------: | :-------------------: |
| single |    11.75     |   38.41ms ± 64.91ms   |
| multi  |    30.29     |  136.28ms ± 151.76ms  |
| async  |    132.85    |   47.42ms ± 61.04ms   |

可以看出 虽然有部分影响因素未消除，但是实验结果基本符合预期。

### 5.3 性能分析

- 单线程版在高并发下饱和严重，吞吐量最低。
- 多线程版利用多核优势，明显提升吞吐量，但线程切换开销与内存占用较大。
- `Tokio` 异步版在相同硬件与参数下性能最佳，可在单或少量线程内高效处理大量连接。