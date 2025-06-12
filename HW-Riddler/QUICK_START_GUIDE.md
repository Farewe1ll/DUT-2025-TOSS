# HW-Riddler 快速使用指南

## 🚀 一分钟快速开始

### 安装和运行
```bash
# 1. 构建项目
cd /Users/farewe1ll/TOSS
cargo build --release

# 2. 查看帮助
Riddler --help

# 3. 发送第一个请求
Riddler request -u "https://httpbin.org/get"
```

## 📊 核心功能演示

### 🌐 HTTP请求
```bash
# GET 请求
Riddler request -u "https://api.github.com"

# POST 请求with JSON
Riddler request -m POST -u "https://httpbin.org/post" \
  -H "Content-Type:application/json" \
  -b '{"name": "test", "value": 123}'

# 带自定义头的请求
Riddler request -u "https://httpbin.org/headers" \
  -H "User-Agent:HW-Riddler/1.0" \
  -H "X-API-Key:your-key"
```

### 🍪 Cookie管理
```bash
# 添加Cookie
Riddler cookie add -c "session=abc123; Path=/; Domain=.example.com" \
  -u "https://example.com"

# 列出Cookie
Riddler cookie list

# 清理过期Cookie
Riddler cookie clean
```

### 📊 性能分析 (⭐ 核心功能)
```bash
# 快速性能检测
Riddler analyze -u "https://your-api.com/endpoint"

# 详细分析 (推荐)
Riddler analyze -u "https://your-api.com/slow-endpoint" -i 10 -r

# 6000ms+问题诊断
Riddler analyze -u "https://httpbin.org/delay/7" -i 3 -r
```

### 📋 日志和重放
```bash
# 查看最近请求
Riddler logs -l 5

# 搜索特定请求
Riddler logs -q "api.github.com"

# 重放最近请求
Riddler replay -l 3 -c 2 -d 500
```

### 🔧 代理服务器
```bash
# 启动代理
Riddler proxy -a 127.0.0.1 -p 8080

# 配置浏览器使用代理: 127.0.0.1:8080
# 然后查看拦截的流量
Riddler logs -s monitored
```

### 📦 网络监听 (需要sudo)
```bash
# 开始包监听
sudo Riddler monitor -i en0 -f "tcp port 80 or tcp port 443"

# 实时重放监听的请求
sudo Riddler monitor -i en0 --replay
```

> 监听结束后需要先按下 Ctrl + C 再输入 Q 最后按下 Enter 退出监听

## 🚨 响应问题诊断

### 典型使用场景
```bash
# 1. 快速检测是否有慢响应问题
Riddler analyze -u "https://your-slow-api.com"

# 2. 详细分析 (如果发现问题)
Riddler analyze -u "https://your-slow-api.com" -i 10 -r

# 3. 查看生成的报告
cat performance_report.json | jq .
```

### 分析结果解读

#### 性能分级
- ⚡ **优秀** (< 100ms): 无需优化
- ✅ **良好** (100-500ms): 正常表现
- ⚖️ **一般** (500-1000ms): 可接受
- ⚠️ **较差** (1000-3000ms): 需要关注
- 🚨 **关键** (> 3000ms): 急需优化

#### 常见问题和解决方案
1. **首次请求很慢，后续正常** → 冷启动效应
   - 解决: 使用连接池，启用HTTP/2

2. **所有请求都慢** → 网络/服务器问题
   - 解决: 检查网络连接，服务器负载

3. **偶尔出现超长延迟** → 网络拥塞
   - 解决: 使用CDN，优化路由

## 💡 高级使用技巧

### 1. 批量测试多个API
```bash
# 创建测试脚本
echo '#!/bin/bash
urls=(
  "https://api1.example.com"
  "https://api2.example.com"
  "https://api3.example.com"
)

for url in "${urls[@]}"; do
  echo "Testing $url..."
  Riddler analyze -u "$url" -i 5 -r
  echo "---"
done' > batch_test.sh

chmod +x batch_test.sh
./batch_test.sh
```

### 2. 性能监控脚本
```bash
# 持续监控API性能
while true; do
  Riddler analyze -u "https://critical-api.com" -i 3
  sleep 300  # 每5分钟检测一次
done
```

### 3. Cookie会话管理
```bash
# 登录并保存会话
Riddler request -m POST -u "https://site.com/login" \
  -H "Content-Type:application/json" \
  -b '{"username":"user", "password":"pass"}'

# 使用保存的Cookie访问受保护资源
Riddler request -u "https://site.com/protected"
```

## 🔍 故障排除

### 常见问题

#### 1. 网络监听权限错误
```bash
# macOS
sudo chown root:admin ./Riddler
sudo chmod +s ./Riddler

# Linux
sudo setcap cap_net_raw+ep Riddler
```

#### 2. SSL证书错误
```bash
# 跳过SSL验证 (仅测试环境)
export RIDDLER_SKIP_SSL_VERIFY=1
Riddler request -u "https://self-signed.badssl.com"
```

#### 3. 请求超时
```bash
# 增加超时时间
Riddler request -u "https://slow-site.com" -t 60
```

### 日志调试
```bash
# 启用详细日志
export RUST_LOG=debug
Riddler request -u "https://httpbin.org/get"
```

## 📊 演示脚本

```bash
cd /Users/farewe1ll/TOSS/HW-Riddler
./test_demo.sh
```

## 🎯 最佳实践

### 1. 性能分析
- 使用多轮迭代 (`-i 10`) 获得准确结果
- 生成JSON报告 (`-r`) 用于详细分析
- 关注首次请求vs后续请求的性能差异

### 2. Cookie管理
- 定期清理过期Cookie (`cookie clean`)
- 按域名组织Cookie (`cookie list -d domain.com`)

### 3. 请求重放
- 使用适当的延迟避免服务器过载 (`-d 500`)
- 限制重放次数避免影响目标服务 (`-c 2`)

### 4. 网络监听
- 使用精确的BPF过滤器减少无关流量
- 定期停止监听避免日志文件过大

## 📚 更多资源

- **完整文档**: `README.md`
- **项目完成报告**: `PROJECT_COMPLETION_REPORT.md`
- **CLI 帮助**: `Riddler --help` 和 `Riddler <command> --help`

---

**HW-Riddler - 让网络性能分析变得简单！** 🚀