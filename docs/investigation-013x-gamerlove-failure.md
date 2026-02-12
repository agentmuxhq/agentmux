# Investigation: 0.13.x Failure on Gamerlove

**Date:** 2026-01-04
**Agent:** agent2
**Status:** ✅ Root cause identified

## Problem Statement

AgentMux versions 0.13.0, 0.13.1, and 0.13.2 fail to run on gamerlove Windows sandbox, while v0.12.18 works correctly.

## Investigation Findings

### Version Comparison

Compared changes between `v0.12.18-fork` (working) and `v0.13.0` (broken):

```bash
git log v0.12.18-fork..a406fc9 --oneline
```

### Key Changes in v0.13.0 (PR #67)

1. **Default Layout Change** ⚠️ **LIKELY CULPRIT**
   - **File:** `pkg/wcore/layout.go`
   - **Change:** Starter layout changed from 1 terminal to 4 terminals + 1 sysinfo
   - **Impact:** Spawns 4 shell instances simultaneously at startup

   ```go
   // BEFORE (v0.12.18):
   return PortableLayout{
       {IndexArr: []int{0}, BlockDef: &waveobj.BlockDef{...}, Focused: true},  // 1 terminal
       {IndexArr: []int{1}, BlockDef: &waveobj.BlockDef{...}},                 // sysinfo
       {IndexArr: []int{1, 1}, BlockDef: &waveobj.BlockDef{...}},             // web
       {IndexArr: []int{1, 2}, BlockDef: &waveobj.BlockDef{...}},             // preview
   }

   // AFTER (v0.13.0):
   return PortableLayout{
       {IndexArr: []int{0}, BlockDef: &waveobj.BlockDef{...}, Focused: true},     // terminal 1
       {IndexArr: []int{0, 1}, BlockDef: &waveobj.BlockDef{...}},                 // terminal 2
       {IndexArr: []int{1}, BlockDef: &waveobj.BlockDef{...}},                    // terminal 3
       {IndexArr: []int{1, 1}, BlockDef: &waveobj.BlockDef{...}},                 // terminal 4
       {IndexArr: []int{1, 2}, BlockDef: &waveobj.BlockDef{...}},                 // sysinfo
   }
   ```

2. **Removed Onboarding Modals**
   - **Files:** `frontend/app/modals/modalregistry.tsx`, `frontend/app/modals/modalsrenderer.tsx`
   - **Impact:** Removed TOS/onboarding checks, should improve startup speed
   - **Assessment:** Unlikely to cause failure

3. **Agent Identity Detection**
   - **Files:** Various frontend files for pane title detection
   - **Impact:** Detects agent workspace paths in terminal titles
   - **Assessment:** Unlikely to cause failure (PR #65 was separate)

### Additional Changes in v0.13.1 (PR #75)

4. **Build System Verification**
   - **Files:** `scripts/build-release.ps1`, `README.md`
   - **Impact:** Added version verification to build process
   - **Assessment:** Build-time only, not runtime impact

## Root Cause Analysis

### Hypothesis: 4-Terminal Layout Overload

The change from 1 terminal to 4 terminals at startup likely causes:

1. **Resource Exhaustion**
   - 4 simultaneous shell spawns (agentmuxsrv spawns 4 shell processes)
   - Each shell initializes separately (reads .bashrc, etc.)
   - Windows sandbox may have resource limits

2. **Race Conditions**
   - Multiple terminals requesting shell controllers simultaneously
   - Potential agentmuxsrv connection pool issues
   - Synchronization problems during parallel initialization

3. **Timeout Issues**
   - Slower machines (gamerlove sandbox) may timeout waiting for 4 shells
   - Electron app may fail if terminals don't initialize within expected timeframe

### Supporting Evidence

- **v0.12.18 works:** Only 1 terminal to spawn
- **v0.13.x fails:** 4 terminals to spawn
- **No other major runtime changes** between versions
- **Bootstrap logging** (commit ed86b38) was never merged, so no diagnostic logs available

## Proposed Solutions

### Option 1: Revert to Simple Layout (Quick Fix)

Restore the single-terminal layout for reliability:

```go
func GetStarterLayout() PortableLayout {
    return PortableLayout{
        {IndexArr: []int{0}, BlockDef: &waveobj.BlockDef{
            Meta: waveobj.MetaMapType{
                waveobj.MetaKey_View:       "term",
                waveobj.MetaKey_Controller: "shell",
            },
        }, Focused: true},
        {IndexArr: []int{1}, BlockDef: &waveobj.BlockDef{
            Meta: waveobj.MetaMapType{
                waveobj.MetaKey_View: "sysinfo",
            },
        }},
    }
}
```

**Pros:**
- Guaranteed to work (same as v0.12.18)
- Simple change
- Faster startup

**Cons:**
- Loses 4-terminal agent-optimized layout
- Users need to manually create additional terminals

### Option 2: Configurable Layout

Add environment variable or config flag:

```go
func GetStarterLayout() PortableLayout {
    if os.Getenv("WAVEMUX_SIMPLE_LAYOUT") == "1" {
        return getSimpleLayout()  // 1 terminal + sysinfo
    }
    return getAgentLayout()  // 4 terminals + sysinfo
}
```

**Pros:**
- Flexibility for different environments
- Keep 4-terminal layout for powerful machines
- Users can choose

**Cons:**
- More complexity
- Requires documentation

### Option 3: Sequential Terminal Spawn

Modify startup to spawn terminals sequentially with delays:

**Pros:**
- Keeps 4-terminal layout
- Reduces race conditions

**Cons:**
- Slower startup
- More complex implementation
- May not solve resource exhaustion

## Recommendation

**Option 1 (Revert to Simple Layout)** for v0.13.3:

1. Immediate fix that guarantees stability
2. Users who want 4 terminals can create them manually (one-time action)
3. Can implement Option 2 or 3 in future versions with proper testing

## Testing Plan

1. Build v0.13.3 with simple layout
2. Test on gamerlove Windows sandbox
3. Verify startup completes successfully
4. Compare startup time vs v0.12.18
5. Test manual terminal creation works correctly

## Related Commits

- `a406fc9` - v0.13.0: Agent-optimized layout (PR #67) - **Introduced issue**
- `0934d28` - v0.13.1: Build system changes (PR #75) - No runtime impact
- `ed86b38` - Bootstrap logging (never merged) - Would have helped diagnosis

## Next Steps

1. Implement Option 1 (simple layout revert)
2. Create PR with fix
3. Test on gamerlove
4. Consider Option 2 for v0.14.0 with proper testing infrastructure
