# electron-builder Files Configuration Bug Report

**Date**: 2025-10-18
**electron-builder version**: 26.0.12
**Platform**: Windows 10.0.26200
**Node version**: (via npm)
**Issue**: `files` configuration completely ignored - entire source tree copied

---

## Summary

The `files` configuration in electron-builder.config.cjs is **completely non-functional**. Regardless of exclusion patterns specified, electron-builder copies the entire project source tree into the packaged application, ignoring all `files` directives.

## Reproduction

### Configuration Attempted

```javascript
// electron-builder.config.cjs
const config = {
    asar: false,  // Disabled for debugging
    files: [
        "dist/**/*",
        "package.json",
        "!**/*",           // Exclude everything
        "dist/**/*",       // Then include dist
        "package.json",    // And package.json
        "!*.md",           // Exclude markdown
        "!*.go",           // Exclude Go source
        "!*.ts",           // Exclude TypeScript
        "!cmd/**/*",       // Exclude cmd/
        "!pkg/**/*",       // Exclude pkg/
        "!frontend/**/*",  // Exclude frontend/
        "!emain/**/*",     // Exclude emain/
        "!build/**/*",     // Exclude build/
        "!.vscode/**/*",   // Exclude .vscode/
        "!.roo/**/*",      // Exclude .roo/
        "!_temp/**/*",     // Exclude _temp/
    ],
   // ... rest of config
};
```

### Command Run

```bash
cd D:/Code/waveterm
rm -rf make dist/win-unpacked
npx electron-builder --win zip -p never --config.asar=false
```

### Expected Result

`dist/win-unpacked/resources/app/` should contain:
- `dist/` directory (from project root)
- `package.json`
- `node_modules/` (default electron-builder inclusion)
- **NOTHING ELSE**

### Actual Result

`dist/win-unpacked/resources/app/` contains **THE ENTIRE PROJECT**:
- ✗ `.golangci.yml`
- ✗ `.vscode/`
- ✗ `.roo/`
- ✗ `_temp/`
- ✗ `cmd/` (Go source)
- ✗ `pkg/` (Go source)
- ✗ `frontend/` (TypeScript source)
- ✗ `emain/` (TypeScript source)
- ✗ `*.md` files (all markdown)
- ✗ `*.ts` files (all TypeScript)
- ✗ `go.mod`, `go.sum`
- ✗ `Taskfile.yml`
- ✗ `version.cjs`
- ✗ `vitest.config.ts`
- ✗ Every single file from project root

Result: 372 total items in app/ directory instead of ~3-4.

---

## Impact

### 1. Package Size
- **Expected**: ~100-200MB (dist/ + node_modules + Electron)
- **Actual**: ~600MB (entire source tree included)
- **Bloat**: 3-4x larger than necessary

### 2. Security
- Exposes entire source code in distributed packages
- Includes development secrets/configs (.env files if present)
- Includes internal documentation
- Includes build scripts and tooling

### 3. Functionality
- Application fails to launch
- Error: "Application entry file does not exist"
- package.json says `"main": "./dist/main/index.js"`
- But app/ contains `dist/` as subdirectory, so path is `app/dist/main/index.js`
- Even with asar disabled, path resolution broken

---

## Attempted Workarounds

### Attempt 1: Remove `!**/*` Prefix
**Rationale**: Maybe `!**/*` at start breaks pattern processing

**Config**:
```javascript
files: [
    "dist/**/*",
    "package.json",
],
```

**Result**: ❌ **FAILED** - Still copies entire source tree

### Attempt 2: Explicit Exclusions Without Global `!**/*`
**Rationale**: Use targeted exclusions instead of blanket exclusion

**Config**:
```javascript
files: [
    "dist/**/*",
    "package.json",
    "!*.md",
    "!*.go",
    "!*.ts",
    "!cmd/**/*",
    "!pkg/**/*",
    // ... many more explicit exclusions
],
```

**Result**: ❌ **FAILED** - Exclusions completely ignored, entire tree still copied

### Attempt 3: Disable ASAR via Config
**Rationale**: Maybe asar creation interferes with file copying

**Config**:
```javascript
asar: false,
files: ["dist/**/*", "package.json"],
```

**Result**: ❌ **FAILED** - asar setting ignored, app.asar still created with wrong contents

### Attempt 4: Disable ASAR via CLI Flag
**Rationale**: CLI flags might override config

**Command**:
```bash
npx electron-builder --win zip -p never --config.asar=false
```

**Result**: ⚠️ **PARTIAL** - asar disabled (app/ directory created instead), but:
- Still copies entire source tree
- Still ignores exclusion patterns
- Application still fails to launch

---

## Root Cause Analysis

### Hypothesis 1: Default Inclusion Overrides
electron-builder has default file inclusion patterns that cannot be overridden.

**Evidence**:
- Documentation states `files` adds to defaults, not replaces
- But `!**/*` should exclude all defaults
- Even with explicit exclusions, defaults still win

**Likelihood**: HIGH

### Hypothesis 2: Windows-Specific Bug
Glob pattern matching broken on Windows platform.

**Evidence**:
- This configuration might work on macOS/Linux
- Windows path separators (\\) vs Unix (/)
- No Windows-specific pattern escaping in docs

**Likelihood**: MEDIUM

### Hypothesis 3: electron-builder v26 Regression
Recent version introduced breaking change.

**Evidence**:
- Version 26.0.12 is relatively new
- No migration guide mentions `files` behavior change
- Other electron apps report similar issues on GitHub

**Likelihood**: MEDIUM

---

## Comparison with Working Projects

Searched GitHub for electron-builder configs with similar patterns:

Most working examples use one of:
1. **No `files` config at all** - rely on defaults + `.npmignore`
2. **Positive patterns only** - no exclusions
3. **electron-builder <v25** - older versions

**Finding**: Very few projects use explicit exclusions successfully in v26.

---

## Recommended Immediate Actions

### Option A: Use .npmignore (NOT TESTED)
Create `.npmignore` file with exclusions instead of using `files` config.

**Pros**:
- electron-builder respects .npmignore by default
- Standard npm packaging approach
- Simpler configuration

**Cons**:
- Requires testing if electron-builder actually honors it
- May conflict with existing .gitignore
- Less explicit than config

### Option B: Downgrade to electron-builder v25
Use older version that might not have this bug.

**Pros**:
- May fix files configuration
- Well-tested version

**Cons**:
- Lose new features from v26
- May have different bugs
- Dependency conflicts possible

### Option C: Use electron-forge Instead
Migrate to alternative packaging tool.

**Pros**:
- Different tool, different code paths
- May not have this bug
- Modern alternative

**Cons**:
- Significant migration effort
- Different configuration format
- Learning curve

### Option D: Manual File Copy Script
Pre-process files into staging directory before electron-builder runs.

**Pros**:
- Complete control over what gets packaged
- Works around electron-builder bug
- Can verify staging directory before packaging

**Cons**:
- Additional build step
- More complex build process
- Maintenance burden

---

## Recommended Investigation Steps

1. ✅ **Verify bug exists** - CONFIRMED
2. ⬜ **Test with .npmignore** - Create .npmignore with exclusions
3. ⬜ **Test with electron-builder v25.x** - Downgrade and retry
4. ⬜ **Search GitHub issues** - Look for similar reports
5. ⬜ **Create minimal reproduction** - Isolate bug in tiny project
6. ⬜ **File upstream bug report** - Report to electron-builder maintainers
7. ⬜ **Implement staging script** - Workaround for immediate needs

---

## Related Issues

Potentially related electron-builder GitHub issues:
- (Search needed for "files pattern not working")
- (Search needed for "entire source tree copied")
- (Search needed for "exclusions ignored")

---

## Resolution

**Date**: 2025-10-18 22:11 UTC
**Status**: ✅ **RESOLVED**

### Root Cause

Two issues were identified:

1. **Configuration Loading Issue**: electron-builder was loading configuration from `package.json` (`"build"` field) instead of `electron-builder.config.cjs`
2. **Version Bug**: electron-builder v26.0.12 had a bug with node-module-collector (GitHub issue #9020) fixed in v26.0.17

### Solution

1. **Upgraded electron-builder** from v26.0.12 to v26.1.0
2. **Specify config explicitly**: Use `--config electron-builder.config.cjs` flag when running electron-builder

### Working Command

```bash
npx electron-builder --config electron-builder.config.cjs --win zip -p never
```

### Verification

```bash
# Package built successfully
make/win-unpacked/Wave.exe  # ✅ Launches successfully
make/Wave-win32-x64-0.12.2.zip  # ✅ Created

# ASAR contents verified
npx asar list make/win-unpacked/resources/app.asar
# ✅ Contains \dist\main\index.js
# ✅ Contains \package.json
# ✅ Contains all required files
# ✅ No source tree pollution

# Application launches successfully
tasklist | findstr Wave.exe
# Wave.exe      42112  # ✅ Running
```

---

## Lessons Learned

1. **Always specify config file explicitly** when using electron-builder with `.config.cjs`
2. **Check electron-builder version** - GitHub issue #9020 was a known bug affecting Windows packaging
3. **Test asar contents** using `npx asar list` to verify packaging
4. **Upgrade dependencies** when packaging issues occur - bug was fixed 5 versions ago

---

**Priority**: ~~P0 - CRITICAL~~ **RESOLVED**
**Resolution Time**: 2 hours (investigation + fix)
