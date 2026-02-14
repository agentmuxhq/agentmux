# Retrospective: Backend Version Isolation Implementation Failure

**Date**: 2026-02-14
**Issue**: Backend version isolation implemented but "wsh not found" error persists
**PR**: #297 - Backend Version Isolation (v0.27.7)
**Status**: FAILURE - Feature not working as designed

---

## What Was Supposed to Happen

### Design Intent (from BACKEND_VERSION_ISOLATION_SPEC.md)

**Problem**: Different AgentMux versions sharing backends causes wrong binaries/scripts to be used.

**Solution**: Version-specific lock files
- v0.27.6 uses `wave-0.27.6.lock`
- v0.27.7 uses `wave-0.27.7.lock`
- Different versions → separate backends → separate caches

**Expected Flow**:
```
1. User runs v0.27.7 portable
2. Backend acquires wave-0.27.7.lock
3. Backend calls InitCustomShellStartupFiles()
4. Cache version check: compares .version file with WaveVersion
5. If mismatch → regenerates scripts + deploys wsh-0.27.7
6. User opens terminal → shell script finds wsh-0.27.7 → works ✅
```

---

## What Actually Happened

### User Test (2026-02-14)

```
1. User built v0.27.7 portable ✅
2. User closed all instances ✅
3. User ran v0.27.7 portable ✅
4. Backend started (confirmed by user) ✅
5. User opened terminal
6. Error: "wsh not found" ❌
```

**Error Output**:
```
PowerShell 7.5.4
wsh: C:\Users\area54\AppData\Roaming\com.a5af.agentmux\shell\pwsh\wavepwsh.ps1:21
Line |
  21 |  $agentmux_swaptoken_output = wsh token $env:AGENTMUX_SWAPTOKEN pwsh 2 …
     |                               ~~~
     | The term 'wsh' is not recognized as a name of a cmdlet, function, script
     | file, or executable program.
```

**Critical Observation**: Script path is `C:\Users\area54\AppData\Roaming\com.a5af.agentmux\shell\pwsh\wavepwsh.ps1`

This is the **default installed location**, not a portable-specific directory.

---

## Initial Flawed Analysis (What I Did Wrong)

### Mistake 1: Assumed Backend Wasn't Running

**Assumption**: "No backend process, so cache never regenerated"

**Evidence Gathered**:
```bash
tasklist //FI "IMAGENAME eq agentmuxsrv.exe"
# No results
```

**Conclusion**: Backend not running

**User Correction**: "thats impossible, i just closed all instances. the backend was opened"

**Why I Was Wrong**:
- Checked at wrong time (after user already tested and closed)
- Didn't ask user to verify backend status during the test
- Made conclusion without confirming with user first

### Mistake 2: Assumed Portable Wasn't Extracted

**Assumption**: "Portable ZIP not extracted, user running installed version"

**Evidence Gathered**:
```bash
ls -la "C:\Users\area54\Desktop\AgentMux"*/
# No results
```

**Conclusion**: Portable not extracted

**Why I Was Wrong**:
- Assumed extraction location without asking
- User may have extracted to different location
- Didn't verify with user before making assumption

### Mistake 3: Pattern of Shallow Analysis

**What I Did**:
1. See error
2. Form quick hypothesis
3. Run single command to "prove" hypothesis
4. Declare conclusion
5. Hypothesis proven wrong by reality
6. Repeat with new hypothesis

**What I Should Have Done**:
1. See error
2. Gather ALL relevant data first
3. Analyze data comprehensively
4. Form hypothesis based on complete picture
5. Test hypothesis thoroughly
6. Document findings

---

## Deep Analysis: Why The Fix Didn't Work

### Theory 1: Cache Versioning Check Failed to Detect Mismatch

**Hypothesis**: `isCacheValid()` returned `true` when it should have returned `false`

**Code Review** (`pkg/util/shellutil/shellutil.go`):
```go
func isCacheValid(waveHome string) bool {
    versionFile := filepath.Join(waveHome, "shell", ".version")
    data, err := os.ReadFile(versionFile)
    if err != nil {
        return false // No file = invalid
    }
    var cached ShellCacheVersion
    if err := json.Unmarshal(data, &cached); err != nil {
        log.Printf("[shellutil] Corrupt version file, invalidating cache: %v", err)
        return false // Corrupt = invalid
    }
    if cached.AgentMuxVersion != wavebase.WaveVersion {
        log.Printf("[shellutil] Cache version mismatch: cached=%s, current=%s",
            cached.AgentMuxVersion, wavebase.WaveVersion)
        return false // Version mismatch = invalid
    }
    return true // All checks passed = valid
}
```

**Test**: What does `.version` file contain?

**Evidence Needed**:
- Contents of `C:\Users\area54\AppData\Roaming\com.a5af.agentmux\shell\.version`
- Backend logs showing cache validation decision
- Value of `wavebase.WaveVersion` in running backend

**Possible Failure Modes**:
1. `.version` file says "0.27.7" (matches) but was created by OLD backend before we added versioning
2. `wavebase.WaveVersion` in portable binary is wrong (stale build)
3. Cache validation code not being called at all

### Theory 2: Portable Using Wrong Data Directory

**Hypothesis**: Portable backend is using shared data directory instead of portable-specific directory

**Evidence**:
- Error shows script path: `C:\Users\area54\AppData\Roaming\com.a5af.agentmux\`
- This is the SHARED data directory (not portable-specific)

**Code Review** (`pkg/wavebase/wavebase.go`):

How does portable mode work?

**Config Home** (from environment):
```go
func CacheAndRemoveEnvVars() error {
    DataHome_VarCache = os.Getenv(WaveDataHomeEnvVar)
    if DataHome_VarCache == "" {
        return fmt.Errorf("%s not set", WaveDataHomeEnvVar)
    }
    os.Unsetenv(WaveDataHomeEnvVar)
    // ...
}
```

**Question**: Does portable set `WAVETERM_DATA_HOME` to a portable-specific directory?

**Expected for Portable**:
- Portable should use: `<portable-dir>\data\` or similar
- NOT: `C:\Users\area54\AppData\Roaming\com.a5af.agentmux\`

**If This Is The Issue**:
- Portable and installed AgentMux are using THE SAME data directory
- Both versions writing to same cache
- Backend version isolation (separate lock files) doesn't help if they share the same cache directory!

**Critical Flaw in Design**:
We implemented version-specific LOCK FILES but didn't ensure version-specific DATA DIRECTORIES for portables.

### Theory 3: wsh Binary Not Deployed

**Hypothesis**: `initCustomShellStartupFilesInternal()` failed to deploy wsh binary

**Code Review** (`pkg/util/shellutil/shellutil.go`):

What deploys wsh?

```go
func initCustomShellStartupFilesInternal() error {
    // ... generates shell scripts ...

    // Does it deploy wsh binary? Let me check...
    // (Need to read full function)
}
```

**Evidence Needed**:
- Check if `C:\Users\area54\AppData\Roaming\com.a5af.agentmux\bin\wsh-*.exe` exists
- Check if portable directory has wsh binary
- Backend logs showing wsh deployment

**Observation**: Earlier check showed:
```bash
ls -la "C:\Users\area54\AppData\Roaming\com.a5af.agentmux\bin\wsh"*.exe
# No results
```

So NO wsh binary in the expected location!

### Theory 4: Portable Detection Failed in Shell Script

**Hypothesis**: Shell script's portable detection logic failed

**Code Review** (`pkg/util/shellutil/shellintegration/pwsh_agentmuxpwsh.sh`):

```powershell
# Detect portable mode: check if wsh exists in AgentMux app directory
$portableWshPath = $null
if ($env:AGENTMUX -and $env:AGENTMUX -ne "1") {
    $appDir = Split-Path -Parent $env:AGENTMUX
    $portableWsh = Get-ChildItem -Path $appDir -Filter "wsh-*.exe" -File -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($portableWsh) {
        $portableWshPath = $appDir
    }
}

# Use portable path if available, otherwise use installed path
if ($portableWshPath) {
    $env:PATH = $portableWshPath + ";" + $env:PATH
} else {
    $env:PATH = "C:\Users\area54\AppData\Roaming\com.a5af.agentmux\bin" + ";" + $env:PATH
}
```

**Portable Detection Depends On**:
1. `$env:AGENTMUX` is set to the AgentMux.exe path (not "1")
2. wsh-*.exe exists in the same directory as AgentMux.exe

**Evidence Needed**:
- Value of `$env:AGENTMUX` in the terminal
- Contents of portable directory (does it have wsh-*.exe?)
- Whether portable binary is setting AGENTMUX env var correctly

**Possible Failure**:
- Portable sets `AGENTMUX=1` (the fallback) instead of the exe path
- Portable doesn't have wsh binary in the same directory as AgentMux.exe

---

## The Most Likely Root Cause

Based on evidence gathered:

### ROOT CAUSE: Portable Shares Data Directory with Installed Version

**Evidence**:
1. Error shows script in `C:\Users\area54\AppData\Roaming\com.a5af.agentmux\`
2. This is the shared/installed data directory
3. No wsh binaries in `com.a5af.agentmux\bin\`

**What This Means**:
- Portable backend uses SAME data directory as installed version
- Both write to SAME cache
- Backend version isolation (separate locks) doesn't help
- Race condition: whichever backend starts first writes cache, other uses it

**Why Backend Version Isolation Didn't Fix It**:
```
Our Fix:
  v0.27.6 → wave-0.27.6.lock (separate lock)
  v0.27.7 → wave-0.27.7.lock (separate lock)
  ✅ Different backends run

But:
  Both backends write to: C:\Users\area54\AppData\Roaming\com.a5af.agentmux\shell\
  ❌ Same cache directory!
```

**Timeline of Failure**:
```
1. User ran portable v0.27.7
2. Backend acquired wave-0.27.7.lock ✅
3. Backend checked cache in com.a5af.agentmux\shell\
4. Found OLD cache from previous version
5. isCacheValid() checked .version file
   - If .version missing: regenerated scripts but no wsh binary deployed
   - If .version exists with old version: should have regenerated (WHY DIDN'T IT?)
6. User opened terminal
7. Shell script looked for wsh in com.a5af.agentmux\bin\
8. No wsh found ❌
```

---

## Critical Questions Still Unanswered

### Question 1: Does Portable Use Separate Data Directory?

**Need to verify**:
- How does Tauri portable set environment variables?
- Does portable set `WAVETERM_DATA_HOME` to portable-specific path?
- Or does it default to system AppData?

**Test**:
- Run portable backend with logging
- Check value of `wavebase.GetWaveDataDir()`
- Should be portable-specific, not `C:\Users\...\AppData\Roaming\com.a5af.agentmux\`

### Question 2: Was wsh Binary Ever Deployed?

**Need to verify**:
- Does `initCustomShellStartupFilesInternal()` deploy wsh?
- Or does something else deploy it?
- Check backend logs for wsh deployment

**Test**:
- Read full implementation of `initCustomShellStartupFilesInternal()`
- Check if wsh deployment is part of shell integration init
- Verify portable package contains wsh binary

### Question 3: Why Didn't Cache Invalidation Trigger?

**Need to verify**:
- Was `isCacheValid()` called?
- What did it return?
- If it returned false, did regeneration happen?
- If it returned true, why (version should have mismatched)?

**Test**:
- Add debug logging to `isCacheValid()`
- Check `.version` file contents
- Verify `wavebase.WaveVersion` value in binary

### Question 4: Is Portable Binary Built Correctly?

**Need to verify**:
- Does portable AgentMux.exe contain v0.27.7 code?
- Does portable agentmuxsrv contain v0.27.7 code?
- Check `ExpectedVersion` constant in binary

**Test**:
- Run portable backend, check logs for version
- Verify version mismatch warning doesn't appear

---

## What We Actually Know (Facts vs Assumptions)

### ✅ FACTS (Confirmed)

1. **PR #297 merged** - Code changes are in main branch
2. **Portable built** - `agentmux-0.27.7-x64-portable.zip` created (35 MB)
3. **Portable contains wsh** - `wsh-0.27.7-windows.x64.exe` in package
4. **User ran portable** - User confirmed they closed all instances and ran portable
5. **Backend started** - User confirmed "the backend was opened"
6. **Terminal opened** - User got error in terminal
7. **Error location** - Script in `C:\Users\area54\AppData\Roaming\com.a5af.agentmux\shell\pwsh\`
8. **No wsh binary** - Earlier check showed no wsh in `com.a5af.agentmux\bin\`

### ❓ ASSUMPTIONS (Not Confirmed)

1. **Portable extracted to Desktop** - NEVER VERIFIED WHERE USER EXTRACTED
2. **Backend not running** - CHECKED AT WRONG TIME (after test)
3. **Cache version mismatch detected** - NEVER CHECKED .version FILE
4. **Portable uses separate data directory** - NEVER VERIFIED
5. **initCustomShellStartupFiles() was called** - NEVER CHECKED LOGS
6. **isCacheValid() returned false** - NEVER VERIFIED
7. **Scripts regenerated** - NEVER CONFIRMED
8. **wsh deployment attempted** - NEVER VERIFIED

---

## Lessons Learned

### Anti-Pattern: Hypothesis-Driven Debugging (What I Did)

```
1. See symptom
2. Jump to conclusion
3. Run one command to "confirm"
4. Declare root cause
5. Get proven wrong
6. Repeat with new hypothesis
```

**Problems**:
- Wastes time with false starts
- Frustrates user
- Never builds complete picture
- Misses actual root cause

### Correct Pattern: Evidence-Driven Debugging (What I Should Do)

```
1. See symptom
2. List ALL unknowns
3. Gather ALL evidence systematically
4. Document facts vs assumptions
5. Analyze complete data set
6. Form hypothesis with supporting evidence
7. Test hypothesis
8. Confirm or reject with evidence
```

**Benefits**:
- Systematic coverage of possibilities
- No false starts
- User sees organized investigation
- Finds actual root cause

### Specific Mistakes Made

1. **Didn't ask user for key information upfront**
   - Where did you extract the portable?
   - Is backend still running now?
   - Can you check terminal env vars?

2. **Didn't check logs before making assumptions**
   - Backend logs would show version, data dir, cache decisions
   - Never looked at actual runtime behavior

3. **Didn't verify implementation assumptions**
   - Assumed portable uses separate data dir (NEVER VERIFIED)
   - Assumed wsh deployment happens in init (NEVER CONFIRMED)
   - Assumed cache validation works (NEVER TESTED)

4. **Made temporal logic errors**
   - Checked process list AFTER user finished test
   - Expected to find extracted files without knowing extraction location

5. **Didn't read the full error carefully**
   - Script path `com.a5af.agentmux` is SHARED directory
   - This is immediate red flag for data directory issue
   - Should have caught this on first read

---

## Next Steps: Proper Investigation Plan

### Phase 1: Gather Evidence (No Assumptions)

**User Questions**:
1. Where did you extract the portable? (exact path)
2. Is backend still running now? (if yes, PID?)
3. When you opened terminal, what was `$env:AGENTMUX` value?
4. Can you open portable again and immediately check backend process?

**File System Inspection**:
1. Check `.version` file contents: `cat C:\Users\area54\AppData\Roaming\com.a5af.agentmux\shell\.version`
2. List all lock files: `ls C:\Users\area54\AppData\Roaming\com.a5af.agentmux\wave*.lock`
3. Check portable directory structure: `ls <portable-extract-path>`
4. Verify wsh in portable: `ls <portable-extract-path>\wsh*.exe`

**Backend Logs**:
1. Find backend log location (check Tauri portable log path)
2. Read backend startup logs
3. Check for "Shell integration cache is valid" message
4. Check for version mismatch warnings

**Binary Verification**:
1. Check portable agentmuxsrv version constant
2. Verify ExpectedVersion vs WaveVersion in logs

### Phase 2: Code Review (Complete Understanding)

**Shell Integration Init**:
1. Read FULL `initCustomShellStartupFilesInternal()` implementation
2. Trace wsh deployment code
3. Understand data directory logic for portables
4. Check if Tauri portable sets environment variables correctly

**Cache Versioning**:
1. Trace `isCacheValid()` call path
2. Verify it's called in portable mode
3. Check if it handles missing .version file correctly

### Phase 3: Root Cause Identification

After gathering ALL evidence:
1. Analyze complete picture
2. Identify actual failure point
3. Determine if it's:
   - Design flaw (wrong approach)
   - Implementation bug (code error)
   - Build issue (binary problem)
   - Configuration issue (env vars)

### Phase 4: Fix

Once root cause confirmed:
1. Design proper fix
2. Implement fix
3. Test fix thoroughly
4. Document fix and verification steps

---

## Hypotheses to Test (Ranked by Likelihood)

### Hypothesis 1: Portable Uses Shared Data Directory (HIGH)

**Evidence Supporting**:
- Script path shows `com.a5af.agentmux` (shared directory)
- No wsh in shared bin directory

**Evidence Against**:
- None yet

**To Prove/Disprove**:
- Check backend logs for data directory path
- Verify Tauri portable env var setup

**If True**:
- Backend version isolation doesn't help (same cache for all versions)
- Need portable-specific data directory
- Major design flaw in portable implementation

### Hypothesis 2: wsh Deployment Doesn't Happen in Shell Init (MEDIUM)

**Evidence Supporting**:
- No wsh binary found in expected location

**Evidence Against**:
- Shell integration should deploy wsh (from original design)

**To Prove/Disprove**:
- Read full `initCustomShellStartupFilesInternal()` code
- Check if wsh deployment is separate step

**If True**:
- Need to fix wsh deployment logic
- Implementation bug, not design flaw

### Hypothesis 3: Cache Validation Incorrectly Returns True (MEDIUM)

**Evidence Supporting**:
- Scripts exist (suggests cache was considered "valid")

**Evidence Against**:
- Version should have changed, causing invalidation

**To Prove/Disprove**:
- Check .version file contents
- Check backend logs for cache validation decision

**If True**:
- Bug in `isCacheValid()` logic
- Need to fix version comparison

### Hypothesis 4: Portable Binary Has Wrong Version (LOW)

**Evidence Supporting**:
- None

**Evidence Against**:
- Build process updates version automatically

**To Prove/Disprove**:
- Check backend logs for version mismatch warning
- Verify portable binary version constants

**If True**:
- Build process issue
- Need to rebuild

---

## Action Items

### Immediate (Before Further Development)

1. **Stop making assumptions** - Gather evidence first
2. **Ask user for complete context** - Get all relevant info upfront
3. **Read logs thoroughly** - Don't guess, observe actual behavior
4. **Verify design assumptions** - Check if portable data dir is separate
5. **Document unknowns** - Write them down before investigating

### For This Issue

1. **Work with user to gather evidence** - Get answers to Phase 1 questions
2. **Read complete shell init implementation** - Understand full flow
3. **Check backend logs** - See what actually happened
4. **Verify portable data directory** - Confirm if it's separate or shared
5. **Trace wsh deployment** - Find where/when/how it happens

### For Future Development

1. **Add comprehensive logging** - Every decision point should log
2. **Test portables thoroughly** - Don't assume they work like installed
3. **Verify design assumptions early** - Check early, not after implementation
4. **Write integration tests** - Test end-to-end flows
5. **Create debugging guide** - Document how to investigate issues

---

## Conclusion

**What I Did Wrong**:
- Made rapid-fire hypotheses without evidence
- Ran single commands to "confirm" assumptions
- Never built complete picture
- Frustrated user with repeated wrong guesses

**What I Should Have Done**:
- Gathered ALL evidence systematically
- Read logs and verified runtime behavior
- Asked user for complete context
- Analyzed data before making conclusions
- Documented facts vs assumptions clearly

**Current Status**:
- Backend version isolation implemented correctly (code-wise)
- But feature NOT WORKING for portable builds
- Root cause likely: portable using shared data directory
- Need to complete proper investigation per plan above

**Next Action**:
Work with user to gather Phase 1 evidence, then proceed systematically through investigation plan.

---

**Date**: 2026-02-14
**Author**: AgentA (Claude Code)
**Status**: ROOT CAUSE FOUND - Portable packaging issue
**Priority**: Critical - Blocks portable multi-version support

---

## ADDENDUM: Root Cause Identified (2026-02-14 00:42 UTC)

### Actual Root Cause: Portable Packaging Mismatch

**Backend version isolation is working correctly!** The real bug is a mismatch between:

**Portable Package Structure:**
```
agentmux-0.27.7-x64-portable/
  ├── agentmux.exe
  ├── agentmuxsrv.x64.exe
  └── wsh-0.27.7-windows.x64.exe  ← Root directory
```

**Backend Code Expects:**
```go
func GetWaveAppBinPath() string {
    return filepath.Join(GetWaveAppPath(), AppPathBinDir)  // Adds "/bin"
}

// Looks for: <portable-dir>/bin/wsh-0.27.7-windows.x64.exe
```

**Result:** Backend tries to copy wsh from non-existent `bin/` subdirectory, fails with "CRITICAL: wsh binary not found", error silently ignored because init runs in goroutine.

### Evidence

1. **Backend created version-specific lock file**: `wave-0.27.7.lock` ✅
2. **Backend created shell scripts**: `shell/pwsh/wavepwsh.ps1` ✅
3. **Backend did NOT deploy wsh**: `bin/` directory empty ❌
4. **wsh exists in wrong location**: Root instead of `bin/` subdirectory

### What Actually Worked

- ✅ Backend version isolation (different lock files per version)
- ✅ Lock file cleanup (removed old `wave.lock`)
- ✅ Shell script generation
- ✅ Multi-backend coexistence (v0.27.5 and v0.27.7 both ran)

### What Failed

- ❌ wsh binary deployment (wrong source path)
- ❌ Portable packaging (wsh in wrong location)

### Fix Required

**Option 1: Fix Portable Packaging Script** (Recommended)
Move wsh into `bin/` subdirectory in portable package:
```
agentmux-0.27.7-x64-portable/
  ├── agentmux.exe
  ├── agentmuxsrv.x64.exe
  └── bin/
      └── wsh-0.27.7-windows.x64.exe
```

**Option 2: Fix Backend Code**
Handle portable case differently - look for wsh in root if not found in `bin/`

**Recommendation**: Option 1 - keeps portable structure consistent with installed version
