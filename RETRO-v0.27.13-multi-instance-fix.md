# Retrospective: v0.27.13 Multi-Instance Fix

**Date:** 2026-02-15
**PR:** #308 (builds on #307)
**Version:** 0.27.12 → 0.27.13
**Author:** AgentX

---

## Timeline

### Issue 1: Multi-Instance Endpoint Collision (v0.27.12, PR #307)
- Multiple AgentMux instances wrote their endpoint files to the same path, causing the second instance to overwrite the first's configuration.
- **Fix:** Instance-aware endpoint file paths. Merged as v0.27.12.

### Issue 2: Grey Screen on Startup (v0.27.12 → v0.27.13)
- After switching from `app_data_dir()` (AppData\Roaming) to `app_local_data_dir()` (AppData\Local), the backend failed to start because `instances\default\db\` didn't exist.
- **Error:** `error initializing filestore: unable to open database file: The system cannot find the path specified.`
- **Root cause:** The Go backend expects the `db/` subdirectory to exist before initializing SQLite. With the new data directory, nothing had created it yet.

### Issue 3: Grey Screen on Instance Fallback
- When a stale process held the default lock, the backend fell back to `instance-1`, which also had no `db/` directory.
- Same error, different instance directory.
- **Lesson:** Pre-creating only the `default` instance directory in Rust wasn't enough. The Go backend itself needed to create `db/` after acquiring its lock (for any instance).

### Issue 4: Websocket Route Collision (the real multi-instance bug)
- Two AgentMux exe processes share the same Go backend (correct behavior for same-version).
- Both Tauri windows have label `"main"` (each is a separate process).
- Both called `initTauriWave()`, which loads the EXISTING windowId/tabId from the backend.
- Result: both windows subscribed to the same tab's websocket route. The second connection replaced the first → input to window 1 stopped working.
- **Error in logs:** `[websocket] warning: replacing existing connection for route "tab:e01167f8..."`

---

## Key Discoveries

### 1. Tauri Multi-Process != Electron Multi-Window
In Electron, `BrowserWindow` instances share a single main process. `isMainWindow()` distinguishes them by reference.

In Tauri, each exe launch is a **separate OS process** with its own `"main"` window label. `is_main_window()` always returns `true` for every instance because every process calls its window `"main"`. This meant the existing main-vs-new-window branching logic was useless for multi-instance.

### 2. Backend Reuse is Correct Architecture
Initial instinct was to make each frontend spawn its own backend. This was wrong.

**Correct behavior:** Same-version frontends share one backend process. Different-version frontends spawn separate backends. The backend handles multi-tenancy through separate window/tab objects.

The issue wasn't backend sharing — it was that both frontends loaded the *same* window/tab instead of creating their own.

### 3. Two Init Paths Already Existed
The codebase already had the solution:
- `initTauriWave()` — loads existing window from backend (for the "owner" of the backend)
- `initTauriNewWindow()` — creates a NEW window via `WindowService.CreateWindow()` RPC

The missing piece was knowing *which path to take*. The `is_reused` flag solves this: the process that spawned the backend uses `initTauriWave()`, and any process that connects to an already-running backend uses `initTauriNewWindow()`.

### 4. Directory Creation Responsibility
The Go backend's lock acquisition (`AcquireWaveLockWithAutoInstance`) creates instance directories for non-default instances, but not the `db/` subdirectory within them. The `db/` subdirectory was implicitly expected to exist.

**Fix was two-layered:**
- Rust side: pre-create `default` instance dirs as a safety net before spawning
- Go side: create `db/` dir after lock acquisition for *any* instance (the authoritative fix)

---

## The Fix (Summary)

### Data Flow: `is_reused` Flag
```
sidecar.rs (spawn_backend)
  → is_reused: true  (if connecting to existing backend)
  → is_reused: false (if freshly spawned)

  → BackendSpawnResult.is_reused
    → AppState.backend_endpoints.is_reused
      → get_backend_endpoints command (returns is_reused to frontend)
      → backend-ready event (emits is_reused to frontend)

frontend/tauri-api.ts
  → window.__WAVE_BACKEND_REUSED__ = is_reused

frontend/wave.ts
  → if (isMain && !isBackendReused) → initTauriWave()    // owner
  → else                            → initTauriNewWindow() // guest
```

### Directory Creation
```
Rust (before spawn):   data_dir/instances/default/db/     (safety net)
Go (after lock):       {instance_dir}/db/                  (authoritative)
```

---

## Lessons Learned

1. **Test multi-instance explicitly.** A single-instance test passing does not mean multi-instance works. The websocket collision only manifests with 2+ windows.

2. **Understand the existing architecture before adding workarounds.** The `initTauriNewWindow()` path already existed and did exactly what was needed. The fix was routing to it correctly, not writing new initialization logic.

3. **Platform differences matter.** Tauri's process model is fundamentally different from Electron's. Assumptions about window labels, main process singleton behavior, and IPC patterns don't transfer.

4. **Directory creation should be close to usage.** The Go backend should ensure its own directories exist rather than relying on the Rust launcher to predict every possible instance directory.

5. **Build and test locally before pushing.** Multiple rounds of push → test → fix could have been one round with local testing from the start.

---

## Files Changed (7 files, +28/-8 lines)

| File | Change |
|------|--------|
| `src-tauri/src/sidecar.rs` | `is_reused` field on `BackendSpawnResult` |
| `src-tauri/src/state.rs` | `is_reused` on `BackendEndpoints` |
| `src-tauri/src/lib.rs` | Store + emit `is_reused` |
| `src-tauri/src/commands/backend.rs` | Return `is_reused` from IPC command |
| `cmd/server/main-server.go` | Create `db/` dir after lock acquisition |
| `frontend/util/tauri-api.ts` | Capture `__WAVE_BACKEND_REUSED__` global |
| `frontend/wave.ts` | Branch on `isBackendReused` for init path |
