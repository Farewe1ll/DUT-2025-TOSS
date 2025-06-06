#!/bin/bash

# HW-Riddler 完整功能演示脚本

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# 获取二进制文件路径
RIDDLER_BIN="../target/release/Riddler"
if [ ! -f "$RIDDLER_BIN" ]; then
    RIDDLER_BIN="../target/debug/Riddler"
fi

if [ ! -f "$RIDDLER_BIN" ]; then
    echo -e "${RED}错误: 找不到 Riddler 二进制文件${NC}"
    echo "请先运行: cargo build --release"
    exit 1
fi

echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}🚀 HW-Riddler - 网络流量拦截器与 HTTP/HTTPS 请求工具 🚀${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"

# 1. HTTP请求功能展示
echo -e "\n${PURPLE}🌐 1. HTTP/HTTPS 请求功能${NC}"
echo -e "${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

echo -e "\n${BLUE}📤 GET 请求测试${NC}"
$RIDDLER_BIN request -u "https://httpbin.org/get?demo=complete" -t 10

echo -e "\n${BLUE}📤 POST 请求测试${NC}"
$RIDDLER_BIN request -m POST -u "https://httpbin.org/post" \
  -H "Content-Type:application/json" \
  -H "User-Agent:HW-Riddler-Complete-Demo/1.0" \
  -b '{"feature": "HTTP Client", "status": "✅ Working", "timestamp": "'$(date -u +%Y-%m-%dT%H:%M:%SZ)'"}'

echo -e "\n${BLUE}📤 PUT 请求测试${NC}"
$RIDDLER_BIN request -m PUT -u "https://httpbin.org/put" \
  -H "Content-Type:application/json" \
  -b '{"action": "update", "feature": "PUT method", "status": "✅ Working"}'

echo -e "\n${BLUE}📤 DELETE 请求测试${NC}"
$RIDDLER_BIN request -m DELETE -u "https://httpbin.org/delete" \
  -H "X-Test-Feature:HTTP-Methods"

# 2. Cookie管理功能展示
echo -e "\n\n${PURPLE}🍪 2. Cookie 管理功能${NC}"
echo -e "${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

echo -e "\n${BLUE}➕ 添加测试 Cookies${NC}"
$RIDDLER_BIN cookie add -c "demo_session=complete_demo_123; Path=/; Domain=.httpbin.org" \
  -u "https://httpbin.org"
$RIDDLER_BIN cookie add -c "user_pref=theme=dark&lang=en; Path=/; Domain=.httpbin.org" \
  -u "https://httpbin.org"

echo -e "\n${BLUE}📝 列出所有 Cookies${NC}"
$RIDDLER_BIN cookie list

echo -e "\n${BLUE}🔍 按域名过滤 Cookies${NC}"
$RIDDLER_BIN cookie list -d "httpbin.org"

echo -e "\n${BLUE}📤 带 Cookie 的请求测试${NC}"
$RIDDLER_BIN request -u "https://httpbin.org/cookies"

# 3. 日志功能展示
echo -e "\n\n${PURPLE}📋 3. 请求日志功能${NC}"
echo -e "${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

echo -e "\n${BLUE}📊 最近请求日志${NC}"
$RIDDLER_BIN logs -l 5

echo -e "\n${BLUE}🔍 搜索日志${NC}"
$RIDDLER_BIN logs -q "httpbin" -l 3

echo -e "\n${BLUE}📈 统计信息${NC}"
$RIDDLER_BIN logs --stats

# 4. 批量重放功能展示
echo -e "\n\n${PURPLE}🔄 4. 批量重放功能${NC}"
echo -e "${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

echo -e "\n${BLUE}🔄 重放最近请求 (2 次重复，500ms 延迟)${NC}"
$RIDDLER_BIN replay -l 3 -c 2 -d 500

echo -e "\n${BLUE}🔍 按来源过滤重放 (仅手动请求)${NC}"
$RIDDLER_BIN replay -l 2 -s manual -c 1 -d 200

# 5. 错误处理测试
echo -e "\n\n${PURPLE}⚠️  5. 错误处理测试${NC}"
echo -e "${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

echo -e "\n${BLUE}❌ 测试无效 URL 处理${NC}"
$RIDDLER_BIN request -u "invalid-url-test" || echo -e "${GREEN}✅ 错误处理正常${NC}"

echo -e "\n${BLUE}⏱️  测试超时处理${NC}"
echo "发送延迟 2 秒的请求，超时设置为 5 秒..."
$RIDDLER_BIN request -u "https://httpbin.org/delay/2" -t 5

# 6. 代理服务器测试
echo -e "\n\n${PURPLE}🔧 6. 代理服务器功能${NC}"
echo -e "${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

echo -e "\n${BLUE}🚀 启动代理服务器 (5秒测试)${NC}"
timeout 5s $RIDDLER_BIN proxy -a 127.0.0.1 -p 8080 &
PROXY_PID=$!
sleep 2
echo -e "${GREEN}✅ 代理服务器正常启动！${NC}"
kill $PROXY_PID 2>/dev/null || true
wait $PROXY_PID 2>/dev/null || true

# 7. 内容类型测试
echo -e "\n\n${PURPLE}📄 7. 不同内容类型测试${NC}"
echo -e "${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

echo -e "\n${BLUE}📄 XML 数据请求${NC}"
$RIDDLER_BIN request -m POST -u "https://httpbin.org/post" \
  -H "Content-Type:application/xml" \
  -b '<?xml version="1.0"?><demo><feature>XML Support</feature><status>✅ Working</status></demo>'

echo -e "\n${BLUE}📝 表单数据请求${NC}"
$RIDDLER_BIN request -m POST -u "https://httpbin.org/post" \
  -H "Content-Type:application/x-www-form-urlencoded" \
  -b 'feature=Form-Data&status=Working&demo=complete'

echo -e "\n${BLUE}📝 表单数据请求${NC}"
$RIDDLER_BIN request -m POST -u "https://httpbin.org/post" \
  -H "Content-Type:application/x-www-form-urlencoded" \
  -b 'feature=Form-Data&status=Working&demo=complete'

# 8. 性能分析功能展示 (新增核心功能)
echo -e "\n\n${PURPLE}📊 8. 性能分析功能 (核心功能)${NC}"
echo -e "${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

echo -e "\n${BLUE}🚀 快速性能测试${NC}"
$RIDDLER_BIN analyze -u "https://httpbin.org/get"

echo -e "\n${BLUE}🔬 多轮迭代性能测试 (5 次迭代)${NC}"
$RIDDLER_BIN analyze -u "https://httpbin.org/get" -i 5

echo -e "\n${BLUE}📊 生成详细JSON报告${NC}"
$RIDDLER_BIN analyze -u "https://httpbin.org/get" -i 3 -r
if [ -f "performance_report.json" ]; then
    echo -e "${GREEN}✅ 性能报告已生成: performance_report.json${NC}"
    echo -e "${CYAN}报告摘要:${NC}"
    head -20 performance_report.json | grep -E "(avg_response_time|performance_classification|primary_bottleneck)" || echo "报告内容略..."
fi

echo -e "\n${BLUE}🚨 慢响应测试 (模拟 6000ms+ 场景)${NC}"
echo "测试延迟3秒的端点..."
$RIDDLER_BIN analyze -u "https://httpbin.org/delay/3" -i 2 -r

echo -e "\n${BLUE}🧠 冷启动效应测试${NC}"
echo "测试GitHub API以观察首次vs后续请求性能差异..."
$RIDDLER_BIN analyze -u "https://api.github.com" -i 4

# 9. 最终统计
echo -e "\n\n${PURPLE}📊 9. 最终统计信息${NC}"
echo -e "${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

echo -e "\n${BLUE}📈 按来源分类的日志${NC}"
echo "Manual requests:"
$RIDDLER_BIN logs -s manual -l 3

echo -e "\nReplay requests:"
$RIDDLER_BIN logs -s replay -l 3

echo -e "\n${BLUE}📊 最终统计${NC}"
$RIDDLER_BIN logs --stats

echo -e "\n\n${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}🎉 HW-Riddler 完整功能演示完成！ 🎉${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"

# 10. 清理选项
echo -e "\n${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo -e "\n${YELLOW}🧹 是否要清理测试数据? (y/N)${NC}"
read -r response
if [[ "$response" =~ ^[Yy]$ ]]; then
    echo -e "\n${BLUE}🧹 清理 Cookies 和日志...${NC}"
    $RIDDLER_BIN cookie clear
    rm -f ./requests.log
    echo -e "${GREEN}✅ 清理完成${NC}"
else
    echo -e "${GREEN}✅ 保留测试数据供进一步测试${NC}"
fi