# WaveMux Pane Border Fix Analysis

## Problem Statement

The pane border system has the following issues:

1. **Double borders between adjacent panes**: Each pane has its own 2px border, so two adjacent panes create a 4px gap between them, while panes at window edges only have 2px.

2. **Split border colors**: When a selected (green/accent) pane is adjacent to an unselected (white) pane, the shared border appears half green and half white instead of fully green.

3. **Green should take precedence**: Selected pane borders (accent color) should always override unselected borders (white) on shared edges.

## Current Implementation

### File Locations
- **Block borders**: `frontend/app/block/block.scss` (lines 401-454)
- **Tile layout/gaps**: `frontend/layout/lib/tilelayout.scss`
- **Block component**: `frontend/app/block/blockframe.tsx`

### How it works now

```scss
// tilelayout.scss - Gap between panes
--gap-size-px: 5px;

.tile-node:not(:only-child) .tile-leaf {
    padding: calc(var(--gap-size-px) / 2);  // 2.5px padding creates gaps
}

// block.scss - Border styling
.block-mask {
    border: 2px solid #ffffff;  // Unselected border
    border-radius: var(--block-border-radius);
    z-index: var(--zindex-block-mask-inner);
}

&.block-focused {
    position: relative;
    z-index: 10;  // Elevate above adjacent blocks

    .block-mask {
        border: 2px solid var(--accent-color);  // Selected (green) border
        outline: 2px solid var(--accent-color);
        outline-offset: -2px;
        z-index: calc(var(--zindex-block-mask-inner) + 1);
    }
}
```

### Past Fix Attempts
- **PR #37**: "Pane border width at window edges matches interior borders (#17)"
- **PR #36**: "Selected terminal shows solid green border (#14)"
- **PR #18**: "Fix focused terminal border highlighting (Issue #14)"

These PRs attempted to fix the issue using z-index elevation, but the core problems remain.

---

## Solution Options

### Option A: Collapse Adjacent Borders (CSS Negative Margin)

Use negative margins to collapse adjacent borders so they overlap instead of stack.

```scss
.block-mask {
    border: 2px solid #ffffff;
    // Negative margin on right and bottom to overlap with adjacent panes
    margin-right: -2px;
    margin-bottom: -2px;
}

// First column gets left border
.tile-node:first-child .block-mask {
    margin-left: 0;
}

// Last row gets bottom border back
.tile-node:last-child .block-mask {
    margin-bottom: 0;
}

// Focused block gets higher z-index so its border wins
&.block-focused .block-mask {
    z-index: 20;  // Higher than unfocused
    border-color: var(--accent-color);
}
```

**Pros:**
- Simple CSS-only solution
- No layout changes needed

**Cons:**
- Complex edge case handling for grid positions
- May require JS to determine grid position

---

### Option B: Single Border Container (Shared Border Approach)

Instead of each pane having its own border, create a shared border system at the layout level.

```scss
.tile-layout {
    // Layout handles gaps with CSS grid gap
    gap: 2px;
    background-color: #ffffff;  // Unfocused border color shows in gaps
}

.tile-node {
    background-color: var(--block-bg-color);
    // No border on individual blocks
}

// Focused block gets outline only
.block.block-focused {
    outline: 2px solid var(--accent-color);
    outline-offset: -2px;
    z-index: 10;
}
```

**Pros:**
- Clean separation - layout handles structure, blocks handle content
- No double borders possible

**Cons:**
- Significant refactor of layout system
- Background color approach may have issues with transparency

---

### Option C: Box-Shadow Instead of Border (Recommended)

Replace borders with inset box-shadows that don't take up layout space.

```scss
.block-mask {
    border: none;
    box-shadow: inset 0 0 0 2px #ffffff;  // White inset shadow for unselected
}

&.block-focused .block-mask {
    box-shadow: inset 0 0 0 2px var(--accent-color);  // Green for selected
    z-index: 10;  // Still elevate to overlay adjacent unselected blocks
}
```

**Pros:**
- Box-shadows don't affect layout, so no double-width issue
- Easy to change colors
- z-index naturally makes focused box-shadow overlay adjacent ones

**Cons:**
- Slightly different visual rendering than borders
- May need adjustment for border-radius

---

### Option D: Outline-Only Approach

Use only outlines (not borders) which don't affect layout and stack visually.

```scss
.block-mask {
    border: none;
    outline: 2px solid #ffffff;
    outline-offset: -2px;  // Inset so it doesn't overflow
}

&.block-focused .block-mask {
    outline: 2px solid var(--accent-color);
    z-index: 10;  // Elevate so green outline overlays white
}
```

**Pros:**
- Outlines don't affect layout
- Clean CSS-only solution
- z-index handles precedence naturally

**Cons:**
- Outline doesn't respect border-radius in all browsers
- May have clipping issues

---

### Option E: Half-Border Approach with Position Classes

Apply borders only to specific sides based on pane position in the grid.

```tsx
// In blockframe.tsx - determine position
const isLeftEdge = /* compute from layout */;
const isTopEdge = /* compute from layout */;

className={clsx("block-mask", {
    "border-left": isLeftEdge,
    "border-top": isTopEdge,
    // Right and bottom borders always applied
})}
```

```scss
.block-mask {
    border: 2px solid #ffffff;
    border-left: none;
    border-top: none;
}

// Edge cases
.block-mask.border-left { border-left: 2px solid #ffffff; }
.block-mask.border-top { border-top: 2px solid #ffffff; }

// Focused overrides
&.block-focused .block-mask {
    border-color: var(--accent-color);
    z-index: 10;
}
```

**Pros:**
- Precise control over which borders appear
- Mathematically correct border widths

**Cons:**
- Requires layout system to provide position info
- More complex implementation
- Adjacent selected/unselected still problematic

---

## Recommended Solution: Option C (Box-Shadow)

The box-shadow approach is recommended because:

1. **No layout impact**: Box-shadows don't affect element sizing
2. **Natural stacking**: Higher z-index elements' shadows overlay lower ones
3. **Minimal code change**: Replace border declarations with box-shadow
4. **Green precedence**: Focused block's elevated z-index makes its shadow win

### Implementation Steps

1. **Update `block.scss`**:
```scss
.block-mask {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    // border: 2px solid #ffffff;  // REMOVE
    box-shadow: inset 0 0 0 2px #ffffff;  // ADD
    pointer-events: none;
    border-radius: var(--block-border-radius);
    z-index: var(--zindex-block-mask-inner);
}

&.block-focused {
    position: relative;
    z-index: 10;

    .block-mask {
        // border: 2px solid var(--accent-color);  // REMOVE
        // outline: 2px solid var(--accent-color);  // REMOVE
        // outline-offset: -2px;  // REMOVE
        box-shadow: inset 0 0 0 2px var(--accent-color);  // ADD
        z-index: calc(var(--zindex-block-mask-inner) + 1);
    }
}
```

2. **Test scenarios**:
   - Single pane (no borders should show, or minimal)
   - Two horizontal panes (shared vertical border)
   - Two vertical panes (shared horizontal border)
   - 2x2 grid (all shared borders)
   - Selected adjacent to unselected (green should win)

3. **Edge cases to verify**:
   - Magnified/ephemeral blocks
   - Block previews
   - AI panel adjacency

---

## Files to Modify

| File | Changes |
|------|---------|
| `frontend/app/block/block.scss` | Replace border with box-shadow in `.block-mask` |
| `frontend/app/block/blockframe.tsx` | No changes needed |
| `frontend/layout/lib/tilelayout.scss` | May need gap adjustment |

---

## Testing Checklist

- [ ] Single block - no visible border doubling
- [ ] 2 horizontal blocks - equal border between and at edges
- [ ] 2 vertical blocks - equal border between and at edges
- [ ] 2x2 grid - consistent borders everywhere
- [ ] Selected + unselected adjacent - green border fully visible
- [ ] Three blocks: unselected | selected | unselected - green shows on both sides
- [ ] Magnified block borders correct
- [ ] Block preview borders correct
- [ ] AI panel visible - borders still work

---

## Alternative: Hybrid Approach

If box-shadow alone doesn't work perfectly, combine with the collapse approach:

```scss
.block.block-frame-default {
    padding: 0;  // Remove padding that creates double borders
    margin: 1px;  // Half the desired border width
}

.block-mask {
    box-shadow: inset 0 0 0 1px #ffffff;  // Half-width shadow
}

// Focused gets full treatment
&.block-focused {
    margin: 0;  // No margin - takes full space
    z-index: 10;

    .block-mask {
        box-shadow: inset 0 0 0 2px var(--accent-color);
    }
}
```

This creates 2px total between unfocused blocks (1px + 1px) and lets focused blocks overlay with their full 2px green border.

---

## Next Steps

1. Create a branch: `agenta/fix-pane-border-doubling`
2. Implement Option C (box-shadow approach)
3. Test all scenarios from checklist
4. If issues, try hybrid approach
5. Create PR with before/after screenshots
