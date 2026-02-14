# Portable WSH Path Detection Fix + AGENTMUX Rebrand

## Problem Statement

**Issue 1 - WSH Not Found**: In portable builds, the `wsh` command is not found when running shell commands, causing errors:

```powershell
wsh: The term 'wsh' is not recognized as a name of a cmdlet, function, script file,
or executable program.
```

**Issue 2 - Legacy Naming**: Environment variables still use "WAVETERM" prefix from the old WaveTerm branding, should be "AGENTMUX".

**Root Causes**:
1. The shell integration script's portable mode detection relies on `$env:WAVETERM` being set to the **executable path**, but the backend currently sets it to the string `"1"`.
2. All environment variables use legacy "WAVETERM" prefix instead of "AGENTMUX".

## Current Behavior

### Backend (shellcontroller.go:708)
```go
token.Env["WAVETERM"] = "1"  // ❌ Just a flag, not a path
```

### Shell Integration (pwsh_agentmuxpwsh.sh:3-18)
```powershell
# Detect portable mode: check if wsh exists in AgentMux app directory
$portableWshPath = $null
if ($env:WAVETERM) {
    $appDir = Split-Path -Parent $env:WAVETERM  # ❌ FAILS: Can't split "1"
    $portableWsh = Get-ChildItem -Path $appDir -Filter "wsh-*.exe" ...
    if ($portableWsh) {
        $portableWshPath = $appDir
    }
}
```

**Result**:
- `Split-Path -Parent "1"` returns current directory, not the app directory
- Portable detection fails
- `wsh` not added to PATH
- Shell integration breaks

## Expected Behavior

### Portable Build Structure
```
agentmux-0.27.3-x64-portable/
├── agentmux.exe
├── agentmuxsrv.x64.exe
├── wsh-0.27.3-windows.x64.exe  ← Must be found and added to PATH
└── README.txt
```

### Detection Flow
1. Backend sets `WAVETERM=/path/to/agentmux.exe`
2. Shell script extracts parent directory: `/path/to/`
3. Searches for `wsh-*.exe` in same directory
4. Finds `wsh-0.27.3-windows.x64.exe`
5. Adds `/path/to/` to PATH
6. `wsh` command now works ✅

## Proposed Solution

### Change 1: Rebrand to AGENTMUX + Set Executable Path

**File**: `pkg/blockcontroller/shellcontroller.go`
**Location**: Line ~705-729 in `makeSwapToken()` function

**Before**:
```go
token.Env["TERM_PROGRAM"] = "waveterm"
token.Env["WAVETERM_BLOCKID"] = bc.BlockId
token.Env["WAVETERM_VERSION"] = wavebase.WaveVersion
token.Env["WAVETERM"] = "1"  // ❌ Wrong: legacy name + not a path
// ... more WAVETERM_* variables
token.Env["WAVETERM_TABID"] = tabId
token.Env["WAVETERM_WORKSPACEID"] = wsId
token.Env["WAVETERM_CLIENTID"] = clientData.OID
token.Env["WAVETERM_CONN"] = remoteName
```

**After**:
```go
token.Env["TERM_PROGRAM"] = "agentmux"  // ✅ Rebranded
token.Env["AGENTMUX_BLOCKID"] = bc.BlockId  // ✅ Rebranded
token.Env["AGENTMUX_VERSION"] = wavebase.WaveVersion  // ✅ Rebranded

// Set AGENTMUX to executable path for portable mode detection
// This allows shell integration scripts to find wsh binary in same directory
exePath, err := os.Executable()
if err == nil {
    token.Env["AGENTMUX"] = exePath  // ✅ Path + rebranded
} else {
    log.Printf("warn: failed to get executable path: %v, using fallback", err)
    token.Env["AGENTMUX"] = "1"
}

// ... more AGENTMUX_* variables (all rebranded)
token.Env["AGENTMUX_TABID"] = tabId
token.Env["AGENTMUX_WORKSPACEID"] = wsId
token.Env["AGENTMUX_CLIENTID"] = clientData.OID
token.Env["AGENTMUX_CONN"] = remoteName
```

**Rationale**:
- `os.Executable()` returns the absolute path to the running binary
- For portable: `/path/to/extracted/agentmuxsrv.x64.exe` (sidecar binary)
- For installed: `C:\Users\...\AppData\Local\agentmux\agentmuxsrv.exe`
- Shell scripts can now reliably extract the parent directory
- Backward compatible: Falls back to `"1"` if path unavailable

### Change 2: Update Shell Integration Scripts (REQUIRED)

**Files**:
- `pkg/util/shellutil/shellintegration/pwsh_agentmuxpwsh.sh`
- `pkg/util/shellutil/shellintegration/bash_bashrc.sh`
- `pkg/util/shellutil/shellintegration/zsh_zshrc.sh`
- `pkg/util/shellutil/shellintegration/fish_agentmuxfish.sh`

**Before** (PowerShell example):
```powershell
# Detect portable mode: check if wsh exists in AgentMux app directory
$portableWshPath = $null
if ($env:WAVETERM) {  # ❌ Legacy name
    $appDir = Split-Path -Parent $env:WAVETERM
    ...
}
```

**After** (PowerShell example):
```powershell
# Detect portable mode: check if wsh exists in AgentMux app directory
$portableWshPath = $null
if ($env:AGENTMUX -and $env:AGENTMUX -ne "1") {  # ✅ Rebranded + safety check
    # Get directory containing the AgentMux binary
    $appDir = Split-Path -Parent $env:AGENTMUX

    # Look for wsh binary in same directory
    $portableWsh = Get-ChildItem -Path $appDir -Filter "wsh-*.exe" -File -ErrorAction SilentlyContinue | Select-Object -First 1

    if ($portableWsh) {
        $portableWshPath = $appDir
    }
}
```

**Changes**:
- `$env:WAVETERM` → `$env:AGENTMUX` (rebranded)
- Added check: `$env:AGENTMUX -ne "1"` for backward compatibility
- Same logic for bash/zsh/fish shell integration scripts

## Testing

### Unit Test (Go)
```go
func TestWavetermEnvPath(t *testing.T) {
    // Create a shell controller
    bc := &ShellController{...}

    // Generate swap token
    token := bc.makeSwapToken(...)

    // Verify WAVETERM is set to a path, not "1"
    waveterm := token.Env["WAVETERM"]
    assert.NotEqual(t, "1", waveterm)
    assert.True(t, filepath.IsAbs(waveterm))

    // Verify path exists and is executable
    info, err := os.Stat(waveterm)
    assert.NoError(t, err)
    assert.False(t, info.IsDir())
}
```

### Integration Test (Manual)

**Setup**:
1. Build portable package
2. Extract to `C:\Test\AgentMux\`
3. Run `agentmux.exe`
4. Open PowerShell terminal

**Verification**:
```powershell
# 1. Check WAVETERM is set to path
echo $env:WAVETERM
# Expected: C:\Test\AgentMux\agentmuxsrv.x64.exe

# 2. Check wsh is in PATH
where.exe wsh
# Expected: C:\Test\AgentMux\wsh-0.27.3-windows.x64.exe

# 3. Verify wsh works
wsh --version
# Expected: wsh version 0.27.3

# 4. Test shell integration
wsh completion powershell
# Expected: Completion script output (no errors)
```

**Success Criteria**:
- ✅ No "wsh not found" errors
- ✅ Shell integration loads without errors
- ✅ `wsh` command works from any directory
- ✅ Works on fresh Windows install (no prior config)

## Backward Compatibility

### Installed Builds
- Still work correctly
- `WAVETERM` points to installed binary path
- Shell integration uses `{{.WSHBINDIR}}` template variable as primary
- Portable detection is a fallback/override mechanism

### Old Shell Integration Scripts
- If `WAVETERM="1"`, `Split-Path` returns current directory
- Falls back to `{{.WSHBINDIR}}` template (which is always set correctly)
- No regression for existing users

### Remote Connections
- `WAVETERM` only used for local shells
- Remote shells use remote-specific wsh deployment
- No impact on SSH/WSL connections

## Implementation Steps

1. **Code Change**: Update `shellcontroller.go` line ~708
2. **Build Test**: Verify compilation succeeds
3. **Local Test**: Test portable build manually
4. **Integration Test**: Run full test suite
5. **Documentation**: Update portable build README
6. **Release**: Include in next patch version (0.27.4)

## Files to Modify

### Required
- `pkg/blockcontroller/shellcontroller.go` - Set WAVETERM to executable path

### Optional (Enhancement)
- `pkg/util/shellutil/shellintegration/pwsh_agentmuxpwsh.sh` - Add debug output
- `pkg/util/shellutil/shellintegration/bash_bashrc.sh` - Same for bash
- `pkg/util/shellutil/shellintegration/zsh_zshrc.sh` - Same for zsh
- `pkg/util/shellutil/shellintegration/fish_agentmuxfish.sh` - Same for fish

## Related Issues

- **PR #284**: Originally implemented portable shell integration detection
- **Issue**: "wsh not found" in portable builds on unconfigured systems
- **Root Cause**: Incomplete implementation - detection logic added but env var not set correctly

## Success Metrics

**Before**:
- ❌ Fresh portable extraction: `wsh` not found
- ❌ Requires manual PATH configuration
- ❌ Poor first-run experience

**After**:
- ✅ Fresh portable extraction: `wsh` works immediately
- ✅ Zero configuration required
- ✅ Excellent first-run experience
- ✅ Works on any Windows system (including air-gapped)

## Security Considerations

- `os.Executable()` is safe - returns path to running binary
- No user input involved
- No path traversal risk
- No privilege escalation
- Standard Go stdlib function

## Performance Impact

- Negligible: `os.Executable()` called once per shell spawn
- Fast operation (~microseconds)
- No disk I/O beyond what already happens
- No network calls

## Alternative Solutions Considered

### Alternative 1: Environment Variable at Launch
Set `WAVETERM` in Tauri launcher before spawning backend.

**Pros**: Separation of concerns
**Cons**: Requires Rust changes, more complex, breaks remote shells

### Alternative 2: Config File
Store executable path in config file.

**Pros**: Persistent
**Cons**: Stale if binary moves, overcomplicated for simple need

### Alternative 3: WSH in System PATH
Add wsh to system PATH during extraction.

**Pros**: Works everywhere
**Cons**: Not portable (modifies system), requires admin, pollution

**Selected Solution**: Setting `WAVETERM` to `os.Executable()` is the simplest, most reliable approach that requires minimal code changes and no external dependencies.

---

**Author**: AgentA
**Date**: 2026-02-13
**Status**: Ready for Implementation
**Priority**: High (affects all portable users)
