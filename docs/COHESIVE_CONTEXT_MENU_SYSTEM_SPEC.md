# Cohesive Context Menu System - Specification

**Date**: 2026-02-13
**Status**: 📋 Design Phase
**Target Version**: 0.28.0
**Estimated Effort**: 8-12 hours

---

## Vision

**Design a unified context menu system that works consistently across all UI surfaces:**

1. **System Tray** (backend, Go)
2. **Tab Bar** (frontend, already implemented)
3. **Terminal Panes** (frontend, currently none)
4. **Preview Panes** (frontend, partially implemented)
5. **Web View Panes** (frontend)
6. **Code Editor Panes** (frontend)
7. **AI Chat Panes** (frontend)
8. **Sysinfo Panes** (frontend)
9. **Other View Types** (launcher, help, etc.)

**Goal**: Every surface shows relevant actions + common base items (like version)

---

## Current State Analysis

### What Exists ✅

| Surface | Context Menu | Items |
|---------|-------------|-------|
| **Tab Bar** | ✅ Working | Version, Widgets |
| **Preview (Files)** | ⚠️ Code exists, NOT working | New File, Rename, Delete, Copy Path |
| **System Tray** | ⚠️ Stub only | None (to add) |

**Note**: Preview has `handleFileContextMenu` defined in code, but user reports it doesn't appear on right-click. Needs investigation/fix.

### What's Missing ❌

| Surface | Context Menu | Status |
|---------|-------------|--------|
| **Terminal Panes** | ❌ None | Right-click does nothing |
| **Web View** | ❌ None | Falls back to browser default |
| **Code Editor** | ❌ None | Falls back to Monaco default |
| **AI Chat** | ❌ None | Right-click does nothing |
| **Sysinfo** | ❌ None | Right-click does nothing |

---

## Architecture: Compositional Menu System

### Concept: Base + Extensions

Every context menu is composed of:
1. **Base Items** - Common across all surfaces (version, about, etc.)
2. **View-Specific Items** - Unique to that view type
3. **Selection-Specific Items** - Based on what's selected (optional)

**Example - Terminal Pane**:
```
[View-Specific Items]
  Copy              (if text selected)
  Paste
  Clear Scrollback
  ─────────────────
[Base Items]
  AgentMux v0.27.3  (click to copy)
```

**Example - Preview Pane (File Selected)**:
```
[Selection-Specific Items]
  Open
  Rename
  Delete
  Copy Path
  ─────────────────
[View-Specific Items]
  New File
  New Folder
  ─────────────────
[Base Items]
  AgentMux v0.27.3
```

**Example - System Tray**:
```
[View-Specific Items]
  Show All Windows
  New Window
  ─────────────────
[Base Items]
  AgentMux v0.27.3
  About
  ─────────────────
  Quit
```

---

## Implementation Architecture

### Frontend: Shared Menu Factory

**Create**: `frontend/app/menu/context-menu-factory.ts`

```typescript
/**
 * Context Menu Factory - Unified menu creation system
 *
 * Creates context menus for all view types with consistent base items
 */

import { MenuBuilder } from "./menu-builder";
import { getApi } from "@/store/global";

/**
 * Create base menu items (common across all contexts)
 */
export function createBaseMenuItems(): MenuBuilder {
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

/**
 * Create terminal-specific context menu
 */
export function createTerminalContextMenu(options: {
    hasSelection: boolean;
    canPaste: boolean;
}): MenuBuilder {
    const menu = new MenuBuilder();

    // Terminal-specific actions
    if (options.hasSelection) {
        menu.add({
            label: "Copy",
            click: () => document.execCommand('copy'),
        });
    }

    if (options.canPaste) {
        menu.add({
            label: "Paste",
            click: async () => {
                const text = await navigator.clipboard.readText();
                // Dispatch paste to terminal
            },
        });
    }

    menu.add({
        label: "Clear Scrollback",
        click: () => {
            // Clear terminal scrollback
        },
    });

    // Add base items
    menu.separator().merge(createBaseMenuItems());

    return menu;
}

/**
 * Create webview-specific context menu
 */
export function createWebViewContextMenu(options: {
    url: string;
}): MenuBuilder {
    const menu = new MenuBuilder();

    menu.add({
        label: "Open in Browser",
        click: () => {
            getApi().openExternal(options.url);
        },
    });

    menu.add({
        label: "Copy URL",
        click: () => {
            navigator.clipboard.writeText(options.url);
        },
    });

    menu.separator().merge(createBaseMenuItems());

    return menu;
}

/**
 * Create code editor-specific context menu
 */
export function createCodeEditorContextMenu(options: {
    hasSelection: boolean;
    language: string;
}): MenuBuilder {
    const menu = new MenuBuilder();

    if (options.hasSelection) {
        menu.add({ label: "Cut", click: () => document.execCommand('cut') });
        menu.add({ label: "Copy", click: () => document.execCommand('copy') });
    }

    menu.add({ label: "Paste", click: () => document.execCommand('paste') });

    menu.separator();

    menu.add({
        label: "Format Document",
        click: () => {
            // Trigger Monaco format
        },
    });

    menu.separator().merge(createBaseMenuItems());

    return menu;
}

/**
 * Create AI chat-specific context menu
 */
export function createAIChatContextMenu(options: {
    hasMessages: boolean;
}): MenuBuilder {
    const menu = new MenuBuilder();

    if (options.hasMessages) {
        menu.add({
            label: "Clear Conversation",
            click: () => {
                // Clear chat history
            },
        });
    }

    menu.add({
        label: "Export Chat",
        click: () => {
            // Export chat as markdown
        },
    });

    menu.separator().merge(createBaseMenuItems());

    return menu;
}

/**
 * Create sysinfo-specific context menu
 */
export function createSysinfoContextMenu(): MenuBuilder {
    const menu = new MenuBuilder();

    menu.add({
        label: "Refresh",
        click: () => {
            // Refresh sysinfo data
        },
    });

    menu.add({
        label: "Copy Stats",
        click: () => {
            // Copy sysinfo to clipboard
        },
    });

    menu.separator().merge(createBaseMenuItems());

    return menu;
}
```

---

## Backend: Tray Menu (Go)

**Update**: `cmd/server/tray.go`

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

    // Tray-specific actions
    mShowAll := systray.AddMenuItem("Show All Windows", "")
    mNewWindow := systray.AddMenuItem("New Window", "")

    systray.AddSeparator()

    // Base items (same structure as frontend)
    mVersion := systray.AddMenuItem(
        fmt.Sprintf("AgentMux v%s", version),
        "Click to copy version",
    )

    mAbout := systray.AddMenuItem("About AgentMux", "")

    systray.AddSeparator()

    // Tray-only action
    mQuit := systray.AddMenuItem("Quit AgentMux", "Quit all instances")

    // Event handlers
    go func() {
        for {
            select {
            case <-mShowAll.ClickedCh:
                log.Println("[tray] Show All Windows clicked")
                // TODO: Broadcast to frontends
            case <-mNewWindow.ClickedCh:
                log.Println("[tray] New Window clicked")
                // TODO: Launch new instance
            case <-mVersion.ClickedCh:
                log.Printf("[tray] Version clicked: %s", version)
                copyToClipboard(version)
            case <-mAbout.ClickedCh:
                log.Println("[tray] About clicked")
                // TODO: Show about dialog
            case <-mQuit.ClickedCh:
                log.Println("[tray] Quit clicked")
                systray.Quit()
                // TODO: Graceful shutdown
            }
        }
    }()
}

func onTrayExit() {
    log.Println("[tray] System tray exiting")
}
```

---

## View Integration Guide

### Terminal View

**File**: `frontend/app/view/term/term.tsx`

**Add**:
```typescript
import { createTerminalContextMenu } from "@/app/menu/context-menu-factory";
import { ContextMenuModel } from "@/app/store/contextmenu";

// Inside TermViewImpl component
const handleContextMenu = useCallback((e: React.MouseEvent) => {
    e.preventDefault();

    const hasSelection = termRef.current?.hasSelection() || false;
    const menu = createTerminalContextMenu({
        hasSelection,
        canPaste: true,
    });

    ContextMenuModel.showContextMenu(menu.build(), e);
}, [termRef]);

// In render
return (
    <div
        className="term-view"
        onContextMenu={handleContextMenu}
    >
        {/* Terminal content */}
    </div>
);
```

### Web View

**File**: `frontend/app/view/webview/webview.tsx`

**Add**:
```typescript
const handleContextMenu = useCallback((e: React.MouseEvent) => {
    e.preventDefault();

    const menu = createWebViewContextMenu({
        url: currentUrl,
    });

    ContextMenuModel.showContextMenu(menu.build(), e);
}, [currentUrl]);
```

### Code Editor

**File**: `frontend/app/view/codeeditor/codeeditor.tsx`

**Add**:
```typescript
const handleContextMenu = useCallback((e: React.MouseEvent) => {
    e.preventDefault();

    const editor = editorRef.current;
    const selection = editor?.getSelection();

    const menu = createCodeEditorContextMenu({
        hasSelection: !selection?.isEmpty(),
        language: model?.getLanguageId() || 'text',
    });

    ContextMenuModel.showContextMenu(menu.build(), e);
}, [editorRef]);
```

### AI Chat

**File**: `frontend/app/view/chat/chat.tsx`

**Add**:
```typescript
const handleContextMenu = useCallback((e: React.MouseEvent) => {
    e.preventDefault();

    const menu = createAIChatContextMenu({
        hasMessages: messages.length > 0,
    });

    ContextMenuModel.showContextMenu(menu.build(), e);
}, [messages]);
```

---

## Menu Consistency Table

| Surface | Base Items | View-Specific | Selection-Specific |
|---------|-----------|---------------|-------------------|
| **System Tray** | Version, About | Show All, New Window, Quit | - |
| **Tab Bar** | Version | Widgets | - |
| **Terminal** | Version | Paste, Clear | Copy (if selection) |
| **Preview** | Version | New File/Folder | Open, Rename, Delete (if file) |
| **Web View** | Version | Open in Browser | - |
| **Code Editor** | Version | Format Document | Cut, Copy (if selection) |
| **AI Chat** | Version | Export Chat | Clear (if messages) |
| **Sysinfo** | Version | Refresh, Copy Stats | - |

---

## Phased Rollout

### Phase 1: Foundation (4 hours)

**Goals**:
- Create `context-menu-factory.ts`
- Implement `createBaseMenuItems()`
- Add tray menu with base items
- Add terminal context menu

**Deliverable**: Tray + Terminal show version

### Phase 2: Core Views (3 hours)

**Goals**:
- Web view context menu
- Code editor context menu
- AI chat context menu

**Deliverable**: 6 surfaces with context menus

### Phase 3: Advanced Features (3 hours)

**Goals**:
- Sysinfo context menu
- Selection-aware menus (copy when text selected)
- Clipboard integration
- About dialog (triggered from multiple places)

**Deliverable**: Full feature set

### Phase 4: Polish (2 hours)

**Goals**:
- Keyboard shortcuts in menus
- Icons for menu items
- Disabled states
- Testing across all views

---

## Benefits of This Architecture

1. **Consistency**: Version appears everywhere users might look
2. **Discoverability**: Users find features through context menus
3. **Maintainability**: Base items defined once, used everywhere
4. **Extensibility**: Easy to add new views or menu items
5. **Flexibility**: Each view can customize while keeping base

---

## Code Structure

```
frontend/app/menu/
├── menu-builder.ts              (existing - no changes)
├── base-menus.ts                (existing - tab bar menus)
├── context-menu-factory.ts      (NEW - unified factory)
└── README.md                    (NEW - usage guide)

cmd/server/
├── tray.go                      (UPDATE - add menu)
├── clipboard_windows.go         (NEW - clipboard)
└── clipboard_unix.go            (NEW - clipboard)

frontend/app/view/
├── term/term.tsx                (UPDATE - add context menu)
├── webview/webview.tsx          (UPDATE - add context menu)
├── codeeditor/codeeditor.tsx    (UPDATE - add context menu)
├── chat/chat.tsx                (UPDATE - add context menu)
└── sysinfo/sysinfo.tsx          (UPDATE - add context menu)
```

---

## Testing Strategy

### Manual Testing Matrix

| View | Right-Click | Shows Menu | Has Version | View Actions Work |
|------|------------|------------|-------------|------------------|
| System Tray | ✅ | 🔲 | 🔲 | 🔲 |
| Tab Bar | ✅ | ✅ | ✅ | ✅ |
| Terminal | ❌ | ❌ | ❌ | ❌ |
| Preview | ❌ | ❌ | ❌ | ❌ |
| Web View | ❌ | ❌ | ❌ | ❌ |
| Code Editor | ❌ | ❌ | ❌ | ❌ |
| AI Chat | ❌ | ❌ | ❌ | ❌ |
| Sysinfo | ❌ | ❌ | ❌ | ❌ |

✅ = Working | 🔲 = To implement | ❌ = Not working (user verified)

### Automated Tests

**Unit tests**:
```typescript
// frontend/app/menu/context-menu-factory.test.ts

describe('ContextMenuFactory', () => {
    it('base menu includes version', () => {
        const menu = createBaseMenuItems();
        expect(menu.build()).toContainEqual(
            expect.objectContaining({ label: expect.stringContaining('AgentMux v') })
        );
    });

    it('terminal menu includes base items', () => {
        const menu = createTerminalContextMenu({ hasSelection: false, canPaste: true });
        const items = menu.build();
        expect(items).toContainEqual(
            expect.objectContaining({ label: expect.stringContaining('AgentMux v') })
        );
    });

    it('terminal menu shows copy only when selection exists', () => {
        const menuWithSel = createTerminalContextMenu({ hasSelection: true, canPaste: true });
        const menuNoSel = createTerminalContextMenu({ hasSelection: false, canPaste: true });

        expect(menuWithSel.build().find(i => i.label === 'Copy')).toBeDefined();
        expect(menuNoSel.build().find(i => i.label === 'Copy')).toBeUndefined();
    });
});
```

---

## Migration Strategy

### Existing Menus

**Tab Bar** (working):
- ✅ Keep as-is
- ✅ Already uses `createTabBarBaseMenu()` which has version
- ⚠️ Optionally refactor to use `createBaseMenuItems()` for consistency

**Preview** (broken - needs fix):
- ⚠️ Code exists in `preview-directory.tsx` but doesn't work
- 🔨 **Fix first**: Debug why `handleFileContextMenu` doesn't trigger
- ➕ Then add base items via `merge(createBaseMenuItems())`
- Possible issues:
  - Event not propagating?
  - ContextMenuModel.showContextMenu not working?
  - Preview view type not rendering the directory component?

---

## Future Enhancements

### Phase 5: Cross-Surface Actions

**"Open in New Window"** - Available in any view:
```typescript
menu.add({
    label: "Open in New Window",
    click: () => {
        // Clone current view to new window
    },
});
```

### Phase 6: Customizable Menus

**User config** (`~/.config/agentmux/context-menus.json`):
```json
{
  "terminal": {
    "additionalItems": [
      {
        "label": "Run Custom Script",
        "command": "bash /path/to/script.sh"
      }
    ]
  }
}
```

### Phase 7: Plugin System

**Allow plugins to contribute menu items**:
```typescript
registerContextMenuItem('terminal', {
    label: 'My Plugin Action',
    icon: 'plugin-icon',
    click: () => { /* plugin code */ },
});
```

---

## Success Criteria

- [ ] All 8+ surfaces have context menus
- [ ] Every menu shows version (click to copy)
- [ ] Base items consistent across surfaces
- [ ] View-specific actions intuitive
- [ ] Selection-aware menus work correctly
- [ ] Clipboard copy works (Windows, macOS, Linux)
- [ ] No regressions in existing menus (tab bar, preview)
- [ ] Code is maintainable (single factory, composable)

---

## Timeline

| Phase | Effort | Deliverable |
|-------|--------|-------------|
| Phase 1: Foundation | 4 hours | Tray + Terminal menus |
| Phase 2: Core Views | 3 hours | +3 more views |
| Phase 3: Advanced | 3 hours | Selection, clipboard, polish |
| Phase 4: Testing | 2 hours | Full test coverage |

**Total**: 12 hours

---

## Open Questions

1. **About Dialog**: Should "About" open a modal or just show version info?
   - **Proposal**: Show version + build info in a small modal

2. **Keyboard Shortcuts**: Should context menu items show shortcuts?
   - **Proposal**: Yes, for common actions (Ctrl+C for Copy, etc.)

3. **Icons**: Should menu items have icons?
   - **Proposal**: Phase 4 - optional icons for visual clarity

4. **Themes**: Should context menus respect user themes?
   - **Proposal**: Yes, already handled by Tauri's context menu theming

---

## Summary

**Architecture**: Compositional menu system with shared base + view-specific items

**Key Files**:
- `frontend/app/menu/context-menu-factory.ts` - Unified factory
- `cmd/server/tray.go` - Backend tray menu
- Individual view files - Integration

**Benefit**: Consistent UX, easy maintenance, scalable to new views

**Recommendation**: Start with Phase 1 (Foundation) to validate approach, then expand

---

**Ready for implementation?** Let me know if this cohesive design makes sense or if you'd like adjustments!
