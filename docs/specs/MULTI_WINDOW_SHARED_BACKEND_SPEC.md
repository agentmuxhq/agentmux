# Multi-Window Shared Backend Specification

**Date**: 2026-02-13
**Goal**: Allow multiple AgentMux windows connecting to a single shared backend
**Status**: Design Phase

---

## Current Behavior vs Desired

### Current (After PR #285)
- Launch instance 1 → Creates backend #1 with data dir #1 ✅
- Launch instance 2 → Creates backend #2 with data dir #2 ✅
- **Problem**: User wants shared backend, not isolated instances

### Desired
- Launch window 1 → Creates backend, window connects ✅
- Launch window 2 → **Reuses existing backend**, new window connects ✅
- Launch window 3 → **Reuses existing backend**, new window connects ✅
- All windows share same workspace/data/terminals

---

## Architecture

### Single Backend, Multiple Windows

```
┌──────────────┐
│  Window 1    │──┐
│  (Frontend)  │  │
└──────────────┘  │
                  │   WebSocket
┌──────────────┐  │   Connection
│  Window 2    │──┼──►  ┌─────────────────┐
│  (Frontend)  │  │     │  agentmuxsrv    │
└──────────────┘  │     │  (Backend)      │
                  │     │  - One instance │
┌──────────────┐  │     │  - One data dir │
│  Window 3    │──┘     │  - Shared state │
│  (Frontend)  │        └─────────────────┘
└──────────────┘
```

---

## Implementation Plan

### Phase 1: Backend Connection Discovery

**File**: `src-tauri/src/sidecar.rs`

When launching a new window:

1. **Check if backend is already running**
   ```rust
   // Try to read existing endpoints from lock file
   let endpoints_file = config_dir.join("wave-endpoints.json");
   if endpoints_file.exists() {
       if let Ok(existing_endpoints) = read_existing_endpoints(&endpoints_file) {
           // Test connection to existing backend
           if test_backend_connection(&existing_endpoints).await.is_ok() {
               log::info!("Found running backend, reusing connection");
               return Ok(existing_endpoints);
           }
       }
   }
   ```

2. **Only spawn backend if none exists**
   ```rust
   // No existing backend found, spawn new one
   log::info!("No running backend found, spawning new agentmuxsrv");
   spawn_new_backend(app).await
   ```

3. **Save endpoints for other windows**
   ```rust
   // After backend starts, save endpoints to file
   let endpoints = BackendSpawnResult {
       ws_endpoint: ws_url,
       web_endpoint: web_url,
   };
   save_endpoints(&endpoints_file, &endpoints)?;
   ```

### Phase 2: Window Management

**File**: `src-tauri/src/lib.rs`

**Window Creation**:
```rust
pub fn create_new_window(app: &AppHandle) -> Result<(), String> {
    let window = tauri::WindowBuilder::new(
        app,
        format!("main-{}", uuid::Uuid::new_v4()), // Unique window ID
        tauri::WindowUrl::App("/".into())
    )
    .title("AgentMux")
    .inner_size(1200.0, 800.0)
    .build()
    .map_err(|e| format!("Failed to create window: {}", e))?;

    Ok(())
}
```

**Tauri Command**:
```rust
#[tauri::command]
async fn open_new_window(app: tauri::AppHandle) -> Result<(), String> {
    create_new_window(&app)
}
```

### Phase 3: Backend Shutdown Coordination

**Problem**: When to shut down shared backend?

**Solution**: Backend stays alive until ALL windows close

**Implementation**:

1. **Track active windows**:
   ```rust
   // In backend or shared state
   struct WindowTracker {
       active_windows: Arc<Mutex<HashSet<String>>>,
   }

   impl WindowTracker {
       fn register_window(&self, window_id: String) {
           self.active_windows.lock().unwrap().insert(window_id);
       }

       fn unregister_window(&self, window_id: String) -> bool {
           let mut windows = self.active_windows.lock().unwrap();
           windows.remove(&window_id);
           windows.is_empty() // Return true if last window
       }
   }
   ```

2. **Window close handler**:
   ```rust
   window.on_window_event(move |event| {
       if let tauri::WindowEvent::CloseRequested { .. } = event {
           let window_id = window.label().to_string();
           let is_last_window = tracker.unregister_window(window_id);

           if is_last_window {
               // Shut down backend
               log::info!("Last window closed, shutting down backend");
               shutdown_backend();
           }
       }
   });
   ```

### Phase 4: Frontend Updates

**File**: `frontend/wave.ts`

**Add "New Window" Action**:
```typescript
// Menu or keyboard shortcut to open new window
async function openNewWindow() {
    await getApi().openNewWindow();
}
```

**File**: `frontend/app/store/wshclientapi.ts`

**Add API method**:
```typescript
openNewWindow(): Promise<void> {
    return invoke("open_new_window");
}
```

---

## Testing Checklist

### Basic Multi-Window
- [ ] Launch AgentMux → Window 1 opens with backend
- [ ] Open new window → Window 2 opens, connects to same backend
- [ ] Open third window → Window 3 opens, connects to same backend
- [ ] Check: Only 1 agentmuxsrv process running
- [ ] Check: All windows show same workspace/data

### Window Independence
- [ ] Create terminal in Window 1
- [ ] Switch to Window 2 → Same terminal visible
- [ ] Close Window 1 → Windows 2 & 3 stay open
- [ ] Check: Backend still running

### Backend Shutdown
- [ ] Close Window 2 → Backend stays alive (Window 3 open)
- [ ] Close Window 3 → Backend shuts down (last window)
- [ ] Reopen AgentMux → New backend starts

### Edge Cases
- [ ] Launch 2 instances rapidly → Only 1 backend spawns
- [ ] Kill backend while windows open → Windows detect disconnect
- [ ] Backend crashes → All windows show error, allow reconnect

---

## Implementation Files

### Backend (Go)
- `pkg/wavebase/wavebase.go` - Lock file logic (no changes needed)
- `cmd/server/main-server.go` - Multi-client support (already works!)

### Frontend (Rust/Tauri)
- `src-tauri/src/sidecar.rs` - Backend discovery + spawn logic
- `src-tauri/src/lib.rs` - Window creation commands
- `src-tauri/src/state.rs` - Shared state for endpoint tracking

### Frontend (TypeScript)
- `frontend/wave.ts` - New window action
- `frontend/app/store/wshclientapi.ts` - API method

---

## Key Insights

1. **Backend is already multi-client capable**
   - WebSocket server supports multiple connections
   - No backend changes needed!

2. **Tauri needs connection reuse**
   - Currently spawns new backend per window
   - Fix: Check for existing backend first

3. **Simpler than multi-instance**
   - No need for instance-specific data dirs
   - No need for lock file coordination
   - Just: "Is backend running? Yes → connect, No → spawn"

---

## Migration from Current PR #285

Current multi-instance code can coexist:
- **Multi-window** (this spec): Default behavior, same data
- **Multi-instance** (PR #285): Opt-in with `--instance=name` flag

Users who want isolated instances can still use the instance flag.

---

## Next Steps

1. Implement backend discovery in `sidecar.rs`
2. Add window creation command
3. Add frontend UI for "New Window"
4. Test multi-window scenarios
5. Document keyboard shortcuts

---

## Estimated Time

- Backend discovery: 30 min
- Window creation: 20 min
- Frontend UI: 15 min
- Testing: 30 min
- **Total**: ~2 hours
