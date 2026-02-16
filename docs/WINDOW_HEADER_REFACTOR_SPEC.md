# Window Header Refactoring Specification

**Date:** 2026-02-15
**Version:** 0.28.1
**Status:** Draft

---

## 1. Executive Summary

The current `TabBar` component is misnamed and architecturally confusing. It serves as the window's title bar/header but its name suggests it manages tabs. Most tab-related functionality is commented out, while the component primarily handles:
- Window chrome (drag regions, window controls)
- Widget buttons (agent, terminal, sysinfo)
- System notifications (updates, errors)

This spec proposes a comprehensive refactoring to clarify responsibilities, improve naming, and properly organize window controls vs. action widgets.

---

## 2. Current State Analysis

### 2.1 Component Hierarchy (as of v0.28.1)

```
TabBar (frontend/app/tab/tabbar.tsx) - 741 lines
├── WindowDrag (left drag region)
├── tab-bar-left (NEW - for agentmux button)
├── [COMMENTED] WorkspaceSwitcher
├── [COMMENTED] Tab management system (~500 lines)
├── [COMMENTED] Add tab button
├── tab-bar-right
    ├── WidgetBar
    │   ├── agent widget
    │   ├── terminal widget
    │   ├── sysinfo widget
    │   ├── help widget
    │   └── devtools widget
    ├── UpdateStatusBanner
    ├── ConfigErrorIcon
    └── Close button (X)
```

### 2.2 Problems Identified

**Naming Issues:**
- `TabBar` implies tab management, but 90% of tab code is disabled
- `WidgetBar` contains action buttons, not traditional widgets
- `tab-bar-left` and `tab-bar-right` are layout containers, not semantic components

**Structural Issues:**
- 500+ lines of commented tab drag-and-drop code creating noise
- Window controls scattered across multiple locations
- No clear separation between:
  - Window chrome (minimize, maximize, close, new window)
  - Action widgets (create agent, terminal, etc.)
  - System status (updates, errors)

**Layout Issues:**
- `agentmux` button (new window) positioned incorrectly
- No minimize/maximize buttons
- Flex layout conflicts between left/right containers
- Missing proper window control affordances

### 2.3 Related Files

```
frontend/app/tab/
├── tabbar.tsx          (741 lines - main component)
├── tabbar.scss         (151 lines - styles)
├── tabbar-model.ts     (tab state management)
├── tab.tsx             (individual tab component - unused)
├── tab.scss            (tab styles - unused)
├── tabcontent.tsx      (tab content wrapper)
├── widgetbar.tsx       (action buttons)
├── updatebanner.tsx    (update notifications)
└── workspaceswitcher.tsx (commented out)
```

---

## 3. Proposed Architecture

### 3.1 New Component Structure

```
WindowHeader (renamed from TabBar)
├── WindowDrag (left)
├── WindowControls (left side)
│   ├── NewWindowButton ("agentmux")
│   ├── MinimizeButton
│   └── MaximizeButton
├── [FUTURE] WorkspaceManager (workspace switcher)
├── [FUTURE] TabStrip (when multi-tab support returns)
├── WindowDrag (right - spacer)
├── SystemStatus (right side)
│   ├── ActionWidgets
│   │   ├── AgentWidget
│   │   ├── TerminalWidget
│   │   ├── SysinfoWidget
│   │   ├── HelpWidget
│   │   └── DevToolsWidget
│   ├── UpdateBanner
│   ├── ConfigError
│   └── CloseButton
```

### 3.2 Component Responsibilities

**WindowHeader** (formerly TabBar)
- Owns window chrome layout
- Manages drag regions
- Coordinates left/right sections
- File: `frontend/app/window/window-header.tsx`

**WindowControls** (new component)
- Left-aligned window management buttons
- New window, minimize, maximize
- Platform-specific behavior (hide on macOS with native controls)
- File: `frontend/app/window/window-controls.tsx`

**ActionWidgets** (renamed from WidgetBar)
- Right-aligned action buttons for creating blocks
- Each widget opens a new pane (agent, terminal, etc.)
- File: `frontend/app/window/action-widgets.tsx`

**SystemStatus** (new container)
- Right-aligned system information
- Update notifications, errors, close button
- File: `frontend/app/window/system-status.tsx`

---

## 4. Detailed Changes

### 4.1 File Reorganization

**Move:** `frontend/app/tab/` → `frontend/app/window/`

```
frontend/app/window/
├── window-header.tsx       (renamed from tabbar.tsx)
├── window-header.scss      (renamed from tabbar.scss)
├── window-controls.tsx     (new - left side buttons)
├── action-widgets.tsx      (renamed from widgetbar.tsx)
├── system-status.tsx       (new - right side status)
├── update-banner.tsx       (moved from tab/)
└── config-error.tsx        (extracted from tabbar.tsx)
```

**Archive commented tab/workspace code:**
```
frontend/app/window/archived/
├── tab.tsx                 (keep for future multi-tab)
├── tab.scss
├── tab-drag-logic.ts       (extracted from window-header.tsx)
├── workspace-switcher.tsx  (keep for future workspace manager)
└── workspace-manager.ts    (workspace state logic)
```

**Note:** Files in `archived/` are preserved for future feature restoration, not dead code.

### 4.2 WindowControls Component

**Purpose:** Left-aligned window management buttons

**API:**
```typescript
interface WindowControlsProps {
    platform: Platform;
    showNativeControls: boolean; // macOS hide if true
}

export const WindowControls: React.FC<WindowControlsProps> = ({
    platform,
    showNativeControls
}) => {
    if (platform === "darwin" && showNativeControls) {
        return null; // macOS handles this
    }

    return (
        <div className="window-controls">
            <NewWindowButton />
            <MinimizeButton />
            <MaximizeButton />
        </div>
    );
};
```

**Buttons:**
- **NewWindowButton:** Opens new AgentMux window (currently "agentmux")
- **MinimizeButton:** Minimizes window via `getApi().minimizeWindow()`
- **MaximizeButton:** Toggles maximize via `getApi().maximizeWindow()`

**Styling:**
```scss
.window-controls {
    display: flex;
    gap: 4px;
    align-items: center;
    padding-left: 4px;
    -webkit-app-region: no-drag;

    button {
        padding: 4px 8px;
        font-size: 11px;
        color: var(--secondary-text-color);
        cursor: pointer;

        &:hover {
            background: var(--hoverbg);
            color: var(--main-text-color);
        }
    }
}
```

### 4.3 ActionWidgets Component

**Purpose:** Right-aligned buttons for creating blocks (formerly WidgetBar)

**Changes:**
- Rename `WidgetBar` → `ActionWidgets`
- Remove special "newwindow" handling (moved to WindowControls)
- Keep widget configuration from `widgets.json`

**API remains similar:**
```typescript
export const ActionWidgets: React.FC = () => {
    const fullConfig = useAtomValue(atoms.fullConfigAtom);
    const widgets = sortByDisplayOrder(fullConfig?.widgets);

    return (
        <div className="action-widgets">
            {widgets?.map((widget, idx) => (
                <ActionWidget key={idx} widget={widget} />
            ))}
            {showHelp && <HelpWidget />}
            <DevToolsWidget />
        </div>
    );
};
```

### 4.4 SystemStatus Component

**Purpose:** Container for right-side system information

**Structure:**
```typescript
export const SystemStatus: React.FC = () => {
    return (
        <div className="system-status">
            <ActionWidgets />
            <UpdateBanner />
            <ConfigError />
            <CloseButton />
        </div>
    );
};
```

**Styling:**
```scss
.system-status {
    display: flex;
    flex-direction: row;
    gap: 6px;
    height: 100%;
    align-items: center;
    flex-grow: 1;
    justify-content: flex-end;
    margin-right: 6px;
}
```

### 4.5 WindowHeader Component

**Simplified structure:**
```typescript
export const WindowHeader: React.FC<WindowHeaderProps> = ({ workspace }) => {
    const settings = useAtomValue(atoms.settingsAtom);
    const platform = PLATFORM;
    const showNativeControls = platform === "darwin" && !settings["window:showmenubar"];

    return (
        <div className="window-header" data-tauri-drag-region>
            <WindowDrag className="left" />

            <WindowControls
                platform={platform}
                showNativeControls={showNativeControls}
            />

            {/* [FUTURE] <WorkspaceManager /> - workspace switcher */}
            {/* [FUTURE] <TabStrip /> - multi-tab support */}

            <SystemStatus />
        </div>
    );
};
```

**Remove:**
- All commented tab drag logic (~500 lines)
- Tab state management hooks
- Workspace switcher rendering
- Inline ConfigError component (extract to file)

---

## 5. CSS Refactoring

### 5.1 Rename Classes

| Old Class | New Class | Purpose |
|-----------|-----------|---------|
| `.tab-bar-wrapper` | `.window-header` | Main container |
| `.tab-bar-left` | `.window-controls` | Left window buttons |
| `.tab-bar-right` | `.system-status` | Right system info |
| `.dev-label` | *(remove)* | Unused |
| `.app-menu-button` | `.menu-button` | Menu trigger |

### 5.2 Layout Strategy

**Use explicit flex layout instead of auto margins:**

```scss
.window-header {
    display: flex;
    flex-direction: row;
    align-items: center;
    height: 33px;
    width: 100vw;
    padding-top: 6px;
    -webkit-app-region: drag;

    .window-drag.left {
        width: var(--default-indent);
        flex-shrink: 0;
    }

    .window-controls {
        // Natural flow after left drag region
        flex-shrink: 0;
    }

    .system-status {
        // Grow to fill space, align right
        flex-grow: 1;
        justify-content: flex-end;
    }
}
```

---

## 6. Migration Plan

### 6.1 Phase 1: Prepare (Non-Breaking)

1. Create new directory structure
2. Copy files to new locations
3. Create new components (WindowControls, SystemStatus)
4. Add tests for new components

### 6.2 Phase 2: Refactor (Breaking)

1. Update imports across codebase
2. Rename TabBar → WindowHeader
3. Replace inline logic with new components
4. Update CSS class names
5. Remove commented code

### 6.3 Phase 3: Polish

1. Add minimize/maximize buttons
2. Test on all platforms (Windows, macOS, Linux)
3. Verify drag regions work correctly
4. Update documentation

### 6.4 Rollback Plan

- Keep old files in `frontend/app/tab/archived/`
- Tag commit before refactor: `v0.28.1-pre-header-refactor`
- Feature flag: `settings["window:use-new-header"]` (default true)

---

## 7. Import Updates

**Files that import TabBar:**

```bash
# Find all imports
grep -r "TabBar" frontend/app --include="*.tsx" --include="*.ts"
```

**Expected locations:**
- `frontend/app/workspace/workspace.tsx` - Main usage
- `frontend/app/app.tsx` - Possible direct import
- Any tests

**Update to:**
```typescript
// Before
import { TabBar } from "@/app/tab/tabbar";

// After
import { WindowHeader } from "@/app/window/window-header";
```

---

## 8. Testing Strategy

### 8.1 Visual Regression Tests

- Screenshot window header on Windows, macOS, Linux
- Verify button positions (left vs right)
- Test with/without updates available
- Test with/without config errors

### 8.2 Functional Tests

**Window Controls:**
- Click "agentmux" → opens new window
- Click minimize → window minimizes
- Click maximize → window toggles fullscreen
- Click close → window closes

**Action Widgets:**
- Click agent → creates agent block
- Click terminal → creates terminal block
- Click sysinfo → creates sysinfo block

### 8.3 Platform-Specific Tests

**macOS:**
- Native controls shown → hide custom controls
- Drag region respects macOS traffic lights

**Windows:**
- All buttons visible
- Snap regions work correctly

---

## 9. Open Questions

1. **Tab system:** When will multi-tab support return? Should we design for it now?
2. **Workspace switcher:** Keep archived or delete permanently?
3. **Minimize/Maximize icons:** Which FA icons? `fa-window-minimize`, `fa-window-maximize`?
4. **New window label:** Keep "agentmux" or change to icon-only?
5. **Platform detection:** Trust `PLATFORM` constant or detect at runtime?

---

## 10. Success Metrics

**Code Quality:**
- Reduce window-header.tsx from 741 → ~150 lines
- Remove 500+ lines of commented code
- Improve component cohesion (single responsibility)

**UX Improvements:**
- Add minimize/maximize buttons (user request)
- Fix agentmux button positioning (left side)
- Clear visual separation: window controls (left) vs system status (right)

**Maintainability:**
- Clear naming (WindowHeader vs TabBar)
- Organized file structure (window/ directory)
- Easier to add future features (tabs, workspace switcher)

---

## 11. Implementation Estimate

**Effort:** 4-6 hours

**Breakdown:**
- Phase 1 (Prepare): 1.5 hours
- Phase 2 (Refactor): 2 hours
- Phase 3 (Polish): 1 hour
- Testing: 1.5 hours

**Risk:** Low (non-critical component, easily reversible)

---

## 12. Appendix: Widget vs Control Clarification

**Window Controls** (left side):
- **Purpose:** Manage the window itself
- **Examples:** New window, minimize, maximize, close
- **User mental model:** "I'm controlling the app window"

**Action Widgets** (right side):
- **Purpose:** Create new content blocks/panes
- **Examples:** Agent, terminal, sysinfo
- **User mental model:** "I'm adding a new pane to my workspace"

**Key Distinction:**
- Controls affect the **window/app**
- Widgets affect the **content/workspace**

This separation improves discoverability and matches user expectations from other applications.

---

## 13. Future Considerations

### 13.1 Workspace Manager [FUTURE]

**Status:** Currently commented out, will be brought back later

**Current Location:** `frontend/app/tab/workspaceswitcher.tsx`

**Purpose:**
- Switch between different workspaces
- Each workspace has its own set of tabs and blocks
- Allows organizing work into separate contexts

**Proposed Location in Refactored Architecture:**

```
WindowHeader
├── WindowDrag (left)
├── WindowControls (new window, min, max)
├── WorkspaceManager ← INSERT HERE (between controls and tabs)
│   ├── Current workspace indicator
│   ├── Workspace dropdown
│   └── Switch/create workspace actions
├── TabStrip (tabs for current workspace)
├── SystemStatus (widgets, close)
```

**Integration Notes:**
- Position: Between WindowControls and TabStrip
- Shows current workspace name
- Click to show workspace list dropdown
- Quick switch between workspaces
- Create/rename/delete workspace actions

**Files to Preserve:**
```
frontend/app/window/archived/
├── workspace-switcher.tsx    (keep for future restoration)
└── workspace-manager.ts       (workspace state logic)
```

**When to Restore:**
- After multi-window support is stable
- When workspace isolation is needed
- Target: v0.29.x or v0.30.x

### 13.2 When Multi-Tab Returns

The TabStrip component would slot between WorkspaceManager and SystemStatus:

```
WindowHeader
├── WindowDrag (left)
├── WindowControls (new window, min, max)
├── WorkspaceManager [FUTURE]
├── TabStrip ← INSERT HERE
│   ├── Tab 1
│   ├── Tab 2
│   └── Add Tab Button
├── SystemStatus (widgets, close)
```

**Combined Layout (All Features Enabled):**

```
WindowHeader
├── WindowDrag (left)
├── WindowControls
│   ├── agentmux (new window)
│   ├── minimize
│   └── maximize
├── WorkspaceManager [FUTURE]
│   └── "Workspace 1 ▼"
├── TabStrip [FUTURE]
│   ├── Tab: Terminal
│   ├── Tab: Agent
│   ├── Tab: Code
│   └── [+] Add Tab
├── WindowDrag (spacer)
├── SystemStatus
│   ├── ActionWidgets (agent, terminal, sysinfo, help, devtools)
│   ├── UpdateBanner
│   ├── ConfigError
│   └── CloseButton (×)
```

---

**End of Specification**
