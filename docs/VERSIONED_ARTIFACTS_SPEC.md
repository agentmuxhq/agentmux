# AgentMux Versioned Build Artifacts Specification

**Status:** Design Phase
**Date:** 2026-02-12
**Author:** AgentX
**Priority:** CRITICAL (Portable versioning required)

---

## Executive Summary

**Problem:** The portable executable lacks version information in its filename, making it impossible to distinguish between different versions when distributed.

**Current State:**
- ✅ **Installer:** `AgentMux_0.25.0_x64-setup.exe` (versioned)
- ❌ **Portable:** `agentmux.exe` (NO version)

**Goal:**
- ✅ **Installer:** `AgentMux-0.25.0-x64-setup.exe` (improve naming)
- ✅ **Portable:** `agentmux-0.25.0-x64.exe` (add version - CRITICAL)

---

## Table of Contents

1. [Problem Statement](#problem-statement)
2. [Current State Analysis](#current-state-analysis)
3. [Tauri Build System Deep Dive](#tauri-build-system-deep-dive)
4. [Naming Convention Standards](#naming-convention-standards)
5. [Solution Design](#solution-design)
6. [Implementation Plan](#implementation-plan)
7. [Testing Strategy](#testing-strategy)
8. [Rollout Plan](#rollout-plan)

---

## Problem Statement

### User Impact

**Scenario 1: Download Multiple Versions**
```
Downloads/
├── agentmux.exe          # Which version is this?
├── agentmux.exe (1)      # Browser renamed it
└── agentmux.exe (2)      # Lost all version info
```

**Scenario 2: Testing Multiple Versions**
```bash
# User wants to test v0.24.0 vs v0.25.0
# Currently impossible without renaming manually
agentmux.exe              # No idea which version
agentmux-backup.exe       # User renamed manually
```

**Scenario 3: Release Distribution**
```
GitHub Releases:
- agentmux.exe            # ❌ Looks the same across all releases
- AgentMux_0.25.0_x64-setup.exe  # ✅ Clear version
```

### Business Impact

- **Support Difficulty:** Users report issues without knowing which version they're running
- **Distribution Confusion:** Multiple downloads of the same file overwrite each other
- **Professional Image:** Inconsistent naming reduces perceived quality
- **Debugging Issues:** Cannot easily identify which build is causing problems

---

## Current State Analysis

### Build Output Structure

**After `task package` completes:**

```
src-tauri/target/release/
├── agentmux.exe                                    ❌ No version
└── bundle/
    └── nsis/
        └── AgentMux_0.25.0_x64-setup.exe          ✅ Has version
```

### Version Sources (Must Stay in Sync)

```javascript
// package.json (primary source of truth)
{
  "name": "agentmux",
  "version": "0.25.0",
  "productName": "AgentMux"
}
```

```toml
# src-tauri/Cargo.toml
[package]
name = "agentmux"
version = "0.25.0"
```

```json
// src-tauri/tauri.conf.json
{
  "productName": "AgentMux",
  "version": "0.25.0",
  "identifier": "com.a5af.agentmux"
}
```

### Version Management Tools

**Existing:**
- `bump-version.sh` - Updates all version files atomically
- `scripts/verify-version.sh` - Validates version consistency

**Gap:**
- No post-build artifact renaming
- No automated release artifact preparation

---

## Tauri Build System Deep Dive

### How Tauri Names Output Files

#### 1. Portable Executable

**Source:** Rust Cargo compilation
**Location:** `src-tauri/target/release/{binary_name}.exe`
**Naming Logic:**

```rust
// From Cargo.toml [package]
name = "agentmux"  // Becomes: agentmux.exe
```

**Configuration Override:**

```json
// tauri.conf.json
{
  "build": {
    "mainBinaryName": "agentmux"  // Optional override
  }
}
```

**Tauri's Limitation:** Version is NOT included in the executable filename by design.

#### 2. NSIS Installer

**Source:** Tauri bundler (tauri-bundler crate)
**Location:** `src-tauri/target/release/bundle/nsis/{ProductName}_{version}_{arch}-setup.exe`
**Naming Logic:**

```rust
// From tauri-bundler source code:
// https://github.com/tauri-apps/tauri/blob/dev/tooling/bundler/src/bundle/windows/nsis.rs

let installer_filename = format!(
    "{}_{}_{}",
    settings.product_name(),
    settings.version_string(),
    arch
);
// Result: "AgentMux_0.25.0_x64-setup.exe"
```

**Configuration Sources:**
- `productName` from `tauri.conf.json`
- `version` from `tauri.conf.json`
- `arch` detected at build time (x64, arm64)

**Tauri's Limitation:** Cannot customize installer filename format.

### Build Flow Diagram

```
┌─────────────────────────────────────────────────────────────┐
│ 1. npm run tauri build                                      │
└─────────────────────────────┬───────────────────────────────┘
                              │
              ┌───────────────┴────────────────┐
              │                                │
              ▼                                ▼
┌──────────────────────────┐    ┌──────────────────────────────┐
│ 2a. Vite Build Frontend  │    │ 2b. Cargo Build Rust App     │
│    Output: dist/frontend │    │    Output: agentmux.exe      │
└──────────────────────────┘    └──────────┬───────────────────┘
                                           │
                                           ▼
                              ┌──────────────────────────────────┐
                              │ 3. Tauri Bundler                 │
                              │    - Packages frontend + rust    │
                              │    - Bundles sidecars            │
                              │    - Creates installer           │
                              └──────────┬───────────────────────┘
                                         │
                                         ▼
                    ┌────────────────────────────────────────┐
                    │ 4. Output Files                        │
                    │    - agentmux.exe (portable, no ver)   │
                    │    - AgentMux_{ver}_{arch}-setup.exe   │
                    └────────────────────────────────────────┘
```

### Why Portable Lacks Version

**From Tauri Documentation:**

> The portable executable in `target/release/` is the **raw Rust binary** output from Cargo. It's not processed by the Tauri bundler, which is why it lacks version information.
>
> The bundler only adds version metadata to **installers** (MSI, NSIS, DMG, DEB, AppImage), not to the standalone executable.

**Technical Reason:**
Rust's Cargo build system names binaries after the package name. There's no built-in mechanism to include version in the binary filename.

---

## Naming Convention Standards

### Industry Standard Patterns

**Portable Executables:**
```
{app-name}-{version}-{arch}.exe
Examples:
  - vscode-1.85.0-x64.exe
  - firefox-120.0-win64.exe
  - slack-4.35.131-x64.exe
  - discord-1.0.9014-x64.exe
```

**Installers:**
```
{AppName}-{version}-{arch}-setup.exe
Examples:
  - VSCodeSetup-1.85.0-x64.exe
  - Firefox Setup 120.0.exe
  - SlackSetup-4.35.131-x64.exe
```

### Proposed AgentMux Standard

**Portable Executable:**
```
agentmux-{version}-{arch}.exe
Examples:
  - agentmux-0.25.0-x64.exe
  - agentmux-0.25.0-arm64.exe
  - agentmux-0.26.0-beta1-x64.exe
```

**Installer:**
```
AgentMux-{version}-{arch}-setup.exe
Examples:
  - AgentMux-0.25.0-x64-setup.exe
  - AgentMux-0.25.0-arm64-setup.exe
```

**Rationale:**
- **Lowercase for portable:** Matches Unix conventions, easier to type in CLI
- **TitleCase for installer:** More polished, Windows-friendly
- **Hyphens:** Better for URLs and cross-platform compatibility than underscores
- **Architecture suffix:** Critical for multi-arch support (x64, arm64)

---

## Solution Design

### Approach: Post-Build Artifact Renaming

**Why This Approach?**
1. ✅ **Tauri doesn't provide native config** for versioned portable names
2. ✅ **Non-invasive:** Doesn't modify Tauri internals
3. ✅ **Flexible:** Easy to change naming convention later
4. ✅ **Reliable:** Runs after successful build
5. ✅ **Cross-platform:** Can be implemented in PowerShell + Bash

**Alternatives Considered:**

| Approach | Pros | Cons | Verdict |
|----------|------|------|---------|
| **Custom Cargo build script** | Native to Rust | Complex, fragile, doesn't work with Tauri | ❌ Rejected |
| **Patch Tauri source** | Full control | Maintenance nightmare, breaks updates | ❌ Rejected |
| **Custom NSIS template** | Controls installer naming | Doesn't help portable, overly complex | ❌ Rejected |
| **Post-build script** | Simple, flexible, maintainable | Requires extra step | ✅ **Selected** |

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     Tauri Build Completes                        │
│  - agentmux.exe (portable)                                      │
│  - AgentMux_0.25.0_x64-setup.exe (installer)                    │
└────────────────────────────┬────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│              Post-Build Script Triggers                         │
│              (scripts/rename-artifacts.ps1)                      │
└────────────────────────────┬────────────────────────────────────┘
                             │
        ┌────────────────────┼────────────────────┐
        │                    │                    │
        ▼                    ▼                    ▼
┌──────────────┐  ┌──────────────────┐  ┌──────────────────┐
│ Read Version │  │ Detect Arch      │  │ Locate Artifacts │
│ from pkg.json│  │ (x64/arm64)      │  │ in target/       │
└──────┬───────┘  └────────┬─────────┘  └────────┬─────────┘
       │                   │                      │
       └───────────────────┴──────────────────────┘
                           │
                           ▼
         ┌──────────────────────────────────────────┐
         │ Rename & Copy to dist/releases/          │
         │ - agentmux-0.25.0-x64.exe (portable)     │
         │ - AgentMux-0.25.0-x64-setup.exe (install)│
         └──────────────────────────────────────────┘
                           │
                           ▼
         ┌──────────────────────────────────────────┐
         │ Generate Checksums                       │
         │ - SHA256SUMS.txt                         │
         │ - Signature for auto-updater             │
         └──────────────────────────────────────────┘
```

---

## Implementation Plan

### Phase 1: Create Post-Build Script (CRITICAL)

**File:** `scripts/rename-artifacts.ps1`

```powershell
#!/usr/bin/env pwsh
#Requires -Version 7.0

<#
.SYNOPSIS
    Rename AgentMux build artifacts with versioned filenames.

.DESCRIPTION
    This script runs after `tauri build` completes and:
    1. Reads version from package.json
    2. Detects architecture (x64, arm64)
    3. Renames portable executable: agentmux.exe → agentmux-{version}-{arch}.exe
    4. Renames installer: AgentMux_{version}_{arch}-setup.exe → AgentMux-{version}-{arch}-setup.exe
    5. Copies artifacts to dist/releases/
    6. Generates SHA256 checksums

.EXAMPLE
    ./scripts/rename-artifacts.ps1

.NOTES
    This script is called automatically by the Taskfile.yml after tauri build.
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# ============================================================================
# Configuration
# ============================================================================

$RootDir = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$PackageJsonPath = Join-Path $RootDir "package.json"
$TauriTargetDir = Join-Path $RootDir "src-tauri" "target" "release"
$DistReleasesDir = Join-Path $RootDir "dist" "releases"

# ============================================================================
# Helper Functions
# ============================================================================

function Get-VersionFromPackageJson {
    if (-not (Test-Path $PackageJsonPath)) {
        throw "package.json not found at: $PackageJsonPath"
    }

    $packageJson = Get-Content $PackageJsonPath -Raw | ConvertFrom-Json
    $version = $packageJson.version

    if ([string]::IsNullOrWhiteSpace($version)) {
        throw "Version not found in package.json"
    }

    Write-Host "✓ Detected version: $version" -ForegroundColor Green
    return $version
}

function Get-Architecture {
    # Detect architecture from environment or default to x64
    $arch = $env:PROCESSOR_ARCHITECTURE

    switch -Regex ($arch) {
        "AMD64|x64" { return "x64" }
        "ARM64" { return "arm64" }
        default { return "x64" }  # Default to x64
    }
}

function Ensure-DirectoryExists {
    param([string]$Path)

    if (-not (Test-Path $Path)) {
        Write-Host "Creating directory: $Path" -ForegroundColor Yellow
        New-Item -ItemType Directory -Path $Path -Force | Out-Null
    }
}

function Copy-AndRenameArtifact {
    param(
        [string]$SourcePath,
        [string]$DestPath
    )

    if (-not (Test-Path $SourcePath)) {
        Write-Warning "Source file not found: $SourcePath"
        return $false
    }

    Write-Host "Copying: $(Split-Path -Leaf $SourcePath)" -ForegroundColor Cyan
    Write-Host "     To: $(Split-Path -Leaf $DestPath)" -ForegroundColor Cyan

    Copy-Item -Path $SourcePath -Destination $DestPath -Force

    if (Test-Path $DestPath) {
        Write-Host "✓ Successfully created: $(Split-Path -Leaf $DestPath)" -ForegroundColor Green
        return $true
    } else {
        Write-Error "Failed to create: $DestPath"
        return $false
    }
}

function Generate-Checksums {
    param([string]$Directory)

    $checksumFile = Join-Path $Directory "SHA256SUMS.txt"
    $files = Get-ChildItem -Path $Directory -Filter "*.exe" | Sort-Object Name

    if ($files.Count -eq 0) {
        Write-Warning "No .exe files found to checksum"
        return
    }

    Write-Host "`nGenerating SHA256 checksums..." -ForegroundColor Yellow

    $checksums = @()
    foreach ($file in $files) {
        $hash = (Get-FileHash -Path $file.FullName -Algorithm SHA256).Hash.ToLower()
        $checksums += "$hash  $($file.Name)"
        Write-Host "  $($file.Name): $hash" -ForegroundColor Gray
    }

    $checksums | Out-File -FilePath $checksumFile -Encoding utf8
    Write-Host "✓ Checksums saved to: SHA256SUMS.txt" -ForegroundColor Green
}

# ============================================================================
# Main Logic
# ============================================================================

Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host "AgentMux Artifact Renaming Script" -ForegroundColor Cyan
Write-Host "========================================`n" -ForegroundColor Cyan

# Step 1: Read version and arch
$Version = Get-VersionFromPackageJson
$Arch = Get-Architecture
Write-Host "✓ Detected architecture: $Arch" -ForegroundColor Green

# Step 2: Ensure output directory exists
Ensure-DirectoryExists -Path $DistReleasesDir

# Step 3: Define source and destination paths
$PortableSource = Join-Path $TauriTargetDir "agentmux.exe"
$InstallerSource = Join-Path $TauriTargetDir "bundle" "nsis" "AgentMux_${Version}_${Arch}-setup.exe"

$PortableDest = Join-Path $DistReleasesDir "agentmux-${Version}-${Arch}.exe"
$InstallerDest = Join-Path $DistReleasesDir "AgentMux-${Version}-${Arch}-setup.exe"

# Step 4: Copy and rename portable executable
Write-Host "`n[1/2] Processing portable executable..." -ForegroundColor Yellow
$portableSuccess = Copy-AndRenameArtifact -SourcePath $PortableSource -DestPath $PortableDest

# Step 5: Copy and rename installer
Write-Host "`n[2/2] Processing installer..." -ForegroundColor Yellow
$installerSuccess = Copy-AndRenameArtifact -SourcePath $InstallerSource -DestPath $InstallerDest

# Step 6: Generate checksums
if ($portableSuccess -or $installerSuccess) {
    Generate-Checksums -Directory $DistReleasesDir
}

# Step 7: Summary
Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host "Summary" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

if ($portableSuccess) {
    Write-Host "✓ Portable: agentmux-${Version}-${Arch}.exe" -ForegroundColor Green
} else {
    Write-Host "✗ Portable: FAILED" -ForegroundColor Red
}

if ($installerSuccess) {
    Write-Host "✓ Installer: AgentMux-${Version}-${Arch}-setup.exe" -ForegroundColor Green
} else {
    Write-Host "✗ Installer: FAILED" -ForegroundColor Red
}

Write-Host "`nArtifacts location: $DistReleasesDir" -ForegroundColor Cyan
Write-Host "========================================`n" -ForegroundColor Cyan

# Exit with error code if any artifact failed
if (-not $portableSuccess -or -not $installerSuccess) {
    exit 1
}

exit 0
```

---

### Phase 2: Integrate with Taskfile

**File:** `Taskfile.yml`

```yaml
package:
    desc: Package the application for the current platform (Tauri).
    cmds:
        - task: build:backend
        - task: tauri:copy-sidecars
        - npm run tauri build {{.CLI_ARGS}}
        - pwsh -File scripts/rename-artifacts.ps1  # ← Add this
    deps:
        - clean
        - npm:install
        - docsite:build:embedded
```

---

### Phase 3: Update Bump Version Script

**File:** `bump-version.sh`

Add a reminder to rebuild after version bump:

```bash
# After version bump completes
echo ""
echo "✓ Version bumped to $NEW_VERSION"
echo ""
echo "Next steps:"
echo "  1. task package           # Rebuild with new version"
echo "  2. git push origin main --tags"
echo ""
```

---

### Phase 4: Update .gitignore

**File:** `.gitignore`

```gitignore
# Build artifacts
dist/releases/*.exe
dist/releases/SHA256SUMS.txt
```

---

### Phase 5: Update Documentation

#### A. README.md

Add section:

```markdown
## Building Release Artifacts

### Quick Build

```bash
task package
```

This creates versioned artifacts in `dist/releases/`:
- `agentmux-{version}-{arch}.exe` (portable)
- `AgentMux-{version}-{arch}-setup.exe` (installer)
- `SHA256SUMS.txt` (checksums)

### Version Management

```bash
# Bump version (updates all files)
./bump-version.sh patch --message "Fix layout orphans"

# Rebuild with new version
task package

# Verify artifacts
ls -lh dist/releases/
```
```

#### B. CONTRIBUTING.md

Add release checklist:

```markdown
## Release Process

1. **Bump Version**
   ```bash
   ./bump-version.sh patch --message "Release description"
   ```

2. **Build Artifacts**
   ```bash
   task package
   ```

3. **Verify Artifacts**
   ```bash
   # Check that files are named correctly
   ls dist/releases/

   # Expected:
   # - agentmux-0.25.0-x64.exe
   # - AgentMux-0.25.0-x64-setup.exe
   # - SHA256SUMS.txt
   ```

4. **Create GitHub Release**
   - Upload artifacts from `dist/releases/`
   - Include `SHA256SUMS.txt` in release notes
```

---

## Testing Strategy

### Test 1: Version Detection

**Objective:** Verify script reads version correctly

```powershell
# Test script
pwsh -File scripts/rename-artifacts.ps1

# Expected output:
# ✓ Detected version: 0.25.0
# ✓ Detected architecture: x64
```

**Pass Criteria:**
- Version matches `package.json`
- Architecture detected correctly

---

### Test 2: Portable Rename

**Objective:** Verify portable executable is renamed with version

**Steps:**
1. Build AgentMux: `task package`
2. Check output: `ls dist/releases/`
3. Verify file exists: `agentmux-0.25.0-x64.exe`

**Pass Criteria:**
- File exists in `dist/releases/`
- Filename includes version and arch
- File is executable (not corrupted)

---

### Test 3: Installer Rename

**Objective:** Verify installer is renamed with better format

**Steps:**
1. Build AgentMux: `task package`
2. Check output: `ls dist/releases/`
3. Verify file: `AgentMux-0.25.0-x64-setup.exe`

**Pass Criteria:**
- File exists in `dist/releases/`
- Hyphens instead of underscores
- Filename matches convention

---

### Test 4: Checksum Generation

**Objective:** Verify SHA256SUMS.txt is created

**Steps:**
1. Build AgentMux: `task package`
2. Check file: `cat dist/releases/SHA256SUMS.txt`

**Pass Criteria:**
- File contains both .exe checksums
- Hashes are valid SHA256 (64 hex characters)
- File format matches BSD checksum style

---

### Test 5: Cross-Platform

**Objective:** Verify script works on different systems

**Platforms to Test:**
- Windows 11 x64 ✓
- Windows 11 ARM64 (if available)
- macOS (via Task + bash equivalent)
- Linux (via Task + bash equivalent)

---

### Test 6: Missing Source Files

**Objective:** Verify script handles missing files gracefully

**Steps:**
1. Delete `agentmux.exe` from `target/release/`
2. Run script: `pwsh scripts/rename-artifacts.ps1`

**Pass Criteria:**
- Script shows warning for missing portable
- Script continues and processes installer
- Exit code: 1 (failure)

---

### Test 7: Version Bump Integration

**Objective:** Verify workflow after version bump

**Steps:**
1. Bump version: `./bump-version.sh patch`
2. Build: `task package`
3. Check artifacts: `ls dist/releases/`

**Pass Criteria:**
- Artifacts reflect new version number
- No old version artifacts remain (cleaned)

---

## Rollout Plan

### Stage 1: Development (v0.25.1)

**Checklist:**
- [ ] Create `scripts/rename-artifacts.ps1`
- [ ] Test script manually
- [ ] Integrate with `Taskfile.yml`
- [ ] Test full build process
- [ ] Update `.gitignore`
- [ ] Commit changes

**Timeline:** 2 hours

---

### Stage 2: Documentation (v0.25.1)

**Checklist:**
- [ ] Update `README.md`
- [ ] Update `CONTRIBUTING.md`
- [ ] Update `BUILD.md`
- [ ] Add this spec to `docs/`
- [ ] Create `docs/RELEASE_PROCESS.md`

**Timeline:** 1 hour

---

### Stage 3: Validation (v0.25.1)

**Checklist:**
- [ ] Run all tests from Testing Strategy
- [ ] Build on clean Windows machine
- [ ] Verify checksums match
- [ ] Test portable exe launches correctly
- [ ] Test installer installs correctly
- [ ] Verify multi-version download scenario works

**Timeline:** 2 hours

---

### Stage 4: CI/CD Integration (v0.26.0)

**Checklist:**
- [ ] Add artifact renaming to GitHub Actions
- [ ] Upload versioned artifacts to releases
- [ ] Include SHA256SUMS.txt in release notes
- [ ] Test auto-updater compatibility

**Timeline:** 4 hours (future)

---

## Success Criteria

### Functional Requirements

✅ **Portable Executable:**
- Filename includes version: `agentmux-{version}-{arch}.exe`
- File is executable and launches correctly
- Version matches `package.json`

✅ **Installer:**
- Filename improved: `AgentMux-{version}-{arch}-setup.exe`
- Installs correctly
- Auto-updater compatible

✅ **Automation:**
- Renaming happens automatically after build
- No manual steps required
- Script handles errors gracefully

✅ **Distribution:**
- Artifacts copied to `dist/releases/`
- SHA256 checksums generated
- Multiple versions can coexist

---

### Non-Functional Requirements

✅ **Performance:**
- Post-build script completes in <5 seconds
- No impact on build time

✅ **Reliability:**
- Script always runs after successful build
- Failures don't leave artifacts in inconsistent state

✅ **Maintainability:**
- Script is well-documented
- Easy to modify naming convention
- Version detection is robust

---

## Future Enhancements

### Phase 5: Cross-Platform Support (v0.26.0)

Create bash equivalent: `scripts/rename-artifacts.sh`

```bash
#!/usr/bin/env bash
# Linux/macOS version of rename-artifacts.ps1
```

### Phase 6: Advanced Naming (v0.27.0)

Support additional metadata:

```
agentmux-0.27.0-beta1-x64.exe      # Pre-release
agentmux-0.27.0-nightly-x64.exe    # Nightly builds
agentmux-0.27.0-x64-signed.exe     # Code signed
```

### Phase 7: Auto-Updater Integration (v0.28.0)

Generate updater manifest:

```json
{
  "version": "0.28.0",
  "pub_date": "2026-03-01T00:00:00Z",
  "url": "https://releases.agentmux.dev/agentmux-0.28.0-x64.exe",
  "signature": "...",
  "notes": "Release notes"
}
```

---

## Appendix: File Structure After Implementation

```
agentmux/
├── scripts/
│   ├── rename-artifacts.ps1           # ← New: Post-build renaming
│   ├── rename-artifacts.sh            # ← Future: Linux/macOS version
│   ├── bump-version.sh                # Existing: Version management
│   └── verify-version.sh              # Existing: Version verification
├── dist/
│   └── releases/                      # ← New: Versioned artifacts output
│       ├── agentmux-0.25.0-x64.exe
│       ├── AgentMux-0.25.0-x64-setup.exe
│       └── SHA256SUMS.txt
├── src-tauri/
│   └── target/
│       └── release/
│           ├── agentmux.exe           # Build output (no version)
│           └── bundle/
│               └── nsis/
│                   └── AgentMux_0.25.0_x64-setup.exe
├── docs/
│   ├── VERSIONED_ARTIFACTS_SPEC.md   # ← This file
│   └── RELEASE_PROCESS.md            # ← New: Release guide
├── Taskfile.yml                       # Updated: Calls rename script
└── .gitignore                         # Updated: Ignores dist/releases/
```

---

## Appendix: Tauri Configuration Reference

### Current Configuration

```json
// src-tauri/tauri.conf.json
{
  "productName": "AgentMux",
  "version": "0.25.0",
  "identifier": "com.a5af.agentmux",
  "build": {
    "devUrl": "http://localhost:5173",
    "frontendDist": "../dist/frontend",
    "beforeDevCommand": "npx vite --config vite.config.tauri.ts",
    "beforeBuildCommand": "npx vite build --config vite.config.tauri.ts"
  },
  "bundle": {
    "active": true,
    "targets": ["nsis"],
    "externalBin": [
      "binaries/agentmuxsrv",
      "binaries/wsh"
    ]
  }
}
```

### No Changes Needed

The Tauri configuration **does not need to change**. The versioning is handled post-build.

---

## Appendix: Comparison Matrix

| Aspect | Before | After |
|--------|--------|-------|
| **Portable Filename** | `agentmux.exe` | `agentmux-0.25.0-x64.exe` |
| **Installer Filename** | `AgentMux_0.25.0_x64-setup.exe` | `AgentMux-0.25.0-x64-setup.exe` |
| **Naming Convention** | Inconsistent (underscore vs hyphen) | Consistent (all hyphens) |
| **Version Clarity** | ❌ Portable unclear | ✅ Both clear |
| **Architecture Info** | ✅ Both have arch | ✅ Both have arch |
| **Multi-version Downloads** | ❌ Files overwrite | ✅ Coexist peacefully |
| **Professional Image** | ⚠️ Inconsistent | ✅ Polished |
| **Automation** | ❌ Manual renaming needed | ✅ Fully automatic |

---

## References

- [Tauri Configuration Reference](https://v2.tauri.app/reference/config/)
- [Tauri Windows Installer Guide](https://v2.tauri.app/distribute/windows-installer/)
- [Tauri NSIS Bundler Source](https://github.com/tauri-apps/tauri/blob/dev/tooling/bundler/src/bundle/windows/nsis.rs)
- [Semantic Versioning](https://semver.org/)
- [Windows File Naming Conventions](https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file)

---

**Status:** Ready for Implementation
**Next Steps:** Create PR with Phase 1 implementation
