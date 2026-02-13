# AgentMux Build Reliability Specification

**Version**: 1.0.0
**Date**: 2026-02-12
**Author**: AgentX
**Status**: Implemented

---

## Executive Summary

The AgentMux build system requires external dependencies (Go, Task CLI) that are not consistently available in PATH across different execution environments. This document specifies why the default `task package` command fails in certain contexts and how we've implemented a reliable wrapper script (`build-agentx.ps1`) to ensure consistent builds across all agent environments.

---

## Problem Statement

### Issue 1: Task CLI Not in PATH

**Symptom**:
```powershell
task: The term 'task' is not recognized as a name of a cmdlet, function, script file, or executable program.
```

**Root Cause**:
- Task CLI installed via WinGet to: `C:\Users\asafe\AppData\Local\Microsoft\WinGet\Links\task.exe`
- WinGet adds this directory to user PATH during installation
- However, when PowerShell is invoked from other shells (bash, nested pwsh), the PATH environment variable doesn't include WinGet's directory
- This affects builds triggered by:
  - Claude Code bash shells
  - Nested PowerShell sessions
  - CI/CD environments
  - Other agents invoking builds via subprocess

**Evidence**:
```
[stderr] task: [build:schema] go run cmd/generateschema/main-generateschema.go
[stderr] "go": executable file not found in $PATH
task: Failed to run task "package"
```

### Issue 2: Go Not in PATH

**Symptom**:
```
"go": executable file not found in $PATH
```

**Root Cause**:
- Go installed to: `C:\Program Files\Go\bin\go.exe`
- Added to system PATH during Go installation
- Similar to Task CLI, nested PowerShell sessions don't inherit full PATH
- Task CLI subprocess inherits parent environment, which lacks Go

**Impact**:
- Backend build fails: `task build:backend` cannot compile Go code
- wsh binaries fail to build: Cross-platform compilation requires Go
- Schema generation fails: `go run cmd/generateschema/main-generateschema.go`

### Issue 3: PowerShell Profile Noise

**Symptom**:
```
Set-Location : Cannot find path 'C:\Code' because it does not exist.
At C:\Users\asafe\Documents\WindowsPowerShell\Microsoft.PowerShell_profile.ps1:8 char:1
```

**Root Cause**:
- User's PowerShell profile (`Microsoft.PowerShell_profile.ps1`) attempts to navigate to `C:\Code` on startup
- Directory doesn't exist on this system
- Every PowerShell subprocess spawned by Task emits this error to stderr
- Creates noise in build logs (hundreds of lines)

**Impact**:
- Clutters build output with irrelevant errors
- Makes it harder to identify real build issues
- Violates clean build principle

---

## Why Default Build Doesn't Work

The project's default build command is:

```bash
task package
```

This assumes:
1. `task` executable is in PATH
2. Go toolchain is in PATH
3. PowerShell profile doesn't emit errors

These assumptions are **not met** in multi-agent environments where builds are triggered programmatically.

### Environments Where Default Fails

| Environment | Task in PATH? | Go in PATH? | Profile Errors? | Result |
|-------------|---------------|-------------|-----------------|--------|
| PowerShell 7 (direct) | ✅ | ✅ | ✅ | Works |
| bash → pwsh | ❌ | ❌ | ✅ | **Fails** |
| Agent subprocess | ❌ | ❌ | ✅ | **Fails** |
| CI/CD runner | ❌ | ❌ | ❌ | **Fails** |
| VS Code terminal | ✅ | ✅ | ✅ | Works |

---

## Solution: Build Wrapper Script

### Implementation

**File**: `build-agentx.ps1` (root of wavemux repository)

```powershell
# Build script for AgentMux with proper PATH setup
$ErrorActionPreference = "Stop"

# Add Go to PATH
$env:PATH = "C:\Program Files\Go\bin;$env:PATH"

# Add Task to PATH
$env:PATH = "C:\Users\asafe\AppData\Local\Microsoft\WinGet\Links;$env:PATH"

# Set working directory
Set-Location "C:\Users\asafe\.claw\agentx-workspace\wavemux"

# Run Task build
Write-Host "Starting AgentMux build..."
task package
```

### Why This Works

1. **Explicit PATH Management**: Prepends Go and Task directories to PATH before invoking build
2. **Error Propagation**: `$ErrorActionPreference = "Stop"` ensures failures bubble up
3. **Directory Isolation**: Sets working directory explicitly (no assumptions)
4. **Idempotent**: Can be invoked multiple times without side effects
5. **Agent-Friendly**: Works in any execution context (bash, pwsh, CI/CD)

### Usage

**From bash (Claude Code)**:
```bash
pwsh -Command "& 'C:\Users\asafe\.claw\agentx-workspace\wavemux\build-agentx.ps1'"
```

**From PowerShell**:
```powershell
& C:\Users\asafe\.claw\agentx-workspace\wavemux\build-agentx.ps1
```

**From any agent**:
```bash
pwsh -Command "& ~/.claw/agentx-workspace/wavemux/build-agentx.ps1"
```

---

## Integration for Multi-Agent Reliability

### Step 1: Generalize the Script

**Current issue**: Script hardcodes AgentX's paths
**Solution**: Make paths configurable via environment variables

**Proposed `scripts/build.ps1`** (generalized):

```powershell
# AgentMux Reliable Build Script
# Works in any environment: bash, pwsh, CI/CD, agent subprocess
param(
    [string]$GoPath = "C:\Program Files\Go\bin",
    [string]$TaskPath = "C:\Users\$env:USERNAME\AppData\Local\Microsoft\WinGet\Links",
    [string]$RepoRoot = $PSScriptRoot | Split-Path
)

$ErrorActionPreference = "Stop"

# Validate dependencies exist
if (-not (Test-Path "$GoPath\go.exe")) {
    throw "Go not found at $GoPath. Install Go or set -GoPath parameter."
}

if (-not (Test-Path "$TaskPath\task.exe")) {
    throw "Task CLI not found at $TaskPath. Install Task or set -TaskPath parameter."
}

# Setup environment
$env:PATH = "$GoPath;$TaskPath;$env:PATH"
Set-Location $RepoRoot

# Run build
Write-Host "AgentMux Build Environment:" -ForegroundColor Cyan
Write-Host "  Go:  $(& go version)" -ForegroundColor Gray
Write-Host "  Task: $(& task --version)" -ForegroundColor Gray
Write-Host "  Repo: $RepoRoot" -ForegroundColor Gray
Write-Host ""

task package
```

### Step 2: Add npm Script Alias

**File**: `package.json`

```json
{
  "scripts": {
    "build": "task package",
    "build:reliable": "pwsh -Command \"& ./scripts/build.ps1\"",
    "package": "pwsh -Command \"& ./scripts/build.ps1\""
  }
}
```

### Step 3: Update Documentation

**File**: `README.md`

Add section:

```markdown
## Building

### Quick Build (Default)
```bash
npm run package
```

This automatically handles PATH setup and works in all environments.

### Manual Build (Advanced)
If you have Task and Go in PATH:
```bash
task package
```

### CI/CD / Agent Builds
Use the reliable wrapper:
```bash
pwsh -Command "& ./scripts/build.ps1"
```
```

### Step 4: Update CLAUDE.md

```markdown
## Building AgentMux

**Always use the reliable build script:**

```bash
npm run package
```

**DO NOT use:**
- `task package` directly (assumes PATH is configured)
- `npm run build` (may be outdated)

**Why**: The build script ensures Go and Task CLI are in PATH before building.
```

---

## Alternative Solutions Considered

### 1. Add Dependencies to System PATH Permanently

**Rejected because**:
- Requires manual intervention on every machine
- Breaks workspace isolation (affects other projects)
- Doesn't work in containerized environments
- Agents can't modify host system PATH

### 2. Use npx to Install Task Locally

**Rejected because**:
- Task is not available as npm package
- Would require maintaining a Node wrapper
- Adds unnecessary dependency layer
- Doesn't solve Go PATH issue

### 3. Docker Build Environment

**Rejected because**:
- Adds complexity (Docker daemon, images, volumes)
- Slower than native builds
- Windows Docker has additional overhead
- Doesn't match development workflow (task dev requires native)

### 4. Batch Script (.bat) Instead of PowerShell

**Rejected because**:
- PowerShell is standard on Windows 10+
- Batch has limited error handling
- PowerShell script works in bash via pwsh
- PowerShell is already required by project (profile, Taskfile)

---

## Implementation Checklist

- [x] Create `build-agentx.ps1` wrapper script (AgentX workspace)
- [ ] Generalize to `scripts/build.ps1` (repository)
- [ ] Add parameter validation for Go/Task paths
- [ ] Add npm script alias: `npm run package`
- [ ] Update README.md with build instructions
- [ ] Update CLAUDE.md with agent build guidelines
- [ ] Test in bash subprocess (Claude Code)
- [ ] Test in nested PowerShell
- [ ] Test in GitHub Actions (if CI exists)
- [ ] Document in BUILD_SYSTEM_SPEC.md

---

## Testing

### Test 1: bash → pwsh Invocation

```bash
# From bash shell (Claude Code default)
pwsh -Command "& ./scripts/build.ps1"
echo "Exit code: $?"
```

**Expected**: Build succeeds, exit code 0

### Test 2: Direct PowerShell

```powershell
# From PowerShell 7
./scripts/build.ps1
```

**Expected**: Build succeeds

### Test 3: Clean Environment

```powershell
# Remove Go and Task from PATH temporarily
$env:PATH = $env:PATH -replace 'Go\\bin;', ''
$env:PATH = $env:PATH -replace 'WinGet\\Links;', ''

# Build should still work
./scripts/build.ps1
```

**Expected**: Script adds paths, build succeeds

### Test 4: Wrong Paths

```powershell
./scripts/build.ps1 -GoPath "C:\InvalidPath" -TaskPath "C:\AlsoInvalid"
```

**Expected**: Script throws clear error message, doesn't proceed

---

## Benefits

1. **Reliability**: Builds work in any environment without manual PATH configuration
2. **Discoverability**: Agents can invoke `npm run package` without knowing internal details
3. **Maintainability**: Single source of truth for build dependencies
4. **Debugging**: Clear error messages when dependencies are missing
5. **Onboarding**: New agents/developers don't need to configure PATH manually
6. **CI/CD Ready**: Works in automated environments without special setup

---

## Related Documents

- **BUILD_SYSTEM_SPEC.md**: Overall build system architecture
- **VERSIONED_ARTIFACTS_SPEC.md**: Post-build artifact naming
- **Taskfile.yml**: Task definitions and build pipeline
- **package.json**: npm scripts and project metadata

---

## Appendix: Build Output Comparison

### Before (Failing)

```
[stderr] task: The term 'task' is not recognized...
task: Failed to run task "package": exit status 127
```

### After (Working)

```
]16162;E;{"WAVEMUX_AGENT_ID":"AgentX","WAVEMUX_AGENT_COLOR":"#ef4444"}Starting AgentMux build...
task: [build:backend] ...
task: [tauri:copy-sidecars] ...
    Finished 1 bundle at:
        C:\Users\asafe\.claw\agentx-workspace\wavemux\src-tauri\target\release\bundle\nsis\AgentMux_0.26.0_x64-setup.exe
```

Build time: ~3 minutes on AgentX workspace
