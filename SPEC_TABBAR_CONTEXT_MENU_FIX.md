# TabBar Context Menu Regression - Fix Specification

**Version:** 1.0
**Status:** Investigation Complete
**Target Release:** 0.28.2
**Created:** 2026-02-15
**Author:** AgentX

---

## Executive Summary

TabBar right-click context menu **was working briefly** (commit 6fddfae - Feb 14, 2026) but has since **regressed to showing default Windows native menu** (minimize, maximize, close).

**Root Cause:** Conflict between `data-tauri-drag-region` attribute and context menu event handling in Tauri v2.

**Impact:** Users cannot access version info, widget toggles, or any custom context menu items from tabbar right-click.

**💡 NEW APPROACH (Option D):** Instead of choosing between native OR custom menu, **combine both** into a hybrid menu:
- Window controls (Minimize, Maximize, Close)
- Separator
- AgentMux version (copyable)
- Widget toggles
- Edit widgets.json

**Effort:**
- **Option D (Hybrid):** Medium - 3-4 hours (backend commands + menu updates)
- **Option A (Fallback):** Low - 1-2 hours (simple attribute change)

---

## Problem Statement

### Current Behavior (Broken)

1. Right-click on tabbar wrapper → **Windows native menu appears** (minimize, maximize, close, etc.)
2. Custom AgentMux context menu **never shows**
3. `handleTabBarContextMenu` function **exists and is called**, but menu doesn't render

### Expected Behavior (Previously Working)

1. Right-click on tabbar wrapper → **AgentMux custom menu appears**
2. Menu shows:
   - AgentMux v{version} (click to copy)
   - ─────────────────
   - Widgets section (toggle checkboxes)
   - Edit widgets.json

### Timeline

| Date | Commit | Status |
|------|--------|--------|
| Feb 13 | 9d6ca68 | ✅ Context menu system implemented |
| Feb 14 | 6fddfae | ✅ **Working** - Real context menu with version display |
| Feb 15 | 71b992b | ❌ **Broken** - Widget reordering PR (unrelated, but context menu stopped working) |
| Feb 15 | 08dadce | ❌ **Still broken** - Zoom PR merged |
| Feb 15 | **Now** | ❌ **Still broken** - Showing native Windows menu |

---

## Root Cause Analysis

### The Conflict

**File:** `frontend/app/tab/tabbar.tsx:681`

```tsx
<div ref={tabbarWrapperRef}
     className="tab-bar-wrapper"
     data-tauri-drag-region           // ⚠️ PROBLEM
     onContextMenu={handleTabBarContextMenu}>
```

**Issue:** In Tauri v2, `data-tauri-drag-region` enables window dragging but **intercepts all mouse events** including right-click, preventing custom context menus from working.

### Why It Worked Before

The context menu implementation was added in commit 6fddfae (Feb 14), and the code structure was correct. However, at some point (possibly during widget changes or another refactor), the interaction between the drag region and context menu handler broke.

### Technical Details

**Tauri Behavior:**
- Elements with `data-tauri-drag-region` get special mouse handling
- Native OS right-click behavior takes precedence over React event handlers
- `onContextMenu` event fires but menu rendering is blocked by OS-level handling

**Evidence:**
1. `handleTabBarContextMenu` function exists and is properly wired
2. `createTabBarMenu` builds menu correctly (version + widgets)
3. `ContextMenuModel.showContextMenu` calls `getApi().showContextMenu`
4. Backend `show_context_menu` command exists and works
5. **But menu never appears** - OS intercepts before Tauri menu can render

---

## Solution Options

### Option A: Remove Drag Region from Wrapper (Recommended)

**Change:**
```tsx
// BEFORE (broken)
<div ref={tabbarWrapperRef}
     className="tab-bar-wrapper"
     data-tauri-drag-region                    // ❌ Remove this
     onContextMenu={handleTabBarContextMenu}>

// AFTER (working)
<div ref={tabbarWrapperRef}
     className="tab-bar-wrapper"
     onContextMenu={handleTabBarContextMenu}>  // ✅ No drag region
```

**Move drag region to specific child elements:**
```tsx
<WindowDrag ref={draggerLeftRef}
            className="left"
            data-tauri-drag-region />  // ✅ Explicit drag area

<WindowDrag ref={draggerRightRef}
            className="right"
            data-tauri-drag-region />  // ✅ Explicit drag area
```

**Pros:**
- ✅ Clean separation of concerns (drag areas vs interactive areas)
- ✅ More granular control over where dragging works
- ✅ Context menu works on entire tabbar wrapper
- ✅ No special cases or workarounds

**Cons:**
- ⚠️ Less of the tabbar is draggable (only WindowDrag areas)
- ⚠️ Might change UX if users expect to drag from anywhere on tabbar

### Option B: Add Explicit Non-Drag Region

**Change:**
```tsx
<div ref={tabbarWrapperRef}
     className="tab-bar-wrapper"
     data-tauri-drag-region
     data-tauri-context-menu-region="true">    // ✅ New attribute
```

**Backend:** Add special handling in Tauri window code to allow context menus in drag regions.

**Pros:**
- ✅ Keeps entire tabbar draggable
- ✅ Explicit marking of context menu zones

**Cons:**
- ❌ Requires backend changes (custom Tauri attribute handling)
- ❌ More complex implementation
- ❌ Not standard Tauri behavior

### Option C: Prevent Default on Wrapper, Delegate to Children

**Change:**
```tsx
const handleTabBarContextMenu = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();  // ✅ Stop event propagation

    // Override Tauri drag region behavior
    const menu = createTabBarMenu(fullConfig);
    ContextMenuModel.showContextMenu(menu.build(), e);
}, [fullConfig]);
```

**Pros:**
- ✅ Minimal code changes
- ✅ Keeps tabbar structure unchanged

**Cons:**
- ⚠️ `e.preventDefault()` already exists - not working
- ❌ React event handling happens **after** Tauri native handling
- ❌ Won't work because OS intercepts before React events fire

### Option D: Hybrid Menu (Standard + Custom) - **NEW RECOMMENDATION**

**Change:**
```tsx
// Keep drag region
<div ref={tabbarWrapperRef}
     className="tab-bar-wrapper"
     data-tauri-drag-region                    // ✅ Keep this
     onContextMenu={handleTabBarContextMenu}>
```

**Update menu builder:**
```tsx
// frontend/app/menu/base-menus.ts
export function createTabBarMenu(fullConfig: any): MenuBuilder {
    const menu = new MenuBuilder();

    // Add standard window controls first
    menu.add({
        label: "Minimize",
        click: () => getApi().minimizeWindow(),
    });

    menu.add({
        label: "Maximize",
        click: () => getApi().maximizeWindow(),
    });

    menu.add({
        label: "Close",
        click: () => getApi().closeWindow(),
    });

    menu.separator();

    // Then add custom items (version)
    const aboutDetails = getApi().getAboutModalDetails();
    menu.add({
        label: `AgentMux v${aboutDetails.version}`,
        click: () => {
            navigator.clipboard.writeText(aboutDetails.version);
            getApi().sendLog(`Version ${aboutDetails.version} copied to clipboard`);
        },
    });

    menu.separator();

    // Then add widgets
    menu.merge(createWidgetsMenu(fullConfig));

    return menu;
}
```

**Add backend window control commands:**
```rust
// src-tauri/src/commands/window.rs

#[tauri::command]
pub fn minimize_window(window: tauri::Window) -> Result<(), String> {
    window
        .minimize()
        .map_err(|e| format!("Failed to minimize: {}", e))
}

#[tauri::command]
pub fn maximize_window(window: tauri::Window) -> Result<(), String> {
    if window.is_maximized()? {
        window
            .unmaximize()
            .map_err(|e| format!("Failed to restore: {}", e))
    } else {
        window
            .maximize()
            .map_err(|e| format!("Failed to maximize: {}", e))
    }
}
```

**Update API types:**
```tsx
// frontend/types/custom.d.ts
type AgentMuxApiType = {
    // ... existing methods
    minimizeWindow: () => void; // minimize-window
    maximizeWindow: () => void; // maximize-window (toggles restore)
}
```

**Pros:**
- ✅ Users get familiar window controls (minimize, maximize, close)
- ✅ PLUS our custom features (version, widgets)
- ✅ Entire tabbar stays draggable
- ✅ Menu appears exactly where users expect it (on drag region)
- ✅ Best UX - combines native feel with custom functionality

**Visual Mockup:**
```
┌─────────────────────────┐
│ Minimize                │
│ Maximize                │
│ Close                   │
├─────────────────────────┤
│ AgentMux v0.28.2  📋    │ ← Click to copy
├─────────────────────────┤
│ Widgets                 │
│   ✓ Agent               │
│   ✓ Terminal            │
│   □ Sysinfo             │
│   ✓ Help                │
├─────────────────────────┤
│ Edit widgets.json       │
└─────────────────────────┘
```

**Cons:**
- ⚠️ Requires backend commands (minimize, maximize)
- ⚠️ Requires frontend API plumbing
- ⚠️ More code changes than Option A
- ⚠️ May still need to remove drag-region if OS intercepts (see Step 5)

---

## Recommended Solution

**Choose Option D: Hybrid Menu (Standard + Custom)** ⭐ **NEW BEST OPTION**

**Rationale:**
1. ✅ Best user experience - familiar controls + custom features
2. ✅ Entire tabbar draggable (no UX regression)
3. ✅ Menu appears exactly where expected
4. ✅ Future-proof - can add more items easily
5. ⚠️ Requires backend work but provides best outcome

**Alternative:** Option A if backend changes are not acceptable

---

## Implementation Plan - Option D (Hybrid Menu) ⭐ RECOMMENDED

### Step 1: Add Backend Window Control Commands

**File:** `src-tauri/src/commands/window.rs`

Add after `close_window` function (around line 100):

```rust
/// Minimize the current window.
#[tauri::command]
pub fn minimize_window(window: tauri::Window) -> Result<(), String> {
    window
        .minimize()
        .map_err(|e| format!("Failed to minimize window: {}", e))
}

/// Toggle maximize/restore the current window.
#[tauri::command]
pub fn maximize_window(window: tauri::Window) -> Result<(), String> {
    let is_maximized = window
        .is_maximized()
        .map_err(|e| format!("Failed to check maximize state: {}", e))?;

    if is_maximized {
        window
            .unmaximize()
            .map_err(|e| format!("Failed to restore window: {}", e))
    } else {
        window
            .maximize()
            .map_err(|e| format!("Failed to maximize window: {}", e))
    }
}
```

**Export commands in `src-tauri/src/lib.rs`:**

Find the `tauri::Builder::default()` block and add to `invoke_handler`:

```diff
 .invoke_handler(tauri::generate_handler![
     // ... existing commands
     commands::window::close_window,
+    commands::window::minimize_window,
+    commands::window::maximize_window,
 ])
```

### Step 2: Add Frontend API Types

**File:** `frontend/types/custom.d.ts`

Around line 101 (after `closeWindow`):

```diff
 closeWindow: (label?: string) => Promise<void>; // close-window
+minimizeWindow: () => void; // minimize-window
+maximizeWindow: () => void; // maximize-window
 toggleDevtools: () => void; // toggle-devtools
```

### Step 3: Wire Up Frontend API Calls

**File:** `frontend/app/store/global.ts`

Find the `waveEventSubscribe` function and add handlers:

```typescript
getApi().onMinimizeWindow(() => {
    // Handler if needed, or just stub
});

getApi().onMaximizeWindow(() => {
    // Handler if needed, or just stub
});
```

**File:** `frontend/app/preload/preload-impl.ts` (if exists)**

Add IPC bindings for the new commands.

### Step 4: Update Menu Builder

**File:** `frontend/app/menu/base-menus.ts`

Replace `createTabBarMenu` function:

```typescript
/**
 * Create the complete tabbar menu (window controls + version + widgets)
 */
export function createTabBarMenu(fullConfig: any): MenuBuilder {
    const menu = new MenuBuilder();
    const aboutDetails = getApi().getAboutModalDetails();
    const version = aboutDetails.version;

    // Window controls
    menu.add({
        label: "Minimize",
        click: () => getApi().minimizeWindow(),
    });

    menu.add({
        label: "Maximize",
        click: () => getApi().maximizeWindow(),
    });

    menu.add({
        label: "Close",
        click: () => getApi().closeWindow(),
    });

    menu.separator();

    // Version info
    menu.add({
        label: `AgentMux v${version}`,
        click: () => {
            navigator.clipboard.writeText(version);
            getApi().sendLog(`Version ${version} copied to clipboard`);
        },
    });

    // Widgets
    if (fullConfig?.widgets && Object.keys(fullConfig.widgets).length > 0) {
        menu.separator();
        menu.merge(createWidgetsMenu(fullConfig));
    }

    return menu;
}
```

### Step 5: Verify Event Handler Prevents OS Menu

**File:** `frontend/app/tab/tabbar.tsx`

**Line 681:** Test if current implementation works:

```tsx
<div ref={tabbarWrapperRef} className="tab-bar-wrapper" data-tauri-drag-region onContextMenu={handleTabBarContextMenu}>
```

**Current handler (line 185):**
```tsx
const handleTabBarContextMenu = useCallback((e: React.MouseEvent) => {
    e.preventDefault();  // ✅ Already present
    const menu = createTabBarMenu(fullConfig);
    ContextMenuModel.showContextMenu(menu.build(), e);
}, [fullConfig]);
```

**⚠️ CRITICAL TEST NEEDED:**

If `data-tauri-drag-region` prevents the custom menu from showing (OS intercepts before React), then:

**Option D.1:** Remove drag-region from wrapper, add window controls to menu
```diff
-<div className="tab-bar-wrapper" data-tauri-drag-region onContextMenu={...}>
+<div className="tab-bar-wrapper" onContextMenu={...}>
```

**Option D.2:** Add `data-tauri-context-menu-region` attribute (if supported by Tauri)

**Option D.3:** Use pointer events CSS to control interaction zones

We'll determine the correct approach after testing.

---

## Implementation Plan - Option A (Fallback)

### Step 1: Remove Drag Region from Wrapper

**File:** `frontend/app/tab/tabbar.tsx`

**Line 681:**
```diff
-    <div ref={tabbarWrapperRef} className="tab-bar-wrapper" data-tauri-drag-region onContextMenu={handleTabBarContextMenu}>
+    <div ref={tabbarWrapperRef} className="tab-bar-wrapper" onContextMenu={handleTabBarContextMenu}>
```

### Step 2: Ensure WindowDrag Components Have Drag Region

**Line 682:**
```tsx
<WindowDrag ref={draggerLeftRef} className="left" />
```

**Verify:** Check `frontend/app/element/windowdrag.tsx` to ensure it sets `data-tauri-drag-region` internally.

---

## Testing Plan

### Phase 1: Verify Current Behavior (Broken State)

**Test 0: Confirm Regression**
1. Run `task dev`
2. Right-click on tabbar wrapper (empty area)
3. ❌ **Currently:** Windows native menu appears (minimize, maximize, close)
4. ✅ **Expected:** AgentMux custom menu should appear

**Analysis:** If Windows menu appears, `data-tauri-drag-region` is blocking custom context menu.

### Phase 2: Test Option D (Hybrid Menu)

**After implementing backend commands and menu updates:**

**Test 1: Custom Menu with Window Controls**
1. Right-click on tabbar wrapper
2. ✅ **Expected:** Custom menu with:
   - Minimize
   - Maximize
   - Close
   - ─────────
   - AgentMux v{version}
   - ─────────
   - Widgets...
3. ❌ **Failure:** If Windows native menu still appears, proceed to Option D.1 (remove drag-region)

**Test 2: Window Control Items Work**
1. Right-click tabbar → Click "Minimize"
2. ✅ **Expected:** Window minimizes
3. Restore window → Right-click tabbar → Click "Maximize"
4. ✅ **Expected:** Window maximizes
5. Right-click tabbar → Click "Maximize" again
6. ✅ **Expected:** Window restores to normal size
7. Right-click tabbar → Click "Close"
8. ✅ **Expected:** Window closes

### Phase 3: Regression Testing

**Test 1: Context Menu Appears**
1. Run `task dev`
2. Right-click on tabbar wrapper (empty area)
3. ✅ **Expected:** AgentMux custom menu appears (with window controls)
4. ❌ **Broken:** Windows native menu appears

**Test 2: Menu Items Functional**
1. Right-click tabbar → Custom menu appears
2. Click "AgentMux v0.28.2"
3. ✅ **Expected:** Version copied to clipboard, log message shown
4. ❌ **Broken:** Nothing happens

**Test 3: Widget Toggles Work**
1. Right-click tabbar → Custom menu appears
2. Click "Agent" checkbox (toggle off)
3. ✅ **Expected:** Agent widget disappears
4. Click "Agent" checkbox (toggle on)
5. ✅ **Expected:** Agent widget reappears

**Test 4: Window Dragging Still Works**
1. Click and hold on WindowDrag left area
2. Move mouse
3. ✅ **Expected:** Window moves
4. Release mouse
5. ✅ **Expected:** Window stays in new position

**Test 5: No Dragging Outside Drag Regions**
1. Click and hold on tabbar wrapper (non-WindowDrag area)
2. Move mouse
3. ✅ **Expected:** Window does NOT move (context menu target area)

### Regression Testing

**Test 1: Close Button Works**
- Click red X close button
- ✅ **Expected:** Window closes

**Test 2: Widget Bar Interactive**
- Click widget icons
- ✅ **Expected:** Widgets toggle visibility

**Test 3: No Layout Breakage**
- Check tabbar rendering
- ✅ **Expected:** No visual regressions, layout intact

---

## Edge Cases

### Case 1: Multi-Window Scenario

**Setup:** Multiple AgentMux windows open

**Test:** Right-click tabbar in Window 2 → Custom menu appears for Window 2

**Expected:** Each window's context menu works independently

### Case 2: macOS vs Windows

**macOS:** Context menu might behave differently (Cmd+Click vs Right-Click)

**Test:** On macOS, verify right-click and Ctrl+Click both trigger menu

**Windows:** Right-click should show custom menu

**Test:** Verify no native Windows menu appears

### Case 3: Widget Menu with No Widgets

**Setup:** `widgets.json` is empty or missing

**Test:** Right-click tabbar → Only version shown (no widget section)

**Expected:** Menu still appears, just with fewer items

---

## Success Criteria

### Core Functionality
- ✅ Right-click on tabbar wrapper shows AgentMux custom menu (NOT Windows native menu)
- ✅ Custom menu includes window controls (Minimize, Maximize, Close)
- ✅ Minimize button minimizes window
- ✅ Maximize button toggles maximize/restore
- ✅ Close button closes window
- ✅ Version item copies to clipboard on click
- ✅ Widget toggles show/hide widgets
- ✅ "Edit widgets.json" opens file in preview pane

### Window Interaction (Option D)
- ✅ Entire tabbar stays draggable (if drag-region kept)
- ✅ Context menu appears on right-click despite drag-region
- ✅ No conflicts between drag and context menu

### Window Interaction (Option A Fallback)
- ✅ Window dragging works in WindowDrag areas
- ✅ Window does NOT drag from non-WindowDrag areas (context menu zones)

### Quality
- ✅ No visual regressions in tabbar layout
- ✅ Works on both Windows and macOS
- ✅ Menu items have consistent styling
- ✅ Separators appear between sections

---

## Rollback Plan

If fix causes issues:

**Revert:**
```bash
git revert <commit-sha>
```

**Alternative:** Re-add `data-tauri-drag-region` to wrapper, investigate Option B (backend changes)

---

## Documentation Updates

### Update Files

1. **COHESIVE_CONTEXT_MENU_SYSTEM_SPEC.md**
   - Update "Current State" to mark TabBar as ✅ Fixed
   - Add note about drag-region conflicts

2. **CONTEXT_MENU_IMPLEMENTATION_SUMMARY.md**
   - Add troubleshooting section: "If context menu doesn't appear, check for `data-tauri-drag-region` conflicts"

3. **CLAUDE.md**
   - Add note: "TabBar context menu requires wrapper WITHOUT `data-tauri-drag-region` - drag regions only on WindowDrag components"

---

## Related Issues

- Issue #289: Window drag permission (where drag-region was added)
- Issue #293: Context menu implementation (working initially)
- PR #313: Widget reordering (may have introduced regression)

---

## Open Questions

1. **When did it actually break?**
   - Needs git bisect between 6fddfae (working) and 08dadce (broken)
   - Likely during widget changes or tabbar refactor

2. **Should we keep entire tabbar draggable?**
   - Current plan: Only WindowDrag areas draggable
   - Alternative: Add more WindowDrag components to fill tabbar space

3. **macOS testing needed?**
   - Context menu behavior might differ on macOS
   - Drag-region handling might be OS-specific

---

## Next Steps

1. **Immediate:** Implement Option A (remove drag-region from wrapper)
2. **Test:** Verify context menu appears and works
3. **Test:** Verify window dragging still works in WindowDrag areas
4. **Document:** Update specs with fix details
5. **Consider:** Add E2E test for tabbar context menu (prevent future regressions)

---

## Appendix: Code Locations

| Component | File | Lines | Description |
|-----------|------|-------|-------------|
| TabBar wrapper | `tabbar.tsx` | 681 | Div with drag-region (remove) |
| Context menu handler | `tabbar.tsx` | 185-189 | handleTabBarContextMenu |
| Menu builder | `base-menus.ts` | 96-100 | createTabBarMenu |
| WindowDrag component | `windowdrag.tsx` | 1-30 | Explicit drag areas |
| Backend command | `contextmenu.rs` | 32-60 | show_context_menu |

---

**Revision History**

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2026-02-15 | AgentX | Initial spec from regression investigation |
