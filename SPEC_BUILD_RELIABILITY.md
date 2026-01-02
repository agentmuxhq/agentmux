# WaveMux Build Reliability Specification

**Date:** 2026-01-02
**Status:** Draft
**Author:** agent2

---

## Problem Statement

The current build process is fragile and fails unpredictably:

1. **Stale artifacts** - Old files in `make/`, `dist/`, `.task/` cause conflicts
2. **Version mismatches** - Backend binaries built with wrong version string
3. **Missing dependencies** - zod/v4, sharp, docusaurus tsconfig errors
4. **Lock files** - wave.lock, wave.sock left by crashed processes
5. **Task caching** - Taskfile marks builds "up to date" when they're not
6. **Multiple build paths** - `task dev`, `task package`, manual portable copy all differ

---

## Current Build Components

| Component | Build Command | Output | Issues |
|-----------|--------------|--------|--------|
| Go backend (wavemuxsrv) | `task build:backend` | `dist/bin/wavemuxsrv.x64.exe` | Version from package.json, task caching |
| Go wsh binaries | `task build:backend` | `dist/bin/wsh-{version}-*` | 8 platform variants |
| TypeScript frontend | `npm run build:prod` | `dist/frontend/` | sharp warnings (non-fatal) |
| Electron main | `npm run build:prod` | `dist/main/` | docs/tsconfig warnings (non-fatal) |
| Electron preload | `npm run build:prod` | `dist/preload/` | - |
| Schema | `task build:backend` | `dist/schema/` | - |
| Docsite | `task docsite:build:embedded` | `dist/docsite/` | Requires docs npm install |

---

## Proposed Solution: Single Reliable Build Script

### Prerequisites

```powershell
# Clean slate - remove ALL stale artifacts
Remove-Item -Recurse -Force -ErrorAction SilentlyContinue make, .task
Remove-Item -Force -ErrorAction SilentlyContinue dist/bin/*

# Kill any running WaveMux processes
taskkill /F /IM WaveMux.exe 2>$null
taskkill /F /IM wavemuxsrv.x64.exe 2>$null

# Remove stale locks
Remove-Item -Force -ErrorAction SilentlyContinue node_modules/electron/dist/wave-data/wave.lock
Remove-Item -Force -ErrorAction SilentlyContinue node_modules/electron/dist/wave-data/wave.sock
```

### Build Steps (In Order)

```powershell
# 1. Ensure dependencies
npm install --legacy-peer-deps

# 2. Build backend (forces rebuild via cache clear)
Remove-Item -Recurse -Force -ErrorAction SilentlyContinue .task
task build:backend

# 3. Build frontend
npm run build:prod

# 4. Package (electron-builder handles icon, signing, etc.)
npm exec electron-builder -- -c electron-builder.config.cjs -p never --win dir
```

### Output

- `make/win-unpacked/WaveMux.exe` - Properly packaged with green icon

---

## Simplification Recommendations

### 1. Remove docs from main build

The docs workspace causes constant issues:
- `docs/tsconfig.json` extends `@docusaurus/tsconfig` (not installed in root)
- `docs/package.json` has React 18 vs root React 19 conflict
- Docsite is optional for development

**Action:** Add `--skip-docs` option or separate docs build entirely.

### 2. Pin zod version

The ai-sdk packages require zod ^3.24 with subpath exports:
```json
"zod": "^3.24.0"
```

**Action:** Update package.json and lock file.

### 3. Create single build script

Instead of multiple task commands, create:

```powershell
# scripts/build-release.ps1
param(
    [switch]$Clean,
    [switch]$SkipBackend,
    [switch]$SkipFrontend
)

$ErrorActionPreference = "Stop"
$Version = (Get-Content package.json | ConvertFrom-Json).version

Write-Host "Building WaveMux v$Version" -ForegroundColor Green

if ($Clean) {
    Write-Host "Cleaning..." -ForegroundColor Yellow
    Remove-Item -Recurse -Force -ErrorAction SilentlyContinue make, .task, dist/bin
    taskkill /F /IM WaveMux.exe 2>$null
    taskkill /F /IM wavemuxsrv.x64.exe 2>$null
}

if (-not $SkipBackend) {
    Write-Host "Building backend..." -ForegroundColor Yellow
    task build:backend
    if ($LASTEXITCODE -ne 0) { throw "Backend build failed" }
}

if (-not $SkipFrontend) {
    Write-Host "Building frontend..." -ForegroundColor Yellow
    npm run build:prod
    if ($LASTEXITCODE -ne 0) { throw "Frontend build failed" }
}

Write-Host "Packaging..." -ForegroundColor Yellow
npm exec electron-builder -- -c electron-builder.config.cjs -p never --win dir
if ($LASTEXITCODE -ne 0) { throw "Packaging failed" }

Write-Host "Success! Output: make/win-unpacked/WaveMux.exe" -ForegroundColor Green
```

### 4. Version bump automation

```powershell
# scripts/bump-and-build.ps1
npm version patch --no-git-tag-version
./scripts/build-release.ps1 -Clean
```

---

## Deployment to Desktop

After successful build:

```powershell
$Version = (Get-Content package.json | ConvertFrom-Json).version
Copy-Item -Recurse -Force make/win-unpacked "C:\Users\asafe\Desktop\WaveMux-$Version"
```

---

## Known Issues to Fix

| Issue | Priority | Solution |
|-------|----------|----------|
| docs/tsconfig.json errors | P1 | Exclude docs from vite-tsconfig-paths |
| zod/v4 import errors | P1 | Pin zod to ^3.24.0 |
| sharp not found | P3 | Optional - just image optimization |
| wave.sock permission denied | P1 | Clean before build |
| Task caching incorrect | P2 | Clear .task before version bump |

---

## Success Criteria

A reliable build means:

1. **Single command** - `./scripts/build-release.ps1 -Clean` produces working exe
2. **Reproducible** - Same inputs = same outputs
3. **Self-cleaning** - Handles stale artifacts automatically
4. **Clear errors** - Fails fast with actionable messages
5. **Correct icon** - Green WaveMux icon, not Electron default

---

## Next Steps

1. [ ] Create `scripts/build-release.ps1`
2. [ ] Fix zod version in package.json
3. [ ] Exclude docs from tsconfig-paths plugin
4. [ ] Test full clean build on gamerlove
5. [ ] Update BUILD.md with simplified instructions
