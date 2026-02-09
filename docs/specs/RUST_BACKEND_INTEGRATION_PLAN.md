# Rust Backend Integration Plan — Replacing the Go Sidecar

**Version:** 0.20.4
**Date:** 2026-02-09
**Lead:** AgentA
**Status:** Planning
**Prerequisite:** Phase 17 Go-to-Rust migration complete (all modules ported)

---

## Executive Summary

WaveMux currently runs a **dual architecture**: Tauri (Rust) launches a Go sidecar (`wavemuxsrv`) that provides the actual backend services (WebSocket RPC, SQLite storage, terminal PTY, AI, SSH). The frontend connects to the Go backend via WebSocket/HTTP.

The Go-to-Rust migration (Phases 9-17) has ported all Go `pkg/` modules to Rust `src-tauri/src/backend/`. However, these Rust modules are **standalone libraries with tests** — they are not yet wired into the running application. The Go sidecar still handles 100% of production traffic.

**Goal:** Replace the Go sidecar with the native Rust backend, eliminating the external process and achieving:
- **Zero sidecar startup latency** (no WAVESRV-ESTART wait)
- **~25MB smaller binary** (no bundled Go executable)
- **Single-process architecture** (Tauri + backend in one binary)
- **Shared memory** between frontend IPC and backend (no WebSocket serialization)
- **Simplified deployment** (one binary, no sidecar management)

---

## Current Architecture (Go Sidecar)

```
┌──────────────────────────────────┐
│  Tauri App (Rust)                │
│  ┌────────────────────────────┐  │
│  │ React Frontend (webview)   │  │
│  │  - xterm.js terminals      │  │
│  │  - AI chat, file preview   │  │
│  │  - wos.ts (WaveObj Store)  │  │
│  └─────────┬──────────────────┘  │
│            │ WebSocket + HTTP     │
│  ┌─────────▼──────────────────┐  │
│  │ tauri-api.ts shim          │  │
│  │  - invoke() for IPC        │  │
│  │  - WebSocket for RPC       │  │
│  └─────────┬──────────────────┘  │
│            │                      │
│  ┌─────────▼──────────────────┐  │
│  │ Tauri IPC (commands/)      │  │  Platform commands, auth,
│  │  - 30+ commands            │  │  window mgmt, stubs
│  │  - 16 are STUBS            │  │
│  └────────────────────────────┘  │
└──────────┬───────────────────────┘
           │ child_process (sidecar)
           ▼
┌──────────────────────────────────┐
│  wavemuxsrv (Go binary, ~25MB)  │
│  - WebSocket RPC server          │ ◄── Frontend connects here
│  - HTTP API server               │
│  - SQLite database (wstore)      │
│  - Terminal PTY management       │
│  - SSH/WSL remote connections    │
│  - AI provider integration       │
│  - wsh shell integration         │
│  - File operations               │
│  - Event bus (pub/sub)           │
└──────────────────────────────────┘
```

### Key Observation

The frontend talks to the Go backend via **two channels**:
1. **WebSocket** — JSON-RPC for all commands (`wos.ts` → `wshrpc`)
2. **HTTP** — File uploads/downloads, some REST endpoints

The Tauri IPC layer (`tauri-api.ts` → `commands/`) only handles platform-level operations (window management, auth key, platform info). All business logic goes through the Go WebSocket.

---

## Target Architecture (Rust-Native Backend)

```
┌──────────────────────────────────────────────┐
│  WaveMux (Single Tauri Binary)               │
│  ┌────────────────────────────────────────┐  │
│  │ React Frontend (webview)               │  │
│  │  - xterm.js terminals                  │  │
│  │  - AI chat, file preview               │  │
│  │  - wos.ts (WaveObj Store)              │  │
│  └─────────┬──────────────────────────────┘  │
│            │ Tauri IPC (invoke + events)      │
│  ┌─────────▼──────────────────────────────┐  │
│  │ tauri-api.ts (UPDATED)                 │  │
│  │  - invoke() for ALL commands           │  │
│  │  - Tauri events replace WebSocket      │  │
│  │  - No more external WS/HTTP            │  │
│  └─────────┬──────────────────────────────┘  │
│            │                                  │
│  ┌─────────▼──────────────────────────────┐  │
│  │ Tauri Commands (EXPANDED)              │  │
│  │  - Platform commands (existing)        │  │
│  │  - RPC commands (NEW - replace stubs)  │  │
│  │  - File commands (NEW)                 │  │
│  │  - Terminal commands (NEW)             │  │
│  └─────────┬──────────────────────────────┘  │
│            │                                  │
│  ┌─────────▼──────────────────────────────┐  │
│  │ Rust Backend (IN-PROCESS)              │  │
│  │  ┌──────────┐  ┌───────────────────┐   │  │
│  │  │ WaveStore│  │ RPC Engine        │   │  │
│  │  │ (SQLite) │  │ + Router          │   │  │
│  │  └──────────┘  └───────────────────┘   │  │
│  │  ┌──────────┐  ┌───────────────────┐   │  │
│  │  │ WCore    │  │ BlockController   │   │  │
│  │  │ (CRUD)   │  │ (Terminal/Shell)  │   │  │
│  │  └──────────┘  └───────────────────┘   │  │
│  │  ┌──────────┐  ┌───────────────────┐   │  │
│  │  │ AI       │  │ Remote/SSH/WSL    │   │  │
│  │  │ (LLM)    │  │ (Connections)     │   │  │
│  │  └──────────┘  └───────────────────┘   │  │
│  │  ┌──────────┐  ┌───────────────────┐   │  │
│  │  │ EventBus │  │ wshutil           │   │  │
│  │  │ (PubSub) │  │ (OSC transport)   │   │  │
│  │  └──────────┘  └───────────────────┘   │  │
│  └────────────────────────────────────────┘  │
└──────────────────────────────────────────────┘
```

---

## Prerequisite Analysis

### What Exists in Rust (Ready)

| Module | Location | Lines | Status | Go Equivalent |
|--------|----------|-------|--------|---------------|
| `waveobj` | `backend/waveobj.rs` | ~400 | Complete | `pkg/waveobj/` |
| `storage/wstore` | `backend/storage/wstore.rs` | ~500 | Complete | `pkg/wstore/` |
| `storage/filestore` | `backend/storage/filestore.rs` | ~300 | Complete | `pkg/filestore/` |
| `wcore` | `backend/wcore.rs` | ~680 | Complete | `pkg/wcore/` |
| `wps` (event broker) | `backend/wps.rs` | ~300 | Complete | `pkg/wps/` |
| `eventbus` | `backend/eventbus.rs` | ~200 | Complete | `pkg/eventbus/` |
| `rpc/engine` | `backend/rpc/engine.rs` | ~790 | Complete | `pkg/wshutil/wshrpc.go` |
| `rpc/router` | `backend/rpc/router.rs` | ~720 | Complete | `pkg/wshutil/wshrouter.go` |
| `rpc_types` | `backend/rpc_types.rs` | ~200 | Complete | `pkg/wshrpc/` |
| `ai/*` | `backend/ai/` | ~800 | Complete | `pkg/waveai/` |
| `blockcontroller` | `backend/blockcontroller/` | ~400 | Complete | `pkg/blockcontroller/` |
| `remote/*` | `backend/remote/` | ~600 | Complete | `pkg/remote/+genconn/` |
| `shellexec` | `backend/shellexec.rs` | ~300 | Complete | `pkg/shellexec/` |
| `wshutil/*` | `backend/wshutil/` | ~1300 | Complete | `pkg/wshutil/` |
| `wslconn` | `backend/wslconn.rs` | ~200 | Complete | `pkg/wslconn/` |
| `wconfig` | `backend/wconfig.rs` | ~200 | Complete | `pkg/wconfig/` |
| + 20 utility modules | `backend/*.rs` | ~3000 | Complete | Various `pkg/*` |

**Total Rust backend code: ~10,000+ lines across 70+ files**

### What's Missing (Must Build for Integration)

| Component | Description | Complexity | Go Reference |
|-----------|-------------|-----------|--------------|
| **1. Backend Startup** | Initialize store, ensure initial data, start services | Medium | `cmd/server/main.go` |
| **2. WebSocket RPC Server** | Accept WS connections from wsh, route messages | High | `pkg/web/ws.go` |
| **3. HTTP API Server** | File upload/download, REST endpoints | Medium | `pkg/web/web.go` |
| **4. Terminal PTY Bridge** | Spawn shells, connect to blockcontroller | High | `pkg/shellexec/` + `pkg/blockcontroller/` |
| **5. Frontend RPC Adapter** | Replace WebSocket with Tauri IPC for frontend | High | New (doesn't exist in Go) |
| **6. wsh Integration** | Domain socket or named pipe for local wsh commands | Medium | `pkg/wshutil/wshserver.go` |
| **7. Stub Replacement** | Replace 16 stub commands with real implementations | Medium | Various |

---

## Implementation Phases

### Phase A: Backend Initialization (Est. 1-2 days)

**Goal:** Boot the Rust backend during Tauri setup(), replacing Go sidecar spawn.

**Changes:**

1. **`src-tauri/src/lib.rs`** — Replace `sidecar::spawn_backend()` with Rust backend init:
   ```rust
   // Instead of spawning wavemuxsrv:
   let store = WaveStore::open(&data_dir)?;
   ensure_initial_data(&store)?;
   let broker = Broker::new();
   let (rpc_engine, rpc_output) = WshRpcEngine::new();
   let router = WshRouter::new();
   // Register default route to RPC engine
   // Store all as managed Tauri state
   ```

2. **`src-tauri/src/state.rs`** — Add backend state:
   ```rust
   pub struct AppState {
       // ... existing fields ...
       pub store: Arc<WaveStore>,
       pub broker: Arc<Broker>,
       pub rpc_engine: Arc<WshRpcEngine>,
       pub router: Arc<WshRouter>,
   }
   ```

3. **Remove sidecar dependency:**
   - Remove `sidecar.rs` (or gate behind feature flag)
   - Remove `sidecar_child` from AppState
   - Remove WAVESRV-ESTART parsing
   - Remove sidecar binary from build

**Risks:**
- SQLite initialization timing (must complete before frontend queries)
- Migration handling (existing Go-created databases)

**Validation:**
- App starts without Go sidecar
- `ensure_initial_data()` creates Client/Window/Workspace/Tab
- State is accessible from Tauri commands

---

### Phase B: Stub Command Implementation (Est. 2-3 days)

**Goal:** Replace all 16 stub commands with real Rust implementations using WaveStore.

**Commands to implement:**

| Stub | Implementation | Rust Module |
|------|---------------|-------------|
| `create_workspace` | `wcore::create_workspace()` | `wcore.rs` |
| `switch_workspace` | `wcore::switch_workspace()` | `wcore.rs` |
| `delete_workspace` | `wcore::delete_workspace()` | `wcore.rs` |
| `create_tab` | `wcore::create_tab()` | `wcore.rs` |
| `close_tab` | `wcore::delete_tab()` | `wcore.rs` |
| `set_active_tab` | `wcore::set_active_tab()` | `wcore.rs` |
| `set_window_init_status` | Already implemented | `stubs.rs` |
| `set_waveai_open` | State toggle | Trivial |
| `show_context_menu` | Tauri native menu API | `menu.rs` |
| `download_file` | Tauri dialog + filestore | `filestore.rs` |
| `quicklook` | Platform-specific preview | OS API |
| `update_wco` | Window controls overlay | Tauri window API |
| `set_keyboard_chord_mode` | State toggle | Trivial |
| `register_global_webview_keys` | Tauri global shortcut plugin | Plugin API |
| `install_update` | Tauri updater plugin | Phase 11 |

**Approach:**
- Each stub gets its own `WaveStore` access via `tauri::State<AppState>`
- Mutations publish events via `Broker` for frontend updates

**Validation:**
- Workspace create/switch/delete works from UI
- Tab create/close works
- Context menus appear

---

### Phase C: Frontend RPC Adapter (Est. 3-4 days)

**Goal:** Route frontend RPC calls through Tauri IPC instead of WebSocket.

This is the **critical integration point**. Currently:
- Frontend (`wos.ts`) → WebSocket → Go `wavemuxsrv` → JSON-RPC handler → response
- Must become: Frontend (`wos.ts`) → Tauri invoke() → Rust RPC engine → response

**Strategy: Tauri Event Bridge**

Instead of rewriting all frontend RPC calls, create a **bidirectional bridge**:

1. **Frontend → Backend (commands):**
   ```typescript
   // NEW Tauri command
   invoke("rpc_call", { command: "getmeta", data: {...}, reqId: "..." })
   ```

2. **Backend → Frontend (events/responses):**
   ```rust
   // Emit Tauri event for each RPC response
   window.emit("rpc-response", response_json)?;
   ```

3. **Frontend → Backend (WebSocket replacement):**
   Create `src-tauri/src/commands/rpc_bridge.rs`:
   ```rust
   #[tauri::command]
   async fn rpc_call(
       command: String,
       data: Value,
       req_id: String,
       state: tauri::State<'_, AppState>,
   ) -> Result<Value, String> {
       let msg = RpcMessage {
           command,
           reqid: req_id,
           data: Some(data),
           ..Default::default()
       };
       state.rpc_engine.handle_message(msg);
       // Wait for response via internal channel
   }
   ```

4. **Frontend adapter** (`frontend/util/rpc-tauri.ts`):
   ```typescript
   // Replace WebSocket RPC with Tauri invoke
   export async function sendRpcCommand(cmd: string, data: any): Promise<any> {
       return invoke("rpc_call", { command: cmd, data, reqId: uuid() });
   }
   ```

**Key Frontend Files to Modify:**
- `frontend/app/store/wos.ts` — WaveObj store (main RPC consumer)
- `frontend/app/store/services.ts` — Service layer
- `frontend/app/store/wps.ts` — Event subscriptions
- `frontend/util/tauri-api.ts` — API shim

**Risks:**
- Streaming responses (terminal output) need Tauri events, not request/response
- wos.ts assumes WebSocket reconnect behavior
- Event subscription model differs (WebSocket push vs Tauri events)

---

### Phase D: Terminal PTY Integration (Est. 3-5 days)

**Goal:** Spawn and manage terminal shells directly from Rust.

This replaces the Go `pkg/shellexec/` + `pkg/blockcontroller/` pipeline.

**Components:**

1. **PTY spawning** — Use `portable-pty` or `pty-process` crate:
   ```rust
   // backend/blockcontroller/shell.rs (already exists, needs PTY impl)
   pub async fn start_shell(block_id: &str, shell_path: &str) -> Result<PtyPair, String>
   ```

2. **Terminal data flow:**
   ```
   xterm.js (frontend)
       ↕ Tauri events (input/output)
   blockcontroller (Rust)
       ↕ PTY read/write
   Shell process (bash/zsh/pwsh)
   ```

3. **New Tauri commands:**
   ```rust
   #[tauri::command]
   async fn terminal_input(block_id: String, data: Vec<u8>, state: ...) -> Result<(), String>

   #[tauri::command]
   async fn terminal_resize(block_id: String, rows: u16, cols: u16, state: ...) -> Result<(), String>
   ```

4. **PTY output streaming:**
   ```rust
   // Spawn background task that reads PTY and emits Tauri events
   tokio::spawn(async move {
       loop {
           let data = pty_reader.read().await;
           window.emit(&format!("terminal-output-{}", block_id), data)?;
       }
   });
   ```

**Dependencies:**
- `portable-pty` crate (cross-platform PTY)
- `tokio` for async I/O
- Existing `blockcontroller/shell.rs` module

**Risks:**
- Windows ConPTY differences from Unix PTY
- Shell environment setup (PATH, TERM, etc.)
- Signal handling (Ctrl+C, Ctrl+Z)

---

### Phase E: WebSocket Server for wsh (Est. 2-3 days)

**Goal:** Keep a WebSocket/domain socket server for `wsh` CLI tool integration.

Even with the frontend using Tauri IPC, `wsh` (the shell integration binary) still needs a way to communicate with the backend. In Go, this was via WebSocket.

**Options:**

| Option | Description | Complexity |
|--------|-------------|-----------|
| **A: Keep Go wsh + WS** | Rust backend hosts a minimal WS server for wsh | Medium |
| **B: Unix socket / named pipe** | wsh connects via local socket instead of WS | Medium |
| **C: Rewrite wsh in Rust** | Full Rust wsh using local socket | High (future) |

**Recommended: Option A** (minimal disruption)

1. Add `axum` or `warp` WebSocket server in Rust backend
2. Listen on localhost with auth token
3. Route wsh messages through `WshRouter`
4. Frontend no longer uses this WebSocket (uses Tauri IPC)

```rust
// src-tauri/src/backend/web/mod.rs (NEW)
pub async fn start_wsh_server(
    router: Arc<WshRouter>,
    auth_token: &str,
    port: u16,
) -> Result<(), String> {
    // axum WebSocket server
    // Authenticate wsh connections
    // Forward messages to router
}
```

**Validation:**
- `wsh` commands work from terminal
- OSC integration (terminal → Tauri → backend) works

---

### Phase F: HTTP API for File Operations (Est. 1-2 days)

**Goal:** Handle file upload/download without the Go HTTP server.

**Options:**

| Option | Description |
|--------|-------------|
| **A: Tauri commands** | File read/write via invoke() + base64 |
| **B: Keep HTTP server** | Minimal HTTP server for large file transfers |
| **C: Tauri plugin-fs** | Use Tauri's filesystem plugin |

**Recommended: Mix of A + C**
- Small files: Tauri invoke with base64 encoding
- Large files: Tauri plugin-fs for streaming
- File browser: Tauri dialog plugin for open/save

---

### Phase G: Go Sidecar Removal (Est. 1 day)

**Goal:** Remove the Go sidecar from the build.

1. Delete or gate `src-tauri/src/sidecar.rs`
2. Remove sidecar binary from `Taskfile.yml` build
3. Remove `tauri.conf.json` sidecar configuration
4. Remove Go binary from packaging
5. Update `tauri-api.ts` to remove WebSocket endpoint handling
6. Remove `WAVESRV-ESTART` parsing
7. Update `state.rs` to remove `sidecar_child`

**Result:** Single binary, ~25MB smaller, no external processes.

---

## Risk Matrix

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Frontend RPC breakage | High | Critical | Incremental migration with feature flag |
| Terminal PTY bugs | Medium | High | Test on all platforms early |
| wsh integration regression | Medium | High | Keep WebSocket server for backward compat |
| SQLite migration issues | Low | High | Schema migration tooling already exists |
| Performance regression | Low | Medium | Benchmark before/after |
| Streaming response latency | Medium | Medium | Use Tauri events (push model) |

---

## Implementation Order & Dependencies

```
Phase A: Backend Initialization
    ↓
Phase B: Stub Replacement ─────────────────┐
    ↓                                       │
Phase C: Frontend RPC Adapter ◄─────────────┘
    ↓
Phase D: Terminal PTY Integration
    ↓
Phase E: WebSocket Server for wsh
    ↓
Phase F: HTTP/File Operations
    ↓
Phase G: Go Sidecar Removal
```

**Critical path:** A → C → D (these block everything else)

---

## Feature Flag Strategy

To avoid breaking the app during integration, use a feature flag:

```rust
// Cargo.toml
[features]
default = ["go-sidecar"]
go-sidecar = []       # Use Go backend (current)
rust-backend = []     # Use Rust backend (new)
```

```rust
// lib.rs setup()
#[cfg(feature = "go-sidecar")]
sidecar::spawn_backend(&handle).await?;

#[cfg(feature = "rust-backend")]
rust_backend::initialize(&handle).await?;
```

This allows parallel development and easy rollback.

---

## Success Criteria

- [ ] WaveMux starts without Go sidecar
- [ ] Terminal (xterm.js) works with Rust PTY backend
- [ ] All stub commands replaced with real implementations
- [ ] Frontend RPC works via Tauri IPC (no WebSocket to backend)
- [ ] wsh CLI tool works via local WebSocket/socket
- [ ] File operations work (browse, preview, upload/download)
- [ ] AI chat works with Rust AI providers
- [ ] SSH remote connections work
- [ ] All existing tests pass
- [ ] Binary size reduced by ~25MB
- [ ] Startup time improved (no sidecar wait)

---

## Estimated Timeline

| Phase | Duration | Cumulative |
|-------|----------|-----------|
| A: Backend Init | 1-2 days | 1-2 days |
| B: Stub Replacement | 2-3 days | 3-5 days |
| C: Frontend RPC | 3-4 days | 6-9 days |
| D: Terminal PTY | 3-5 days | 9-14 days |
| E: wsh Server | 2-3 days | 11-17 days |
| F: File Ops | 1-2 days | 12-19 days |
| G: Sidecar Removal | 1 day | 13-20 days |

**Total: ~13-20 working days**

---

## References

- [TAURI_MIGRATION_STATUS.md](../TAURI_MIGRATION_STATUS.md) — Original Tauri migration
- [PHASE_17_IMPLEMENTATION_PLAN.md](PHASE_17_IMPLEMENTATION_PLAN.md) — Go-to-Rust porting
- [TAURI_PHASE_12_PRODUCTION_READY.md](TAURI_PHASE_12_PRODUCTION_READY.md) — Production readiness
- Go backend entry: `cmd/server/main.go`
- Frontend RPC: `frontend/app/store/wos.ts`
- Current sidecar: `src-tauri/src/sidecar.rs`
