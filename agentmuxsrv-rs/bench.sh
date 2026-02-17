#!/usr/bin/env bash
# Benchmark: Go vs Rust backend comparison
# Measures: binary size, startup time, request latency, throughput, memory
set -euo pipefail

GO_BIN="/Users/asafebgi/agentmux/dist/bin/agentmuxsrv.arm64"
RUST_BIN="/Users/asafebgi/agentmux/target/release/agentmuxsrv-rs"
AUTH_KEY="bench-test-key-$(date +%s)"
BENCH_TMPDIR="/tmp/amx-bench-$$"
rm -rf "$BENCH_TMPDIR"
mkdir -p "$BENCH_TMPDIR"
GO_DATA_DIR="$BENCH_TMPDIR/go-data"
GO_CONFIG_DIR="$BENCH_TMPDIR/go-config"
RUST_DATA_DIR="$BENCH_TMPDIR/rust-data"
RUST_CONFIG_DIR="$BENCH_TMPDIR/rust-config"
mkdir -p "$GO_DATA_DIR" "$GO_CONFIG_DIR" "$RUST_DATA_DIR" "$RUST_CONFIG_DIR"
RESULTS_FILE="/Users/asafebgi/agentmux/agentmuxsrv-rs/BENCHMARK_REPORT.md"
WARMUP_REQUESTS=50
BENCH_REQUESTS=500
CONCURRENT=10

# Ensure binaries exist
if [[ ! -x "$GO_BIN" ]]; then echo "ERROR: Go binary not found at $GO_BIN"; exit 1; fi
if [[ ! -x "$RUST_BIN" ]]; then echo "ERROR: Rust binary not found at $RUST_BIN"; exit 1; fi

# ---- Helper functions ----

start_server() {
    local bin="$1"
    local name="$2"
    local auth="$3"
    local data_dir="$4"
    local config_dir="$5"

    # Start server with stdin kept open (sleep infinity provides non-closing stdin)
    local tmpfile=$(mktemp)
    WAVETERM_AUTH_KEY="$auth" WAVETERM_DATA_HOME="$data_dir" WAVETERM_CONFIG_HOME="$config_dir" \
        "$bin" > /dev/null 2>"$tmpfile" < <(sleep 86400) &
    local pid=$!

    # Wait for ESTART (max 10s)
    local web_addr=""
    for i in $(seq 1 100); do
        if grep -q "WAVESRV-ESTART" "$tmpfile" 2>/dev/null; then
            web_addr=$(grep "WAVESRV-ESTART" "$tmpfile" | sed 's/.*web:\([^ ]*\).*/\1/')
            break
        fi
        sleep 0.1
    done

    if [[ -z "$web_addr" ]]; then
        echo "ERROR: $name failed to start (no ESTART after 10s)"
        kill $pid 2>/dev/null || true
        cat "$tmpfile"
        rm -f "$tmpfile"
        exit 1
    fi

    rm -f "$tmpfile"
    echo "$pid $web_addr"
}

stop_server() {
    local pid="$1"
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
}

measure_startup() {
    local bin="$1"
    local name="$2"
    local data_dir="$3"
    local config_dir="$4"
    local iterations=10
    local total_ms=0

    for i in $(seq 1 $iterations); do
        local iter_data="$data_dir/startup-$i"
        local iter_config="$config_dir/startup-$i"
        mkdir -p "$iter_data" "$iter_config"
        local start_ns=$(python3 -c "import time; print(int(time.time_ns()))")
        local tmpfile=$(mktemp)
        WAVETERM_AUTH_KEY="startup-test-$i" WAVETERM_DATA_HOME="$iter_data" WAVETERM_CONFIG_HOME="$iter_config" \
            "$bin" > /dev/null 2>"$tmpfile" < <(sleep 86400) &
        local pid=$!

        # Wait for ESTART
        for j in $(seq 1 100); do
            if grep -q "WAVESRV-ESTART" "$tmpfile" 2>/dev/null; then
                break
            fi
            sleep 0.01
        done

        local end_ns=$(python3 -c "import time; print(int(time.time_ns()))")
        local elapsed_ms=$(( (end_ns - start_ns) / 1000000 ))
        total_ms=$((total_ms + elapsed_ms))

        kill $pid 2>/dev/null || true
        wait $pid 2>/dev/null || true
        rm -f "$tmpfile"
    done

    echo $((total_ms / iterations))
}

measure_latency() {
    local addr="$1"
    local auth="$2"
    local endpoint="$3"
    local method="$4"
    local body="$5"
    local count="$6"
    local label="$7"

    local timings_file=$(mktemp)

    # Collect all timings using curl -w (microsecond precision)
    for i in $(seq 1 "$count"); do
        if [[ "$method" == "GET" ]]; then
            curl -s -o /dev/null -w '%{time_total}\n' \
                "http://$addr$endpoint" -H "X-AuthKey: $auth" 2>/dev/null
        else
            curl -s -o /dev/null -w '%{time_total}\n' \
                "http://$addr$endpoint" \
                -X POST \
                -H "X-AuthKey: $auth" \
                -H "Content-Type: application/json" \
                -d "$body" 2>/dev/null
        fi
    done > "$timings_file"

    # Process timings with python (convert seconds to microseconds, compute stats)
    python3 -c "
import sys
times = []
for line in open('$timings_file'):
    line = line.strip()
    if line:
        times.append(float(line) * 1_000_000)  # seconds to µs
times.sort()
n = len(times)
if n == 0:
    print('0 0 0 0 0')
else:
    avg = int(sum(times) / n)
    mn = int(times[0])
    mx = int(times[-1])
    p50 = int(times[n // 2])
    p99 = int(times[int(n * 0.99)])
    print(f'{avg} {mn} {mx} {p50} {p99}')
"
    rm -f "$timings_file"
}

measure_memory() {
    local pid="$1"
    # RSS in KB from ps
    ps -o rss= -p "$pid" 2>/dev/null | tr -d ' '
}

measure_throughput() {
    local addr="$1"
    local auth="$2"
    local endpoint="$3"
    local body="$4"
    local count="$5"
    local concurrent="$6"

    # Use python for precise wall-clock timing
    python3 -c "
import subprocess, time, concurrent.futures

def do_request(_):
    subprocess.run([
        'curl', '-s', '-o', '/dev/null',
        '-X', 'POST',
        '-H', 'X-AuthKey: $auth',
        '-H', 'Content-Type: application/json',
        '-d', '$body',
        'http://$addr$endpoint'
    ], capture_output=True)

start = time.monotonic()
with concurrent.futures.ThreadPoolExecutor(max_workers=$concurrent) as pool:
    list(pool.map(do_request, range($count)))
elapsed_ms = int((time.monotonic() - start) * 1000)
if elapsed_ms == 0: elapsed_ms = 1
rps = $count * 1000 // elapsed_ms
print(f'{rps} {elapsed_ms}')
"
}

# ====================================================================
# Run benchmarks
# ====================================================================

echo "================================================"
echo "  Go vs Rust Backend Benchmark"
echo "  $(date)"
echo "  $(uname -m) / $(sw_vers -productVersion 2>/dev/null || uname -r)"
echo "================================================"
echo ""

# ---- 1. Binary size ----
echo "=== Binary Size ==="
GO_SIZE=$(stat -f%z "$GO_BIN")
RUST_SIZE=$(stat -f%z "$RUST_BIN")
echo "  Go:   $(echo "scale=1; $GO_SIZE / 1048576" | bc)MB ($GO_SIZE bytes)"
echo "  Rust: $(echo "scale=1; $RUST_SIZE / 1048576" | bc)MB ($RUST_SIZE bytes)"
echo "  Ratio: $(echo "scale=1; $GO_SIZE / $RUST_SIZE" | bc)x smaller (Rust)"
echo ""

# ---- 2. Build time ----
echo "=== Build Time (release, incremental) ==="
# Go
GO_BUILD_START=$(python3 -c "import time; print(int(time.time_ns()))")
(cd /Users/asafebgi/agentmux && GOOS=darwin GOARCH=arm64 go build -o /tmp/bench-agentmuxsrv ./cmd/server 2>/dev/null)
GO_BUILD_END=$(python3 -c "import time; print(int(time.time_ns()))")
GO_BUILD_MS=$(( (GO_BUILD_END - GO_BUILD_START) / 1000000 ))

# Rust
RUST_BUILD_START=$(python3 -c "import time; print(int(time.time_ns()))")
(cd /Users/asafebgi/agentmux && cargo build --release -p agentmuxsrv-rs 2>/dev/null)
RUST_BUILD_END=$(python3 -c "import time; print(int(time.time_ns()))")
RUST_BUILD_MS=$(( (RUST_BUILD_END - RUST_BUILD_START) / 1000000 ))

echo "  Go:   ${GO_BUILD_MS}ms"
echo "  Rust: ${RUST_BUILD_MS}ms"
echo ""

# ---- 3. Startup time ----
echo "=== Startup Time (avg of 10 launches) ==="
GO_STARTUP=$(measure_startup "$GO_BIN" "Go" "$BENCH_TMPDIR/go-startup" "$BENCH_TMPDIR/go-startup-cfg")
RUST_STARTUP=$(measure_startup "$RUST_BIN" "Rust" "$BENCH_TMPDIR/rust-startup" "$BENCH_TMPDIR/rust-startup-cfg")
mkdir -p "$BENCH_TMPDIR/go-startup" "$BENCH_TMPDIR/go-startup-cfg" "$BENCH_TMPDIR/rust-startup" "$BENCH_TMPDIR/rust-startup-cfg"
echo "  Go:   ${GO_STARTUP}ms"
echo "  Rust: ${RUST_STARTUP}ms"
echo ""

# ---- 4. Start servers for latency/throughput ----
echo "=== Starting servers for latency tests ==="
GO_AUTH="go-bench-key-$$"
RUST_AUTH="rust-bench-key-$$"

read GO_PID GO_ADDR <<< $(start_server "$GO_BIN" "Go" "$GO_AUTH" "$GO_DATA_DIR" "$GO_CONFIG_DIR")
echo "  Go server: PID=$GO_PID addr=$GO_ADDR"

read RUST_PID RUST_ADDR <<< $(start_server "$RUST_BIN" "Rust" "$RUST_AUTH" "$RUST_DATA_DIR" "$RUST_CONFIG_DIR")
echo "  Rust server: PID=$RUST_PID addr=$RUST_ADDR"
echo ""

# Small delay to let DBs settle
sleep 1

# ---- 5. Memory at idle ----
echo "=== Memory (RSS at idle) ==="
GO_MEM=$(measure_memory "$GO_PID")
RUST_MEM=$(measure_memory "$RUST_PID")
echo "  Go:   ${GO_MEM}KB ($(echo "scale=1; $GO_MEM / 1024" | bc)MB)"
echo "  Rust: ${RUST_MEM}KB ($(echo "scale=1; $RUST_MEM / 1024" | bc)MB)"
echo ""

# ---- 6. Warmup ----
echo "=== Warmup ($WARMUP_REQUESTS requests each) ==="
for i in $(seq 1 $WARMUP_REQUESTS); do
    curl -s -o /dev/null "http://$GO_ADDR/" 2>/dev/null
    curl -s -o /dev/null "http://$RUST_ADDR/" 2>/dev/null
done
echo "  Done"
echo ""

# ---- 7. Latency: Health (GET /) ----
echo "=== Latency: GET / (health) — $BENCH_REQUESTS requests ==="
read GO_HEALTH_AVG GO_HEALTH_MIN GO_HEALTH_MAX GO_HEALTH_P50 GO_HEALTH_P99 \
    <<< $(measure_latency "$GO_ADDR" "$GO_AUTH" "/" "GET" "" "$BENCH_REQUESTS" "health")
read RUST_HEALTH_AVG RUST_HEALTH_MIN RUST_HEALTH_MAX RUST_HEALTH_P50 RUST_HEALTH_P99 \
    <<< $(measure_latency "$RUST_ADDR" "$RUST_AUTH" "/" "GET" "" "$BENCH_REQUESTS" "health")
echo "  Go:   avg=${GO_HEALTH_AVG}µs  p50=${GO_HEALTH_P50}µs  p99=${GO_HEALTH_P99}µs  min=${GO_HEALTH_MIN}µs  max=${GO_HEALTH_MAX}µs"
echo "  Rust: avg=${RUST_HEALTH_AVG}µs  p50=${RUST_HEALTH_P50}µs  p99=${RUST_HEALTH_P99}µs  min=${RUST_HEALTH_MIN}µs  max=${RUST_HEALTH_MAX}µs"
echo ""

# ---- 8. Latency: Service (POST /wave/service — GetClientData) ----
SERVICE_BODY='{"service":"client","method":"GetClientData"}'
echo "=== Latency: POST /wave/service (GetClientData) — $BENCH_REQUESTS requests ==="
read GO_SVC_AVG GO_SVC_MIN GO_SVC_MAX GO_SVC_P50 GO_SVC_P99 \
    <<< $(measure_latency "$GO_ADDR" "$GO_AUTH" "/wave/service" "POST" "$SERVICE_BODY" "$BENCH_REQUESTS" "service")
read RUST_SVC_AVG RUST_SVC_MIN RUST_SVC_MAX RUST_SVC_P50 RUST_SVC_P99 \
    <<< $(measure_latency "$RUST_ADDR" "$RUST_AUTH" "/wave/service" "POST" "$SERVICE_BODY" "$BENCH_REQUESTS" "service")
echo "  Go:   avg=${GO_SVC_AVG}µs  p50=${GO_SVC_P50}µs  p99=${GO_SVC_P99}µs  min=${GO_SVC_MIN}µs  max=${GO_SVC_MAX}µs"
echo "  Rust: avg=${RUST_SVC_AVG}µs  p50=${RUST_SVC_P50}µs  p99=${RUST_SVC_P99}µs  min=${RUST_SVC_MIN}µs  max=${RUST_SVC_MAX}µs"
echo ""

# ---- 9. Latency: Reactive (GET /wave/reactive/agents — no auth) ----
echo "=== Latency: GET /wave/reactive/agents (no auth) — $BENCH_REQUESTS requests ==="
read GO_REACT_AVG GO_REACT_MIN GO_REACT_MAX GO_REACT_P50 GO_REACT_P99 \
    <<< $(measure_latency "$GO_ADDR" "" "/wave/reactive/agents" "GET" "" "$BENCH_REQUESTS" "reactive")
read RUST_REACT_AVG RUST_REACT_MIN RUST_REACT_MAX RUST_REACT_P50 RUST_REACT_P99 \
    <<< $(measure_latency "$RUST_ADDR" "" "/wave/reactive/agents" "GET" "" "$BENCH_REQUESTS" "reactive")
echo "  Go:   avg=${GO_REACT_AVG}µs  p50=${GO_REACT_P50}µs  p99=${GO_REACT_P99}µs  min=${GO_REACT_MIN}µs  max=${GO_REACT_MAX}µs"
echo "  Rust: avg=${RUST_REACT_AVG}µs  p50=${RUST_REACT_P50}µs  p99=${RUST_REACT_P99}µs  min=${RUST_REACT_MIN}µs  max=${RUST_REACT_MAX}µs"
echo ""

# ---- 10. Memory under load ----
echo "=== Memory (RSS after $BENCH_REQUESTS requests) ==="
GO_MEM_LOAD=$(measure_memory "$GO_PID")
RUST_MEM_LOAD=$(measure_memory "$RUST_PID")
echo "  Go:   ${GO_MEM_LOAD}KB ($(echo "scale=1; $GO_MEM_LOAD / 1024" | bc)MB)"
echo "  Rust: ${RUST_MEM_LOAD}KB ($(echo "scale=1; $RUST_MEM_LOAD / 1024" | bc)MB)"
echo ""

# ---- 11. Throughput (concurrent requests) ----
echo "=== Throughput: $BENCH_REQUESTS requests, $CONCURRENT concurrent ==="
read GO_RPS GO_TOTAL_MS <<< $(measure_throughput "$GO_ADDR" "$GO_AUTH" "/wave/service" "$SERVICE_BODY" "$BENCH_REQUESTS" "$CONCURRENT")
read RUST_RPS RUST_TOTAL_MS <<< $(measure_throughput "$RUST_ADDR" "$RUST_AUTH" "/wave/service" "$SERVICE_BODY" "$BENCH_REQUESTS" "$CONCURRENT")
echo "  Go:   ${GO_RPS} req/s (${GO_TOTAL_MS}ms total)"
echo "  Rust: ${RUST_RPS} req/s (${RUST_TOTAL_MS}ms total)"
echo ""

# ---- 12. Memory after throughput test ----
echo "=== Memory (RSS after throughput test) ==="
GO_MEM_FINAL=$(measure_memory "$GO_PID")
RUST_MEM_FINAL=$(measure_memory "$RUST_PID")
echo "  Go:   ${GO_MEM_FINAL}KB ($(echo "scale=1; $GO_MEM_FINAL / 1024" | bc)MB)"
echo "  Rust: ${RUST_MEM_FINAL}KB ($(echo "scale=1; $RUST_MEM_FINAL / 1024" | bc)MB)"
echo ""

# ---- Cleanup ----
stop_server "$GO_PID"
stop_server "$RUST_PID"
rm -f /tmp/bench-agentmuxsrv
rm -rf "$BENCH_TMPDIR"

# ---- Write report ----
cat > "$RESULTS_FILE" << REPORT_EOF
# Backend Benchmark Report: Go vs Rust

**Date:** $(date -u +"%Y-%m-%d %H:%M UTC")
**Platform:** $(uname -m) / macOS $(sw_vers -productVersion 2>/dev/null || echo "unknown")
**Go binary:** \`agentmuxsrv.arm64\`
**Rust binary:** \`agentmuxsrv-rs\`

## Binary Size

| Metric | Go | Rust | Winner |
|--------|----|----|--------|
| Binary size | $(echo "scale=1; $GO_SIZE / 1048576" | bc) MB | $(echo "scale=1; $RUST_SIZE / 1048576" | bc) MB | Rust ($(echo "scale=1; $GO_SIZE / $RUST_SIZE" | bc)x smaller) |

## Build Time (incremental release)

| Metric | Go | Rust | Winner |
|--------|----|----|--------|
| Build time | ${GO_BUILD_MS}ms | ${RUST_BUILD_MS}ms | $(if (( GO_BUILD_MS < RUST_BUILD_MS )); then echo "Go"; else echo "Rust"; fi) |

## Startup Time (avg of 10 launches)

| Metric | Go | Rust | Winner |
|--------|----|----|--------|
| Startup | ${GO_STARTUP}ms | ${RUST_STARTUP}ms | $(if (( GO_STARTUP < RUST_STARTUP )); then echo "Go"; else echo "Rust"; fi) |

## Memory Usage (RSS)

| Metric | Go | Rust | Winner |
|--------|----|----|--------|
| Idle | $(echo "scale=1; $GO_MEM / 1024" | bc) MB | $(echo "scale=1; $RUST_MEM / 1024" | bc) MB | $(if (( GO_MEM < RUST_MEM )); then echo "Go"; else echo "Rust"; fi) |
| After ${BENCH_REQUESTS} sequential requests | $(echo "scale=1; $GO_MEM_LOAD / 1024" | bc) MB | $(echo "scale=1; $RUST_MEM_LOAD / 1024" | bc) MB | $(if (( GO_MEM_LOAD < RUST_MEM_LOAD )); then echo "Go"; else echo "Rust"; fi) |
| After throughput test | $(echo "scale=1; $GO_MEM_FINAL / 1024" | bc) MB | $(echo "scale=1; $RUST_MEM_FINAL / 1024" | bc) MB | $(if (( GO_MEM_FINAL < RUST_MEM_FINAL )); then echo "Go"; else echo "Rust"; fi) |

## Request Latency (${BENCH_REQUESTS} sequential requests)

### GET / (health check, no auth)

| Percentile | Go | Rust | Winner |
|-----------|----|----|--------|
| avg | ${GO_HEALTH_AVG} µs | ${RUST_HEALTH_AVG} µs | $(if (( GO_HEALTH_AVG < RUST_HEALTH_AVG )); then echo "Go"; else echo "Rust"; fi) |
| p50 | ${GO_HEALTH_P50} µs | ${RUST_HEALTH_P50} µs | $(if (( GO_HEALTH_P50 < RUST_HEALTH_P50 )); then echo "Go"; else echo "Rust"; fi) |
| p99 | ${GO_HEALTH_P99} µs | ${RUST_HEALTH_P99} µs | $(if (( GO_HEALTH_P99 < RUST_HEALTH_P99 )); then echo "Go"; else echo "Rust"; fi) |
| min | ${GO_HEALTH_MIN} µs | ${RUST_HEALTH_MIN} µs | $(if (( GO_HEALTH_MIN < RUST_HEALTH_MIN )); then echo "Go"; else echo "Rust"; fi) |
| max | ${GO_HEALTH_MAX} µs | ${RUST_HEALTH_MAX} µs | $(if (( GO_HEALTH_MAX < RUST_HEALTH_MAX )); then echo "Go"; else echo "Rust"; fi) |

### POST /wave/service (GetClientData — DB read, auth check)

| Percentile | Go | Rust | Winner |
|-----------|----|----|--------|
| avg | ${GO_SVC_AVG} µs | ${RUST_SVC_AVG} µs | $(if (( GO_SVC_AVG < RUST_SVC_AVG )); then echo "Go"; else echo "Rust"; fi) |
| p50 | ${GO_SVC_P50} µs | ${RUST_SVC_P50} µs | $(if (( GO_SVC_P50 < RUST_SVC_P50 )); then echo "Go"; else echo "Rust"; fi) |
| p99 | ${GO_SVC_P99} µs | ${RUST_SVC_P99} µs | $(if (( GO_SVC_P99 < RUST_SVC_P99 )); then echo "Go"; else echo "Rust"; fi) |
| min | ${GO_SVC_MIN} µs | ${RUST_SVC_MIN} µs | $(if (( GO_SVC_MIN < RUST_SVC_MIN )); then echo "Go"; else echo "Rust"; fi) |
| max | ${GO_SVC_MAX} µs | ${RUST_SVC_MAX} µs | $(if (( GO_SVC_MAX < RUST_SVC_MAX )); then echo "Go"; else echo "Rust"; fi) |

### GET /wave/reactive/agents (no auth, no DB)

| Percentile | Go | Rust | Winner |
|-----------|----|----|--------|
| avg | ${GO_REACT_AVG} µs | ${RUST_REACT_AVG} µs | $(if (( GO_REACT_AVG < RUST_REACT_AVG )); then echo "Go"; else echo "Rust"; fi) |
| p50 | ${GO_REACT_P50} µs | ${RUST_REACT_P50} µs | $(if (( GO_REACT_P50 < RUST_REACT_P50 )); then echo "Go"; else echo "Rust"; fi) |
| p99 | ${GO_REACT_P99} µs | ${RUST_REACT_P99} µs | $(if (( GO_REACT_P99 < RUST_REACT_P99 )); then echo "Go"; else echo "Rust"; fi) |
| min | ${GO_REACT_MIN} µs | ${RUST_REACT_MIN} µs | $(if (( GO_REACT_MIN < RUST_REACT_MIN )); then echo "Go"; else echo "Rust"; fi) |
| max | ${GO_REACT_MAX} µs | ${RUST_REACT_MAX} µs | $(if (( GO_REACT_MAX < RUST_REACT_MAX )); then echo "Go"; else echo "Rust"; fi) |

## Throughput (${BENCH_REQUESTS} requests, ${CONCURRENT} concurrent)

| Metric | Go | Rust | Winner |
|--------|----|----|--------|
| Requests/sec | ${GO_RPS} | ${RUST_RPS} | $(if (( GO_RPS > RUST_RPS )); then echo "Go"; else echo "Rust"; fi) |
| Total time | ${GO_TOTAL_MS}ms | ${RUST_TOTAL_MS}ms | $(if (( GO_TOTAL_MS < RUST_TOTAL_MS )); then echo "Go"; else echo "Rust"; fi) |

## Test Configuration

- Health: ${BENCH_REQUESTS} sequential GET requests
- Service: ${BENCH_REQUESTS} sequential POST requests (GetClientData — SQLite read)
- Reactive: ${BENCH_REQUESTS} sequential GET requests (in-memory)
- Throughput: ${BENCH_REQUESTS} total requests, ${CONCURRENT} concurrent (xargs)
- Warmup: ${WARMUP_REQUESTS} requests before measurement
- Startup: 10 launches timed to WAVESRV-ESTART
- All timings use Python3 \`time.time_ns()\` for nanosecond precision
REPORT_EOF

echo "================================================"
echo "  Report written to: $RESULTS_FILE"
echo "================================================"
