# Context Menu Hierarchy Design

**Date**: 2026-02-13
**Version**: 0.26.5
**Purpose**: Unified context menu system with inheritance

---

## Overview

Design a hierarchical context menu system where:
1. **Parent menu** (tabbar) provides base items
2. **Pane menus** inherit parent items and add their own
3. **Consistent UX** across all right-click contexts
4. **Extensible** for different pane types

---

## Current State Analysis

### Existing Context Menus

**1. Tabbar Context Menu** (`frontend/app/tab/tabbar.tsx` line 199)
```tsx
const handleTabBarContextMenu = useCallback((e: React.MouseEvent) => {
    e.preventDefault();

    const menuItems: ContextMenuItem[] = [
        { label: "AgentMux v0.26.5", click: () => copyVersion() },
        { type: "separator" },
        { label: "Widgets", type: "separator" },
        ...widgetMenuItems,
        { type: "separator" },
        { label: "Edit widgets.json", click: () => openConfig() }
    ];

    ContextMenuModel.showContextMenu(menuItems, e);
});
```

**2. Widget Context Menu** (`frontend/app/tab/widgetbar.tsx`)
- Likely has own menu items
- Not inheriting tabbar items

**3. Pane Context Menus** (various pane types)
- Terminal panes
- Preview panes
- Web widget panes
- Currently isolated, no inheritance

### ContextMenuModel API

Location: `frontend/app/store/contextmenu.ts` (likely)

```tsx
ContextMenuModel.showContextMenu(items: ContextMenuItem[], event: React.MouseEvent)
```

---

## Design Goals

### 1. Menu Inheritance
- Child menus automatically include parent items
- Clear visual separation between levels
- Ability to override or hide parent items

### 2. Pane-Specific Actions
Each pane type should add:
- **Close** - Close current pane
- **Maximize** - Maximize pane (fullscreen)
- **Duplicate** - Clone pane
- **Move** - Move to different position
- **Split** - Split pane vertically/horizontally

### 3. Extensibility
- Easy to add new pane types
- Easy to add new menu items
- Composable menu builders

---

## Architecture Design

### Option A: Menu Builder Pattern (Recommended)

```tsx
// frontend/app/menu/menu-builder.ts

export class MenuBuilder {
    private items: ContextMenuItem[] = [];

    // Add single item
    add(item: ContextMenuItem): this {
        this.items.push(item);
        return this;
    }

    // Add separator
    separator(): this {
        this.items.push({ type: "separator" });
        return this;
    }

    // Merge another menu
    merge(other: MenuBuilder, position: 'before' | 'after' = 'after'): this {
        if (position === 'before') {
            this.items = [...other.build(), ...this.items];
        } else {
            this.items = [...this.items, ...other.build()];
        }
        return this;
    }

    // Insert parent menu items
    mergeParent(parentBuilder: MenuBuilder): this {
        return this.merge(parentBuilder, 'before');
    }

    // Build final menu
    build(): ContextMenuItem[] {
        return this.items;
    }

    // Show menu
    show(event: React.MouseEvent): void {
        ContextMenuModel.showContextMenu(this.build(), event);
    }
}

// Base menu builders
export const createTabBarMenu = (): MenuBuilder => {
    const menu = new MenuBuilder();
    const version = getApi().getAboutModalDetails().version;

    menu
        .add({
            label: `AgentMux v${version}`,
            click: () => copyVersion(version)
        })
        .separator();

    return menu;
};

export const createWidgetMenu = (): MenuBuilder => {
    const menu = new MenuBuilder();
    const widgets = getWidgets();

    menu.add({ label: "Widgets", type: "separator" });

    widgets.forEach(widget => {
        menu.add({
            label: widget.label,
            type: "checkbox",
            checked: !widget.hidden,
            click: () => toggleWidget(widget.id)
        });
    });

    menu
        .separator()
        .add({ label: "Edit widgets.json", click: openWidgetsConfig });

    return menu;
};

export const createPaneMenu = (paneId: string): MenuBuilder => {
    const menu = new MenuBuilder();

    menu
        .add({ label: "Close Pane", click: () => closePane(paneId) })
        .add({ label: "Maximize Pane", click: () => maximizePane(paneId) })
        .separator()
        .add({ label: "Duplicate Pane", click: () => duplicatePane(paneId) })
        .add({ label: "Split Horizontal", click: () => splitPane(paneId, 'horizontal') })
        .add({ label: "Split Vertical", click: () => splitPane(paneId, 'vertical') });

    return menu;
};
```

**Usage in Components:**

```tsx
// In Tabbar
const handleTabBarContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();

    const menu = createTabBarMenu()
        .merge(createWidgetMenu());

    menu.show(e);
};

// In Pane
const handlePaneContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();

    const menu = createPaneMenu(paneId)
        .separator()
        .mergeParent(createTabBarMenu())
        .merge(createWidgetMenu());

    menu.show(e);
};
```

### Option B: React Context Pattern

```tsx
// frontend/app/context/MenuContext.tsx

interface MenuContextValue {
    getBaseMenu: () => ContextMenuItem[];
    addToMenu: (items: ContextMenuItem[]) => void;
}

export const MenuContext = createContext<MenuContextValue>(null);

export const MenuProvider: React.FC = ({ children }) => {
    const [baseItems, setBaseItems] = useState<ContextMenuItem[]>([]);

    const getBaseMenu = useCallback(() => {
        const version = getApi().getAboutModalDetails().version;
        return [
            { label: `AgentMux v${version}`, click: () => copyVersion() },
            { type: "separator" }
        ];
    }, []);

    return (
        <MenuContext.Provider value={{ getBaseMenu, addToMenu: setBaseItems }}>
            {children}
        </MenuContext.Provider>
    );
};

// Usage
const { getBaseMenu } = useContext(MenuContext);

const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    const items = [
        ...getBaseMenu(),
        { label: "Pane Action", click: () => {} }
    ];
    ContextMenuModel.showContextMenu(items, e);
};
```

### Option C: HOC Pattern

```tsx
// frontend/app/hoc/withContextMenu.tsx

export const withContextMenu = <P extends {}>(
    Component: React.ComponentType<P>,
    getMenuItems: (props: P) => ContextMenuItem[]
) => {
    return (props: P) => {
        const handleContextMenu = (e: React.MouseEvent) => {
            e.preventDefault();
            const items = getMenuItems(props);
            ContextMenuModel.showContextMenu(items, e);
        };

        return (
            <div onContextMenu={handleContextMenu}>
                <Component {...props} />
            </div>
        );
    };
};
```

---

## Recommended Approach: Menu Builder (Option A)

**Why:**
- ✅ Simple and explicit
- ✅ No React context overhead
- ✅ Easy to test
- ✅ Flexible composition
- ✅ TypeScript friendly

**Implementation Steps:**

### Phase 1: Create Menu Builder (30 min)

1. Create `frontend/app/menu/menu-builder.ts`
2. Implement MenuBuilder class
3. Add base menu builders (tabbar, widget, pane)
4. Add utility functions

### Phase 2: Refactor Tabbar Menu (15 min)

1. Update `frontend/app/tab/tabbar.tsx`
2. Use MenuBuilder instead of inline array
3. Test tabbar context menu

### Phase 3: Add Pane Menus (45 min)

1. Find all pane types (terminal, preview, web, etc.)
2. Add context menu to each
3. Use `createPaneMenu().mergeParent(createTabBarMenu())`
4. Stub out pane actions (close, maximize, etc.)

### Phase 4: Testing (30 min)

1. Test menu hierarchy
2. Test inheritance
3. Verify visual appearance
4. Cross-platform testing

---

## Menu Structure

### Level 1: Tabbar (Base)
```
AgentMux v0.26.5
─────────────────
Widgets
─────────────────
□ Widget 1
□ Widget 2
─────────────────
Edit widgets.json
```

### Level 2: Pane (Inherits Tabbar)
```
Close Pane
Maximize Pane
─────────────────
Duplicate Pane
Split Horizontal
Split Vertical
─────────────────
[Pane-specific items]
─────────────────
AgentMux v0.26.5      ← Inherited
─────────────────
Widgets               ← Inherited
─────────────────
□ Widget 1            ← Inherited
□ Widget 2            ← Inherited
─────────────────
Edit widgets.json     ← Inherited
```

### Level 3: Terminal Pane (Inherits Pane + Tabbar)
```
Close Pane            ← From Pane
Maximize Pane         ← From Pane
─────────────────
Copy
Paste
Clear Terminal        ← Terminal-specific
─────────────────
Duplicate Pane        ← From Pane
Split Horizontal      ← From Pane
Split Vertical        ← From Pane
─────────────────
AgentMux v0.26.5      ← From Tabbar
─────────────────
Widgets               ← From Tabbar
─────────────────
□ Widget 1            ← From Tabbar
□ Widget 2            ← From Tabbar
─────────────────
Edit widgets.json     ← From Tabbar
```

---

## Pane Actions Implementation

### Stubbed Actions (Phase 3)

```tsx
// frontend/app/pane/pane-actions.ts

export const closePane = (paneId: string) => {
    console.log(`[STUB] closePane: ${paneId}`);
    // TODO: Implement actual close
};

export const maximizePane = (paneId: string) => {
    console.log(`[STUB] maximizePane: ${paneId}`);
    // TODO: Implement fullscreen pane
};

export const duplicatePane = (paneId: string) => {
    console.log(`[STUB] duplicatePane: ${paneId}`);
    // TODO: Clone pane with same content
};

export const splitPane = (paneId: string, direction: 'horizontal' | 'vertical') => {
    console.log(`[STUB] splitPane: ${paneId} ${direction}`);
    // TODO: Create split layout
};

export const movePane = (paneId: string, direction: 'up' | 'down' | 'left' | 'right') => {
    console.log(`[STUB] movePane: ${paneId} ${direction}`);
    // TODO: Move pane in layout
};
```

---

## Files to Create/Modify

### New Files
- [ ] `frontend/app/menu/menu-builder.ts` - MenuBuilder class
- [ ] `frontend/app/menu/base-menus.ts` - Base menu creators
- [ ] `frontend/app/pane/pane-actions.ts` - Pane action stubs

### Modified Files
- [ ] `frontend/app/tab/tabbar.tsx` - Use MenuBuilder
- [ ] `frontend/app/block/blockframe.tsx` - Add pane context menu
- [ ] `frontend/app/view/*/` - Add context menus to each view type

---

## Testing Plan

### Test 1: Tabbar Menu
- Right-click tabbar
- Verify version appears
- Verify widgets appear
- Click version → copies to clipboard

### Test 2: Pane Menu Inheritance
- Right-click on pane
- Verify pane actions appear first
- Verify tabbar items appear after separator
- Verify no duplicates

### Test 3: Pane Actions
- Click "Close Pane" → See console log (stub)
- Click "Maximize Pane" → See console log (stub)
- All stubbed actions → Log correctly

### Test 4: Cross-Platform
- Test on macOS
- Test on Windows (if available)
- Verify menu appearance and behavior

---

## Next Steps

1. **Build completes** → Test on desktop
2. **Create MenuBuilder** → Core infrastructure
3. **Refactor tabbar menu** → Use new builder
4. **Add pane menus** → With inheritance
5. **Test thoroughly** → All menu interactions
6. **Document usage** → For future pane types

---

## Success Criteria

- [ ] Right-click tabbar → Shows version + widgets
- [ ] Right-click pane → Shows pane actions + inherited items
- [ ] No duplicate menu items
- [ ] Clean visual hierarchy
- [ ] All actions log correctly (stubs)
- [ ] Easy to extend for new pane types
- [ ] Works on macOS and Windows
