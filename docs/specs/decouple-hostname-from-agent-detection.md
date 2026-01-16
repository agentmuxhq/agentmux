# Spec: Decouple System Hostname from Agent Detection

## Problem Statement

Terminal panes currently show "AgentA" with colored borders on first load, even when Claude is not running. The system hostname (area54) and work directory (`C:\Systems`) are being used to infer agent identity, but this is incorrect behavior.

**Expected behavior:**
- Default panes show "Terminal" with black/no border color
- Agent name and color only appear when Claude is explicitly running (via claw or similar)
- System hostname should have NO relation to pane title or color

**Current behavior:**
- Panes show "AgentA" because `C:\Systems` path matches `WORK_DIR_AGENT_MAP`
- Colored borders appear based on inferred agent identity
- This happens even without Claude running

## Root Cause Analysis

### Detection Flow (Current)

There are TWO separate detection paths that both use hostname-based inference:

#### Path 1: Title Generation (`autotitle.ts:generateTerminalTitle`)
```
1. Block env vars (cmd:env) → WAVEMUX_AGENT_ID
2. Settings env vars → WAVEMUX_AGENT_ID
3. agent-workspaces directory pattern ← STILL ACTIVE
4. Hostname-based patterns (SSH only) ← FIXED in PR #127
5. Directory basename
6. "Terminal"
```

#### Path 2: Agent Color Detection (`blockframe.tsx:246-258`)
```typescript
let agentId = detectAgentFromEnv(blockEnv);
if (!agentId) {
    agentId = detectAgentFromEnv(settingsEnv);
}
if (!agentId) {
    const cwd = blockData.meta["cmd:cwd"];
    const connName = blockData.meta["connection"];
    agentId = detectAgentFromPath(cwd, connName);  // ← STILL USES HOSTNAME DETECTION
}
```

### The `detectAgentFromPath` Function

This function checks multiple patterns in `autotitle.ts`:

1. **Pattern 1:** `agent-workspaces/agentX` - explicit directory structure
2. **Pattern 2:** User home directory (`/home/area54/`, `/Users/area54/`) → `HOSTNAME_AGENT_MAP`
3. **Pattern 2b:** Work directories (`C:\Systems`) → `WORK_DIR_AGENT_MAP`
4. **Pattern 3:** SSH connection name containing hostname

Patterns 2, 2b, and 3 are the problematic ones - they infer agent identity from system context rather than explicit configuration.

## Solution

### Design Principle

**Agent identity should ONLY come from explicit sources:**
1. Environment variables set by claw/Claude (`WAVEMUX_AGENT_ID`)
2. OSC 16162 E escape sequences sent by shell integration
3. Block-level configuration

**Agent identity should NEVER come from:**
- System hostname
- Working directory path (except explicit `agent-workspaces` pattern)
- User home directory
- SSH connection names

### Implementation

#### Step 1: Remove Hostname-Based Detection from `detectAgentFromPath`

Delete or disable these patterns in `autotitle.ts`:
- `HOSTNAME_AGENT_MAP` usage for home directory matching
- `WORK_DIR_AGENT_MAP` for work directory matching
- Connection name hostname matching

Keep ONLY the `agent-workspaces` pattern as it's an explicit opt-in structure.

#### Step 2: Update Color Detection in `blockframe.tsx`

The color detection path should mirror the title detection path - only use explicit env vars, not path inference.

```typescript
// Current (broken):
if (!agentId) {
    agentId = detectAgentFromPath(cwd, connName);
}

// Fixed:
// Remove this fallback entirely - agent identity only from env vars
```

#### Step 3: Clean Up Unused Code

- Remove `HOSTNAME_AGENT_MAP` constant (or keep only for SSH if needed)
- Remove `WORK_DIR_AGENT_MAP` constant
- Simplify `detectAgentFromPath` to only check `agent-workspaces` pattern

### Files to Modify

1. `frontend/app/block/autotitle.ts`
   - Remove/disable hostname-based patterns in `detectAgentFromPath`
   - Consider renaming to `detectAgentFromWorkspacesPath` and using that everywhere

2. `frontend/app/block/blockframe.tsx`
   - Remove `detectAgentFromPath` fallback in color detection (lines 250-254)
   - Only use `detectAgentFromEnv` for agent identity

3. `frontend/app/block/autotitle.test.ts`
   - Update tests to reflect new behavior
   - Add test: local terminal without env vars shows "Terminal" with no color

## Expected Outcome

| Scenario | Title | Border Color |
|----------|-------|--------------|
| Fresh terminal, no Claude | "Terminal" or dir basename | None (black) |
| Claude running via claw (sets WAVEMUX_AGENT_ID=AgentA) | "AgentA" | Dark blue |
| SSH to agent machine | Detected from hostname | Agent color |
| Working in `agent-workspaces/agent2/` | "Agent2" | Agent2 color |

## Migration Notes

- Existing behavior for SSH connections can be preserved if desired
- Users relying on hostname detection will need to set env vars in their shell profiles
- Shell integration scripts should set `WAVEMUX_AGENT_ID` when Claude is active

## Test Plan

1. Fresh WaveMux launch → should show "Terminal" with black border
2. Launch Claude via claw → should show agent name with color
3. Exit Claude → should revert to "Terminal" (requires OSC sequence on exit)
4. SSH to agent machine → should detect from hostname (if preserved)
5. cd to `agent-workspaces/agentX` → should detect agent (explicit pattern)
