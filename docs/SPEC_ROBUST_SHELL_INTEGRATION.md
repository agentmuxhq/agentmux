# Robust Shell Integration Redesign

**Date**: 2026-02-14
**Status**: Proposal
**Problem**: Shell integration breaks frequently due to stale files, missing binaries, and brittle detection logic

---

## Problem Statement

### Current Issues

1. **Stale Files Persist**
   - User has old file: `C:\Users\asafe\AppData\Roaming\com.a5af.agentmux\shell\pwsh\wavepwsh.ps1`
   - Contains old `$env:WAVETERM` variable (should be `$env:AGENTMUX`)
   - Looks for wsh in wrong directory (root instead of `bin/`)
   - Missing error suppression on cleanup commands

2. **Brittle Binary Detection**
   ```powershell
   # Current approach - too many failure points
   if ($env:AGENTMUX -and $env:AGENTMUX -ne "1") {
       $appDir = Split-Path -Parent $env:AGENTMUX
       $portableBinDir = Join-Path $appDir "bin"
       $portableWsh = Get-ChildItem -Path $portableBinDir -Filter "wsh-*.exe" ...
   }
   ```
   - Depends on `$env:AGENTMUX` being set correctly
   - Assumes specific directory structure (`./bin/`)
   - No fallback if detection fails

3. **No Graceful Degradation**
   ```powershell
   # Current - crashes if wsh not found
   wsh token $env:AGENTMUX_SWAPTOKEN pwsh 2>$null | Out-String
   wsh completion powershell | Out-String | Invoke-Expression
   ```
   - If `wsh` not in PATH → error spam
   - No validation before calling `wsh`
   - User sees error messages on every shell startup

4. **No Self-Healing**
   - File generated once, persists forever
   - Version changes require manual intervention
   - No detection of stale/corrupt files

5. **Complex State Management**
   - Relies on: version cache file, environment variables, binary location, template substitution
   - Too many moving parts = more failure modes

---

## Root Cause Analysis

**From exploration findings:**

```
Server Startup → InitCustomShellStartupFiles() → Version Check → File Generation
                                                      ↓
                                        isCacheValid(~/.waveterm/shell/.version)
                                                      ↓
                                        If version mismatch → regenerate templates
                                                      ↓
                                        WriteTemplateToFile(wavepwsh.ps1)
```

**Issues:**
1. Version check happens at **server startup**, not shell startup
2. Old files **overwritten**, not removed (path changes leave orphans)
3. Template variables substituted **once**, no runtime validation
4. No detection of **missing binaries** at shell startup time

**Example Failure Scenario:**
1. User runs AgentMux v0.27.7 → generates shell integration with `$env:WAVETERM`
2. Update to v0.27.8 → templates updated to use `$env:AGENTMUX`
3. Server regenerates file at `~/.waveterm/shell/pwsh/wavepwsh.ps1`
4. But shell is still sourcing OLD file at `C:\Users\asafe\AppData\Roaming\com.a5af.agentmux\shell\pwsh\wavepwsh.ps1`
5. Shell integration broken, wsh errors on every startup

---

## Design Goals

### Primary Objectives

1. **Always Work** - Shell integration should never break user's shell
2. **Self-Healing** - Auto-recover from stale files, missing binaries
3. **Graceful Degradation** - Missing features don't prevent shell from working
4. **Simple Detection** - One robust method to find wsh binary
5. **Defensive Coding** - Every operation validates and handles errors

### Non-Goals

- Don't change backend template generation (low-risk approach)
- Don't require user intervention (fully automatic)
- Don't break existing installations (backward compatible)

---

## Proposed Solution

### Architecture: Self-Validating Shell Integration

**Key Principle:** Every shell startup validates and self-heals.

```powershell
# New structure
1. Version Guard      - Check if this file is current version
2. Binary Discovery   - Robust search for wsh binary
3. PATH Setup         - Add binary to PATH (if found)
4. Defensive Execution - Only call wsh if binary exists
5. Error Suppression  - All cleanup operations use -ErrorAction SilentlyContinue
```

---

## Detailed Design

### 1. Self-Versioning Header

**Add version metadata to generated file:**

```powershell
# AgentMux Shell Integration for PowerShell
# Generated Version: 0.27.9
# Template Version: 3
# Generated: 2026-02-14T17:00:00Z
# DO NOT EDIT - This file is auto-generated

# Self-validation: Check if regeneration needed
$AGENTMUX_SHELL_VERSION = "0.27.9"
$AGENTMUX_TEMPLATE_VERSION = 3

# Compare with running version (if available)
if ($env:AGENTMUX_VERSION -and $env:AGENTMUX_VERSION -ne $AGENTMUX_SHELL_VERSION) {
    Write-Host "[AgentMux] Shell integration outdated (file: $AGENTMUX_SHELL_VERSION, running: $env:AGENTMUX_VERSION)" -ForegroundColor Yellow
    Write-Host "[AgentMux] Restart AgentMux to regenerate shell integration" -ForegroundColor Yellow
}
```

**Benefits:**
- User sees clear message if file is stale
- No silent failures
- Easy debugging (version visible in file)

---

### 2. Robust Binary Discovery

**Current Issues:**
- Depends on `$env:AGENTMUX` being set correctly
- Assumes specific directory structure
- No fallback strategy

**New Approach: Multi-Strategy Search**

```powershell
# Strategy 1: Explicit wsh binary directory (template-injected, most reliable)
$wshBinaryDir = {{.WSHBINDIR_PWSH}}  # e.g., "C:\Users\...\AppData\Roaming\com.a5af.agentmux\bin"

# Strategy 2: Portable mode detection (for .zip distributions)
$portableBinDir = $null
if ($env:AGENTMUX -and (Test-Path $env:AGENTMUX -PathType Leaf)) {
    $appDir = Split-Path -Parent $env:AGENTMUX
    $candidateDir = Join-Path $appDir "bin"
    if (Test-Path $candidateDir -PathType Container) {
        $portableBinDir = $candidateDir
    }
}

# Strategy 3: Already in PATH (wsh installed globally)
$wshInPath = $null
$wshCommand = Get-Command wsh -ErrorAction SilentlyContinue
if ($wshCommand) {
    $wshInPath = Split-Path -Parent $wshCommand.Source
}

# Select best strategy (priority: portable > installed > PATH)
$selectedWshDir = $null
if ($portableBinDir) {
    $selectedWshDir = $portableBinDir
    # Validate: ensure wsh binary actually exists
    $wshTest = Get-ChildItem -Path $portableBinDir -Filter "wsh*.exe" -File -ErrorAction SilentlyContinue | Select-Object -First 1
    if (-not $wshTest) {
        $selectedWshDir = $null  # Detection failed, fall back
    }
} elseif (Test-Path $wshBinaryDir -PathType Container) {
    $selectedWshDir = $wshBinaryDir
} elseif ($wshInPath) {
    $selectedWshDir = $wshInPath
}

# Add to PATH only if found
if ($selectedWshDir) {
    $env:PATH = $selectedWshDir + "{{.PATHSEP}}" + $env:PATH
}
```

**Benefits:**
- Three independent strategies (redundancy)
- Explicit validation at each step
- Falls back gracefully if detection fails
- No assumptions about directory structure

---

### 3. Defensive wsh Command Execution

**Current Issues:**
```powershell
# Crashes if wsh not found
wsh token $env:AGENTMUX_SWAPTOKEN pwsh 2>$null | Out-String
wsh completion powershell | Out-String | Invoke-Expression
```

**New Approach: Validate Before Execute**

```powershell
# Helper function: Check if wsh is accessible
function Test-WshAvailable {
    $cmd = Get-Command wsh -ErrorAction SilentlyContinue
    return ($null -ne $cmd)
}

# Only execute if wsh exists
if (Test-WshAvailable) {
    # Token swap (dynamic shell configuration)
    if ($env:AGENTMUX_SWAPTOKEN) {
        try {
            $swapOutput = wsh token $env:AGENTMUX_SWAPTOKEN pwsh 2>$null | Out-String
            if ($swapOutput -and $swapOutput.Trim() -ne "") {
                Invoke-Expression $swapOutput
            }
        } catch {
            # Silent failure - don't spam user's shell
            Write-Verbose "[AgentMux] Token swap failed: $_"
        }
    }

    # Load completions
    try {
        $completions = wsh completion powershell 2>$null | Out-String
        if ($completions -and $completions.Trim() -ne "") {
            Invoke-Expression $completions
        }
    } catch {
        Write-Verbose "[AgentMux] Completion loading failed: $_"
    }
} else {
    # wsh not available - skip features that depend on it
    Write-Verbose "[AgentMux] wsh binary not found, some features disabled"
}
```

**Benefits:**
- No errors if wsh missing
- Silent fallback (user's shell still works)
- Try/catch prevents crashes from wsh errors
- Verbose logging (available if user debugs)

---

### 4. Bulletproof Cleanup

**Current Issues:**
```powershell
Remove-Variable -Name agentmux_swaptoken_output
Remove-Item Env:AGENTMUX_SWAPTOKEN
# ❌ Error if variables don't exist
```

**New Approach: Always Use -ErrorAction**

```powershell
# Cleanup temporary variables
Remove-Variable -Name agentmux_swaptoken_output -ErrorAction SilentlyContinue
if (Test-Path Env:AGENTMUX_SWAPTOKEN) {
    Remove-Item Env:AGENTMUX_SWAPTOKEN -ErrorAction SilentlyContinue
}
```

**Benefits:**
- No errors on cleanup
- Works whether variables exist or not
- Defensive coding pattern

---

### 5. Shell Integration Features (Unchanged)

These features work independently, don't depend on wsh:

```powershell
# OSC 7 directory tracking
function Global:_agentmux_si_osc7 { ... }

# Agent environment metadata
function Global:_agentmux_si_agent_env { ... }

# tmux/screen detection
function Global:_agentmux_si_blocked { ... }

# Prompt integration (optional)
# ... existing code ...
```

**Keep these as-is** - they're robust and don't have failure modes.

---

## Complete New Template

**File:** `pkg/util/shellutil/shellintegration/pwsh_agentmuxpwsh.sh`

```powershell
# ============================================================================
# AgentMux Shell Integration for PowerShell
# ============================================================================
# Generated Version: {{.AGENTMUX_VERSION}}
# Template Version: 3
# Generated: {{.TIMESTAMP}}
# DO NOT EDIT - This file is auto-generated
# ============================================================================

# We source this file with: pwsh -NoExit -File <this-file>

# ----------------------------------------------------------------------------
# 1. VERSION GUARD
# ----------------------------------------------------------------------------
$AGENTMUX_SHELL_VERSION = "{{.AGENTMUX_VERSION}}"
$AGENTMUX_TEMPLATE_VERSION = 3

# Warn if file is stale (optional, non-breaking)
if ($env:AGENTMUX_VERSION -and $env:AGENTMUX_VERSION -ne $AGENTMUX_SHELL_VERSION) {
    Write-Host "[AgentMux] Shell integration outdated (file: $AGENTMUX_SHELL_VERSION, running: $env:AGENTMUX_VERSION)" -ForegroundColor Yellow
    Write-Host "[AgentMux] Restart AgentMux to regenerate" -ForegroundColor Yellow
}

# ----------------------------------------------------------------------------
# 2. BINARY DISCOVERY (Multi-Strategy)
# ----------------------------------------------------------------------------

# Strategy 1: Template-injected binary directory (most reliable)
$wshBinaryDir = {{.WSHBINDIR_PWSH}}

# Strategy 2: Portable mode (check for ./bin/ subdirectory)
$portableBinDir = $null
if ($env:AGENTMUX -and (Test-Path $env:AGENTMUX -PathType Leaf)) {
    $appDir = Split-Path -Parent $env:AGENTMUX
    $candidateDir = Join-Path $appDir "bin"
    if (Test-Path $candidateDir -PathType Container) {
        # Validate: ensure wsh binary exists
        $wshTest = Get-ChildItem -Path $candidateDir -Filter "wsh*.exe" -File -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($wshTest) {
            $portableBinDir = $candidateDir
        }
    }
}

# Strategy 3: Already in PATH (globally installed)
$wshInPath = $null
$wshCommand = Get-Command wsh -ErrorAction SilentlyContinue
if ($wshCommand) {
    $wshInPath = Split-Path -Parent $wshCommand.Source
}

# Select best strategy (priority: portable > installed > PATH)
$selectedWshDir = $null
if ($portableBinDir) {
    $selectedWshDir = $portableBinDir
} elseif (Test-Path $wshBinaryDir -PathType Container) {
    $selectedWshDir = $wshBinaryDir
} elseif ($wshInPath) {
    $selectedWshDir = $wshInPath
}

# ----------------------------------------------------------------------------
# 3. PATH SETUP
# ----------------------------------------------------------------------------

if ($selectedWshDir) {
    # Prepend to PATH (only if not already present)
    if ($env:PATH -notlike "*$selectedWshDir*") {
        $env:PATH = $selectedWshDir + "{{.PATHSEP}}" + $env:PATH
    }
} else {
    Write-Verbose "[AgentMux] wsh binary not found - some features will be unavailable"
}

# ----------------------------------------------------------------------------
# 4. HELPER FUNCTIONS
# ----------------------------------------------------------------------------

function Test-WshAvailable {
    $cmd = Get-Command wsh -ErrorAction SilentlyContinue
    return ($null -ne $cmd)
}

# ----------------------------------------------------------------------------
# 5. TOKEN SWAP (Dynamic Shell Configuration)
# ----------------------------------------------------------------------------

if (Test-WshAvailable) {
    if ($env:AGENTMUX_SWAPTOKEN) {
        try {
            $agentmux_swaptoken_output = wsh token $env:AGENTMUX_SWAPTOKEN pwsh 2>$null | Out-String
            if ($agentmux_swaptoken_output -and $agentmux_swaptoken_output.Trim() -ne "") {
                Invoke-Expression $agentmux_swaptoken_output
            }
        } catch {
            Write-Verbose "[AgentMux] Token swap failed: $_"
        }

        # Cleanup
        Remove-Variable -Name agentmux_swaptoken_output -ErrorAction SilentlyContinue
        if (Test-Path Env:AGENTMUX_SWAPTOKEN) {
            Remove-Item Env:AGENTMUX_SWAPTOKEN -ErrorAction SilentlyContinue
        }
    }
}

# ----------------------------------------------------------------------------
# 6. LOAD COMPLETIONS
# ----------------------------------------------------------------------------

if (Test-WshAvailable) {
    try {
        $completions = wsh completion powershell 2>$null | Out-String
        if ($completions -and $completions.Trim() -ne "") {
            Invoke-Expression $completions
        }
    } catch {
        Write-Verbose "[AgentMux] Completion loading failed: $_"
    }
}

# ----------------------------------------------------------------------------
# 7. SHELL INTEGRATION FEATURES
# ----------------------------------------------------------------------------

# tmux/screen detection
function Global:_agentmux_si_blocked {
    return ($env:TMUX -or $env:STY -or $env:TERM -like "tmux*" -or $env:TERM -like "screen*")
}

# OSC 7 directory tracking
function Global:_agentmux_si_osc7 {
    if (_agentmux_si_blocked) { return }

    # Get hostname (allow empty for file:/// format)
    $hostname = $env:COMPUTERNAME
    if (-not $hostname) { $hostname = "" }
    $hostname = $hostname.ToLower()

    # Encode current directory path
    $encoded_pwd = [System.Uri]::EscapeDataString($PWD.Path)

    # Send OSC 7 sequence
    $osc7 = "`e]7;file://$hostname/$encoded_pwd`a"
    Write-Host -NoNewline $osc7
}

# Agent environment metadata (OSC 16162)
function Global:_agentmux_si_agent_env {
    if (_agentmux_si_blocked) { return }

    if ($env:WAVEMUX_AGENT_ID) {
        $agent_env_json = @{
            "WAVEMUX_AGENT_ID" = $env:WAVEMUX_AGENT_ID
        } | ConvertTo-Json -Compress

        $encoded = [System.Uri]::EscapeDataString($agent_env_json)
        $osc16162 = "`e]16162;$encoded`a"
        Write-Host -NoNewline $osc16162
    }
}

# Pre-command hook (runs before each command)
function Global:_agentmux_si_precmd {
    _agentmux_si_osc7
    _agentmux_si_agent_env
}

# Install prompt hook (if not in tmux/screen)
if (-not (_agentmux_si_blocked)) {
    $Global:_agentmux_precmd_installed = $true

    # Add to PSReadLine PreExecuteHandlers (PowerShell 7+)
    if (Get-Module -Name PSReadLine -ErrorAction SilentlyContinue) {
        Set-PSReadLineOption -AddToHistoryHandler {
            param($line)
            _agentmux_si_precmd
            return $true
        } -ErrorAction SilentlyContinue
    }
}

# ----------------------------------------------------------------------------
# END OF AGENTMUX SHELL INTEGRATION
# ----------------------------------------------------------------------------
```

---

## Backend Changes

### Template Variable Additions

**File:** `pkg/util/shellutil/shellutil.go`

```go
// Update InitRcFiles() to include new template variables
params := map[string]string{
    "WSHBINDIR":          HardQuote(absWshBinDir),
    "WSHBINDIR_PWSH":     HardQuotePowerShell(absWshBinDir),
    "PATHSEP":            pathSep,
    "AGENTMUX_VERSION":   wavebase.WaveVersion,           // NEW
    "TIMESTAMP":          time.Now().Format(time.RFC3339), // NEW
}
```

### Version Cache Enhancement

**Optional improvement:** Include template version in cache file

```go
// File: pkg/util/shellutil/shellutil.go
type ShellCacheVersion struct {
    AgentMuxVersion  string `json:"agentmux_version"`
    TemplateVersion  int    `json:"template_version"`  // NEW
    CacheCreatedAt   string `json:"cache_created_at"`
}

const CURRENT_TEMPLATE_VERSION = 3  // Increment when template structure changes

func isCacheValid(waveHome string) bool {
    // ... existing code ...

    // Check both app version AND template version
    if cacheVersion.AgentMuxVersion != wavebase.WaveVersion {
        return false
    }

    // NEW: Check if template structure changed
    if cacheVersion.TemplateVersion < CURRENT_TEMPLATE_VERSION {
        return false  // Force regeneration on template updates
    }

    return true
}
```

**Benefits:**
- Force regeneration when template logic changes (not just app version)
- Allows hotfixes to shell integration without bumping app version
- More granular control over cache invalidation

---

## Migration Strategy

### Phase 1: Update Template (Low Risk)

1. Replace `pkg/util/shellutil/shellintegration/pwsh_agentmuxpwsh.sh` with new template
2. Add `AGENTMUX_VERSION` and `TIMESTAMP` to template params
3. No backend logic changes yet

**Test:**
- Build portable package
- Extract and run
- Open PowerShell tab
- Verify no errors, wsh commands work

### Phase 2: Add Template Versioning (Optional)

1. Add `TEMPLATE_VERSION` constant and cache field
2. Update `isCacheValid()` to check template version
3. Bump `TEMPLATE_VERSION` to force regeneration

**Test:**
- Run AgentMux with new binary
- Verify shell integration regenerates automatically
- Check that version warnings appear correctly

### Phase 3: Cleanup Old Files (Future)

**Optional enhancement:** Add cleanup logic similar to lock files

```go
// File: pkg/util/shellutil/shellutil.go
func CleanupOldShellIntegrationFiles(waveHome string) error {
    shellDir := filepath.Join(waveHome, "shell")

    // Find all shell integration files
    matches, _ := filepath.Glob(filepath.Join(shellDir, "**/*.ps1"))

    for _, file := range matches {
        // Check if file has version metadata
        content, err := os.ReadFile(file)
        if err != nil {
            continue
        }

        // Extract version from file
        // If version < current, delete file
        // ...
    }

    return nil
}
```

---

## Testing Plan

### Test Cases

| Test | Scenario | Expected Result |
|------|----------|-----------------|
| **T1** | Fresh install (no shell integration) | File generated, wsh works, no errors |
| **T2** | Version mismatch (file: 0.27.8, binary: 0.27.9) | Warning shown, features still work |
| **T3** | Missing wsh binary | No errors, shell starts normally, features disabled |
| **T4** | Portable mode (wsh in ./bin/) | Detects portable bin, uses it successfully |
| **T5** | Installed mode (wsh in AppData) | Uses installed binary path |
| **T6** | wsh already in PATH | Uses existing PATH entry, no duplication |
| **T7** | AGENTMUX env var not set | Falls back to template-injected path |
| **T8** | Token swap with invalid token | Silent failure, no error spam |
| **T9** | Completion loading failure | Silent failure, shell still works |
| **T10** | Running in tmux | Shell integration disabled, no OSC sequences |

### Manual Testing Steps

```powershell
# Test 1: Fresh install
Remove-Item -Recurse -Force ~/.waveterm/shell
# Start AgentMux, open PowerShell tab
# Expected: File generated, no errors

# Test 2: Version mismatch
# Edit wavepwsh.ps1, change $AGENTMUX_SHELL_VERSION to "0.27.0"
# Start new PowerShell tab
# Expected: Warning message, but features still work

# Test 3: Missing wsh binary
Rename-Item ~/.waveterm/bin/wsh.exe wsh.exe.bak
# Start new PowerShell tab
# Expected: No errors, verbose message about missing wsh

# Test 4: Portable mode
# Extract portable ZIP, run agentmux.exe
# Expected: Detects ./bin/wsh-*.exe, uses it

# Test 5: Token swap failure
$env:AGENTMUX_SWAPTOKEN = "invalid-token"
# Source wavepwsh.ps1
# Expected: No error output

# Test 6: Cleanup validation
$env:AGENTMUX_SWAPTOKEN = "test"
Remove-Item Env:AGENTMUX_SWAPTOKEN  # Simulate already cleaned
# Run cleanup section again
# Expected: No errors
```

---

## Success Criteria

✅ **No Errors on Shell Startup**
- User never sees error messages when opening PowerShell
- Missing wsh binary = silent degradation, not crash

✅ **Self-Healing**
- Stale file detection via version guard
- User notified if file outdated
- Features still work even with version mismatch

✅ **Graceful Degradation**
- Missing wsh binary → features disabled, shell works
- Invalid token → silent failure
- Completion errors → no crash

✅ **Robust Detection**
- Three independent strategies for finding wsh
- Explicit validation at each step
- No assumptions about directory structure

✅ **Defensive Coding**
- All cleanup uses `-ErrorAction SilentlyContinue`
- All wsh calls wrapped in try/catch
- All operations validate before execute

---

## Rollback Plan

If new template causes issues:

1. **Immediate:** Revert `pwsh_agentmuxpwsh.sh` to previous version
2. **Rebuild:** Create new portable package
3. **Deploy:** Users extract new portable ZIP

**Risk:** Very low - new template is backward compatible

---

## Future Enhancements

### 1. Self-Updating Shell Integration

Add update check to shell integration file:

```powershell
# Check if newer version available
$latestVersion = wsh version --shell-integration 2>$null
if ($latestVersion -and $latestVersion -ne $AGENTMUX_SHELL_VERSION) {
    Write-Host "[AgentMux] New shell integration available, updating..." -ForegroundColor Cyan
    wsh update-shell-integration powershell
    # Re-source this file
    . $PSCommandPath
}
```

### 2. Diagnostic Command

Add `wsh diagnose shell` command:

```bash
wsh diagnose shell
```

Output:
```
AgentMux Shell Integration Diagnostics
======================================
Shell: PowerShell 7.5.4
Integration File: ~/.waveterm/shell/pwsh/wavepwsh.ps1
File Version: 0.27.9
Template Version: 3
Binary Version: 0.27.9

✅ wsh binary found at: ~/.waveterm/bin/wsh.exe
✅ Shell integration up-to-date
✅ Completions loaded
✅ Token swap working

Recommendations: None
```

### 3. Portable Mode Indicator

Show portable mode in prompt:

```powershell
if ($portableBinDir) {
    $env:AGENTMUX_MODE = "portable"
} else {
    $env:AGENTMUX_MODE = "installed"
}
```

User can add to prompt:
```powershell
"[AgentMux:$env:AGENTMUX_MODE] $pwd> "
```

---

## Related Issues

- #295 - Rebrand cache versioning (shell integration uses old WAVETERM var)
- #300 - Portable wsh deployment (bin/ subdirectory)
- User reported: `wsh` command not found errors (this spec)

---

## Questions

1. **Should we auto-update shell integration files?**
   - Pro: Users always have latest fixes
   - Con: Unexpected changes to user's shell environment
   - **Recommendation:** Warn but don't auto-update (current design)

2. **Should we cleanup old shell integration paths?**
   - Pro: No stale files
   - Con: Risk of deleting user customizations
   - **Recommendation:** Add cleanup in Phase 3 (future enhancement)

3. **Should we support custom shell integration paths?**
   - Pro: Power users can customize
   - Con: More complexity
   - **Recommendation:** Not needed for v1 (keep simple)

---

## Implementation Checklist

### Backend Changes
- [ ] Update `pwsh_agentmuxpwsh.sh` template
- [ ] Add `AGENTMUX_VERSION` to template params
- [ ] Add `TIMESTAMP` to template params
- [ ] (Optional) Add template versioning to cache
- [ ] (Optional) Add cleanup logic for old files

### Testing
- [ ] Test fresh install
- [ ] Test version mismatch warning
- [ ] Test missing wsh binary
- [ ] Test portable mode detection
- [ ] Test installed mode detection
- [ ] Test cleanup error suppression
- [ ] Test token swap failure handling
- [ ] Test completion loading failure

### Documentation
- [ ] Update shell integration docs
- [ ] Add troubleshooting guide
- [ ] Document version guard feature
- [ ] Add debugging tips (verbose mode)

### Release
- [ ] Bump template version constant
- [ ] Create portable package
- [ ] Test on Windows
- [ ] Create PR
- [ ] Merge and deploy

---

## Timeline

**Estimated Effort:** 2-3 hours (template update + testing)

- **Phase 1 (Template Update):** 1 hour
- **Phase 2 (Template Versioning):** 30 min
- **Testing:** 1 hour
- **Documentation:** 30 min

**Target Release:** Next AgentMux version (0.27.10 or 0.28.0)

---

## Conclusion

This redesign addresses the root causes of shell integration failures:

1. **Stale files** → Version guard warns users
2. **Brittle detection** → Multi-strategy search with validation
3. **Hard failures** → Graceful degradation with error suppression
4. **No self-healing** → Version checking and validation

The new template is **backward compatible** and **low risk** - existing users won't be broken, and new users get a more robust experience.

**Key Innovation:** Shell integration that validates itself on every startup and degrades gracefully instead of crashing.
