# Backend Comparison: Go vs Rust

## Summary
The Go backend (agentmuxsrv) was removed in PR #238. The Rust backend replaced most functionality, but some critical pieces are missing.

---

## Go Backend Initialization (cmd/server/main-server.go)

### Data & Storage
- ✅ `wstore.InitWStore()` - Initialize Wave store
- ✅ `filestore.InitFilestore()` - Initialize file store
- ✅ `wcore.EnsureInitialData()` - Create initial client/window/workspace/tab

### Communication & RPC
- ❌ **`web.RunWebSocketServer(wsListener)`** - WebSocket server for frontend RPC
- ❌ **`web.RunWebServer(webListener)`** - HTTP web server (blocking main thread)
- ✅ `wshutil.RunWshRpcOverListener(unixListener)` - wsh IPC server

### Reactive Messaging (MISSING!)
- ❌ **`initReactiveHandler()`** - Initializes reactive agent-to-agent messaging
  - Creates input sender for block controller
  - Calls `reactive.InitGlobalHandler(inputSender)`
  - Syncs agent registrations from blocks
  - Starts cross-host polling service
- ❌ **`webhookdelivery.InitializeWebhookService()`** - Webhook delivery service
- ❌ **`web.RunReactiveServer(reactivePort)`** - Dedicated reactive server (optional)

### Block Management
- ❌ **`blocklogger.InitBlockLogger()`** - Initialize block logger
- ✅ Block controller functionality (ported to Rust)

### Config & Watching
- ✅ `startConfigWatcher()` - Watch config files for changes
- ✅ Config loading

### Other Services
- ❌ `createMainWshClient()` - Create main wsh client
- ❌ `stdinReadWatch()` - Monitor stdin for parent process death
- ❌ `telemetryLoop()` - Telemetry collection
- ❌ `updateTelemetryCountsLoop()` - Telemetry counts
- ❌ `startupActivityUpdate()` - Activity tracking
- ✅ `sigutil.InstallShutdownSignalHandlers()` - Signal handlers
- ✅ `shellutil.InitCustomShellStartupFiles()` - Shell integration files

---

## Rust Backend Initialization (src-tauri/src/rust_backend.rs)

### What the Rust Backend Does
- ✅ Opens WaveStore (SQLite)
- ✅ Opens FileStore (SQLite)
- ✅ Ensures initial data (client/window/workspace/tab)
- ✅ Initializes pub/sub broker with Tauri event delivery
- ✅ Initializes RPC engine
- ✅ Loads config from disk
- ✅ Starts config file watcher
- ✅ Generates auth key
- ✅ Spawns async router initialization
- ✅ Starts wsh IPC server (async)
- ✅ Emits "backend-ready" event to frontend

### What the Rust Backend is Missing
1. **WebSocket Server** - Frontend in Go version connected via WebSocket, not Tauri IPC
2. **HTTP Web Server** - Served static files and handled HTTP requests
3. **Reactive Messaging System**:
   - No `reactive.InitGlobalHandler()` call
   - No agent registration syncing
   - No cross-host polling startup
4. **Block Logger** - Logging for block operations
5. **Webhook Delivery Service** - For reactive messaging
6. **Main wsh Client Creation** - May be needed for some operations
7. **Stdin monitoring** - Parent process death detection
8. **Telemetry** - Usage tracking (may be intentionally removed)

---

## Critical Missing Piece: Reactive Messaging

The `initReactiveHandler()` function in Go backend:

```go
func initReactiveHandler() {
    // Create input sender that uses blockcontroller.SendInput
    inputSender := func(blockId string, inputData []byte) error {
        return blockcontroller.SendInput(blockId, &blockcontroller.BlockInputUnion{
            InputData: inputData,
        })
    }

    // Initialize the global handler with the input sender
    reactive.InitGlobalHandler(inputSender)

    // Sync existing agent registrations from blocks (in background)
    go func() {
        time.Sleep(2 * time.Second)
        ctx := context.Background()
        if err := reactive.GetGlobalHandler().SyncAgentsFromBlocks(ctx); err != nil {
            log.Printf("warning: failed to sync agent registrations: %v\n", err)
        }
    }()

    // Start cross-host polling service (if configured)
    go func() {
        time.Sleep(3 * time.Second)
        if err := reactive.StartGlobalPoller(); err != nil {
            log.Printf("warning: failed to start cross-host poller: %v\n", err)
        }
    }()
}
```

This is **NOT called** anywhere in the Rust backend!

---

## Communication Architecture Difference

### Go Backend (Old)
```
Frontend → WebSocket → Go Backend (agentmuxsrv) → WaveStore/Services
```

### Rust Backend (Current)
```
Frontend → Tauri IPC → Rust Backend → WaveStore/Services
```

The frontend was likely expecting WebSocket connections but now uses Tauri IPC. This works for most RPC calls, but some initialization or event handling may be different.

---

## Hypothesis: Grey Screen Root Cause

The grey screen may be caused by:

1. **Missing WebSocket/Web Server**: Frontend may be waiting for WebSocket connection that never comes
2. **Backend-ready event mismatch**: Frontend expects different initialization signal
3. **Missing reactive system**: Some UI components may depend on reactive messaging being initialized
4. **Block logger**: Some blocks may fail to initialize without block logger

---

## Next Steps

1. ✅ Frontend correctly uses Tauri IPC (not WebSocket) when Rust backend detected
2. ✅ Frontend receives "backend-ready" event successfully
3. ✅ Reactive messaging NOT needed for basic UI (only for agent-to-agent features)
4. ❌ **ROOT CAUSE FOUND: Initial tab has NO BLOCKS**

## **ROOT CAUSE: Grey Screen**

**Problem:** Rust backend `create_tab()` creates tabs with empty `blockids: vec![]`.

**Result:** Frontend TabContent component (line 64-65) renders `null` for empty tabs:
```tsx
} else if (tabData?.blockids?.length == 0) {
    innerContent = null;  // ← Grey screen!
}
```

**Solution:** Modify `ensure_initial_data()` to create at least one initial terminal block in the first tab.

---

## Implementation Fix

In `src-tauri/src/backend/wcore.rs`, modify `ensure_initial_data()` to create an initial block:

```rust
// After creating initial tab:
create_tab(store, &ws.oid)?;

// ADD: Create initial terminal block
let mut meta = MetaMapType::new();
meta.insert("view".to_string(), "term".into());
create_block(store, &tab.oid, meta)?;
```
