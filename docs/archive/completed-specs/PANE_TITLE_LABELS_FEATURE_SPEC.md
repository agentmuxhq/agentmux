# Feature Specification: Optional Pane Title Labels

**Version:** 1.0
**Date:** 2025-10-06
**Status:** Draft
**Author:** Claude Code Agent

---

## Executive Summary

Add optional configurable title labels atop each pane in Wave Terminal, providing users with contextual information about pane contents without sacrificing screen real estate. This feature enhances workspace organization and navigation while maintaining Wave's clean, minimal aesthetic.

---

## Problem Statement

### Current Limitations
- Users cannot easily distinguish between multiple panes of the same type (e.g., multiple terminals, multiple preview windows)
- No visual indicator of pane purpose or content when working with many open panes
- Difficult to navigate complex layouts with numerous blocks
- Context switching requires mental overhead to identify which pane contains what

### User Pain Points
1. **Lost Context:** "Which terminal was I working in for the backend vs frontend?"
2. **Visual Clutter:** Too many similar-looking panes without differentiation
3. **Navigation Friction:** Must click into panes to identify their contents
4. **Workflow Interruption:** Constantly searching for the "right" pane

---

## Goals & Objectives

### Primary Goals
1. **Enhanced Discoverability:** Users can quickly identify pane contents at a glance
2. **Improved Organization:** Enable logical grouping and labeling of related panes
3. **Minimal Intrusion:** Maintain Wave's clean UI with optional, space-efficient labels
4. **Flexible Configuration:** Support various display modes to accommodate different workflows

### Success Metrics
- Reduced time to locate specific panes (measurable via user studies)
- Increased user satisfaction with workspace management
- High adoption rate (>40% of users enable labels within first month)
- Minimal performance impact (<5ms render time per label)

### Non-Goals
- Full window management system (use existing layout system)
- Per-block custom styling beyond title labels
- Tab-style pane organization (different feature)

---

## User Stories

### As a power user with many panes open:
> "I want to label my panes so I can quickly identify which terminal is running my dev server, which is for git commands, and which is for SSH sessions."

### As a developer context-switching between projects:
> "I want persistent labels on my panes so when I return to my workspace tomorrow, I immediately know which panes belong to which project."

### As a new Wave user:
> "I want optional labels that don't clutter my screen, but help me learn how to organize my workspace effectively."

### As a minimalist:
> "I want to disable pane labels entirely and maintain a clean, distraction-free interface."

---

## Detailed Feature Description

### Visual Design

#### Label Appearance
- **Position:** Top edge of each pane, below tab bar, above block content
- **Height:** Compact (24px default, configurable 20-32px)
- **Style:**
  - Background: Semi-transparent dark overlay (`rgba(0, 0, 0, 0.6)`)
  - Text: Primary text color with secondary accent
  - Font: System font, 12px, medium weight
  - Padding: 4px horizontal, 2px vertical
  - Border: Subtle bottom border (1px, theme-dependent)

#### Label States
1. **Default:** Visible with full opacity
2. **Hover:** Slight highlight, show edit icon if editable
3. **Active Pane:** Accent color border/highlight
4. **Collapsed:** Hidden when pane height < threshold (150px)
5. **Focus Mode:** Optional auto-hide (show on hover only)

#### Label Content Options
- **Custom Text:** User-defined label (e.g., "Backend Server", "DB Logs", "Notes")
- **Auto-Generated:** Based on block type and content
  - Terminal: Current directory or last command
  - Preview: File name
  - Code Editor: File path
  - Chat: Channel name
- **Icon + Text:** Optional leading icon for quick visual scanning
- **Timestamp:** Optional last-updated time for dynamic content

### Configuration System

#### Global Settings (`~/.waveterm/config.json`)
```json
{
  "pane-labels": {
    "enabled": true,
    "display-mode": "always",  // "always" | "on-hover" | "never"
    "height": 24,
    "show-icons": true,
    "auto-generate": true,
    "font-size": 12,
    "max-length": 50,
    "position": "top"  // future: "bottom" | "overlay"
  }
}
```

#### Per-Pane Configuration (Block Metadata)
```typescript
interface BlockMeta {
  // ... existing fields
  "pane-title"?: string;           // Custom title
  "pane-title:icon"?: string;      // Font Awesome icon class
  "pane-title:color"?: string;     // Accent color
  "pane-title:hide"?: boolean;     // Override global setting
  "pane-title:auto"?: boolean;     // Use auto-generated title
}
```

#### Widget Configuration (`~/.waveterm/widgets.json`)
Add a new widget for quick label toggle:
```json
{
  "pane-labels-toggle": {
    "icon": "tag",
    "label": "labels",
    "description": "Toggle pane title labels",
    "blockdef": {
      "meta": {
        "view": "pane-labels-settings"
      }
    }
  }
}
```

### User Interface Components

#### 1. Title Bar Component (`frontend/app/block/titlbar.tsx`)
```typescript
interface TitleBarProps {
  blockId: string;
  title?: string;
  icon?: string;
  color?: string;
  editable: boolean;
  onTitleChange?: (newTitle: string) => void;
}

const TitleBar = memo(({ blockId, title, icon, color, editable, onTitleChange }: TitleBarProps) => {
  const [isEditing, setIsEditing] = useState(false);
  const [localTitle, setLocalTitle] = useState(title || "");
  const globalSettings = useAtomValue(atoms.settingsAtom);

  if (!globalSettings["pane-labels"]?.enabled) {
    return null;
  }

  const displayMode = globalSettings["pane-labels"]?.["display-mode"] || "always";
  const [isHovered, setIsHovered] = useState(false);

  if (displayMode === "never") return null;
  if (displayMode === "on-hover" && !isHovered) return null;

  return (
    <div
      className="pane-title-bar"
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
    >
      {icon && <i className={makeIconClass(icon)} style={{ color }} />}
      {isEditing ? (
        <input
          value={localTitle}
          onChange={(e) => setLocalTitle(e.target.value)}
          onBlur={() => {
            setIsEditing(false);
            onTitleChange?.(localTitle);
          }}
          autoFocus
        />
      ) : (
        <span
          className="pane-title-text"
          onClick={() => editable && setIsEditing(true)}
        >
          {localTitle || "Untitled Pane"}
        </span>
      )}
      {editable && isHovered && (
        <IconButton
          icon="pencil"
          size="small"
          onClick={() => setIsEditing(true)}
        />
      )}
    </div>
  );
});
```

#### 2. Auto-Title Generator (`frontend/app/block/autotitle.ts`)
```typescript
export function generateAutoTitle(block: Block): string {
  const view = block.meta?.view;

  switch (view) {
    case "term":
      return generateTerminalTitle(block);
    case "preview":
      return generatePreviewTitle(block);
    case "codeeditor":
      return generateEditorTitle(block);
    case "chat":
      return generateChatTitle(block);
    default:
      return `${view || "Block"} #${block.oid.slice(0, 8)}`;
  }
}

function generateTerminalTitle(block: Block): string {
  // Parse terminal state for current directory or command
  const cwd = block.meta?.["term:cwd"] || "~";
  const lastCmd = block.meta?.["term:lastcmd"];

  if (lastCmd) {
    return `${basename(cwd)}: ${truncate(lastCmd, 30)}`;
  }
  return basename(cwd) || "Terminal";
}

function generatePreviewTitle(block: Block): string {
  const file = block.meta?.file;
  return file ? basename(file) : "Preview";
}

function generateEditorTitle(block: Block): string {
  const file = block.meta?.file;
  if (!file) return "Editor";

  const parts = file.split("/");
  if (parts.length > 2) {
    return `.../${parts[parts.length - 2]}/${parts[parts.length - 1]}`;
  }
  return file;
}

function generateChatTitle(block: Block): string {
  const channel = block.meta?.["chat:channel"];
  return channel || "Chat";
}
```

#### 3. Settings Panel Integration
Add to existing settings view (`frontend/app/view/settings/settings.tsx`):

```typescript
<SettingsSection title="Pane Title Labels">
  <Toggle
    label="Enable Pane Labels"
    checked={settings["pane-labels"]?.enabled ?? true}
    onChange={(val) => updateSetting("pane-labels.enabled", val)}
  />

  <Select
    label="Display Mode"
    value={settings["pane-labels"]?.["display-mode"] ?? "always"}
    options={[
      { value: "always", label: "Always Visible" },
      { value: "on-hover", label: "Show on Hover" },
      { value: "never", label: "Never Show" }
    ]}
    onChange={(val) => updateSetting("pane-labels.display-mode", val)}
  />

  <Toggle
    label="Auto-Generate Titles"
    checked={settings["pane-labels"]?.["auto-generate"] ?? true}
    onChange={(val) => updateSetting("pane-labels.auto-generate", val)}
  />

  <Toggle
    label="Show Icons"
    checked={settings["pane-labels"]?.["show-icons"] ?? true}
    onChange={(val) => updateSetting("pane-labels.show-icons", val)}
  />

  <Slider
    label="Label Height"
    min={20}
    max={32}
    value={settings["pane-labels"]?.height ?? 24}
    onChange={(val) => updateSetting("pane-labels.height", val)}
  />
</SettingsSection>
```

### Context Menu Integration

Add to block context menu (`frontend/app/block/block.tsx`):

```typescript
const blockContextMenu: ContextMenuItem[] = [
  // ... existing items
  {
    label: "Edit Pane Title",
    click: () => setEditingTitle(true)
  },
  {
    label: "Auto-Generate Title",
    click: async () => {
      const autoTitle = generateAutoTitle(block);
      await RpcApi.SetMetaCommand(TabRpcClient, {
        oref: WOS.makeORef("block", block.oid),
        meta: { "pane-title": autoTitle }
      });
    }
  },
  {
    label: "Clear Title",
    click: async () => {
      await RpcApi.SetMetaCommand(TabRpcClient, {
        oref: WOS.makeORef("block", block.oid),
        meta: { "pane-title": "" }
      });
    }
  },
  { type: "separator" },
  // ... rest of menu
];
```

### Keyboard Shortcuts

Add to keybindings system:
- `Cmd+Shift+R` (Mac) / `Ctrl+Shift+R` (Win/Linux): Rename focused pane
- `Cmd+Shift+T` (Mac) / `Ctrl+Shift+T` (Win/Linux): Toggle labels visibility
- `Cmd+Shift+A` (Mac) / `Ctrl+Shift+A` (Win/Linux): Auto-generate title for focused pane

---

## Technical Implementation

### Architecture Overview

```
┌─────────────────────────────────────────────────────┐
│  Block Component (block.tsx)                        │
│  ┌───────────────────────────────────────────────┐  │
│  │ TitleBar Component (if enabled)               │  │
│  │  - Displays custom or auto-generated title    │  │
│  │  - Handles editing and updates                │  │
│  └───────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────┐  │
│  │ BlockFrame (existing content area)            │  │
│  │  - Terminal / Preview / Editor / etc.         │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘

Settings Flow:
User → Settings UI → ConfigService → WshServer → config.json
                                   ↓
                         Atoms (Jotai) → React Re-render

Metadata Flow:
User Edit → RpcApi.SetMetaCommand → WshServer → DB Update
                                              ↓
                              Block Atom Update → TitleBar Re-render
```

### File Structure

**New Files:**
```
frontend/app/block/titlebar.tsx       // Main title bar component
frontend/app/block/titlebar.scss      // Styles
frontend/app/block/autotitle.ts       // Auto-title generation logic
frontend/types/panetitle.d.ts         // TypeScript definitions
```

**Modified Files:**
```
frontend/app/block/block.tsx          // Integrate TitleBar component
frontend/app/block/block.scss         // Layout adjustments
frontend/app/store/global.ts          // Add settings atoms
frontend/types/gotypes.d.ts           // Extend BlockMeta interface
pkg/wshrpc/wshclient/wshclientutil.go // Add RPC methods if needed
pkg/waveobj/metaconsts.go             // Add metadata constants
```

### Data Model Changes

#### Block Metadata Extensions
```go
// pkg/waveobj/metaconsts.go
const (
    // ... existing constants
    MetaKey_PaneTitle       = "pane-title"        // Custom title text
    MetaKey_PaneTitleIcon   = "pane-title:icon"   // Icon class
    MetaKey_PaneTitleColor  = "pane-title:color"  // Accent color
    MetaKey_PaneTitleHide   = "pane-title:hide"   // Hide override
    MetaKey_PaneTitleAuto   = "pane-title:auto"   // Auto-generate flag
)
```

#### Settings Schema
```go
// pkg/wconfig/settingsconfig.go
type PaneLabelSettings struct {
    Enabled      bool   `json:"enabled"`
    DisplayMode  string `json:"display-mode"`  // "always" | "on-hover" | "never"
    Height       int    `json:"height"`
    ShowIcons    bool   `json:"show-icons"`
    AutoGenerate bool   `json:"auto-generate"`
    FontSize     int    `json:"font-size"`
    MaxLength    int    `json:"max-length"`
    Position     string `json:"position"`      // "top" | future: "bottom"
}

// Add to SettingsType
type SettingsType struct {
    // ... existing fields
    PaneLabels *PaneLabelSettings `json:"pane-labels,omitempty"`
}
```

### Rendering Pipeline

1. **Block Component Mount:**
   - Check global `pane-labels.enabled` setting
   - Retrieve block metadata for custom title
   - Generate auto-title if no custom title and `auto-generate` enabled
   - Render TitleBar component if conditions met

2. **Title Bar Render:**
   - Apply display-mode logic (always/on-hover/never)
   - Render icon if enabled and provided
   - Render title text (custom or auto-generated)
   - Attach edit handlers if editable

3. **Title Update Flow:**
   - User edits title → Local state update
   - onBlur → Call RpcApi.SetMetaCommand
   - Backend updates block metadata
   - Atom updates trigger re-render
   - New title displayed

### Performance Considerations

1. **Memoization:**
   - Memoize TitleBar component with `React.memo`
   - Memoize auto-title generation per block
   - Cache computed titles in block atom

2. **Lazy Rendering:**
   - Don't render TitleBar if globally disabled
   - Use virtual scrolling for many panes (existing)
   - Debounce auto-title updates (500ms)

3. **CSS Optimization:**
   - Use CSS transforms for hover effects
   - Hardware-accelerated animations
   - Minimize reflows with fixed heights

4. **Benchmarks (Target):**
   - Title bar render: <5ms
   - Auto-title generation: <10ms
   - Settings update: <50ms
   - No impact on terminal performance

---

## Use Cases & Examples

### Use Case 1: Full-Stack Developer
**Scenario:** Working on a web app with separate frontend, backend, and database terminals.

**Before:**
- 3 identical-looking terminal panes
- Must click each to see which is which
- Frequently runs commands in wrong terminal

**After:**
```
┌─────────────────────────┐  ┌─────────────────────────┐  ┌─────────────────────────┐
│ 🖥️  Frontend (npm dev)  │  │ ⚙️  Backend (go run)    │  │ 🗄️  PostgreSQL          │
│ ─────────────────────── │  │ ─────────────────────── │  │ ─────────────────────── │
│ $ npm run dev           │  │ $ go run main.go        │  │ $ psql -d mydb          │
│ > dev server on :3000   │  │ Server listening :8080  │  │ mydb=#                  │
└─────────────────────────┘  └─────────────────────────┘  └─────────────────────────┘
```

**Configuration:**
```json
{
  "pane-title": "Frontend (npm dev)",
  "pane-title:icon": "desktop",
  "pane-title:color": "#00D8FF"
}
```

### Use Case 2: DevOps Engineer
**Scenario:** Managing multiple SSH sessions across different servers.

**Before:**
- 6 SSH terminal panes
- Server hostnames not visible
- Must check prompt to identify server

**After:**
```
┌─────────────────────────┐  ┌─────────────────────────┐  ┌─────────────────────────┐
│ 🖥️  prod-web-01          │  │ 🖥️  prod-web-02          │  │ 🗄️  prod-db-primary     │
├─────────────────────────┤  ├─────────────────────────┤  ├─────────────────────────┤
┌─────────────────────────┐  ┌─────────────────────────┐  ┌─────────────────────────┐
│ 🧪  staging-web-01       │  │ 🛠️  dev-01               │  │ 📊  monitoring-01        │
└─────────────────────────┘  └─────────────────────────┘  └─────────────────────────┘
```

**Auto-Generated Titles:**
- Parsed from SSH connection metadata
- Hostname + connection status
- Color-coded by environment (prod=red, staging=yellow, dev=green)

### Use Case 3: Content Creator
**Scenario:** Writing documentation with multiple preview and editor panes.

**Before:**
- 4 preview panes showing different .md files
- Must scroll to top of each to see filename
- Loses track of which doc is which

**After:**
```
┌──────────────────────────────────┐  ┌──────────────────────────────────┐
│ 📝  ARCHITECTURE.md               │  │ 📝  API_REFERENCE.md             │
│ ──────────────────────────────── │  │ ──────────────────────────────── │
│ # System Architecture            │  │ # API Reference                  │
│                                  │  │                                  │
│ ## Overview                      │  │ ## Endpoints                     │
└──────────────────────────────────┘  └──────────────────────────────────┘
┌──────────────────────────────────┐  ┌──────────────────────────────────┐
│ 📝  DEPLOYMENT.md                 │  │ 📝  CHANGELOG.md                 │
│ ──────────────────────────────── │  │ ──────────────────────────────── │
│ # Deployment Guide               │  │ # Changelog                      │
└──────────────────────────────────┘  └──────────────────────────────────┘
```

**Auto-Generated Titles:**
- Filename from `block.meta.file`
- Icon based on file type
- Minimal, clean aesthetic

### Use Case 4: Minimalist User
**Scenario:** Prefers clean interface, no clutter.

**Configuration:**
```json
{
  "pane-labels": {
    "enabled": true,
    "display-mode": "on-hover"  // Only show on hover
  }
}
```

**Result:**
- Panes appear label-free by default
- Hover over any pane → Title fades in
- Best of both worlds: clean UI + contextual help when needed

---

## Edge Cases & Constraints

### Edge Cases

1. **Very Small Panes:**
   - Title bar consumes too much space
   - **Solution:** Auto-hide titles when pane height < 150px

2. **Long Titles:**
   - Overflow breaks layout
   - **Solution:** Truncate with ellipsis, show full title in tooltip

3. **Rapid Pane Creation:**
   - Auto-title generation lags
   - **Solution:** Debounce auto-title updates, show "Loading..." placeholder

4. **Conflicting Custom vs Auto Titles:**
   - User sets custom title, then auto-generate runs
   - **Solution:** Custom titles always override auto-generated

5. **Theme Compatibility:**
   - Labels may not match all custom themes
   - **Solution:** Use theme variables, allow custom CSS via settings

### Constraints

1. **Performance:**
   - Must not impact terminal rendering performance
   - Target: <5ms per title bar render

2. **Screen Real Estate:**
   - Labels should be compact (default 24px height)
   - Must be disableable for maximizing content area

3. **Accessibility:**
   - Labels must be keyboard navigable
   - Screen reader compatible

4. **Cross-Platform:**
   - Consistent behavior on Mac, Windows, Linux
   - Handle different font rendering

5. **Backward Compatibility:**
   - Existing workspaces continue working without labels
   - New metadata keys don't break old clients

---

## Migration & Rollout Plan

### Phase 1: Core Implementation (2-3 weeks)
1. Implement TitleBar component and basic rendering
2. Add settings UI and global configuration
3. Implement manual title editing via context menu
4. Basic styling and theme integration

**Deliverable:** Users can manually add titles to panes

### Phase 2: Auto-Generation (1-2 weeks)
1. Implement auto-title generators for each block type
2. Add auto-generate toggle to settings
3. Implement keyboard shortcuts
4. Add widget bar toggle button

**Deliverable:** Titles automatically populate based on content

### Phase 3: Polish & Optimization (1 week)
1. Performance optimization and memoization
2. Edge case handling (small panes, long titles)
3. Accessibility improvements
4. Comprehensive testing

**Deliverable:** Production-ready feature

### Phase 4: Beta Testing (1-2 weeks)
1. Release to beta users
2. Gather feedback via in-app survey
3. Iterate on UX based on feedback
4. Fix bugs and edge cases

**Deliverable:** Stable, user-tested feature

### Phase 5: General Availability
1. Announce in release notes
2. Create tutorial video/documentation
3. Monitor adoption metrics
4. Iterate based on user feedback

---

## Testing Strategy

### Unit Tests
```typescript
// frontend/app/block/titlebar.test.tsx
describe("TitleBar Component", () => {
  it("renders custom title when provided", () => {
    render(<TitleBar title="My Custom Title" />);
    expect(screen.getByText("My Custom Title")).toBeInTheDocument();
  });

  it("hides when display-mode is 'never'", () => {
    render(<TitleBar displayMode="never" />);
    expect(screen.queryByRole("heading")).not.toBeInTheDocument();
  });

  it("shows edit button on hover when editable", () => {
    render(<TitleBar editable={true} />);
    const titleBar = screen.getByRole("heading");
    fireEvent.mouseEnter(titleBar);
    expect(screen.getByRole("button", { name: /edit/i })).toBeVisible();
  });
});

// frontend/app/block/autotitle.test.ts
describe("Auto Title Generator", () => {
  it("generates terminal title from cwd", () => {
    const block = { meta: { view: "term", "term:cwd": "/home/user/projects" } };
    expect(generateAutoTitle(block)).toBe("projects");
  });

  it("generates preview title from filename", () => {
    const block = { meta: { view: "preview", file: "/docs/README.md" } };
    expect(generateAutoTitle(block)).toBe("README.md");
  });
});
```

### Integration Tests
```typescript
describe("Title Bar Integration", () => {
  it("updates backend when title is edited", async () => {
    const { user } = render(<Block blockId="test-123" />);

    // Click title to edit
    await user.click(screen.getByText(/untitled/i));

    // Type new title
    const input = screen.getByRole("textbox");
    await user.clear(input);
    await user.type(input, "New Title");

    // Blur to save
    await user.tab();

    // Verify RPC call
    expect(mockRpcApi.SetMetaCommand).toHaveBeenCalledWith(
      expect.anything(),
      expect.objectContaining({
        meta: { "pane-title": "New Title" }
      })
    );
  });
});
```

### Visual Regression Tests
- Screenshot comparison for different themes
- Hover state verification
- Layout consistency across pane sizes

### Manual Testing Checklist
- [ ] Title displays correctly in all block types
- [ ] Edit functionality works for custom titles
- [ ] Auto-generation works for supported block types
- [ ] Settings UI updates take effect immediately
- [ ] Keyboard shortcuts function correctly
- [ ] Context menu items work as expected
- [ ] Titles persist across app restarts
- [ ] Performance remains acceptable with many panes
- [ ] Accessibility (keyboard nav, screen reader)
- [ ] Cross-platform consistency (Mac, Windows, Linux)

---

## Documentation Requirements

### User Documentation
1. **Feature Guide:** "Using Pane Title Labels"
   - How to enable/disable labels
   - Editing custom titles
   - Auto-generation options
   - Keyboard shortcuts

2. **Tutorial Video:** "Organizing Your Workspace with Pane Labels"
   - 2-3 minute walkthrough
   - Common use cases
   - Tips & tricks

3. **Settings Reference:** Update settings documentation with new options

### Developer Documentation
1. **Architecture Doc:** "Pane Title Labels Implementation"
   - Component structure
   - Data flow
   - Extension points

2. **API Reference:** New RPC methods and metadata keys

3. **Contributing Guide:** How to add auto-title support for new block types

---

## Future Enhancements (Out of Scope for v1)

### Phase 2 Features
1. **Bottom Position:** Allow titles at bottom of panes
2. **Overlay Mode:** Floating titles that don't take vertical space
3. **Color Coding:** Full custom color palettes for titles
4. **Emoji Support:** Allow emojis in titles for visual categorization
5. **Title Templates:** Pre-defined title formats (e.g., "Type: Name - Time")

### Advanced Features
1. **Smart Titles:** ML-based title suggestions from content
2. **Title Search:** Search panes by title
3. **Title Groups:** Hierarchical labeling (Project > Service > Instance)
4. **Title Macros:** Dynamic titles with variables (e.g., `{cwd} - {user}@{host}`)
5. **Title History:** Track title changes over time

### Integration Features
1. **Workspace Templates:** Save layouts with pre-labeled panes
2. **Title Sync:** Sync titles across devices via cloud settings
3. **Plugin API:** Allow plugins to customize title generation
4. **External Integration:** Pull titles from external tools (e.g., Jira tickets, GitHub PRs)

---

## Open Questions

1. **Should titles be searchable?**
   - Would require search index integration
   - Potentially high value for power users

2. **Should we support rich text in titles?**
   - Markdown formatting, links, etc.
   - Increases complexity significantly

3. **How to handle very dynamic content?**
   - E.g., terminal running `top` - title constantly changes
   - May need rate limiting or opt-out

4. **Should titles be part of workspace save/restore?**
   - Persistence across sessions
   - Syncing across devices

5. **Custom CSS for advanced users?**
   - Allow full style customization
   - Risk of breaking layouts

---

## Success Criteria

### Launch Criteria (MVP)
- [ ] Users can manually set custom titles on any pane
- [ ] Auto-generation works for 4+ block types (term, preview, editor, chat)
- [ ] Settings UI allows global enable/disable and display mode selection
- [ ] Context menu provides edit, auto-generate, and clear options
- [ ] Keyboard shortcuts implemented and documented
- [ ] Performance impact < 5ms per title bar render
- [ ] Zero breaking changes to existing workspaces
- [ ] Documentation complete (user guide + developer docs)
- [ ] 90%+ test coverage on new code

### Post-Launch Metrics (3 months)
- Adoption rate: >40% of active users enable labels
- Retention: <5% of users disable after enabling
- Performance: 95th percentile render time <5ms
- User satisfaction: >4.0/5 in feature survey
- Bug reports: <10 critical issues per month

---

## Appendix

### Related Features
- **Horizontal Widget Bar:** Complementary feature for workspace customization
- **Tab Grouping:** Future feature that could integrate with title labels
- **Workspace Presets:** Could include pre-labeled pane configurations

### Design Alternatives Considered

#### Alternative 1: Tab-Style Labels
- **Pros:** More compact, familiar pattern
- **Cons:** Conflicts with existing tab bar, less space-efficient

#### Alternative 2: Pane Badges
- **Pros:** Minimal visual footprint
- **Cons:** Limited information, harder to read at a glance

#### Alternative 3: Overlay Labels
- **Pros:** Zero vertical space consumption
- **Cons:** Can obscure content, accessibility concerns

**Decision:** Chose top title bar for best balance of visibility, usability, and aesthetics.

### References
- [VSCode Pane Titles](https://code.visualstudio.com/docs/getstarted/userinterface)
- [Tmux Window Names](https://man.openbsd.org/tmux.1#WINDOWS_AND_PANES)
- [iTerm2 Tab Titles](https://iterm2.com/documentation-preferences-appearance.html)

---

**Status:** Ready for Review
**Next Steps:** Technical design review with Wave Terminal core team
**Approvers:** [@sawka, @red, @evan]

---

## Addendum: Agent Identity Integration (2025-12-24)

### Context: Reactive Agent Communication

This pane labeling feature is a **prerequisite** for the reactive agent communication system. When multiple Claude Code agents run simultaneously in different panes, we need to:

1. **Display** agent identity (agent1, agent2, agentx) in pane titles
2. **Register** pane-to-agent mappings for webhook routing
3. **Inject** GitHub events into the correct pane

### Extended User Story

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  AGENT PANE IDENTIFICATION FLOW                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  1. Agent2 spawns in a AgentMux terminal pane                                │
│          │                                                                  │
│          ▼                                                                  │
│  2. Pane title auto-detects agent identity:                                 │
│     - From CWD: C:/Code/agent-workspaces/agent2 → "agent2"                  │
│     - From env: WAVEMUX_AGENT_ID=agent2                                     │
│     - From branch: git branch → agent2/feature-x                            │
│          │                                                                  │
│          ▼                                                                  │
│  3. Pane title displays: "🤖 Agent2 | agentmux/agent2/fix-auth"              │
│          │                                                                  │
│          ▼                                                                  │
│  4. Registration sent to AWS: {agent_id, pane_id, branches}                 │
│          │                                                                  │
│          ▼                                                                  │
│  5. GitHub event for agent2/fix-auth → routed to this pane                  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Agent-Specific Auto-Title Logic

Extend `autotitle.ts` to detect agent identity:

```typescript
function generateTerminalTitle(block: Block): string {
  const cwd = block.meta?.["term:cwd"] || "~";

  // Detect agent from workspace path
  const agentMatch = cwd.match(/agent-workspaces[\\\/](agent\d+|agentx)/i);
  if (agentMatch) {
    const agentId = agentMatch[1].toLowerCase();
    const gitBranch = block.meta?.["term:git-branch"];

    if (gitBranch) {
      return `🤖 ${agentId} | ${gitBranch}`;
    }
    return `🤖 ${agentId} | ${basename(cwd)}`;
  }

  // Fallback to standard terminal title
  const lastCmd = block.meta?.["term:lastcmd"];
  if (lastCmd) {
    return `${basename(cwd)}: ${truncate(lastCmd, 30)}`;
  }
  return basename(cwd) || "Terminal";
}
```

### Agent Registry Metadata

Extend block metadata for agent tracking:

```go
// pkg/waveobj/metaconsts.go
const (
    // ... existing pane-title constants
    MetaKey_AgentId         = "agent:id"          // Detected agent identity
    MetaKey_AgentBranches   = "agent:branches"    // Active git branches
    MetaKey_AgentWorkspace  = "agent:workspace"   // Workspace path
    MetaKey_AgentRegistered = "agent:registered"  // AWS registration status
)
```

### Registration Hook

Add to `wsh` commands:

```bash
# Auto-register on terminal spawn (via shell rc file)
wsh register-agent --auto

# Manual registration
wsh register-agent agent2

# Check registration status
wsh agent-status
```

### Visual Design: Agent Panes

**Minimal change - just replace "Terminal" text with agent identity:**

```
Before:  [🖥️] Terminal │                              │ ⚙️ ⤢ ✕
After:   [🖥️] Agent2   │  Fixing auth bug in login.ts │ ⚙️ ⤢ ✕
                ↑                    ↑
         frame:title          term:activity
```

| Pane Type | Label | Activity (gap) |
|-----------|-------|----------------|
| agent1 | `Agent1` | `Reviewing PR #42 comments` |
| agent2 | `Agent2` | `Fixing auth bug in login.ts` |
| agent3 | `Agent3` | `Running integration tests` |
| agentx | `AgentX` | `Creating database migration` |
| non-agent | `Terminal` | *(empty or cwd)* |

**No icon changes** - keep the existing terminal icon, just change the text.

### Implementation Priority

This addendum proposes implementing pane labels in two phases:

**Phase 1: Basic Labels (existing spec)**
- Manual title editing
- Auto-generation for all block types
- Settings UI

**Phase 2: Agent Integration (this addendum)**
- Agent identity detection from CWD/env
- Git branch tracking in title
- AWS registration integration
- Webhook routing groundwork

### Connection to Reactive Agent Communication

Once pane labels with agent identity are implemented:

1. **Lambda can route events** - Branch name → agent ID → pane ID
2. **Visual confirmation** - User sees which agent is in which pane
3. **Registration is automatic** - No manual mapping required
4. **Terminal injection works** - Events delivered to correct pane

This feature is the **foundation** for SPEC_REACTIVE_AGENT_COMMUNICATION.md.

---

## Extended Feature: Claude Code Title Integration

### Background: Claude Code Window Titles

Claude Code automatically updates the terminal window title using OSC (Operating System Command) escape sequences. These provide an AI-generated summary of current activity:

```
\033]0;Claude: Reviewing PR feedback\007
\033]0;Claude: Running npm test\007
\033]0;Claude: Creating commit for fix\007
```

**Current behavior:** These updates go to the OS window title (PowerShell/iTerm2/etc.)

**Opportunity:** AgentMux can intercept these and display them in the pane title bar.

### Proposed Integration

Combine agent identity (static) with Claude's activity summary (dynamic):

```
┌─────────────────────────────────────────────────────────────┐
│ 🤖 Agent2 | Fixing authentication bug in login.ts          │  ← Dynamic!
├─────────────────────────────────────────────────────────────┤
│ $ claude                                                    │
│ I'll fix the authentication bug. Let me first read...      │
└─────────────────────────────────────────────────────────────┘
```

### Technical Implementation

#### 1. Capture OSC Title Sequences

In the terminal PTY handler, intercept OSC 0/1/2 sequences:

```go
// pkg/wshutil/ptyhandler.go
func (h *PtyHandler) handleOSCSequence(code int, data string) {
    switch code {
    case 0, 2: // Set window title
        h.updatePaneTitle(data)
    case 1: // Set icon name (less common)
        // ignore or handle separately
    }
}

func (h *PtyHandler) updatePaneTitle(title string) {
    // Strip "Claude: " prefix if present
    activitySummary := strings.TrimPrefix(title, "Claude: ")

    // Publish WPS event for title update
    wps.Broker.Publish(wps.WaveEvent{
        Event:  wps.Event_TermTitleUpdate,
        Scopes: []string{h.BlockId},
        Data: map[string]string{
            "title":    title,
            "activity": activitySummary,
        },
    })
}
```

#### 2. Store in Block Metadata

```go
// New metadata keys
const (
    MetaKey_TermTitle       = "term:title"        // Raw terminal title
    MetaKey_TermActivity    = "term:activity"     // Parsed activity summary
    MetaKey_TermTitleTime   = "term:title-time"   // Last update timestamp
)
```

#### 3. Frontend Title Composition

```typescript
function generateAgentTerminalTitle(block: Block): string {
    const agentId = block.meta?.["agent:id"];
    const activity = block.meta?.["term:activity"];
    const gitBranch = block.meta?.["term:git-branch"];

    // Priority: Activity summary > Git branch > CWD
    if (agentId && activity) {
        return `🤖 ${agentId} | ${activity}`;
    }
    if (agentId && gitBranch) {
        return `🤖 ${agentId} | ${gitBranch}`;
    }
    if (agentId) {
        return `🤖 ${agentId}`;
    }

    // Fallback for non-agent terminals
    return activity || block.meta?.["term:cwd"] || "Terminal";
}
```

### User Experience

**Multiple agents working simultaneously:**

```
┌────────────────────────────────────────────┐  ┌────────────────────────────────────────────┐
│ 🤖 Agent1 | Reviewing PR #42 comments      │  │ 🤖 Agent2 | Running integration tests      │
├────────────────────────────────────────────┤  ├────────────────────────────────────────────┤
│ Looking at the reviewer feedback...        │  │ npm test -- --integration                  │
│                                            │  │ PASS src/api/auth.test.ts                  │
└────────────────────────────────────────────┘  └────────────────────────────────────────────┘

┌────────────────────────────────────────────┐  ┌────────────────────────────────────────────┐
│ 🤖 Agent3 | Creating database migration    │  │ 🤖 AgentX | Waiting for user input         │
├────────────────────────────────────────────┤  ├────────────────────────────────────────────┤
│ I'll create a migration for the new...     │  │ What authentication method would you...    │
└────────────────────────────────────────────┘  └────────────────────────────────────────────┘
```

**At a glance, you see:**
- Which agent is which
- What each is currently doing
- Activity updates in real-time

### Title Update Debouncing

Claude Code updates titles frequently. To avoid flicker:

```typescript
const TITLE_DEBOUNCE_MS = 500;

// Debounce rapid title updates
const debouncedTitleUpdate = useMemo(
    () => debounce((title: string) => {
        setDisplayTitle(title);
    }, TITLE_DEBOUNCE_MS),
    []
);
```

### Configuration Options

```json
{
  "pane-labels": {
    "agent-titles": {
      "enabled": true,
      "show-activity": true,      // Show Claude's activity summary
      "show-agent-id": true,      // Show agent identity prefix
      "activity-max-length": 50,  // Truncate long summaries
      "update-debounce-ms": 500   // Debounce rapid updates
    }
  }
}
```

### Exact UI Location

Replace "Terminal" label with agent identity, use the gap for activity:

**Current (non-agent terminal):**
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ [🖥️] Terminal │                                       │ ⚙️ ⤢ ✕ │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Agent terminal (proposed):**
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ [🖥️] Agent2   │  Fixing auth bug in login.ts          │ ⚙️ ⤢ ✕ │
│       ↑       │              ↑                        │         │
│  frame:title  │  block-frame-textelems-wrapper        │         │
│  (metadata)   │  (Claude's activity via term:activity)│         │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Key files:**

| Element | File | Line | Change |
|---------|------|------|--------|
| Label | `blockframe.tsx` | 231-233 | `frame:title` metadata → "Agent2" |
| Activity | `blockframe.tsx` | 309 | `headerTextElems` → Claude's summary |

**Implementation (simplest approach):**

The header already supports `frame:title` override - just set the metadata:

```go
// When agent is detected (in Go backend or via wsh command)
blockMeta["frame:title"] = "Agent2"
```

That's it. The existing code at `blockframe.tsx:231-233` handles this:

```tsx
if (blockData?.meta?.["frame:title"]) {
    viewName = blockData.meta["frame:title"];
}
```

For the activity summary, populate `viewText` in the terminal view model:

```typescript
get viewText(): HeaderElem[] {
    const activity = this.blockMeta?.["term:activity"];
    if (activity) {
        return [{
            elemtype: "text",
            text: activity,
            className: "agent-activity-text"
        }];
    }
    return [];
}
```

### OSC Sequence Reference

| Sequence | Purpose | Example |
|----------|---------|---------|
| `\033]0;Title\007` | Set window title + icon | Most common |
| `\033]1;Icon\007` | Set icon name only | Rare |
| `\033]2;Title\007` | Set window title only | Some terminals |

Claude Code uses `\033]0;...\007` format.

### Implementation Priority

This is an **extended feature** building on the basic pane labels:

1. **Phase 1:** Basic pane labels (manual + auto-generated)
2. **Phase 2:** Agent identity detection
3. **Phase 3:** OSC title capture + dynamic activity display ← This feature
4. **Phase 4:** AWS registration + webhook routing

### Benefits

| Benefit | Description |
|---------|-------------|
| **Real-time visibility** | See what each agent is doing without clicking into panes |
| **Zero configuration** | Works automatically with Claude Code's existing title updates |
| **Debugging aid** | Quickly identify stuck or waiting agents |
| **Coordination** | Understand agent activity when managing multiple parallel tasks |

### Considerations

1. **Non-Claude terminals** - Fall back to CWD/git branch for regular terminals
2. **Title spam** - Some tools update titles very frequently; debounce required
3. **Long titles** - Truncate with ellipsis, show full title on hover
4. **Privacy** - Activity summaries may contain sensitive info; consider hide option
