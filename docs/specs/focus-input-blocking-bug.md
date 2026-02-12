# Spec: Focus/Input Blocking Bug

**Issue:** Cannot type in a pane until clicking another pane first, then returning
**Severity:** High (blocks user interaction)
**Component:** frontend/app/block/

---

## Problem Description

When a terminal pane should be focused, keyboard input is blocked. The user must click on a different pane first, then click back to the original pane before typing works.

This suggests the focus state is visually indicated but input events are not being routed correctly.

---

## Root Cause Analysis

Based on codebase exploration:

### Relevant Files
- `frontend/app/block/block.tsx` - Focus handling logic
- `frontend/app/block/block.scss` - Visual focus states

### Suspected Causes

1. **`disablePointerEvents` flag** (`block.tsx:~25`)
   - The BlockFrame component accepts a `disablePointerEvents` prop
   - If this gets stuck as `true`, clicks won't register properly

2. **`dummy-focus` element** (`block.tsx`)
   - A hidden input element is used to capture focus
   - If this element isn't properly receiving focus, keyboard input fails

3. **Focus event propagation** (`handleChildFocus`)
   - The `handleChildFocus` callback manages focus state
   - Race conditions or stale closures could cause incorrect state

4. **Z-index layering** (`block.scss:~28-35`)
   - Focused blocks get `z-index: var(--zindex-block-mask-inner)`
   - If z-index isn't applied correctly, click events may go to wrong element

---

## Solution Options

### Option A: Audit Focus State Flow (Recommended)

1. Add console logging to track focus state changes
2. Verify `dummy-focus` element receives focus on pane click
3. Check `disablePointerEvents` isn't getting stuck
4. Ensure `handleChildFocus` updates parent state correctly

```typescript
// Add debugging in block.tsx
const handleFocus = (e: React.FocusEvent) => {
    console.log('[FOCUS] Block focused:', blockId, e.target);
    // existing logic
};
```

### Option B: Simplify Focus Model

Replace the hidden input approach with direct focus management:
- Use `tabIndex={0}` on the block container
- Handle keyboard events at container level
- Remove `dummy-focus` element

### Option C: Event Delegation Fix

Ensure click events properly bubble and trigger focus:
```typescript
// In block container onClick
const handleBlockClick = (e: React.MouseEvent) => {
    e.currentTarget.focus();
    // Notify parent of focus change
};
```

---

## Testing Plan

1. Open AgentMux with 2+ terminal panes
2. Click on pane A - verify typing works immediately
3. Click on pane B - verify typing works immediately
4. Return to pane A - verify typing works without extra clicks
5. Test with rapid pane switching
6. Test after window loses/regains focus

---

## Implementation Steps

1. Add focus state logging to identify where flow breaks
2. Reproduce issue and capture logs
3. Fix identified root cause
4. Remove debug logging
5. Test all focus scenarios
6. Deploy and verify

---

## Files to Modify

- `frontend/app/block/block.tsx` - Primary focus handling
- `frontend/app/block/blockframe.tsx` - Frame focus delegation
- Possibly `frontend/app/tab/tabcontent.tsx` - Tab-level focus management
