# Backend Benchmark Report: Go vs Rust

**Date:** 2026-02-16 23:42 UTC
**Platform:** arm64 / macOS 26.2
**Go binary:** `agentmuxsrv.arm64`
**Rust binary:** `agentmuxsrv-rs`

## Binary Size

| Metric | Go | Rust | Winner |
|--------|----|----|--------|
| Binary size | 28.5 MB | 3.1 MB | Rust (9.0x smaller) |

## Build Time (incremental release)

| Metric | Go | Rust | Winner |
|--------|----|----|--------|
| Build time | 3457ms | 187ms | Rust |

## Startup Time (avg of 10 launches)

| Metric | Go | Rust | Winner |
|--------|----|----|--------|
| Startup | 59ms | 79ms | Go |

## Memory Usage (RSS)

| Metric | Go | Rust | Winner |
|--------|----|----|--------|
| Idle | 32.1 MB | 8.9 MB | Rust |
| After 500 sequential requests | 38.4 MB | 9.4 MB | Rust |
| After throughput test | 38.4 MB | 9.8 MB | Rust |

## Request Latency (500 sequential requests)

### GET / (health check, no auth)

| Percentile | Go | Rust | Winner |
|-----------|----|----|--------|
| avg | 371 µs | 369 µs | Rust |
| p50 | 367 µs | 368 µs | Go |
| p99 | 536 µs | 455 µs | Rust |
| min | 261 µs | 274 µs | Go |
| max | 740 µs | 499 µs | Rust |

### POST /wave/service (GetClientData — DB read, auth check)

| Percentile | Go | Rust | Winner |
|-----------|----|----|--------|
| avg | 500 µs | 396 µs | Rust |
| p50 | 484 µs | 393 µs | Rust |
| p99 | 884 µs | 491 µs | Rust |
| min | 365 µs | 248 µs | Rust |
| max | 1711 µs | 1156 µs | Rust |

### GET /wave/reactive/agents (no auth, no DB)

| Percentile | Go | Rust | Winner |
|-----------|----|----|--------|
| avg | 383 µs | 369 µs | Rust |
| p50 | 380 µs | 367 µs | Rust |
| p99 | 512 µs | 473 µs | Rust |
| min | 276 µs | 277 µs | Go |
| max | 654 µs | 750 µs | Go |

## Throughput (500 requests, 10 concurrent)

| Metric | Go | Rust | Winner |
|--------|----|----|--------|
| Requests/sec | 528 | 545 | Rust |
| Total time | 946ms | 916ms | Rust |

## Test Configuration

- Health: 500 sequential GET requests
- Service: 500 sequential POST requests (GetClientData — SQLite read)
- Reactive: 500 sequential GET requests (in-memory)
- Throughput: 500 total requests, 10 concurrent (xargs)
- Warmup: 50 requests before measurement
- Startup: 10 launches timed to WAVESRV-ESTART
- All timings use Python3 `time.time_ns()` for nanosecond precision
