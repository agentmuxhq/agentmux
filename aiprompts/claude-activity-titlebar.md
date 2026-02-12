# Claude Activity in Title Bar - Implementation Spec

## Overview

Display Claude Code's activity summaries in AgentMux terminal pane title bars, providing real-time visibility into what Claude is doing.

## Current State

### What Exists
1. **OSC 0/2 Handler** (`termwrap.ts:168-211`)
   - Receives window title updates from Claude Code
   - Strips "Claude: " or "Claude Code: " prefixes
   - Debounces updates (300ms)
   - Stores activity in `term:activity` metadata

2. **Title Display** (`blockframe.tsx:233-265`)
   - Reads `frame:title` for explicit overrides
   - Agent detection for terminal blocks (AgentA, AgentB, etc.)
   - Does NOT currently read `term:activity`

### The Gap
`term:activity` is stored but **never displayed**. Claude's activity summaries are captured but invisible to users.

## Claude Code Activity Format

Claude Code sends OSC 0/2 sequences with short summaries:
```
\e]0;Reading files...\a
\e]0;Searching codebase...\a
\e]0;Writing src/foo.ts...\a
\e]0;Running npm test...\a
\e]0;Thinking...\a
\e]0;Claude Code: Idle\a
```

The handler already strips "Claude: " and "Claude Code: " prefixes.

## Implementation Plan

### Phase 1: Display Activity in Title Bar

**File: `frontend/app/block/blockframe.tsx`**

After agent detection (around line 259), add activity display logic:

```typescript
// Read Claude activity for terminal blocks
let activityText: string | null = null;
if (blockData?.meta?.view === "term") {
    const activity = blockData.meta["term:activity"] as string | undefined;
    if (activity && activity !== "Idle" && activity.length > 0) {
        activityText = activity;
    }
}
```

**Display Options:**

Option A: **Replace title when active**
```typescript
if (activityText) {
    viewName = activityText;  // "Reading files..." replaces "AgentA"
}
```

Option B: **Append to title**
```typescript
if (activityText) {
    viewName = `${viewName} - ${activityText}`;  // "AgentA - Reading files..."
}
```

Option C: **Use header text area** (preferred)
```typescript
if (activityText) {
    headerTextUnion = activityText;  // Shows in secondary text area
}
```

### Phase 2: Visual Indicator

Add a subtle animation or icon when Claude is active:

```typescript
// In BlockFrame_Header render
const isClaudeActive = activityText && activityText !== "Idle";

// Add pulsing indicator or spinner
{isClaudeActive && <div className="claude-activity-indicator" />}
```

CSS:
```scss
.claude-activity-indicator {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent-color);
    animation: pulse 1.5s ease-in-out infinite;
}

@keyframes pulse {
    0%, 100% { opacity: 0.4; }
    50% { opacity: 1; }
}
```

### Phase 3: Activity Timeout

Clear stale activity after inactivity:

**File: `termwrap.ts`**

Add timeout to clear activity:
```typescript
const ACTIVITY_TIMEOUT_MS = 30000; // 30 seconds
const activityTimeoutMap = new Map<string, ReturnType<typeof setTimeout>>();

function handleOscTitleCommand(data: string, blockId: string, loaded: boolean): boolean {
    // ... existing debounce logic ...

    // Clear any existing activity timeout
    const existingActivityTimeout = activityTimeoutMap.get(blockId);
    if (existingActivityTimeout) {
        clearTimeout(existingActivityTimeout);
    }

    // Set timeout to clear activity
    const activityTimeout = setTimeout(() => {
        activityTimeoutMap.delete(blockId);
        fireAndForget(async () => {
            await services.ObjectService.UpdateObjectMeta(WOS.makeORef("block", blockId), {
                "term:activity": null,
            });
        });
    }, ACTIVITY_TIMEOUT_MS);

    activityTimeoutMap.set(blockId, activityTimeout);

    // ... rest of handler ...
}
```

## Data Flow

```
Claude Code                    AgentMux
    |                              |
    |--OSC 0: "Reading files..."-->|
    |                              |
    |                    termwrap.ts: handleOscTitleCommand()
    |                        - Strip "Claude: " prefix
    |                        - Debounce 300ms
    |                        - UpdateObjectMeta("term:activity")
    |                              |
    |                    blockframe.tsx: BlockFrame_Header
    |                        - Read blockData.meta["term:activity"]
    |                        - Display in title bar
    |                        - Show activity indicator
    |                              |
    |<----Title bar updates--------|
```

## Integration with Agent Colors

When both agent and activity are present:

| Agent | Activity | Display |
|-------|----------|---------|
| AgentA | null | "AgentA" (blue) |
| AgentA | "Reading..." | "AgentA - Reading..." (blue) |
| null | "Reading..." | "Reading..." (default) |
| null | null | "terminal" (default) |

The agent color persists even when activity text is shown.

## Testing

1. **Manual Test:**
   ```bash
   # Simulate Claude activity
   printf '\e]0;Reading files...\a'
   sleep 2
   printf '\e]0;Writing code...\a'
   sleep 2
   printf '\e]0;Idle\a'
   ```

2. **E2E Test:**
   ```typescript
   // e2e/claude-activity.test.ts
   test('displays Claude activity in title bar', async () => {
       // Send OSC 0 with activity
       await terminal.type("printf '\\e]0;Testing...\\a'");
       await terminal.press('Enter');

       // Verify title bar shows activity
       const header = await page.locator('.block-frame-header');
       await expect(header).toContainText('Testing...');
   });
   ```

## Considerations

1. **Performance**: Debouncing prevents rapid re-renders
2. **Clarity**: "Idle" state should hide the activity indicator
3. **Overflow**: Long activity text should truncate with ellipsis
4. **Persistence**: Activity should clear on terminal close/reset

## Files to Modify

1. `frontend/app/block/blockframe.tsx` - Read and display activity
2. `frontend/app/block/blockframe.scss` - Activity indicator styles
3. `frontend/app/view/term/termwrap.ts` - Activity timeout (optional)
4. `e2e/claude-activity.test.ts` - E2E tests (new file)

## Success Criteria

- [ ] Claude activity visible in pane title bar
- [ ] Activity updates in real-time (with debounce)
- [ ] "Idle" state hides activity indicator
- [ ] Agent color persists with activity text
- [ ] Activity clears after timeout or terminal close
