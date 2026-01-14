# Spec: Agent Color Borders

**Feature:** Extend agent title color to pane borders
**Type:** Enhancement
**Component:** frontend/app/block/

---

## Feature Description

Currently, agent-identified panes show the agent's color in the header/title bar. This feature extends that color to the pane borders:

1. **Agent-colored borders** - Use agent color instead of default accent color
2. **Thicker side borders** - Increase side borders to 4px total (2px per adjacent pane)
3. **Visual distinction** - Make it immediately clear which agent owns which pane

---

## Current State

### Agent Color Detection (`autotitle.ts`)

```typescript
export function detectAgentColor(env: Record<string, string>): string | null {
    const agentId = detectAgentFromEnv(env);
    if (!agentId) return null;

    // Returns color like "#00ff00" for AgentA
    return AgentColors[agentId] || DefaultAgentColor;
}
```

### Current Border Implementation (`block.scss`)

```scss
.block .block-mask {
    border: 1px solid var(--border-color);  // White, all sides
}

.block.block-focused .block-mask {
    border: 2px solid var(--accent-color);  // Green, all sides
}
```

### Header Color Application (`blockframe.tsx`)

Agent color is applied to header via inline styles or CSS variable.

---

## Design Decisions

### Border Sizing

| Component | Unfocused | Focused |
|-----------|-----------|---------|
| Top/Bottom | 1px | 2px |
| Left/Right | 2px | 4px |

The 2px per side creates 4px total between adjacent panes, making agent boundaries clear.

### Color Priority

1. Agent color (if agent detected)
2. Accent color (if focused, no agent)
3. Border color (unfocused, no agent)

### Visual Hierarchy

```
┌─────────────────┬─────────────────┐
│   AgentA (red)  │  AgentX (blue)  │
│ ┌─────────────┐ │ ┌─────────────┐ │
│ │             │ │ │             │ │
│ │  terminal   │ │ │  terminal   │ │
│ │             │ │ │             │ │
│ └─────────────┘ │ └─────────────┘ │
└─────────────────┴─────────────────┘
        ▲                   ▲
     2px red             2px blue
        └───────4px gap─────┘
```

---

## Implementation

### Step 1: Add Agent Color CSS Variable

In `blockframe.tsx`, set a CSS variable for agent color:

```typescript
const agentColor = detectAgentColor(blockEnv);

const blockStyle: React.CSSProperties = {
    '--block-agent-color': agentColor || 'transparent',
} as React.CSSProperties;
```

### Step 2: Update Border Styles

In `block.scss`:

```scss
.block .block-mask {
    // Default unfocused state
    border-top: 1px solid var(--border-color);
    border-bottom: 1px solid var(--border-color);
    border-left: 2px solid var(--border-color);
    border-right: 2px solid var(--border-color);

    // Agent color override (if set)
    border-color: var(--block-agent-color, var(--border-color));
}

.block.block-focused .block-mask {
    border-top: 2px solid var(--accent-color);
    border-bottom: 2px solid var(--accent-color);
    border-left: 4px solid var(--accent-color);
    border-right: 4px solid var(--accent-color);

    // Agent color takes priority when present
    border-color: var(--block-agent-color, var(--accent-color));
}
```

### Step 3: Handle No-Agent Case

When no agent is detected, fall back to current behavior:
- Unfocused: white borders
- Focused: accent color (green) borders

### Step 4: Dynamic Updates

When agent color changes via OSC 16162 E command:
1. Block metadata updates
2. React re-renders blockframe
3. CSS variable updates
4. Border color changes automatically

---

## Testing Plan

1. **Agent panes** - Verify agent color shows on borders
2. **Non-agent panes** - Verify default colors still work
3. **Focus states** - Verify border thickens on focus
4. **Adjacent panes** - Verify 4px gap between different agents
5. **Same agent adjacent** - Verify colors match seamlessly
6. **Dynamic changes** - Send OSC command, verify border updates

---

## Files to Modify

1. `frontend/app/block/block.scss` - Border styling with agent variable
2. `frontend/app/block/blockframe.tsx` - Set CSS variable from agent color
3. `frontend/app/block/autotitle.ts` - Ensure color detection exports correctly

---

## Future Considerations

- Gradient borders for multi-agent collaboration
- Pulsing/glow effect for active agent
- Configurable border widths in settings
- Color-blind friendly patterns as alternative
