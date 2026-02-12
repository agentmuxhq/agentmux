# Spec: Border Color Not Solid

**Issue:** Focused pane border shows mixed green/white instead of solid accent color
**Severity:** Medium (visual inconsistency)
**Component:** frontend/app/block/

---

## Problem Description

When a pane is focused, the border should be solid accent-color (green). Instead, it appears mixed with white or the unfocused border color is showing through.

---

## Root Cause Analysis

### Current Implementation (`block.scss`)

```scss
.block.block-focused .block-mask {
    border: 2px solid var(--accent-color);
    // ...
}

.block .block-mask {
    border: 1px solid var(--border-color);
    // ...
}
```

### Suspected Causes

1. **Border width transition**
   - Going from 1px unfocused to 2px focused
   - During transition or with subpixel rendering, white may show through

2. **Overlapping borders**
   - Adjacent panes each have their own border
   - When focused, borders may overlap causing visual artifacts

3. **Box-sizing issues**
   - `box-sizing: border-box` affects how borders are rendered
   - 1px vs 2px difference may cause layout shift

4. **CSS variable resolution**
   - `--accent-color` may not be resolving correctly
   - Fallback values might be white

5. **Z-index layering**
   - `.block-mask-inner` has different z-index when focused
   - Background of lower layer might be visible

---

## Solution Options

### Option A: Consistent Border Width (Recommended)

Use 2px border for both states, only change color:

```scss
.block .block-mask {
    border: 2px solid var(--border-color);  // Always 2px
}

.block.block-focused .block-mask {
    border-color: var(--accent-color);  // Only change color
}
```

This eliminates width transitions and layout shifts.

### Option B: Outline Instead of Border

Use CSS outline which doesn't affect layout:

```scss
.block .block-mask {
    border: none;
    outline: 2px solid transparent;
}

.block.block-focused .block-mask {
    outline-color: var(--accent-color);
}
```

### Option C: Box-shadow for Focus Ring

Use inset box-shadow for focus indicator:

```scss
.block.block-focused .block-mask {
    box-shadow: inset 0 0 0 2px var(--accent-color);
}
```

This overlays on top and won't show underlying colors.

---

## Debugging Steps

1. Inspect focused pane with DevTools
2. Check computed styles for `.block-mask`
3. Verify `--accent-color` value
4. Check for overlapping elements
5. Test with forced solid color (no variable)

---

## Testing Plan

1. Open AgentMux with 2+ panes in a split layout
2. Click each pane - verify solid green border appears
3. Check corners for any white showing through
4. Check edges between adjacent panes
5. Test with different layouts (horizontal, vertical, grid)
6. Verify unfocused panes have consistent white border

---

## Implementation Steps

1. Identify exact cause via DevTools inspection
2. Apply Option A (consistent 2px border)
3. Test all layout configurations
4. Verify no layout shift when focus changes
5. Deploy and verify

---

## Files to Modify

- `frontend/app/block/block.scss` - Border styling
- Possibly `frontend/app/theme/` - Variable definitions
