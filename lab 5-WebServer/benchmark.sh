#!/usr/bin/env bash
# Simple benchmark script using wrk
# Install wrk: https://github.com/wg/wrk

DURATION="5s"
THREADS=8
CONNECTIONS=2000
URL="http://127.0.0.1:7878/"

# Check if wrk is installed
if ! command -v wrk > /dev/null; then
  echo "Error: 'wrk' not found. Please install via 'brew install wrk'"
  exit 1
fi

# 增大最大文件描述符数量，防止大量并发时受限
ulimit -n 10240 || echo "Warning: unable to increase file descriptor limit"

# Determine modes to benchmark
if [[ -z "$1" || "$1" = "all" ]]; then
  modes=("single" "multi" "async")
else
  case "$1" in
    single|multi|async) modes=("$1") ;;
    *) echo "Usage: $0 [single|multi|async|all]"; exit 1 ;;
  esac
fi

# Build the project once
cargo build --release

# Benchmark each mode and collect results
results_rps=()
results_lat=()
for mode in "${modes[@]}"; do
  echo -e "\nStarting $mode server..."
  "../target/release/WebServer" $mode &
  SERVER_PID=$!
  sleep 1

  echo "Benchmarking $mode server ($DURATION, threads=$THREADS, connections=$CONNECTIONS)"
  # 加入 Keep-Alive HTTP 头
  output=$(wrk -t$THREADS -c$CONNECTIONS -d$DURATION -H "Connection: keep-alive" $URL)
  rps=$(echo "$output" | awk '/Requests\/sec/ {print $2}')
  # 提取延迟中位数和标准差
  latency=$(echo "$output" | awk '/Latency/ {print $2}')
  stdev=$(echo "$output" | awk '/Latency/ {print $4}')

  results_rps+=("$rps")
  results_lat+=("$latency ± $stdev")

  echo "Stopping $mode server (pid $SERVER_PID)"
  kill $SERVER_PID 2>/dev/null
  wait $SERVER_PID 2>/dev/null
  echo
done

# Display summary table
printf "\n%-10s %-12s %-16s\n" "Mode" "Reqs/sec" "Latency (med ± stdev)"
for i in "${!modes[@]}"; do
  printf "%-10s %-12s %-16s\n" "${modes[i]}" "${results_rps[i]}" "${results_lat[i]}"
done
