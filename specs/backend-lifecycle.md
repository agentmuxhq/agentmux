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

### Phase 1: Reliable Cleanup (Short-term)

#### 1a. Frontend shutdown: close stdin pipe before kill

Instead of just `child.kill()`, close the child's stdin handle first. This triggers the backend's stdin EOF handler for a graceful shutdown, then kill as fallback after a timeout.

```rust
// In on_window_event CloseRequested handler:
if let Some(child) = sidecar.take() {
    // Close stdin to trigger graceful shutdown
    drop(child.stdin()); // if available

    // Give backend 2s to exit gracefully
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let _ = child.kill(); // force kill if still alive
    });
}
```

#### 1b. Backend: add idle timeout

If no WebSocket clients are connected for N seconds (e.g., 30s), the backend should self-terminate. This catches all cases where the frontend dies without cleanup.

```rust
// In main.rs, add to tokio::select!:
_ = idle_watchdog(ws_client_count.clone(), Duration::from_secs(30)) => {
    tracing::info!("No frontend connections for 30s, shutting down");
}
```

The WebSocket upgrade handler already tracks connections. Add an atomic counter:
- Increment on WS connect
- Decrement on WS disconnect
- Watchdog task: if count == 0, start a timer. If still 0 after timeout, cancel.

#### 1c. PID-based liveness check on reuse

When reading `wave-endpoints.json`, verify the recorded PID is still alive before attempting HTTP:

```rust
let pid = json["pid"].as_u64().unwrap_or(0);
if pid > 0 && !is_process_alive(pid as u32) {
    tracing::warn!("Backend PID {} is dead, removing stale endpoints file");
    let _ = std::fs::remove_file(&endpoints_file);
    // proceed to spawn
}
```

### Phase 2: Robust Lifecycle (Medium-term)

#### 2a. Named mutex / lock file

On startup, the backend creates a lock file (`{instance_dir}/agentmuxsrv.lock`) with its PID. The frontend checks this lock before reuse. On backend exit (via `Drop` or atexit), the lock is removed.

#### 2b. Heartbeat protocol (backend -> frontend)

The frontend already writes heartbeats. Add a backend heartbeat:
- Backend writes `{instance_dir}/agentmuxsrv.heartbeat` every 5s
- Frontend checks this file's mtime before reuse
- If stale (>15s old), consider the backend dead

#### 2c. Graceful shutdown RPC

Add a `Shutdown` RPC command that the frontend calls before killing. This lets the backend:
- Flush pending writes
- Close WebSocket connections cleanly
- Remove lock/endpoints files
- Exit with code 0

### Phase 3: Multi-Window Awareness (Long-term)

#### 3a. Connection registry

Backend tracks connected frontends (window ID + connect time). Exposed via RPC for debugging.

#### 3b. Last-frontend-disconnected event

When the last WS client disconnects, start the idle timer. If a new client connects within the timeout, cancel the timer. This replaces the blunt "no connections for N seconds" with a precise lifecycle.

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
