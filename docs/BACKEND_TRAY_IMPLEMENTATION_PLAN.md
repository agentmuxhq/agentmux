# Backend-Managed Tray Icon - Implementation Plan

**Date**: 2026-02-13
**Status**: 📋 Ready to Implement
**Target Version**: 0.28.0
**Estimated Effort**: 6-8 hours

---

## Architecture Overview

### Current (Multi-Tray)
```
Frontend 1 (Tauri) → Tray Icon 1
Frontend 2 (Tauri) → Tray Icon 2
Frontend 3 (Tauri) → Tray Icon 3

Backend (agentmuxsrv) → No tray
```

### Target (Single Tray)
```
Frontend 1 (Tauri) → No tray ┐
Frontend 2 (Tauri) → No tray ├─► Backend (agentmuxsrv) → Single Tray Icon
Frontend 3 (Tauri) → No tray ┘
```

**Key Benefits**:
- ✅ Exactly 1 tray icon (1 backend = 1 tray)
- ✅ Tray persists when all frontends close
- ✅ Tray lifecycle tied to backend (clean shutdown)
- ✅ Centralized state management

---

## Go Tray Library Selection

### Option: `github.com/getlantern/systray` ⭐ RECOMMENDED

**Pros**:
- ✅ Cross-platform (Windows, macOS, Linux)
- ✅ Active maintenance (last commit: recent)
- ✅ Simple API
- ✅ Supports icons, menus, tooltips
- ✅ Used in production apps (Lantern)

**Basic API**:
```go
import "github.com/getlantern/systray"

func main() {
    systray.Run(onReady, onExit)
}

func onReady() {
    systray.SetIcon(iconData)
    systray.SetTitle("AgentMux")
    systray.SetTooltip("AgentMux Terminal")

    mShow := systray.AddMenuItem("Show Window", "")
    mNew := systray.AddMenuItem("New Window", "")
    systray.AddSeparator()
    mQuit := systray.AddMenuItem("Quit", "Quit AgentMux")

    go func() {
        for {
            select {
            case <-mShow.ClickedCh:
                handleShowWindow()
            case <-mNew.ClickedCh:
                handleNewWindow()
            case <-mQuit.ClickedCh:
                systray.Quit()
            }
        }
    }()
}
```

**Dependency**:
```bash
cd cmd/server
go get github.com/getlantern/systray
```

---

## Windows Subsystem Compatibility

### The `-H windowsgui` Flag

**Current**: Backend built with `-H windowsgui` to hide console window

**Question**: Does this conflict with tray icon?

**Answer**: ✅ NO CONFLICT!

**Explanation**:
- `-H windowsgui` = Windows GUI subsystem (no console allocation)
- Tray icons ARE GUI elements and work fine with `windowsgui`
- The flag only prevents console window, not GUI elements

**We can keep** `-H windowsgui` AND have tray icon!

---

## Implementation Steps

### Step 1: Add Tray Library to Backend (30 min)

**Files**:
- `cmd/server/main-server.go`
- `go.mod` / `go.sum`

**Commands**:
```bash
cd cmd/server
go get github.com/getlantern/systray
go mod tidy
```

**Code**:
```go
// cmd/server/main-server.go

import (
    "github.com/getlantern/systray"
    _ "embed"
)

//go:embed assets/icon.ico
var iconData []byte

func main() {
    // Start tray in separate goroutine
    go systray.Run(onTrayReady, onTrayExit)

    // Continue with normal server startup
    startServer()
}

func onTrayReady() {
    systray.SetIcon(iconData)
    systray.SetTitle("AgentMux")
    systray.SetTooltip("AgentMux - AI Terminal")

    buildTrayMenu()
}

func onTrayExit() {
    log.Println("Tray icon exiting")
}
```

### Step 2: Embed Icon in Backend (30 min)

**Source icon**: `src-tauri/icons/icon.ico` (already exists)

**Copy to backend**:
```bash
mkdir -p cmd/server/assets
cp src-tauri/icons/icon.ico cmd/server/assets/
```

**Embed in binary**:
```go
// cmd/server/main-server.go

import _ "embed"

//go:embed assets/icon.ico
var iconData []byte

func onTrayReady() {
    systray.SetIcon(iconData)
    // ...
}
```

**Build verification**:
```bash
task build:backend
# Icon should be embedded in binary
```

### Step 3: Build Tray Menu (1 hour)

**Menu structure**:
```
AgentMux
├── Show All Windows
├── New Window
├── ──────────────
├── Settings (future)
├── About
├── ──────────────
└── Quit AgentMux
```

**Implementation**:
```go
// cmd/server/tray.go (new file)

package main

import "github.com/getlantern/systray"

func buildTrayMenu() {
    mShowAll := systray.AddMenuItem("Show All Windows", "Show all AgentMux windows")
    mNew := systray.AddMenuItem("New Window", "Open a new window")

    systray.AddSeparator()

    mAbout := systray.AddMenuItem("About", "About AgentMux")

    systray.AddSeparator()

    mQuit := systray.AddMenuItem("Quit AgentMux", "Quit AgentMux and all windows")

    // Event loop
    go func() {
        for {
            select {
            case <-mShowAll.ClickedCh:
                handleShowAllWindows()
            case <-mNew.ClickedCh:
                handleNewWindow()
            case <-mAbout.ClickedCh:
                handleAbout()
            case <-mQuit.ClickedCh:
                handleQuit()
            }
        }
    }()
}
```

### Step 4: Backend → Frontend IPC (2 hours)

**Challenge**: Backend needs to tell frontends to show/hide/create windows

**Solution**: Use existing WebSocket connection + WAVESRV-EVENT mechanism

**Backend sends events**:
```go
// pkg/wshrpc/events.go (new or extend existing)

type TrayEvent struct {
    EventType string `json:"event_type"` // "show-all", "new-window", "quit"
}

func handleShowAllWindows() {
    event := TrayEvent{EventType: "show-all"}
    broadcastToAllClients("tray:action", event)
}

func handleNewWindow() {
    event := TrayEvent{EventType: "new-window"}
    // Send to any connected client (they'll create new window)
    broadcastToAllClients("tray:action", event)
}

func handleQuit() {
    event := TrayEvent{EventType: "quit"}
    broadcastToAllClients("tray:action", event)

    // Wait for clients to disconnect gracefully
    time.Sleep(1 * time.Second)

    // Shutdown backend
    systray.Quit()
    os.Exit(0)
}
```

**Frontend receives events**:
```typescript
// frontend/app/store/global.ts (or appropriate location)

// Listen for tray events
WOS.getWaveObjectValue<TrayEvent>(
    WOS.makeORef("tray", "action")
).subscribe((event) => {
    switch (event.event_type) {
        case "show-all":
            // Show all windows
            showAllWindows();
            break;
        case "new-window":
            // Open new window via Tauri command
            getApi().openNewWindow();
            break;
        case "quit":
            // Close this window
            window.close();
            break;
    }
});
```

### Step 5: Remove Frontend Tray (15 min)

**Files**:
- `src-tauri/src/lib.rs`
- `src-tauri/src/tray.rs` (can delete)

**Changes**:
```rust
// src-tauri/src/lib.rs - remove tray setup

// BEFORE:
if let Err(e) = tray::build_tray(&handle) {
    tracing::error!("Failed to build system tray: {}", e);
}

// AFTER:
// Tray now managed by backend - no frontend tray needed
```

**Delete**:
```bash
rm src-tauri/src/tray.rs
```

**Update lib.rs imports**:
```rust
// Remove tray module
// mod tray;  // ← Delete this line
```

### Step 6: Frontend Event Handlers (1 hour)

**Show All Windows**:
```typescript
// frontend/app/tray-handler.ts (new file)

export function showAllWindows() {
    // Get all windows via Tauri
    getApi().listWindows().then(windows => {
        windows.forEach(window => {
            // Show and focus each window
            getApi().focusWindow(window.label);
        });
    });
}
```

**New Window**:
```typescript
export function createNewWindow() {
    getApi().openNewWindow().then(label => {
        console.log(`New window created: ${label}`);
    });
}
```

### Step 7: Testing (2 hours)

**Test matrix**:

| Scenario | Expected Result |
|----------|----------------|
| Launch 1 instance | 1 tray icon appears |
| Launch 3 instances | Still 1 tray icon |
| Click "Show All Windows" | All 3 windows come to front |
| Click "New Window" | 4th window opens |
| Close all windows | Tray remains (backend still running) |
| Click tray after all closed | New window opens |
| Click "Quit" | All windows close, tray disappears, backend exits |

**Manual test procedure**:
```bash
# 1. Build new version
task build:backend
task package:portable

# 2. Extract and run
cd ~/Desktop
unzip agentmux-0.28.0-x64-portable.zip -d agentmux-test
cd agentmux-test
./agentmux.exe

# 3. Launch multiple instances
./agentmux.exe &
./agentmux.exe &
./agentmux.exe &

# 4. Check tray
# Should see ONLY 1 icon in system tray

# 5. Test tray menu
# Right-click tray → "Show All Windows"
# All 3 windows should come to front

# 6. Test new window
# Right-click tray → "New Window"
# 4th window should open

# 7. Close all windows (X button)
# Tray should remain

# 8. Click tray "Show All Windows"
# Should open a new window

# 9. Click "Quit"
# All windows close, tray disappears
```

---

## Code Organization

### New Files

```
cmd/server/
├── assets/
│   └── icon.ico              (embedded in binary)
├── tray.go                   (tray menu and events)
└── main-server.go            (initialize tray)

pkg/wshrpc/
└── tray_events.go            (tray event types)

frontend/app/
└── tray-handler.ts           (frontend tray event handlers)
```

### Modified Files

```
cmd/server/main-server.go     (add tray initialization)
go.mod                        (add systray dependency)
src-tauri/src/lib.rs          (remove frontend tray)
frontend/app/store/global.ts  (add tray event listener)
```

### Deleted Files

```
src-tauri/src/tray.rs         (no longer needed)
```

---

## Dependency Management

### Go Dependencies

```bash
cd ~/agentmux
go get github.com/getlantern/systray@latest
go mod tidy
```

**go.mod changes**:
```go
require (
    github.com/getlantern/systray v1.2.2  // New
    // ... existing dependencies
)
```

### Rust Dependencies

**No changes needed** (removing Tauri tray code)

---

## Build Process

### No Changes to Taskfile.yml

The `-H windowsgui` flag stays - it's compatible with tray icons!

```yaml
# Taskfile.yml - NO CHANGES NEEDED
GO_LDFLAGS: '{{if eq OS "windows"}}-s -w -H windowsgui{{else}}-s -w{{end}}'
```

### Verify Embedded Icon

```bash
# After build, verify icon is embedded
task build:backend

# Check binary size (should be slightly larger with embedded icon)
ls -lh dist/bin/agentmuxsrv.x64.exe
```

---

## Event Flow Diagrams

### Tray Click → Show All Windows

```
User clicks tray "Show All Windows"
    ↓
Backend: handleShowAllWindows()
    ↓
Backend: broadcastToAllClients("tray:action", {event_type: "show-all"})
    ↓
Frontend 1,2,3: Receive WebSocket event
    ↓
Frontend: showAllWindows()
    ↓
Tauri: focusWindow() for each window
    ↓
Result: All windows come to front
```

### Tray Click → New Window

```
User clicks tray "New Window"
    ↓
Backend: handleNewWindow()
    ↓
Backend: broadcastToAllClients("tray:action", {event_type: "new-window"})
    ↓
Frontend 1 (first to receive): createNewWindow()
    ↓
Tauri: openNewWindow()
    ↓
Result: New Tauri window spawns, connects to same backend
```

### Tray Click → Quit

```
User clicks tray "Quit"
    ↓
Backend: handleQuit()
    ↓
Backend: broadcastToAllClients("tray:action", {event_type: "quit"})
    ↓
All Frontends: Receive event
    ↓
All Frontends: window.close()
    ↓
Backend: Waits 1 second
    ↓
Backend: systray.Quit()
    ↓
Backend: os.Exit(0)
    ↓
Result: Everything shuts down gracefully
```

---

## Edge Cases & Error Handling

### Case 1: Backend starts before frontend

**Scenario**: Backend tray shows, but no frontends connected yet

**Behavior**: Tray clicks queue events. When frontend connects, events are delivered.

**Alternatively**: Ignore clicks until at least one frontend connects.

### Case 2: All frontends disconnect

**Scenario**: User closes all windows, tray remains

**Behavior**: Backend keeps running. Clicking tray does nothing (no frontends to notify).

**Solution**: Add special handling:
```go
func handleShowAllWindows() {
    if len(connectedClients) == 0 {
        // No clients connected - spawn new frontend
        launchNewFrontend()
    } else {
        // Broadcast to existing clients
        broadcastToAllClients("tray:action", {event_type: "show-all"})
    }
}

func launchNewFrontend() {
    // Launch agentmux.exe from same directory
    cmd := exec.Command("./agentmux.exe")
    cmd.Start()
}
```

### Case 3: Backend crashes

**Scenario**: Backend dies while frontends are open

**Current behavior**: Frontends detect disconnect, show error

**Tray impact**: Tray disappears (good - indicates backend is dead)

**No changes needed** - existing error handling sufficient

---

## Performance Considerations

### Memory

**Tray overhead**: ~1-2 MB (systray library + icon)

**Impact**: Negligible (backend already ~30 MB)

### CPU

**Tray events**: Minimal (event-driven, no polling)

**Menu rendering**: OS-handled, no backend CPU usage

---

## Compatibility

### Windows

✅ Fully supported (primary target)

### macOS

✅ Supported by systray library
- macOS uses menu bar instead of system tray
- Same code works, different UI location

### Linux

✅ Supported by systray library
- Uses system tray (varies by desktop environment)

---

## Rollback Plan

If backend tray causes issues:

1. **Quick revert**: Re-enable frontend tray
   ```rust
   // src-tauri/src/lib.rs
   if let Err(e) = tray::build_tray(&handle) {
       tracing::error!("Failed to build system tray: {}", e);
   }
   ```

2. **Restore file**: `src-tauri/src/tray.rs` from git

3. **Remove backend tray**:
   ```go
   // cmd/server/main-server.go
   // Comment out: go systray.Run(...)
   ```

4. **Rebuild**: `task package:portable`

**Rollback time**: ~15 minutes

---

## Success Criteria

- [ ] 1 instance: 1 tray icon
- [ ] 5 instances: 1 tray icon (not 5)
- [ ] Tray menu opens on right-click
- [ ] "Show All Windows" brings all windows to front
- [ ] "New Window" opens new window
- [ ] "Quit" closes all windows and backend
- [ ] Tray persists when all windows closed
- [ ] Backend builds successfully with embedded icon
- [ ] No console window appears (windowsgui flag works)

---

## Timeline

| Step | Time | Cumulative |
|------|------|-----------|
| 1. Add tray library | 30 min | 30 min |
| 2. Embed icon | 30 min | 1 hour |
| 3. Build menu | 1 hour | 2 hours |
| 4. Backend→Frontend IPC | 2 hours | 4 hours |
| 5. Remove frontend tray | 15 min | 4.25 hours |
| 6. Frontend handlers | 1 hour | 5.25 hours |
| 7. Testing | 2 hours | 7.25 hours |

**Total**: 7-8 hours

---

## Next Steps

1. **Create feature branch**: `agentx/backend-tray`
2. **Implement steps 1-6** (5 hours)
3. **Test thoroughly** (2 hours)
4. **Create PR** with before/after screenshots
5. **Document in changelog**

---

## Questions Before Starting

- [x] Icon format: Use `.ico` for Windows? Yes
- [x] Keep windowsgui flag? Yes - compatible with tray
- [x] Tray behavior when no frontends? Launch new frontend on click
- [x] Event mechanism? Use existing WebSocket + WAVESRV-EVENT

**Ready to implement**: ✅ YES

---

## References

- [systray library](https://github.com/getlantern/systray)
- [Go embed directive](https://pkg.go.dev/embed)
- [Windows system tray](https://learn.microsoft.com/en-us/windows/win32/shell/notification-area)
- [Multi-window implementation](./MULTI_WINDOW_IMPLEMENTATION.md)
