# 代理配置说明

## 问题描述

项目在下载 Hugging Face 模型时会出现网络连接错误：

```
thread 'main' panicked at src/embeddings.rs:10:62:
Unable to load model: request error: https://huggingface.co/BAAI/bge-small-en-v1.5/resolve/main/config.json: Connection Failed: tls connection init failed: unexpected end of file
```

## 解决方案

本项目已配置支持代理服务器来解决 Hugging Face 模型下载问题。

### 代理配置

默认代理设置：
- **HTTP/HTTPS 代理**: `http://127.0.0.1:7897`
- **SOCKS5 代理**: `http://127.0.0.1:7897` (通过 HTTP 代理转发)

### 代理实现方式

1. **环境变量设置**: 程序启动时自动设置以下环境变量：
   - `HTTP_PROXY`
   - `HTTPS_PROXY`
   - `http_proxy`
   - `https_proxy`
   - `ALL_PROXY`
   - `all_proxy`

2. **ureq 代理支持**: 使用 `ureq` v2.12.1 的 `proxy-from-env` 功能，自动从环境变量读取代理配置。

### 相关文件

- `src/proxy_config.rs`: 代理配置模块
- `src/main.rs`: 在程序启动时初始化代理
- `src/embeddings.rs`: 使用代理下载嵌入模型
- `src/llm.rs`: 使用代理下载语言模型

### 配置选项

可以通过环境变量控制代理行为：

```bash
# 禁用代理（如果网络直连可用）
export HF_USE_PROXY=false

# 启用代理（默认）
export HF_USE_PROXY=true
```

### 测试代理配置

运行测试程序验证代理是否正常工作：

```bash
cargo run --bin test_proxy
```

成功输出示例：
```
Proxy configured: http://127.0.0.1:7897
Environment variables set:
  HTTP_PROXY: http://127.0.0.1:7897
  HTTPS_PROXY: http://127.0.0.1:7897
Testing Hugging Face API connection with proxy...
Attempting to download config.json...
✅ Successfully connected and downloaded config.json
```

### 手动代理设置

如果需要使用不同的代理地址，可以在运行程序前手动设置环境变量：

```bash
export HTTP_PROXY=http://your-proxy:port
export HTTPS_PROXY=http://your-proxy:port
cargo run -- ask "your question"
```

### 代理服务器要求

- 支持 HTTP CONNECT 方法（用于 HTTPS 流量）
- 支持标准的 HTTP 代理协议
- 能够访问 `huggingface.co` 域名

### 故障排除

1. **代理连接超时**: 检查代理服务器是否运行在 `127.0.0.1:7897`
2. **认证问题**: 当前配置不支持需要认证的代理，如需要请修改 `src/proxy_config.rs`
3. **域名解析**: 确保代理服务器能够解析 `huggingface.co`

### 技术细节

- 使用 `ureq` 2.12.1 作为 HTTP 客户端
- 通过 `hf-hub` 0.3.2 与 Hugging Face Hub 交互
- 环境变量在程序启动时设置，确保所有 HTTP 请求都使用代理
- `lazy_static` 确保模型只加载一次，避免重复下载