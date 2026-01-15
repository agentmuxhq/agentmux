# Regression Analysis: Agent Detection Issues

**Date:** 2026-01-15
**Version:** 0.15.10
**Issue:** Terminal panes show "AgentA" with blue border despite removal of hostname-based detection

## Observed Symptoms

1. **Flicker behavior**: Terminal shows "Terminal" in black for a brief moment, then switches to blue "AgentA"
2. **Double display**: Path/summary still showing twice in the pane header

## What This Tells Us

### Symptom 1: The Flicker

The flicker from "Terminal" → "AgentA" indicates:
- Initial render is correct (no agent detected)
- Something **async** is updating the state to "AgentA" after mount
- The agent identity is NOT coming from code-level detection (that would be synchronous)
- The agent identity IS coming from **persisted data** that loads asynchronously

**Likely sources:**
1. Block metadata in SQLite database (`waveterm.db`) with `cmd:env.WAVEMUX_AGENT_ID`
2. Settings loading from `fullConfigAtom` with `cmd:env.WAVEMUX_AGENT_ID`
3. OSC escape sequence from shell (but user says Claude isn't running)

### Symptom 2: Double Display

My change from `getEffectiveTitle(blockData, true, ...)` to `false` should have disabled auto-generation in TitleBar. If it's still doubling, either:
1. The change wasn't actually in the build
2. There's another component rendering the same info
3. TitleBar is showing custom title that equals the header content

## Code Path Analysis

### Path 1: Header Agent Detection (blockframe.tsx:240-254)

```typescript
if (!blockData?.meta?.["frame:title"] && blockData?.meta?.view === "term") {
    const fullConfig = globalStore.get(atoms.fullConfigAtom);
    const settingsEnv = fullConfig?.settings?.["cmd:env"];  // <-- ASYNC LOAD
    const blockEnv = blockData.meta["cmd:env"];              // <-- FROM DATABASE

    let agentId = detectAgentFromEnv(blockEnv);              // Priority 1: block env
    if (!agentId) {
        agentId = detectAgentFromEnv(settingsEnv);           // Priority 2: settings env
    }
    if (agentId) {
        viewName = agentId;                                   // Sets "AgentA"
        agentColor = detectAgentColor(mergedEnv, agentId);   // Sets blue
    }
}
```

**Problem:** `blockData.meta["cmd:env"]` comes from the database. If the block was created when `WAVEMUX_AGENT_ID=AgentA` was set, that value is **persisted in the block's metadata**.

### Path 2: TitleBar (blockframe.tsx:702-708)

```typescript
<TitleBar
    blockId={nodeModel.blockId}
    blockMeta={blockData.meta}
    title={getEffectiveTitle(blockData, false, settingsEnv)}  // Changed to false
/>
```

**Question:** Is this change actually in the 0.15.10 build? Need to verify.

## Root Cause Hypothesis

### Primary: Block Metadata Persistence

When a terminal block is created while `WAVEMUX_AGENT_ID=AgentA` is set (either in settings or environment), that value gets stored in the block's metadata (`cmd:env`).

Even after we:
1. Removed hostname-based detection code
2. Cleared settings.json files
3. Cleared the dev database (`node_modules/electron/dist/wave-data/db/`)

The **production database** still has blocks with cached `cmd:env` values.

### Secondary: Settings Persistence

The `fullConfigAtom` loads settings asynchronously. If settings still have `WAVEMUX_AGENT_ID`, it will apply after initial render (causing the flicker).

**Location of production settings:**
- `~/.waveterm/config/settings.json`
- `~/.config/waveterm/settings.json` (Linux)
- `%APPDATA%/waveterm/config/settings.json` (Windows)

### Tertiary: Wrong Database Cleared

I cleared `node_modules/electron/dist/wave-data/db/` which is the **development** database used by `task dev`.

But the portable build uses a **different data directory**:
- Windows: `%APPDATA%/WaveMux/` or wherever the portable is extracted

## Verification Steps Needed

1. **Check if my code change is in the build:**
   ```
   Search the packaged JS for "getEffectiveTitle" and verify parameter is `false`
   ```

2. **Find production data directory:**
   ```
   Check where portable WaveMux stores its data
   ```

3. **Check production settings.json:**
   ```
   Look for WAVEMUX_AGENT_ID in production settings
   ```

4. **Check block metadata in database:**
   ```
   Blocks may have cmd:env persisted from previous sessions
   ```

## Proposed Solutions

### Solution A: Clear Production Data

The portable build has its own data directory. Need to:
1. Find where portable stores data
2. Clear settings.json `cmd:env`
3. Delete database to reset all blocks

### Solution B: Don't Read cmd:env from Block Metadata

The issue is that `blockData.meta["cmd:env"]` persists forever. We could:
1. Only read `WAVEMUX_AGENT_ID` from **live process environment**, not stored metadata
2. Add logic to ignore persisted `cmd:env` for agent detection
3. Use a different metadata key that we explicitly clear on session end

### Solution C: OSC Sequence Approach (Cleanest)

The **correct** design is:
1. Default state: "Terminal" with black header
2. When Claude starts: Shell sends OSC 16162 E to set `WAVEMUX_AGENT_ID`
3. When Claude exits: Shell sends OSC 16162 E to clear it
4. Agent identity is **runtime only**, never persisted

This requires changes to:
- `cmd:env` handling to distinguish "session env" vs "persistent env"
- Shell integration to send clear sequence on exit

## Blockers

1. **Don't know production data path** - Need to find where portable stores settings/database
2. **Block metadata persistence** - Even if we fix code, existing blocks have cached data
3. **Unclear on OSC handling** - Need to understand how `cmd:env` gets populated from OSC sequences

## Immediate Next Steps

1. Find the portable's data directory path
2. Inspect production `settings.json` for `WAVEMUX_AGENT_ID`
3. Verify my code change is actually in the 0.15.10 build
4. Consider whether block-level `cmd:env` should be used for agent detection at all

## KEY FINDING: Portable Data Location

**The portable stores ALL data next to the executable:**

```
<extracted_zip>/
├── WaveMux.exe
├── wave-data/           <-- ALL DATA HERE
│   ├── config/
│   │   └── settings.json   <-- May have WAVEMUX_AGENT_ID
│   └── db/
│       └── waveterm.db     <-- Blocks have cached cmd:env
```

From `emain/platform.ts:39-50`:
```typescript
function findAvailableDataDirectory(): string {
    const exeDir = path.dirname(app.getPath("exe"));
    const primaryDataDir = path.join(exeDir, "wave-data");
    // ...
    return primaryDataDir;
}
```

**This means:** The user's portable has persistent data from previous sessions where `WAVEMUX_AGENT_ID=AgentA` was set.

## The Real Problem

I've been clearing:
- `node_modules/electron/dist/wave-data/` (dev mode data)
- `~/.config/waveterm/settings.json` (legacy location)

But the **portable build** uses:
- `<extracted_folder>/wave-data/` (next to WaveMux.exe)

The portable's data has:
1. `settings.json` with `WAVEMUX_AGENT_ID=AgentA`
2. Database with blocks that have cached `cmd:env`

## Immediate Fix

Tell user to delete `wave-data/` folder next to the extracted portable, OR:
Clear the portable's settings:
```
<portable_folder>/wave-data/config/settings.json -> {}
```

## Longer-Term Fix Needed

The design is flawed. Agent identity shouldn't persist in:
1. Block metadata (`cmd:env`) - This is meant for shell environment, not agent display
2. Global settings - This is for user preferences, not runtime state

Agent identity should be:
1. Set via OSC 16162 when Claude starts
2. Cleared via OSC 16162 when Claude exits
3. **Never persisted between sessions**

Need to refactor to separate:
- `cmd:env` = Shell environment variables (persist)
- `agent:id` = Agent display identity (runtime only, don't persist)

---

## Code Verification (2026-01-15 04:30)

### Verified Changes in Build

1. **blockframe.tsx:706** - Confirmed `getEffectiveTitle(blockData, false, ...)` in built JS
2. **blockframe.tsx:246-253** - Only checks `detectAgentFromEnv()`, no path fallback
3. **autotitle.ts** - `detectAgentFromPath` now only checks agent-workspaces pattern

### Fresh Instance Test

Launched 0.15.10 with deleted `wave-data/`:
- Fresh database created
- `~/.config/waveterm/settings.json` is `{}`
- No `cmd:env` anywhere

**Expected result:** Terminal should show "Terminal" with black header.

### Remaining Questions

1. **Did user test fresh instance?** - If testing old extracted folder with cached `wave-data/`, AgentA would persist
2. **What exactly shows twice?** - Need screenshot or more specific description:
   - Is it `viewName` ("Terminal"/"AgentA") showing twice?
   - Is it the path ("wavemux") showing twice?
   - Is it TitleBar + header both showing same thing?

### Test Instruction for User

To verify fix, please:
1. Extract a **fresh** copy of `WaveMux-win32-x64-0.15.10.zip`
2. Delete `wave-data/` folder if it exists in the extracted location
3. Launch WaveMux.exe
4. Observe the terminal pane header - should show "Terminal" in black

If still shows "AgentA", check:
- `wave-data/config/settings.json` - should be empty or not exist
- Look at the log file `wave-data/waveapp.log` for any clues
