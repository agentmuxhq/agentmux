# Agent Detection Retrospective

**Date:** 2026-01-15
**Author:** AgentA
**Issue:** Going in circles on agent identity/color detection implementation

---

## Timeline of Changes

### PR #120 - Original Implementation (Working)
**"feat: per-pane agent colors via shell environment variables"**

- Implemented OSC 16162 command "E" for shell-to-frontend communication
- Shell integration script (`agentmux-agent.sh`) sends env vars via OSC
- `termwrap.ts` receives OSC and updates block's `cmd:env` metadata
- `blockframe.tsx` reads from `blockData.meta["cmd:env"]` for agent identity
- **Result:** Per-pane agent detection worked correctly

### PR #132 - Broke It
**"fix: decouple system hostname from agent detection"**

- Changed `blockframe.tsx` to read from `settings["cmd:env"]` instead of block's `cmd:env`
- Reasoning was to avoid "persisted data" issues
- **Result:** Lost per-pane capability - all panes showed same agent from settings

### Today's Session - Further Confusion

1. Started investigating "dual summaries" regression
2. Found and fixed duplicate `term:activity` display (correct fix)
3. Then changed agent detection to read from `process.env` (wrong)
4. User pointed out "each pane needs its own environment"
5. Realized we broke the original working implementation

---

## Root Cause Analysis

### Why We Went in Circles

1. **Misunderstood the persistence problem**
   - The original complaint was about *stale* agent identity persisting after Claude exits
   - We incorrectly concluded that reading from block's `cmd:env` was the problem
   - Actually, the problem was that `cmd:env` wasn't being *cleared* when the agent exits

2. **Conflated different issues**
   - Issue A: Agent identity persisting after session ends (stale data)
   - Issue B: Agent identity being the same for all panes (lost per-pane)
   - We "fixed" Issue A by breaking Issue B

3. **Lost sight of the architecture**
   - OSC 16162 E → `termwrap.ts` → block's `cmd:env` → `blockframe.tsx`
   - This pipeline was correct and working
   - We broke it by bypassing block's `cmd:env`

4. **No clear documentation of the design**
   - Each "fix" was made without referencing the original design
   - No single source of truth for how agent detection should work

---

## The Correct Architecture

```
┌─────────────────┐     OSC 16162 E      ┌─────────────────┐
│  Shell Process  │ ──────────────────── │   termwrap.ts   │
│                 │  {WAVEMUX_AGENT_ID}  │                 │
│ export AGENT=X  │                      │ Updates block's │
└─────────────────┘                      │ cmd:env metadata│
                                         └────────┬────────┘
                                                  │
                                                  ▼
                                         ┌─────────────────┐
                                         │  blockframe.tsx │
                                         │                 │
                                         │ Reads block's   │
                                         │ cmd:env for     │
                                         │ agent identity  │
                                         └────────┬────────┘
                                                  │
                                                  ▼
                                         ┌─────────────────┐
                                         │  Header Display │
                                         │                 │
                                         │ Shows agent name│
                                         │ with color      │
                                         └─────────────────┘
```

### Key Points:
1. **Each pane has its own block** with its own `cmd:env`
2. **Shell sends OSC 16162 E** when `WAVEMUX_AGENT_ID` changes
3. **Block metadata is per-pane** - different panes can have different agents
4. **Settings `cmd:env` is global** - same for all panes (wrong for this use case)
5. **`process.env` is process-wide** - same for all panes (wrong for this use case)

---

## The Real Problem We Should Have Solved

The original complaint about "stale agent identity" was likely because:

1. Claude/agent sets `WAVEMUX_AGENT_ID=AgentA` in shell
2. Shell integration sends OSC 16162 E to update block's `cmd:env`
3. Agent exits, but `cmd:env` still has `WAVEMUX_AGENT_ID=AgentA`
4. Pane continues showing "AgentA" even though Claude is gone

### Correct Solution (Not Implemented Yet)

When agent exits, shell should:
1. `unset WAVEMUX_AGENT_ID`
2. Send OSC 16162 E with empty/cleared env vars
3. Block's `cmd:env` gets updated to remove agent identity
4. Pane reverts to showing "Terminal"

This requires updating `agentmux-agent.sh` to handle agent exit, NOT changing where we read agent identity from.

---

## Immediate Fix Needed

Revert `blockframe.tsx` to read from block's `cmd:env`:

```typescript
// CORRECT - per-pane agent detection
const blockEnv = blockData.meta["cmd:env"] as Record<string, string> | undefined;
const agentId = detectAgentFromEnv(blockEnv);
```

NOT from settings (global) or process.env (process-wide).

---

## Future Work

1. **Fix stale agent identity properly**
   - Update `agentmux-agent.sh` to clear agent on exit
   - Or implement a timeout/heartbeat mechanism
   - Or use `term:activity` as signal (if "Idle" for X seconds, clear agent)

2. **Document the architecture**
   - Add this retro to docs/specs/
   - Update CLAUDE.md with agent detection design
   - Add comments in code referencing the design doc

3. **Add tests**
   - E2E test: Set agent in pane A, verify pane B is unaffected
   - E2E test: Clear agent, verify pane reverts to "Terminal"

---

## Lessons Learned

1. **Understand the existing implementation before "fixing"**
   - Read the original PR and understand the design
   - Don't assume the current code is wrong without understanding why it was written that way

2. **Separate symptoms from root causes**
   - "Agent persists after exit" ≠ "Don't read from block's cmd:env"
   - The persistence is correct; the clearing on exit was missing

3. **Test the fix against the original use case**
   - Per-pane agent detection must work
   - Any change that breaks per-pane is wrong

4. **Document design decisions**
   - OSC 16162 pipeline should be documented
   - Future developers (including AI agents) need this context

---

## Action Items

- [ ] Revert blockframe.tsx to read from block's `cmd:env`
- [ ] Test per-pane agent detection works
- [ ] Document OSC 16162 agent pipeline
- [ ] Implement proper agent exit handling in shell integration
- [ ] Add E2E tests for per-pane agent detection
