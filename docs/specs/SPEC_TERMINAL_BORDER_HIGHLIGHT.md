# WaveTerm Terminal Border Highlight Specification

**Version:** 1.0
**Date:** 2025-10-19
**Status:** Design Phase
**Target:** WaveTerm Fork v0.12.4+

---

## Executive Summary

Add high-contrast white borders to all unselected terminal blocks and maintain the existing green border for selected blocks. Additionally, minimize or eliminate the gap between terminal windows for a more compact layout.

---

## 1. Current State Analysis

### 1.1 Current Border System

**Location:** `frontend/app/block/block.scss`

**Current Implementation:**
- Selected blocks: 2px solid border with `var(--accent-color)` (green)
  - Line 438: `.block-focused .block-mask { border: 2px solid var(--accent-color); }`
- Unselected blocks: 2px **transparent** border
  - Line 407: `.block-mask { border: 2px solid transparent; }`
- Block frame padding: 1px
  - Line 69: `&.block-frame-default { padding: 1px; }`

**Key Classes:**
- `.block-mask` - Overlay that handles border display (lines 401-434)
- `.block-focused` - Applied when block is selected (lines 436-447)
- `.block-frame-default` - Main block container (lines 67-448)

### 1.2 Current Gap System

**Location:** `frontend/app/tab/tabcontent.tsx`

**Current Implementation:**
- Gap between blocks controlled by `window:tilegapsize` setting
- Default value: **3px** (configurable)
- Defined in:
  - Setting: `pkg/wconfig/defaultconfig/settings.json:15`
  - Frontend usage: `frontend/app/tab/tabcontent.tsx:15-18`
- Gap applied through TileLayout component via `gapSizePx` prop (line 46)

**Tailwind Padding:**
- Container padding: `pt-[3px] pr-[3px]` (top and right 3px)
  - Line 70 in `tabcontent.tsx`: `<div className="...pt-[3px] pr-[3px]">`

---

## 2. Proposed Changes

### 2.1 Border Styling

**Goal:** Make all terminal blocks visually distinct with high-contrast borders

**Changes:**

#### A. Unselected Terminal Border (NEW)
```scss
// In frontend/app/block/block.scss, modify .block-mask default border
.block-mask {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    border: 2px solid #ffffff;  // CHANGED: was transparent, now hard white
    pointer-events: none;
    padding: 2px;
    border-radius: var(--block-border-radius);
    z-index: var(--zindex-block-mask-inner);

    // ... rest of existing properties
}
```

#### B. Selected Terminal Border (KEEP EXISTING)
```scss
// Keep existing green border for focused blocks
&.block-focused {
    .block-mask {
        border: 2px solid var(--accent-color);  // GREEN - no change
    }
}
```

**Behavior:**
1. **User clicks terminal A**: Terminal A border turns **green** (#00FF00 or var accent)
2. **User clicks terminal B**:
   - Terminal A border changes from green → **white** (#ffffff)
   - Terminal B border changes from white → **green**
3. All non-focused terminals always have **hard white** border

### 2.2 Gap Minimization

**Goal:** Reduce or eliminate spacing between terminal blocks

**Changes Required:**

#### A. Change Default Gap Setting
**File:** `pkg/wconfig/defaultconfig/settings.json`

```json
{
    "window:tilegapsize": 0  // CHANGED: was 3, now 0 for minimal padding
}
```

#### B. Remove Container Padding
**File:** `frontend/app/tab/tabcontent.tsx` (line 70)

```tsx
// BEFORE:
<div className="flex flex-row flex-grow min-h-0 w-full items-center justify-center overflow-hidden relative pt-[3px] pr-[3px]">

// AFTER:
<div className="flex flex-row flex-grow min-h-0 w-full items-center justify-center overflow-hidden relative">
```

**Removes:** 3px top and 3px right padding

#### C. Schema Update (if needed)
**File:** `schema/settings.json` (line 185+)

Update default value documentation:
```json
"window:tilegapsize": {
    "type": "integer",
    "title": "Window Tile Gap Size",
    "description": "Gap size between blocks in pixels",
    "default": 0  // CHANGED: was 3
}
```

---

## 3. Visual Comparison

### Before:
```
┌─────────────────────┐  3px gap  ┌─────────────────────┐
│ Terminal A          │           │ Terminal B          │
│ (no visible border) │           │ (green border)      │
│                     │           │ [SELECTED]          │
└─────────────────────┘           └─────────────────────┘
     3px padding
```

### After:
```
┌─────────────────────┬─────────────────────┐
│ Terminal A          │ Terminal B          │
│ (white border)      │ (green border)      │
│                     │ [SELECTED]          │
└─────────────────────┴─────────────────────┘
     0px gap, 0px padding
```

---

## 4. Implementation Plan

### Phase 1: Border Changes
**File:** `frontend/app/block/block.scss`

1. Locate `.block-mask` class (line ~407)
2. Change `border: 2px solid transparent;` → `border: 2px solid #ffffff;`
3. Test selection behavior:
   - Click different terminals
   - Verify green → white transition
   - Verify white → green transition

**Acceptance Criteria:**
- All unselected blocks show white border
- Selected block shows green border
- Border transitions smoothly on focus change

### Phase 2: Gap Removal
**Files:**
- `pkg/wconfig/defaultconfig/settings.json`
- `frontend/app/tab/tabcontent.tsx`
- `schema/settings.json`

1. Change default `window:tilegapsize` from 3 → 0
2. Remove `pt-[3px] pr-[3px]` from TabContent container
3. Test layout with multiple terminals
4. Verify no unwanted overlapping

**Acceptance Criteria:**
- No visible gap between terminals
- Borders don't overlap (each terminal has distinct 2px border)
- Layout remains stable during drag/drop operations

### Phase 3: Testing

**Test Cases:**
1. Single terminal display
2. 2x2 grid of terminals
3. 3x3 grid of terminals
4. Rapid focus switching between terminals
5. Drag and drop terminal repositioning
6. Window resize with multiple terminals

**Edge Cases:**
- Very small terminal windows
- Magnified blocks
- Ephemeral/preview blocks
- Block dragging state
- Masked/loading states

---

## 5. Technical Considerations

### 5.1 Border Color Specifics

**Hard White:**
- Use `#ffffff` (pure white) instead of `var(--main-text-color)`
- Ensures maximum contrast regardless of theme
- Consistent across all color schemes

**Alternative (if theme-aware is needed):**
```scss
border: 2px solid rgb(from var(--main-text-color) r g b / 1.0);
```
But **#ffffff** is recommended for "hard white" as requested.

### 5.2 Border Width Consistency

- Keep 2px border width (existing standard)
- Matches selected border width for visual consistency
- Changing width would require layout adjustment

### 5.3 Gap Size Settings

**User Configuration:**
- Users can still manually override `window:tilegapsize` in settings
- Default changes to 0, but setting remains configurable
- Document in UI that 0 is the new recommended value

**TileLayout Integration:**
- Gap size passed through `gapSizePx` prop to TileLayout component
- Layout engine (frontend/layout/lib/TileLayout.tsx) handles spacing
- Setting to 0 should work without additional code changes

### 5.4 Performance Impact

**Border Rendering:**
- No performance impact (existing border, just color change)
- CSS-only change, no JavaScript overhead

**Gap Reduction:**
- May improve rendering performance (fewer pixels to render between blocks)
- No computational difference in layout algorithm

---

## 6. Code References

### Modified Files

1. **frontend/app/block/block.scss:407**
   - Change `.block-mask` border from transparent to #ffffff

2. **pkg/wconfig/defaultconfig/settings.json:15**
   - Change `window:tilegapsize` from 3 to 0

3. **frontend/app/tab/tabcontent.tsx:70**
   - Remove `pt-[3px] pr-[3px]` from container className

4. **schema/settings.json:185** (optional)
   - Update default value documentation

### Dependent Files (no changes needed)
- `frontend/layout/lib/TileLayout.tsx` - Respects gapSizePx prop
- `pkg/wconfig/settingsconfig.go` - Type definitions
- `pkg/wconfig/metaconsts.go` - Config key constants

---

## 7. User Experience

### Visual Clarity
- **Before:** Hard to distinguish inactive terminals (transparent border)
- **After:** All terminals clearly outlined with high-contrast white borders
- **Selected:** Still uses familiar green border for focus indication

### Space Efficiency
- **Before:** 3px gaps + 3px padding = ~9-12px total spacing
- **After:** 0px gaps + 0px padding + 2px borders = 2px visual separation
- **Benefit:** More screen real estate for terminal content

### Accessibility
- High contrast borders improve visibility
- Clear visual distinction between focused/unfocused states
- Maintains existing color-blind friendly green selection

---

## 8. Rollback Plan

If issues arise:

1. **Revert border color:**
   ```scss
   border: 2px solid transparent;
   ```

2. **Restore gap setting:**
   ```json
   "window:tilegapsize": 3
   ```

3. **Restore padding:**
   ```tsx
   className="...pt-[3px] pr-[3px]"
   ```

All changes are isolated and easily reversible.

---

## 9. Future Enhancements

### 9.1 Customizable Border Colors
Allow users to configure:
- Unfocused border color (default: #ffffff)
- Focused border color (default: var(--accent-color))
- Border width (default: 2px)

**New Settings:**
```json
{
    "window:border-unfocused-color": "#ffffff",
    "window:border-focused-color": "#00ff00",
    "window:border-width": 2
}
```

### 9.2 Hover State Border
Add subtle hover effect:
```scss
.block-mask:hover {
    border-color: rgba(255, 255, 255, 0.8); // Slightly dimmed white
}
```

### 9.3 Multi-Selection Borders
Different color for multi-selected blocks:
```scss
.block-mask.multi-selected {
    border: 2px solid #ffaa00; // Orange for multi-select
}
```

---

## 10. Success Metrics

- Visual distinction between all terminal blocks improved
- User feedback on border visibility (target: 90%+ positive)
- No regression in layout stability or performance
- Gap reduction increases usable screen space by ~5-10%
- Zero overlapping borders or visual artifacts

---

## 11. References

- **Current Block Styling:** `frontend/app/block/block.scss`
- **Gap Configuration:** `pkg/wconfig/defaultconfig/settings.json`
- **TabContent Layout:** `frontend/app/tab/tabcontent.tsx`
- **TileLayout Engine:** `frontend/layout/lib/TileLayout.tsx`
- **Settings Schema:** `schema/settings.json`

---

**Status:** Ready for implementation
**Next Steps:** Implement Phase 1 (border changes), test, then proceed to Phase 2 (gap removal)
