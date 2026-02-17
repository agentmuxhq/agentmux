# Rust Backend Migration - Phased PR Plan

## Overview

Migrate from Go backend (`agentmuxsrv`) to Rust backend (`agentmuxsrv-rs`) via incremental, independently-mergeable PRs. Both backends coexist during development, switchable via env var.

---

## PR 1: Tree-Shake Dead Rust Code

**Scope**: Delete 75 files / 54,360 LOC of dead backend code, remove feature flag, remove unused deps.

### Changes

1. **Delete** entire `src-tauri/src/backend/` directory (75 .rs files)
2. **Edit** `src-tauri/src/lib.rs` line 1-2: remove `#[cfg(feature = "rust-backend")] mod backend;`
3. **Edit** `src-tauri/Cargo.toml`:
   - Remove `rust-backend` feature definition (lines 11-18)
   - Remove optional deps only used by dead code: `base64`, `dirs`, `libc`, `subtle`, `thiserror`
   - Remove `rusqlite` (will be re-added in PR 2's new crate)
4. **Keep**: `menu.rs` (event handler active at lib.rs:167), `commands/backend.rs` (bridges to Go), `tray.rs` (file exists, not mod-declared)

### Verification
- `cargo build --release -p agentmux` succeeds
- `cargo test -p agentmux` passes
- `task dev` starts, Go backend spawns, frontend connects
- Measure binary size before/after (expect ~3-5MB reduction)

---

## PR 2: Scaffold Rust Backend Binary + Backend Switching

**Scope**: New `agentmuxsrv-rs` crate with health endpoint, startup protocol, and env-var-based backend switching in `sidecar.rs`.

### Architecture

**Cargo workspace** at repo root with two members:
```
agentmux/
  Cargo.toml              # NEW workspace root
  src-tauri/Cargo.toml    # existing Tauri app (workspace member)
  agentmuxsrv-rs/         # NEW Rust backend
    Cargo.toml
    src/
      main.rs             # CLI args, WAVESRV-ESTART, stdin watch, signal handling
      server.rs           # Axum HTTP server: health check + stub routes + CORS + auth
      config.rs           # Env var / CLI arg config parsing
```

### Backend Switching

**`sidecar.rs`** reads `AGENTMUX_BACKEND` env var:
- `go` (default) -> spawns `agentmuxsrv` (current behavior)
- `rust` -> spawns `agentmuxsrv-rs`

Both emit identical `WAVESRV-ESTART ws:ADDR web:ADDR version:VER buildtime:TIME instance:ID` on stderr. Rest of sidecar.rs unchanged.

### Key Changes

| File | Change |
|------|--------|
| `Cargo.toml` (root, NEW) | Workspace definition with members |
| `src-tauri/Cargo.toml` | Add `[workspace]` membership |
| `agentmuxsrv-rs/Cargo.toml` | New crate: tokio, axum, tower-http, serde, clap, tracing |
| `agentmuxsrv-rs/src/main.rs` | Startup: CLI args, env vars, TCP listeners, ESTART emit, stdin watch |
| `agentmuxsrv-rs/src/server.rs` | Axum: `GET /` health, stub 501s for all Go routes, CORS, auth middleware |
| `agentmuxsrv-rs/src/config.rs` | Parse `WAVETERM_AUTH_KEY`, `WAVETERM_DATA_HOME`, etc. |
| `src-tauri/src/sidecar.rs:162-170` | Conditional sidecar name based on `AGENTMUX_BACKEND` |
| `src-tauri/tauri.conf.json:34-37` | Add `"binaries/agentmuxsrv-rs"` to `externalBin` |
| `Taskfile.yml` | Add `build:backend:rust` task, update `tauri:copy-sidecars` |

### Stub Routes (return 501)

All Go HTTP routes stubbed so the binary can be swapped in without 404s:
- `POST /wave/service`, `GET /wave/file`, `GET /wave/stream-file`
- `GET /ws` (WebSocket upgrade), `POST /wave/aichat`
- `/wave/reactive/*`, `/vdom/*`, `/docsite/*`, `/schema/*`

### Tests

**Rust unit tests** (`#[cfg(test)]` in each module):
- Config parsing from env vars / CLI args
- Auth middleware: rejects bad key, accepts good key
- CORS headers present on responses
- Health endpoint returns 200

**Integration tests** (`agentmuxsrv-rs/tests/`):
- Start binary as subprocess, parse WAVESRV-ESTART from stderr
- HTTP GET health check returns 200
- Unauthenticated request returns 401
- Closing stdin causes process exit
- Stub routes return 501 (not 404)

**Bi-support smoke test**:
- `AGENTMUX_BACKEND=go task dev` -> Go backend works
- `AGENTMUX_BACKEND=rust task dev` -> Rust backend starts, frontend sees health check

---

## PR 3: WebSocket + Service Endpoint

**Scope**: Implement WebSocket handler at `/ws` and RPC dispatch at `/wave/service` so frontend can connect.

### Changes
- `agentmuxsrv-rs/src/ws.rs` - WebSocket upgrade, ping/pong protocol, message routing
- `agentmuxsrv-rs/src/service.rs` - Service dispatch (stub handlers, proper error format)
- Update `server.rs` to wire real handlers instead of 501 stubs

### Tests
- WebSocket connect + ping/pong roundtrip
- Service endpoint: proper JSON error format for unknown methods
- Auth rejection on both endpoints
- **Parity test**: Same request to Go and Rust, compare response shape

---

## PR 4: SQLite Storage (waveobj CRUD)

**Scope**: Port wstore to Rust so backend can read/write the same SQLite database.

### Changes
- `agentmuxsrv-rs/src/db/mod.rs` - Connection pool (Mutex<Connection>), WAL mode
- `agentmuxsrv-rs/src/db/waveobj.rs` - CRUD: get, update, delete, list
- `agentmuxsrv-rs/src/db/migrations.rs` - Schema setup matching Go's migrations
- Add `rusqlite` dependency

### Tests
- CRUD against in-memory SQLite
- Migration creates correct schema
- **Parity test**: Write with Go, read with Rust (and vice versa)
- Concurrent read stress test

---

## PR 5: Core RPC Methods

**Scope**: Implement essential RPC methods so frontend can perform basic operations.

### Methods
- `getmeta` / `setmeta` - Read/write waveobj metadata
- `createblock` / `deleteblock` - Block lifecycle
- `resolveids` - Object reference resolution
- `eventreadhistory` / `eventpublish` - Event basics

### Tests
- Each method unit-tested against in-memory store
- End-to-end: RPC call via WebSocket, verify response
- **Parity test**: Same RPC sequence against both backends

---

## PR 6+: Feature Ports (one subsystem per PR)

| PR | Feature | Go Source | Key Rust Crate |
|----|---------|-----------|----------------|
| 6 | PTY/Shell | `pkg/blockcontroller` | `portable-pty` |
| 7 | File streaming | `pkg/filestore` | `tokio-fs` |
| 8 | Config watching | `pkg/wconfig` | `notify` |
| 9 | SSH connections | `pkg/remote` | `russh` |
| 10 | AI/LLM | `pkg/aiusechat` | `reqwest` |
| 11 | Reactive messaging | `pkg/reactive` | native channels |
| 12 | Telemetry | `pkg/telemetry` | `tracing-opentelemetry` |
| 13 | Remove Go backend | - | - |

Each PR: implement feature -> replace stub -> unit tests -> parity tests -> verify both backends.

---

## Testing Strategy

### Test Layers
1. **Rust unit tests** (`cargo test -p agentmuxsrv-rs`) - Fast, every commit
2. **Integration tests** (subprocess-based) - Start backend, test via HTTP/WS
3. **Parity tests** - Same requests to Go & Rust, compare responses
4. **Existing Go tests** (`go test ./...`) - Regression safety net
5. **Playwright E2E** (`e2e/`) - Full app with either backend

### CI Addition (`.github/workflows/tauri-build.yml`)
```yaml
- name: Build and test Rust backend
  run: |
    cargo test -p agentmuxsrv-rs
    cargo build --release -p agentmuxsrv-rs
```
