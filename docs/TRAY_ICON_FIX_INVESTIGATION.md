# System Tray Icon Fix - Investigation & Implementation Plan

**Date**: 2026-02-13
**Status**: 🔍 Investigation Complete, Ready for Implementation
**Version**: 0.27.2+

---

## Problem Statement (Corrected)

### Initial Confusion

**User reported**: "Every instance shows 2 additional icons in the taskbar"

**AgentX misunderstood**: Thought user meant Windows taskbar

**Actual issue**: User meant **system tray** (notification area), not taskbar

### Corrected Problem

When running multiple AgentMux instances:
- **Expected**: 1 system tray icon globally (for all instances)
- **Actual**: 2 system tray icons PER instance

**Example with 4 instances**:
- System tray icons: **8** (should be 1)
- Taskbar icons: 4 (correct - one per window)

---

## Root Cause Analysis

### Issue 1: Duplicate Tray Icons Per Instance (FIXED ✅)

**Cause**: Two places creating tray icons in EACH instance:

1. **Declarative** (`tauri.conf.json`):
```json
"trayIcon": {
  "iconPath": "icons/icon.png",
  "iconAsTemplate": true
}
```

2. **Programmatic** (`src-tauri/src/lib.rs` line 113):
```rust
if let Err(e) = tray::build_tray(&handle) {
    tracing::error!("Failed to build system tray: {}", e);
}
```

**Fix Applied**: Removed declarative config, keeping only programmatic (has menu functionality)

**Result**: Now 1 tray icon per instance (instead of 2)

### Issue 2: Multiple Tray Icons Across Instances (TODO ⏳)

**Cause**: Each Tauri frontend process creates its own tray icon

**Current architecture**:
```
Instance 1 (Tauri Process 1) → Tray Icon 1
Instance 2 (Tauri Process 2) → Tray Icon 2
Instance 3 (Tauri Process 3) → Tray Icon 3
Instance 4 (Tauri Process 4) → Tray Icon 4

Shared Backend (agentmuxsrv) → No tray management
```

**Desired architecture**:
```
Instance 1 (Tauri Process 1) ┐
Instance 2 (Tauri Process 2) │
Instance 3 (Tauri Process 3) ├─► Single Tray Icon (owned by ???)
Instance 4 (Tauri Process 4) ┘

Shared Backend (agentmuxsrv) → ??? Maybe owns tray?
```

---

## Investigation: Options for Single Global Tray Icon

### Option 1: Backend-Managed Tray ⭐ (RECOMMENDED)

**Concept**: Move tray icon ownership from frontend to backend

**Architecture**:
```
┌─────────────────┐
│   Frontend 1    │──┐
│   (No Tray)     │  │
└─────────────────┘  │
                     │
┌─────────────────┐  │    WebSocket
│   Frontend 2    │  ├──►┌──────────────────┐
│   (No Tray)     │  │   │  agentmuxsrv     │
└─────────────────┘  │   │                  │
                     │   │  + Tray Icon     │◄── Single tray
┌─────────────────┐  │   │  + Tray Menu     │
│   Frontend 3    │──┘   │  + Click Handler │
│   (No Tray)     │      └──────────────────┘
└─────────────────┘
```

**Pros**:
- ✅ Single tray icon guaranteed (one backend = one tray)
- ✅ Tray persists when all frontends closed (until backend shuts down)
- ✅ Consistent with backend-owns-state architecture
- ✅ Already have Go UI libraries available (e.g., `github.com/getlantern/systray`)

**Cons**:
- ❌ Backend currently headless (built with `-H windowsgui`)
- ❌ Tray icon would use different icon rendering than Tauri
- ❌ Click events need to communicate to frontend (which window to show?)
- ❌ More complex implementation (new dependency, IPC)

**Implementation Complexity**: 🔴 High

**Dependencies**:
- Go tray library: `github.com/getlantern/systray` or `github.com/caseymrm/menuet`
- Backend-to-frontend IPC for tray events

**Backend changes**:
```go
// cmd/server/main-server.go
import "github.com/getlantern/systray"

func onReady() {
    systray.SetIcon(iconData)
    systray.SetTitle("AgentMux")

    mShow := systray.AddMenuItem("Show Window", "")
    mQuit := systray.AddMenuItem("Quit", "")

    go func() {
        for {
            select {
            case <-mShow.ClickedCh:
                // Send event to all connected frontends
                broadcastEvent("show-window")
            case <-mQuit.ClickedCh:
                // Shutdown all frontends + backend
                shutdown()
            }
        }
    }()
}
```

---

### Option 2: First-Instance Pattern

**Concept**: First frontend instance owns tray, subsequent instances skip tray creation

**Architecture**:
```
┌─────────────────┐
│   Frontend 1    │──► Tray Icon (OWNER) ◄── Single tray
│   (First)       │
└─────────────────┘

┌─────────────────┐
│   Frontend 2    │──► No Tray
│   (Secondary)   │
└─────────────────┘

┌─────────────────┐
│   Frontend 3    │──► No Tray
│   (Secondary)   │
└─────────────────┘
```

**Pros**:
- ✅ Simpler than backend approach
- ✅ Keeps tray in Rust/Tauri (consistent theming)
- ✅ Moderate implementation complexity

**Cons**:
- ❌ Tray disappears when first instance closes (even if others open)
- ❌ Need coordination mechanism (lock file, named mutex)
- ❌ Race condition if two instances launch simultaneously

**Implementation Complexity**: 🟡 Medium

**Coordination mechanism**:
```rust
// Check if we're the first instance
let tray_lock_file = config_dir.join(".tray-owner.lock");

if !tray_lock_file.exists() {
    // We're first! Create lock and tray
    std::fs::write(&tray_lock_file, process::id().to_string())?;

    if let Err(e) = tray::build_tray(&handle) {
        tracing::error!("Failed to build system tray: {}", e);
    }

    // Clean up lock on exit
    // ... (problem: what if we crash?)
} else {
    tracing::info!("Tray already owned by another instance, skipping");
}
```

**Problem**: If first instance crashes, tray disappears AND lock file remains (blocks future trays)

---

### Option 3: Single-Instance Application Mode

**Concept**: Tauri runs in single-instance mode. Launching again creates new window in existing process.

**Architecture**:
```
┌─────────────────────────────────┐
│   Tauri Process (Single)        │
│                                  │
│   ┌──────────┐  ┌──────────┐   │
│   │ Window 1 │  │ Window 2 │   │
│   └──────────┘  └──────────┘   │
│                                  │
│   Tray Icon (Single)             │◄── Always exactly 1
└─────────────────────────────────┘
```

**Pros**:
- ✅ Guaranteed single tray (single process = single tray)
- ✅ Tauri has built-in support (`tauri-plugin-single-instance`)
- ✅ Cleanest solution architecturally
- ✅ Matches how VS Code, Chrome, etc. work

**Cons**:
- ❌ Major architectural change (multiple processes → single process)
- ❌ Affects entire window management system
- ❌ May require rewrite of multi-window coordination

**Implementation Complexity**: 🔴 Very High

**Tauri plugin**:
```rust
// src-tauri/src/lib.rs
.plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
    // Called when user launches second instance
    // Instead of new process, open new window in THIS process
    let window = create_new_window(app);
    window.show().unwrap();
}))
```

**References**:
- https://github.com/tauri-apps/plugins-workspace/tree/v2/plugins/single-instance

---

### Option 4: Coordinator Process

**Concept**: Lightweight coordinator owns tray, frontends communicate via IPC

**Architecture**:
```
┌─────────────────┐
│  Tray Manager   │──► Tray Icon (OWNER) ◄── Single tray
│  (Lightweight)  │
└────────┬────────┘
         │ IPC
    ┌────┴─────┬─────────┐
    │          │         │
┌───▼──┐  ┌───▼──┐  ┌──▼───┐
│ UI 1 │  │ UI 2 │  │ UI 3 │
└──────┘  └──────┘  └──────┘
```

**Pros**:
- ✅ Clean separation of concerns
- ✅ Tray never disappears (manager runs independently)

**Cons**:
- ❌ New process/service to manage
- ❌ Complex IPC setup
- ❌ Lifecycle management (who starts/stops coordinator?)

**Implementation Complexity**: 🔴 Very High

---

## Comparison Matrix

| Option | Complexity | Single Tray? | Survives Window Close? | Tauri Native? |
|--------|-----------|--------------|------------------------|---------------|
| **Backend Tray** | High | ✅ | ✅ | ❌ |
| **First-Instance** | Medium | ✅ | ❌ | ✅ |
| **Single-Instance Mode** | Very High | ✅ | ✅ | ✅ |
| **Coordinator Process** | Very High | ✅ | ✅ | ❌ |

---

## Recommended Approach

### Phase 1: Quick Fix (DONE ✅)
Remove duplicate tray icons per instance (declarative + programmatic)

**Status**: Implemented in v0.27.2

### Phase 2: Hybrid Approach (RECOMMENDED)

**Combine First-Instance + Backend Fallback**:

1. **First instance**: Creates tray icon (Tauri-based)
2. **Subsequent instances**: No tray
3. **Tray lifecycle**:
   - Lock file coordinates ownership
   - When first instance closes, transfer ownership to next alive instance
   - If no instances remain, backend could spawn minimal tray (optional)

**Implementation**:
```rust
// src-tauri/src/lib.rs setup

let tray_lock = TrayLockManager::new(config_dir);

match tray_lock.try_acquire() {
    Ok(lock) => {
        // We own the tray
        if let Err(e) = tray::build_tray(&handle) {
            tracing::error!("Failed to build tray: {}", e);
        }

        // Store lock in app state
        app.manage(lock);

        // Release on exit
        // ... cleanup handler
    }
    Err(_) => {
        tracing::info!("Tray owned by another instance");
    }
}
```

**Lock manager**:
```rust
// src-tauri/src/tray_lock.rs

use std::fs::{File, OpenOptions};
use std::io::Write;

pub struct TrayLockManager {
    lock_file: PathBuf,
    lock: Option<File>,
}

impl TrayLockManager {
    pub fn try_acquire(&mut self) -> Result<(), Error> {
        // Atomic create-if-not-exists
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)  // Fails if exists
            .open(&self.lock_file)?;

        // Write our PID
        writeln!(file, "{}", process::id())?;
        self.lock = Some(file);
        Ok(())
    }
}

impl Drop for TrayLockManager {
    fn drop(&mut self) {
        // Release lock on exit
        std::fs::remove_file(&self.lock_file).ok();
    }
}
```

---

## Alternative: Notification-Only Tray

**Simplify**: Instead of interactive tray, use notification-only mode

```rust
// Minimal tray - no menu, no clicks
// Just shows app is running
TrayIconBuilder::new()
    .icon(icon.clone())
    .tooltip("AgentMux")
    .build(app)?;
```

**Pros**: Simpler (no menu event handling)
**Cons**: Less useful (can't show/hide windows from tray)

---

## Implementation Plan

### Step 1: Tray Lock Manager (2-3 hours)
- Create `src-tauri/src/tray_lock.rs`
- Implement `TrayLockManager` with atomic file lock
- Handle cleanup on drop
- Add PID staleness check (if lock file exists, verify PID still alive)

### Step 2: Conditional Tray Creation (1 hour)
- Modify `src-tauri/src/lib.rs` setup
- Wrap `tray::build_tray()` in lock acquisition
- Store lock in app state
- Test with multiple instances

### Step 3: Lock Transfer on Exit (2 hours)
- Detect when tray owner is closing
- Signal next available instance to take over
- Use IPC or file-based signaling
- Graceful handoff

### Step 4: Testing (2 hours)
- Launch 4 instances → verify 1 tray
- Close instances in random order → verify tray persists
- Crash test → verify stale lock cleanup
- Rapid launch test → verify no race conditions

**Total Estimate**: 7-8 hours

---

## Future Enhancements

### Backend Tray (v2)
If first-instance pattern proves problematic, migrate to backend-owned tray:
- Add `systray` dependency to Go backend
- Remove frontend tray code
- Implement backend→frontend event bridge

### Single-Instance Mode (v3)
For ultimate simplicity, migrate to Tauri single-instance plugin:
- Major refactor, but cleanest architecture
- Matches industry standard (Chrome, VS Code, etc.)

---

## Files to Modify

### New Files
- `src-tauri/src/tray_lock.rs` - Lock coordination

### Modified Files
- `src-tauri/src/lib.rs` - Conditional tray creation
- `src-tauri/Cargo.toml` - Dependencies (if needed)
- `docs/TRAY_ICON_FIX_INVESTIGATION.md` - This file

### Unchanged Files
- `src-tauri/src/tray.rs` - Tray menu logic (reused)
- `src-tauri/tauri.conf.json` - Already fixed (removed declarative tray)

---

## Testing Checklist

- [ ] Single instance: 1 tray icon
- [ ] 4 instances: 1 tray icon
- [ ] Close first instance: tray persists
- [ ] Close all instances: tray disappears
- [ ] Tray menu "Show/Hide" works
- [ ] Tray menu "Quit" closes all instances
- [ ] Stale lock recovery (kill -9 first instance)
- [ ] Rapid launches (no race conditions)

---

## Questions for User

1. **Tray behavior preference**:
   - Option A: Tray disappears when last window closes
   - Option B: Tray persists even with no windows (backend keeps it)

2. **Tray menu actions**:
   - "Show Window" → Which window? (first, last, all?)
   - "New Window" → Should tray menu have this?

3. **Implementation priority**:
   - Quick fix (first-instance pattern) → ~8 hours
   - OR wait for full refactor (single-instance mode) → ~40 hours

---

## References

- [Tauri Tray Icon Docs](https://v2.tauri.app/develop/system-tray/)
- [Tauri Single Instance Plugin](https://github.com/tauri-apps/plugins-workspace/tree/v2/plugins/single-instance)
- [Go systray library](https://github.com/getlantern/systray)
- [Multi-window implementation](./MULTI_WINDOW_IMPLEMENTATION.md)

---

## Summary

**Immediate Fix** (v0.27.2): Removed duplicate tray icons per instance (2→1)

**Next Fix** (v0.28.0): Implement first-instance tray ownership pattern (N→1 across instances)

**Long-term** (v0.30.0+): Consider single-instance mode for cleanest architecture
