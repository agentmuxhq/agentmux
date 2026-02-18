# Spec: server.rs Modularization

## Problem

`agentmuxsrv-rs/src/server.rs` is 1333 lines containing 7 unrelated concerns:

| Section | Lines | What |
|---------|-------|------|
| AppState + router | 1-116 | Router setup, CORS, route registration |
| Auth middleware | 134-169 | X-AuthKey / query param auth |
| Service dispatch | 171-665 | `POST /wave/service` — 400-line match block |
| File endpoint | 667-752 | `GET /wave/file` |
| Reactive endpoints | 754-882 | 9 handlers for `/wave/reactive/*` |
| WebSocket | 884-929 | `GET /ws` — **stub, drops all incoming RPC** |
| AI Chat | 931-969 | `POST /wave/aichat` SSE streaming |
| Docsite + Schema | 971-1048 | Static file serving |
| Tests | 1050-1333 | 13 integration tests |

This made it easy to miss that the WebSocket handler is a non-functional stub (line 920: `// pass-through for now`) — the comment that caused the blank screen bug.

## Target Structure

```
agentmuxsrv-rs/src/
  server/
    mod.rs              # AppState, build_router(), auth_middleware, health, stub_501
    service.rs          # handle_service + dispatch_service + get_object_by_oref + update_object_meta
    websocket.rs        # handle_ws + handle_ws_connection (+ future RPC wiring)
    files.rs            # handle_wave_file, handle_docsite, handle_schema, mime_from_path
    reactive.rs         # all 9 reactive handlers + query/request structs
    ai.rs               # handle_ai_chat
    tests.rs            # all integration tests (cfg(test))
```

## File Breakdown

### `server/mod.rs` (~120 lines)

Keep only the core wiring:

- `pub struct AppState` (lines 33-43)
- `pub fn build_router()` (lines 46-116) — references handlers from submodules
- `async fn health_handler()` (lines 120-125)
- `async fn stub_501()` (lines 127-132)
- `async fn auth_middleware()` (lines 137-169)
- Re-exports: `pub use self::service::*;` etc. as needed

### `server/service.rs` (~500 lines)

The HTTP service dispatch — biggest single piece:

- `async fn handle_service()` (lines 175-185)
- `fn dispatch_service()` (lines 187-592) — the 400-line match block
- `fn get_object_by_oref()` (lines 595-625)
- `fn update_object_meta()` (lines 628-665)

Takes `AppState` via Axum `State` extractor. No API changes.

### `server/websocket.rs` (~50 lines, will grow)

- `async fn handle_ws()` (lines 888-893)
- `async fn handle_ws_connection()` (lines 895-929)

This is the stub that needs RPC wiring. Having it in its own file makes the TODO impossible to miss. Future work: wire incoming messages to `rpc::engine::RpcEngine`, register handlers for `getfullconfig` and other commands.

### `server/files.rs` (~120 lines)

Static file serving:

- `struct FileQueryParams` (lines 672-677)
- `async fn handle_wave_file()` (lines 679-752)
- `async fn handle_docsite()` (lines 1018-1033)
- `async fn handle_schema()` (lines 975-1012)
- `fn mime_from_path()` (lines 1035-1048)

### `server/reactive.rs` (~130 lines)

All reactive/poller handlers + their request/query structs:

- `handle_reactive_inject` (line 758)
- `handle_reactive_agents` (line 766)
- `handle_reactive_agent` (line 778) + `AgentQuery`
- `handle_reactive_audit` (line 811) + `AuditQuery` + `default_audit_limit`
- `handle_reactive_register` (line 826) + `RegisterRequest`
- `handle_reactive_unregister` (line 848) + `UnregisterRequest`
- `handle_reactive_poller_stats` (line 856)
- `handle_reactive_poller_config` (line 869) + `PollerConfigRequest`
- `handle_reactive_poller_status` (line 877)

### `server/ai.rs` (~40 lines)

- `async fn handle_ai_chat()` (lines 935-969)

### `server/tests.rs` (~280 lines)

All 13 integration tests, gated behind `#[cfg(test)]`. Uses `test_state()` and `test_router()` helpers.

## Rules

1. **No behavior changes** — pure mechanical extraction. Every handler keeps its exact signature and logic.
2. **All handlers take `State(state): State<AppState>`** — AppState stays in `mod.rs`, handlers import it.
3. **`build_router()` stays in `mod.rs`** — it references all handler functions from submodules.
4. **Handler visibility** — handlers are `pub(super)` so `mod.rs` can reference them but they're not public API.
5. **Tests stay in one file** — `tests.rs` imports from `super::*` and keeps existing test helpers.
6. **Imports cleaned up per-file** — each file only imports what it needs.

## Migration

The old `server.rs` becomes `server/mod.rs` (Rust module convention). Git will show this as a rename + the new files as additions.

Steps:
1. `mkdir agentmuxsrv-rs/src/server/`
2. `git mv agentmuxsrv-rs/src/server.rs agentmuxsrv-rs/src/server/mod.rs`
3. Extract each section into its file
4. Add `mod` declarations and adjust imports in `mod.rs`
5. `cargo build -p agentmuxsrv-rs` — must compile with zero new warnings
6. `cargo test -p agentmuxsrv-rs` — all 13 tests must pass

## Verification

```bash
# Must pass with no regressions
cargo test -p agentmuxsrv-rs
cargo clippy -p agentmuxsrv-rs

# Line count check: total should be ~1350 (original + mod declarations)
find agentmuxsrv-rs/src/server -name "*.rs" -exec wc -l {} +
```
