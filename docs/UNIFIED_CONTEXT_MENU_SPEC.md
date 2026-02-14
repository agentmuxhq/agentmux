# Unified Context Menu Specification

**Date**: 2026-02-13
**Status**: 📋 Spec/Design Phase
**Target Version**: 0.28.0
**Estimated Effort**: 4-6 hours

---

## Objective

Implement synchronized right-click context menus for both:
1. **System tray** (backend-managed, Go/systray)
2. **Tab bar** (frontend, TypeScript/React)

**Phase 1 Goal**: Display version information only

---

## Current State

### Frontend (Tab Bar) ✅

**Already implemented**:
- Context menu system via `MenuBuilder` class
- Tab bar right-click shows version + widget toggles
- Architecture: `frontend/app/menu/menu-builder.ts`

**Example**:
```typescript
// frontend/app/menu/base-menus.ts
export function createTabBarBaseMenu(): MenuBuilder {
    const menu = new MenuBuilder();
    const aboutDetails = getApi().getAboutModalDetails();
    const version = aboutDetails.version;

    menu.add({
        label: `AgentMux v${version}`,
        click: () => {
            navigator.clipboard.writeText(version);
            getApi().sendLog(`Version ${version} copied to clipboard`);
        },
    });

    return menu;
}
```

**Display**:
```
Tab Bar (right-click)
├── AgentMux v0.27.3
├── ──────────────
└── Widgets (...)
```

### Backend (System Tray) ⚠️

**Current state**: Stub only (icon appears, no menu)

**Needs**: Menu implementation using `systray.AddMenuItem()`

**Systray API**:
```go
import "github.com/getlantern/systray"

func onTrayReady() {
    systray.SetIcon(iconData)
    systray.SetTooltip("AgentMux")

    // Add menu items
    mVersion := systray.AddMenuItem("AgentMux v0.27.3", "")
    mQuit := systray.AddMenuItem("Quit", "Quit AgentMux")

    // Handle clicks
    go func() {
        for {
            select {
            case <-mVersion.ClickedCh:
                // Copy version to clipboard?
            case <-mQuit.ClickedCh:
                systray.Quit()
            }
        }
    }()
}
```

---

## Problem: Menu Synchronization

The menus are defined in **different languages** with **different APIs**:

| Location | Language | API | Menu Definition |
|----------|----------|-----|-----------------|
| Tab Bar | TypeScript | Tauri context menu | `MenuBuilder` class |
| System Tray | Go | systray library | `systray.AddMenuItem()` |

**Challenge**: How to keep them synchronized?

---

## Architecture Options

### Option 1: Duplicate Definitions (SIMPLEST) ⭐

**Concept**: Define menu separately in both places

**Frontend**:
```typescript
// frontend/app/menu/base-menus.ts
export function createTabBarBaseMenu(): MenuBuilder {
    const version = getApi().getAboutModalDetails().version;

    return new MenuBuilder()
        .add({
            label: `AgentMux v${version}`,
            click: () => {
                navigator.clipboard.writeText(version);
            },
        });
}
```

**Backend**:
```go
// cmd/server/tray.go
func buildTrayMenu() {
    version := WaveVersion

    mVersion := systray.AddMenuItem(
        fmt.Sprintf("AgentMux v%s", version),
        "Click to copy version",
    )

    go func() {
        <-mVersion.ClickedCh
        copyToClipboard(version) // Windows: use syscall or exec
    }()
}
```

**Pros**:
- ✅ Simple to implement
- ✅ No IPC needed
- ✅ Each menu uses native API idiomatically
- ✅ Independent failures (frontend menu works even if backend menu fails)

**Cons**:
- ❌ Two definitions to maintain
- ❌ Can drift out of sync
- ❌ Different behavior possible (one copies to clipboard, other doesn't)

**Verdict**: ✅ **Recommended for Phase 1** - Get menus working quickly, refactor later if needed

---

### Option 2: Shared JSON Config

**Concept**: Define menu structure in JSON, load in both places

**Menu definition**:
```json
// menu-config.json
{
  "version": "1.0",
  "items": [
    {
      "id": "version",
      "label": "AgentMux v{VERSION}",
      "action": "copy-version",
      "tooltip": "Click to copy version"
    },
    {
      "type": "separator"
    },
    {
      "id": "quit",
      "label": "Quit",
      "action": "quit"
    }
  ]
}
```

**Frontend loader**:
```typescript
// Load JSON, convert to MenuBuilder
const menuConfig = await loadMenuConfig();
const menu = new MenuBuilder();

menuConfig.items.forEach(item => {
    if (item.type === 'separator') {
        menu.separator();
    } else {
        menu.add({
            label: item.label.replace('{VERSION}', version),
            click: () => handleAction(item.action),
        });
    }
});
```

**Backend loader**:
```go
// Load JSON, convert to systray menu
menuConfig := loadMenuConfig()

for _, item := range menuConfig.Items {
    if item.Type == "separator" {
        systray.AddSeparator()
    } else {
        label := strings.Replace(item.Label, "{VERSION}", version, -1)
        menuItem := systray.AddMenuItem(label, item.Tooltip)
        go handleMenuClick(menuItem, item.Action)
    }
}
```

**Pros**:
- ✅ Single source of truth
- ✅ Easy to add new menu items (edit JSON)
- ✅ No code changes needed for menu structure
- ✅ Could be user-configurable

**Cons**:
- ❌ More complex (parser + loader + action dispatcher)
- ❌ Limited to common denominator of both APIs
- ❌ Harder to implement dynamic menus (widgets, etc.)
- ❌ Need to bundle JSON in both binaries

**Verdict**: 🟡 **Consider for future** - Good for complex menus, overkill for Phase 1

---

### Option 3: Frontend-Driven IPC

**Concept**: Frontend sends menu definition to backend via WebSocket

**Flow**:
```
Frontend loads → Sends menu JSON to backend → Backend builds systray menu
```

**Frontend**:
```typescript
// Send menu definition to backend
const menuDef = createTabBarBaseMenu().build();
getApi().updateTrayMenu(menuDef);
```

**Backend**:
```go
// Receive menu definition via WebSocket
type MenuUpdate struct {
    Items []MenuItem `json:"items"`
}

func handleMenuUpdate(update MenuUpdate) {
    // Clear existing menu
    clearTrayMenu()

    // Rebuild from frontend definition
    for _, item := range update.Items {
        systray.AddMenuItem(item.Label, item.Tooltip)
    }
}
```

**Pros**:
- ✅ Frontend is source of truth
- ✅ Can update tray menu dynamically
- ✅ Frontend has full context (widgets, config)

**Cons**:
- ❌ Complex IPC protocol
- ❌ Tray menu depends on frontend being connected
- ❌ What if no frontends are connected?
- ❌ systray library doesn't support clearing/rebuilding menus easily

**Verdict**: ❌ **Not recommended** - Over-engineered, tray should be independent

---

### Option 4: Backend-Driven Query

**Concept**: Backend defines menu, frontend queries it

**Flow**:
```
Frontend requests menu → Backend returns menu JSON → Frontend builds
```

**Frontend**:
```typescript
// Query backend for menu structure
const menuDef = await getApi().getTrayMenuDefinition();
const menu = buildMenuFromDefinition(menuDef);
```

**Backend**:
```go
// Expose menu definition via RPC
func GetTrayMenuDefinition() MenuDefinition {
    return MenuDefinition{
        Items: []MenuItem{
            {Label: fmt.Sprintf("AgentMux v%s", WaveVersion)},
            {Type: "separator"},
            {Label: "Quit"},
        },
    }
}
```

**Pros**:
- ✅ Backend is source of truth
- ✅ Version always from backend (authoritative)

**Cons**:
- ❌ Frontend-specific items (widgets) harder to include
- ❌ Extra RPC call on every menu open
- ❌ Backend doesn't have frontend context

**Verdict**: ❌ **Not recommended** - Backend shouldn't define frontend menus

---

## Recommended Approach: Duplicate Definitions (Option 1)

### Why Option 1?

1. **Simplest to implement** - No new infrastructure
2. **Best practices** - Each component owns its menu
3. **Fail-safe** - Menus work independently
4. **Flexible** - Can diverge if needed (tray has "Quit", tabbar doesn't need it)

### Implementation Strategy

#### Phase 1: Version Display Only

**Tab bar** (already done):
```
AgentMux v0.27.3 (click to copy)
```

**System tray** (implement):
```
AgentMux v0.27.3 (click to copy)
```

#### Future Phases:

**Phase 2**: Add common actions
- Both: Version (copy)
- Both: About
- Tray only: Show All Windows
- Tray only: New Window
- Tray only: Quit

**Phase 3**: Diverge as needed
- Tab bar: Widget toggles, workspace actions
- System tray: Window management, quit

---

## Implementation Plan

### Step 1: Update Tray Menu (Go) - 2 hours

**File**: `cmd/server/tray.go`

**Changes**:
```go
package main

import (
    _ "embed"
    "fmt"
    "log"

    "github.com/getlantern/systray"
)

//go:embed assets/icon.ico
var iconData []byte

func InitTray() {
    go systray.Run(onTrayReady, onTrayExit)
}

func onTrayReady() {
    systray.SetIcon(iconData)
    systray.SetTitle("AgentMux")
    systray.SetTooltip("AgentMux - AI Terminal")

    buildTrayMenu()
}

func buildTrayMenu() {
    version := WaveVersion

    // Version item (click to copy)
    mVersion := systray.AddMenuItem(
        fmt.Sprintf("AgentMux v%s", version),
        "Click to copy version to clipboard",
    )

    go func() {
        for {
            select {
            case <-mVersion.ClickedCh:
                log.Printf("[tray] Version clicked: %s", version)
                copyToClipboard(version)
            }
        }
    }()
}

func onTrayExit() {
    log.Println("[tray] System tray exiting")
}
```

### Step 2: Implement Clipboard Copy (Windows) - 1 hour

**File**: `cmd/server/clipboard_windows.go`

**Approach**: Use Windows syscall or exec `clip.exe`

**Option A: exec clip.exe (simpler)**:
```go
//go:build windows

package main

import (
    "log"
    "os/exec"
    "strings"
)

func copyToClipboard(text string) {
    cmd := exec.Command("clip")
    cmd.Stdin = strings.NewReader(text)

    if err := cmd.Run(); err != nil {
        log.Printf("[tray] Failed to copy to clipboard: %v", err)
        return
    }

    log.Printf("[tray] Copied to clipboard: %s", text)
}
```

**Option B: syscall (more complex, no dependencies)**:
```go
//go:build windows

package main

import (
    "syscall"
    "unsafe"
)

var (
    user32           = syscall.NewLazyDLL("user32.dll")
    openClipboard    = user32.NewProc("OpenClipboard")
    closeClipboard   = user32.NewProc("CloseClipboard")
    emptyClipboard   = user32.NewProc("EmptyClipboard")
    setClipboardData = user32.NewProc("SetClipboardData")
)

func copyToClipboard(text string) {
    // Implementation using syscalls...
    // (more complex, see: github.com/atotto/clipboard for reference)
}
```

**Recommendation**: Use **Option A** (clip.exe) for simplicity

### Step 3: Linux/macOS Stubs - 30 min

**File**: `cmd/server/clipboard_unix.go`

```go
//go:build !windows

package main

import (
    "log"
    "os/exec"
)

func copyToClipboard(text string) {
    // macOS: pbcopy
    // Linux: xclip or xsel
    var cmd *exec.Cmd

    // Try pbcopy (macOS)
    if _, err := exec.LookPath("pbcopy"); err == nil {
        cmd = exec.Command("pbcopy")
    } else if _, err := exec.LookPath("xclip"); err == nil {
        cmd = exec.Command("xclip", "-selection", "clipboard")
    } else if _, err := exec.LookPath("xsel"); err == nil {
        cmd = exec.Command("xsel", "--clipboard", "--input")
    } else {
        log.Println("[tray] No clipboard utility found")
        return
    }

    cmd.Stdin = strings.NewReader(text)
    if err := cmd.Run(); err != nil {
        log.Printf("[tray] Failed to copy: %v", err)
    }
}
```

### Step 4: Update Tab Bar Menu (Already Done) ✅

**Current state**: Tab bar already shows version and copies to clipboard

**No changes needed for Phase 1**

### Step 5: Testing - 1 hour

**Test matrix**:

| Action | Tab Bar | System Tray | Expected |
|--------|---------|-------------|----------|
| Right-click | Shows menu ✅ | Shows menu 🔨 | Menu appears |
| Click "AgentMux v0.27.3" | Copies version ✅ | Copies version 🔨 | Version in clipboard |
| Paste | "0.27.3" appears ✅ | "0.27.3" appears 🔨 | Clipboard works |

🔨 = To be implemented

---

## Code Organization

### New Files

```
cmd/server/
├── tray.go                     (update: add buildTrayMenu)
├── clipboard_windows.go        (new: Windows clipboard)
└── clipboard_unix.go           (new: macOS/Linux clipboard)
```

### Modified Files

```
cmd/server/tray.go              (add menu implementation)
```

### Unchanged Files

```
frontend/app/menu/base-menus.ts  (already has version menu)
frontend/app/tab/tabbar.tsx      (already wired up)
```

---

## Future Extensions

### Phase 2: Common Actions

**Both menus**:
```
AgentMux v0.27.3 (copy)
About AgentMux
```

### Phase 3: Tray-Specific Actions

**System tray only**:
```
AgentMux v0.27.3
──────────────
Show All Windows
New Window
──────────────
About
──────────────
Quit AgentMux
```

### Phase 4: Tab Bar-Specific Actions

**Tab bar only**:
```
AgentMux v0.27.3
──────────────
Widgets
  ☑ AI Panel
  ☐ Notifications
  ☐ Sysinfo
──────────────
Edit widgets.json
```

---

## Dependencies

**No new Go dependencies** - systray already included

**Clipboard utilities** (runtime):
- Windows: `clip.exe` (built-in) ✅
- macOS: `pbcopy` (built-in) ✅
- Linux: `xclip` or `xsel` (may need install)

---

## Testing Checklist

- [ ] Tray menu appears on right-click
- [ ] "AgentMux v0.27.3" shows correct version
- [ ] Click version copies to clipboard
- [ ] Paste shows version text
- [ ] Tab bar menu still works (regression test)
- [ ] Windows: clipboard works
- [ ] macOS: clipboard works (if available)
- [ ] Linux: clipboard works (if available)

---

## Rollback Plan

If tray menu causes issues:
1. Comment out `buildTrayMenu()` call
2. Tray reverts to stub (icon only, no menu)
3. Tab bar unaffected

**Rollback time**: 5 minutes

---

## Success Criteria

- [ ] Tray right-click shows menu
- [ ] Menu displays "AgentMux v{VERSION}"
- [ ] Clicking version copies to clipboard
- [ ] Tab bar menu unchanged (regression)
- [ ] Works on Windows (primary platform)
- [ ] Graceful degradation on macOS/Linux

---

## Timeline

| Step | Time | Cumulative |
|------|------|-----------|
| 1. Update tray menu | 2 hours | 2 hours |
| 2. Clipboard copy (Windows) | 1 hour | 3 hours |
| 3. Unix stubs | 30 min | 3.5 hours |
| 4. Tab bar (already done) | 0 min | 3.5 hours |
| 5. Testing | 1 hour | 4.5 hours |

**Total**: 4.5 hours

---

## Open Questions

1. **Clipboard on Linux**: Require xclip/xsel or fail silently?
   - **Decision**: Fail silently with log message (graceful degradation)

2. **Menu separator**: Add separator before future items?
   - **Decision**: No separator needed for single item

3. **Tooltip**: Show tooltip on hover?
   - **Decision**: Yes - "Click to copy version to clipboard"

4. **Log message**: Log when version copied?
   - **Decision**: Yes - helps debugging

---

## Alternative: Use Clipboard Library

**Option**: Use `github.com/atotto/clipboard` instead of manual implementation

**Pros**:
- ✅ Cross-platform
- ✅ Well-tested
- ✅ Simple API: `clipboard.WriteAll(text)`

**Cons**:
- ❌ External dependency
- ❌ Requires CGO on some platforms

**Verdict**: 🟡 **Consider** - If manual implementation problematic, switch to library

---

## References

- [systray library](https://github.com/getlantern/systray)
- [systray example](https://github.com/getlantern/systray/blob/master/example/main.go)
- [Frontend MenuBuilder](../frontend/app/menu/menu-builder.ts)
- [Tab bar menu](../frontend/app/menu/base-menus.ts)
- [atotto/clipboard](https://github.com/atotto/clipboard) (alternative)

---

## Summary

**Approach**: Duplicate menu definitions (Option 1)

**Phase 1**: Version display with clipboard copy

**Implementation**:
- Tray: Add menu via systray library
- Clipboard: Use `clip.exe` on Windows, graceful degradation elsewhere
- Tab bar: Already implemented ✅

**Estimated effort**: 4.5 hours

**Next step**: Review spec → Implement → Test → PR
