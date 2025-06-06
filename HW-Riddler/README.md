# HW-Riddler - 网络流量拦截器和HTTP/HTTPS请求工具

HW-Riddler是一个全面的网络流量分析和HTTP/HTTPS请求工具，提供以下功能：

## 功能特性

### 🌐 网络流量捕获
- 本地网络数据包捕获和拦截
- HTTP/HTTPS请求解析
- 可配置的数据包过滤器
- 实时流量监控

### 🍪 Cookie管理
- 本地Cookie缓存和管理
- Cookie持久化存储
- 过期Cookie自动清理
- 域名过滤支持

### 🚀 HTTP/HTTPS客户端
- 支持所有常见HTTP方法 (GET, POST, PUT, DELETE, etc.)
- 自定义请求头支持
- 请求体和参数配置
- SSL/TLS验证选项
- 自动重定向处理

### 📝 请求日志记录
- 详细的请求/响应日志记录 on
- JSON格式持久化存储
- 请求重放功能
- 日志搜索和过滤
- 请求统计分析

### 📊 性能分析 (核心功能)
- **6000ms+慢响应专项诊断** - 专门识别和分析极慢的HTTP响应
- **多轮迭代性能测试** - 通过多次请求获得统计准确性
- **智能性能瓶颈识别** - 自动识别DNS解析、TCP握手、网络延迟等问题
- **5级性能分类系统** - 优秀(<100ms)/良好(100-500ms)/一般(500-1000ms)/较差(1000-3000ms)/关键(>3000ms)
- **冷启动效应检测** - 识别首次请求vs后续请求的性能差异
- **详细优化建议生成** - 基于分析结果提供具体的性能优化建议
- **JSON格式详细报告** - 生成结构化的性能分析报告
- **实时性能监控** - 实时显示请求进度和初步结果

### 🔄 代理服务器
- HTTP/HTTPS代理服务器
- CONNECT方法支持（HTTPS隧道）
- 流量转发和拦截

## 安装

```bash
# 克隆仓库
git clone <repository-url>
cd HW-Riddler

# 构建项目
cargo build --release

# 运行
./target/release/Riddler --help
```

## 快速开始

### 查看帮助信息

```bash
# 查看主命令帮助
./Riddler --help

# 查看子命令帮助
./Riddler request --help    # HTTP请求参数
./Riddler cookie --help     # Cookie管理参数
./Riddler capture --help    # 网络捕获参数
./Riddler logs --help       # 日志查看参数
./Riddler replay --help     # 请求重放参数
./Riddler proxy --help      # 代理服务器参数
./Riddler analyze --help    # 性能分析参数
```

## 命令参数详解

### 🌐 HTTP请求 (request)
```bash
./Riddler request [选项]
  -m, --method <METHOD>     HTTP方法 (GET, POST, PUT, DELETE, PATCH) [默认: GET]
  -u, --url <URL>          目标URL (必需)
  -H, --headers <HEADER>   自定义请求头 (格式: "Name:Value")
  -b, --body <BODY>        请求体内容
  -t, --timeout <SECONDS>  超时时间(秒) [默认: 30]
```

### 🍪 Cookie管理 (cookie)
```bash
./Riddler cookie <子命令>
  list                     列出所有Cookie
    -d, --domain <DOMAIN>  按域名过滤
  add                      添加Cookie
    -c, --cookie <COOKIE>  Cookie字符串 (必需)
    -u, --url <URL>        关联URL (必需)
  clean                    清理过期Cookie
  clear                    清除所有Cookie
```

### 📦 网络捕获 (capture)
```bash
./Riddler capture [选项]
  -i, --interface <IF>     网络接口 [默认: en0]
  -f, --filter <FILTER>    BPF过滤器 [默认: "tcp port 80 or tcp port 443"]
  -r, --replay             启用自动重放
```

### 📋 日志查看 (logs)
```bash
./Riddler logs [选项]
  -l, --limit <NUMBER>     显示条数 [默认: 10]
  -s, --source <SOURCE>    按来源过滤 (captured/manual/replay)
  -q, --query <QUERY>      搜索关键词
      --stats              显示统计信息
```

### 🔄 请求重放 (replay)
```bash
./Riddler replay [选项]
  -l, --limit <NUMBER>     重放请求数 [默认: 1]
  -s, --source <SOURCE>    按来源过滤 (captured/manual)
  -c, --count <COUNT>      每个请求重复次数 [默认: 1]
  -d, --delay <MS>         重放间隔(毫秒) [默认: 100]
```

### 🔧 代理服务器 (proxy)
```bash
./Riddler proxy [选项]
  -a, --address <ADDR>     绑定地址 [默认: 127.0.0.1]
  -p, --port <PORT>        端口号 [默认: 8080]
```

### 📊 性能分析 (analyze)
```bash
./Riddler analyze [选项]
  -u, --url <URL>          分析目标URL (必需)
  -i, --iterations <NUM>   测试迭代次数 [默认: 5]
  -r, --report             生成JSON报告
```

## 使用示例

### 1. 发送HTTP请求

```bash
# 简单GET请求
./Riddler request -u "https://httpbin.org/get"

# POST请求with JSON数据
./Riddler request -m POST -u "https://httpbin.org/post" \
  -H "Content-Type:application/json" \
  -b '{"key": "value"}'

# 带自定义头的请求
./Riddler request -u "https://httpbin.org/headers" \
  -H "User-Agent:HW-Riddler/1.0" \
  -H "X-Custom-Header:test-value"
```

### 2. Cookie管理

```bash
# 添加Cookie
./Riddler cookie add -c "sessionid=abc123; Path=/; Domain=.example.com" \
  -u "https://example.com"

# 列出所有Cookie
./Riddler cookie list

# 按域名过滤Cookie
./Riddler cookie list -d "example.com"

# 清理过期Cookie
./Riddler cookie clean

# 清除所有Cookie
./Riddler cookie clear
```

### 3. 网络流量捕获

```bash
# 开始网络捕获 (需要root权限)
sudo ./Riddler capture -i en0 -f "tcp port 80 or tcp port 443"

# 启用请求重放
sudo ./Riddler capture -i en0 --replay

# 自定义过滤器
sudo ./Riddler capture -i en0 -f "host www.example.com"
```

### 4. 查看请求日志

```bash
# 查看最近10条日志记录
./Riddler logs

# 查看最近50条记录
./Riddler logs -l 50

# 按来源过滤 (captured/manual/replay)
./Riddler logs -s manual

# 搜索特定内容
./Riddler logs -q "httpbin.org"

# 显示请求统计
./Riddler logs --stats
```

### 5. 性能分析 (核心功能)

```bash
# 🚨 快速性能检测 (推荐用法)
./Riddler analyze -u "https://api.example.com"

# 🔬 高精度多轮测试 (获得统计准确性)
./Riddler analyze -u "https://httpbin.org/get" -i 10

# 📊 生成详细JSON报告 (用于深度分析)
./Riddler analyze -u "https://slow-endpoint.com" -i 5 -r

# 🚨 专项慢响应诊断 (6000ms+问题分析)
./Riddler analyze -u "https://httpbin.org/delay/7" -i 3 -r

# 🔍 冷启动效应测试 (首次vs后续请求对比)
./Riddler analyze -u "https://api.github.com" -i 8 -r
```

**🎯 性能分析核心特色:**
- **🚨 6000ms+关键问题检测**: 自动识别超长响应时间并提供专项诊断
- **🧠 智能瓶颈分析**: 区分DNS解析、TCP握手、服务器处理、网络传输延迟
- **📈 统计级性能评估**: 通过多轮测试消除偶然性，获得可靠性能数据
- **❄️ 冷启动效应识别**: 检测首次请求与后续请求的性能差异
- **🎯 精准优化建议**: 基于实际测试结果提供具体的性能优化方案

**🔬 性能分类标准:**
- ⚡ **优秀** (< 100ms): 极快响应，用户体验佳
- ✅ **良好** (100-500ms): 正常响应速度
- ⚖️ **一般** (500-1000ms): 可接受的响应时间
- ⚠️ **较差** (1000-3000ms): 响应偏慢，需要优化
- 🚨 **关键** (> 3000ms): 严重性能问题，急需处理

**📋 生成的分析报告包含:**
- 详细的响应时间统计 (平均值、最小值、最大值、标准差)
- 性能瓶颈识别和原因分析
- 冷启动效应检测结果
- 具体的优化建议和解决方案
- 与行业标准的性能对比

### 6. 启动代理服务器

```bash
# 启动默认代理 (127.0.0.1:8080)
./Riddler proxy

# 自定义地址和端口
./Riddler proxy -a 0.0.0.0 -p 3128
```

## 配置

默认配置包括：

- Cookie存储路径: `./cookies.json`
- 请求日志路径: `./requests.log`
- 默认网络接口: `en0`
- 默认代理端口: `8080`

## 系统要求

### macOS
- 网络捕获功能需要root权限
- 需要安装libpcap开发库

### Linux
- 网络捕获功能需要root权限或CAP_NET_RAW capability
- 需要安装libpcap-dev

### 依赖库
- Rust 1.70+
- libpcap
- OpenSSL

## 示例场景

### 性能分析深度案例

```bash
# 🎯 真实场景: 诊断生产环境API慢响应问题
./Riddler analyze -u "https://production-api.company.com/heavy-endpoint" -i 10 -r

# 📊 结果分析:
# - 首次请求: 8316ms (冷启动)
# - 后续请求: ~300ms (缓存生效)
# - 识别出DNS解析(1200ms) + TCP握手(800ms) + TLS握手(2100ms) = 主要延迟
# - 优化建议: 使用CDN、启用HTTP/2、优化DNS解析

# 🔍 A/B测试不同地域的API性能
./Riddler analyze -u "https://us-east-api.service.com/endpoint" -i 5 -r
./Riddler analyze -u "https://eu-west-api.service.com/endpoint" -i 5 -r

# 🚨 模拟超时场景测试
./Riddler analyze -u "https://httpbin.org/delay/8" -i 3 -r
# 预期结果: 检测到关键性能问题(>8000ms)，建议增加超时设置
```

### Web应用测试
```bash
# 1. 启动代理服务器
./Riddler proxy -p 8080

# 2. 配置浏览器使用代理 (127.0.0.1:8080)
# 3. 查看拦截的流量
./Riddler logs -s captured

# 4. 重放特定请求
./Riddler logs -q "login" | # 找到登录请求
./Riddler capture --replay  # 重放请求
```

### API开发和调试
```bash
# 测试API端点
./Riddler request -m POST -u "http://localhost:3000/api/users" \
  -H "Content-Type:application/json" \
  -H "Authorization:Bearer token123" \
  -b '{"name": "John", "email": "john@example.com"}'

# 查看请求详情
./Riddler logs -l 1
```

### 网络流量分析
```bash
# 捕获特定域名的流量
sudo ./Riddler capture -f "host api.example.com"

# 分析捕获的数据
./Riddler logs --stats
./Riddler logs -s captured -q "api.example.com"
```

## 安全注意事项

⚠️ **重要安全提醒:**

1. **网络捕获**: 需要管理员权限，请谨慎使用
2. **Cookie存储**: Cookie以明文形式存储，请保护文件安全
3. **代理服务器**: 仅用于测试环境，不建议生产环境使用
4. **SSL验证**: 默认启用SSL验证，可根据需要配置
5. **数据隐私**: 请求日志可能包含敏感信息，请妥善保管

## 故障排除

### 网络捕获权限问题
```bash
# macOS
sudo chown root:admin /path/to/Riddler
sudo chmod +s /path/to/Riddler

# Linux
sudo setcap cap_net_raw+ep /path/to/Riddler
```

### 编译问题
```bash
# 安装依赖 (macOS)
brew install libpcap

# 安装依赖 (Ubuntu/Debian)
sudo apt-get install libpcap-dev

# 安装依赖 (CentOS/RHEL)
sudo yum install libpcap-devel
```

## 贡献

欢迎提交Issue和Pull Request来改进这个项目！

## 许可证

本项目采用MIT许可证 - 详见 [LICENSE](LICENSE) 文件。
