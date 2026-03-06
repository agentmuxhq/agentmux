# Backend Lifecycle Spec

## Overview

AgentMux runs a Rust backend (`agentmuxsrv-rs`) as a sidecar process managed by the Tauri frontend. This document covers the full lifecycle: spawn, discovery, reuse, and shutdown — plus the known problem of **dangling backend processes** that survive after the frontend exits.

## Architecture

```
Tauri App (frontend)
  |
  +-- spawn_backend() --> agentmuxsrv-rs (sidecar)
  |       |                   |
  |       |                   +-- TCP web server (127.0.0.1:random)
  |       |                   +-- TCP ws server  (127.0.0.1:random)
  |       |                   +-- stdin watch thread (EOF -> shutdown)
  |       |                   +-- signal handler (SIGINT/SIGTERM -> shutdown)
  |       |
  |       +-- wave-endpoints.json (persisted for multi-window reuse)
  |
  +-- heartbeat.rs (writes timestamp every 5s to agentmux.heartbeat)
```

## Spawn Flow

**File:** `src-tauri/src/sidecar.rs` — `spawn_backend()`

1. Check for existing backend via `wave-endpoints.json`:
   - Read `{config_dir}/instances/v{version}/wave-endpoints.json`
   - If version matches, HTTP GET the `web_endpoint` to test responsiveness
   - If responsive: reuse (set `is_reused: true`), adopt its auth key
   - If stale: delete the file, proceed to spawn
2. Create instance directories (`data_dir/instances/v{ver}/db/`, `config_dir/instances/v{ver}/`)
3. Deploy `wsh` binary to `{app_path}/bin/wsh-{ver}-{os}.{arch}`
4. Spawn sidecar with args: `--wavedata`, `--instance`
5. Env vars: `AGENTMUX_AUTH_KEY`, `AGENTMUX_CONFIG_HOME`, `AGENTMUX_DATA_HOME`, `AGENTMUX_APP_PATH`, `AGENTMUX_DEV`, `WCLOUD_ENDPOINT`, `WCLOUD_WS_ENDPOINT`
6. Store `CommandChild` handle in `AppState.sidecar_child`
7. Parse `WAVESRV-ESTART` from stderr to get endpoints
8. Save endpoints + PID + timestamp to `wave-endpoints.json`

## Backend Startup

**File:** `agentmuxsrv-rs/src/main.rs`

1. Init tracing, parse CLI args, load config
2. Migrate `~/.waveterm` -> `~/.agentmux` (one-time)
3. Open SQLite databases (`wave.db`, `filestore.db`)
4. Bootstrap initial data (Client/Window/Workspace/Tab on first launch)
5. Start event infrastructure (EventBus, Broker, sysinfo loop)
6. Bind two TCP listeners on `127.0.0.1:0` (random ports)
7. Emit `WAVESRV-ESTART ws:{addr} web:{addr} version:{ver} buildtime:{time} instance:{id}` on stderr
8. Start stdin watch thread — **exits on EOF**
9. Start signal handler — **exits on SIGINT/SIGTERM**
10. `tokio::select!` over web server, ws server, and cancellation token

## Shutdown Flow

### Normal Shutdown (last window closes)

**File:** `src-tauri/src/lib.rs` — `on_window_event` handler

1. `CloseRequested` event fires
2. Count remaining windows (excluding the closing one)
3. If `remaining_windows == 0`:
   a. Take `sidecar_child` from `AppState`
   b. Call `child.kill()` — sends OS kill signal
   c. Delete `wave-endpoints.json` to prevent stale reuse
   d. Delete `agentmux.heartbeat` file

### Backend Self-Shutdown

The backend has **two** self-shutdown mechanisms:

1. **stdin EOF** (line 168-187 of `main.rs`): A blocking thread reads stdin. When the parent process (Tauri) closes its end of the pipe, `read()` returns 0 bytes, the thread cancels the token, and `tokio::select!` exits.

2. **Signal handler** (line 191-212): Catches SIGINT (Ctrl+C on all platforms) or SIGTERM (Unix only). Cancels the same token.

## The Dangling Process Problem

### Symptoms
- Multiple `agentmuxsrv-rs.x64.exe` processes visible in Task Manager
- No frontend window is open
- New frontend instances may connect to a stale backend via `wave-endpoints.json`
- Memory/CPU waste; port conflicts possible

### Root Causes

**1. `child.kill()` is not guaranteed**

Tauri's `CommandChild::kill()` wraps the OS kill, but:
- On Windows, if the process has already detached or the handle is invalid, the kill silently fails
- If the Tauri app crashes (panic, OOM, SIGKILL), `on_window_event` never runs — `child.kill()` is never called

**2. stdin EOF is unreliable on Windows**

The backend's stdin watch thread should detect parent death via EOF. However:
- Tauri's `shell.sidecar().spawn()` may not properly connect stdin — the sidecar could inherit a console handle that doesn't close when the parent exits
- On Windows, inherited handles behave differently than Unix pipes. The stdin handle may not produce EOF when the parent's process handle closes
- If stdin is connected to a console (not a pipe), `read()` blocks forever

**3. `wave-endpoints.json` outlives the process**

If the frontend crashes before deleting the endpoints file:
- Next launch reads the file, finds the old backend still running
- Connects to it (version matches, HTTP check passes)
- But the old backend may have stale state, wrong auth key, or orphaned controllers

**4. No timeout/watchdog in the backend**

The backend has no concept of "no frontends connected." It serves forever as long as stdin is open and no signal arrives. There's no idle timeout.

## Proposed Fixes

### What was tried and failed: WS-based idle timeout

**Attempted in v0.31.59:** Backend tracked active WS client count. If count reached 0 for 30s (after 60s initial grace), backend self-terminated.

**Why it failed:** WebSocket connections are not continuously stable. React re-renders, tab switches, window refocuses, and multi-window scenarios all cause brief WS disconnects. The backend killed itself while the frontend was still running, causing "Failed to fetch" errors and total instability.

**Lesson:** The backend must NEVER self-terminate based on WS client count. WS connections are a transport detail, not a reliable proxy for "frontend is alive."

### Phase 1: Reliable Cleanup (Implemented)

#### 1a. PID-based kill for reused backends (done, v0.31.59)

When the frontend reuses an existing backend via `wave-endpoints.json`, it stores the PID. On last window close, if no child handle exists (reused case), it falls back to OS-level kill via stored PID (`taskkill /PID` on Windows, `kill` on Unix).

#### 1b. Shutdown WS command (done, v0.31.59)

Backend accepts `{"type":"shutdown"}` via WebSocket. Cancels the shutdown token and exits gracefully. Frontend can send this before closing.

### Phase 2: Crash Recovery (TODO)

The remaining gap: if the Tauri process crashes, neither `child.kill()` nor PID kill runs.

#### 2a. Frontend heartbeat file (already exists)

`src-tauri/src/heartbeat.rs` already writes `agentmux.heartbeat` every 5s. The backend needs to READ this file and self-terminate if it's stale.

```rust
// In main.rs, add heartbeat watchdog:
// Read heartbeat file every 10s. If mtime > 30s old, frontend is dead.
// This is safe because the heartbeat is written by the Tauri process,
// not the WebSocket connection — it survives WS reconnects.
```

This is fundamentally different from WS-based idle detection because:
- The heartbeat is written by the **Tauri native process**, not the webview
- It survives WS reconnects, tab switches, React re-renders
- It only goes stale when the Tauri process actually dies

#### 2b. PID-based liveness check on reuse

Before attempting HTTP health check of an existing backend:
```rust
let pid = json["pid"].as_u64().unwrap_or(0);
if pid > 0 && !is_process_alive(pid as u32) {
    // Backend is dead, remove stale file
    let _ = std::fs::remove_file(&endpoints_file);
}
```

### Phase 3: WebView2 Orphan Cleanup (TODO)

**New problem discovered:** WebView2 (`msedgewebview2.exe`) child processes survive after `agentmux.exe` exits. They hold directory handles on the portable folder, preventing deletion.

#### 3a. Kill WebView2 children on shutdown

In `on_window_event(CloseRequested)`, after killing the backend sidecar, also kill WebView2 processes that are children of the current process:

```rust
// Get our PID, find msedgewebview2.exe processes with our PID as parent
// Kill them with taskkill /T (tree kill)
```

#### 3b. Use system-level WebView2 user data dir

Instead of storing WebView2 data inside the portable folder, use `AppData/Local/agentmux/WebView2`. This avoids directory locks on the portable folder entirely.

### Phase 4: Multi-Window Awareness (Long-term)

#### 4a. Connection registry

Backend tracks connected frontends (window ID + connect time). Exposed via RPC for debugging.

#### 4b. Graceful multi-window shutdown

Frontend sends window ID on WS connect. Backend tracks which windows are alive. Only the last window triggers backend shutdown.

## Files Involved

| File | Role |
|------|------|
| `src-tauri/src/sidecar.rs` | Spawn, discover, reuse backend |
| `src-tauri/src/lib.rs` | Window close -> kill sidecar |
| `src-tauri/src/state.rs` | `AppState.sidecar_child` handle |
| `src-tauri/src/heartbeat.rs` | Frontend heartbeat file |
| `agentmuxsrv-rs/src/main.rs` | Backend startup, stdin watch, signal handler |
| `agentmuxsrv-rs/src/server/` | WebSocket handler (connection tracking) |

## Priority

1. **Phase 1b (idle timeout)** — highest impact, catches all failure modes
2. **Phase 1c (PID check)** — cheap, prevents stale reuse
3. **Phase 1a (stdin close)** — improves normal shutdown path
4. **Phase 2/3** — hardening for production reliability
