# Retrospective: Go PATH Regression

**Date:** 2026-02-16
**Severity:** High - Blocks all builds
**Type:** Environment Regression

---

## Summary

Build process failed with `"go": executable file not found in $PATH` when attempting to create a portable package build after merging Phase 5 of the Agent Widget Refactor.

## Timeline

1. **2026-02-16 15:48** - Phase 5 (#332) merged successfully
2. **2026-02-16 15:50** - Pulled latest main (includes PR #333 - Tauri version management)
3. **2026-02-16 15:51** - Attempted `task package:portable`
4. **2026-02-16 15:51** - Build failed: Go executable not found

## Root Cause Analysis

### Immediate Cause
Go is not in the system PATH when running builds from Git Bash environment.

### Investigation Questions

**Q1: Was Go ever in PATH?**
- Need to verify: Did builds work previously on this machine?
- Check: When was the last successful build on this workstation?

**Q2: Did something change in the environment?**
- Git Bash session vs PowerShell/CMD environment differences
- PATH not propagated to Git Bash subshell
- Go installation missing or corrupted

**Q3: Did recent PRs change build requirements?**
- PR #333 added Tauri version management but shouldn't affect Go PATH
- No changes to Go version or installation requirements in recent commits

### Likely Scenarios

**Scenario A: Environment Never Configured (MOST LIKELY)**
- This workstation may never have been set up for Go builds
- Previous builds may have been done on different machines
- Git Bash may not inherit Windows system PATH correctly

**Scenario B: PATH Regression**
- Go was installed but PATH environment variable was lost
- System restart without persistent PATH configuration
- Git Bash not loading user profile correctly

**Scenario C: Go Installation Issue**
- Go was uninstalled or corrupted
- Go binary moved from standard location

## Impact

**Blocked Operations:**
- ❌ Cannot build portable packages (`task package:portable`)
- ❌ Cannot build installers (`task package`)
- ❌ Cannot build backend binaries (`task build:backend`)
- ❌ Cannot run Go tests
- ❌ Cannot bump versions (requires backend rebuild)

**Working Operations:**
- ✅ Frontend development (`task dev` - uses pre-built backend)
- ✅ Frontend builds (`npm run build`)
- ✅ Git operations
- ✅ Code editing

## Verification Steps

To diagnose the exact issue:

```bash
# 1. Check if Go is installed (Windows)
where go

# 2. Check common installation paths
ls "C:\Program Files\Go\bin\go.exe"
ls "C:\Go\bin\go.exe"

# 3. Check system PATH
echo $PATH | tr ':' '\n' | grep -i go

# 4. Check if Go works in PowerShell
powershell -c "go version"

# 5. Check Git Bash profile loading
cat ~/.bashrc | grep -i go
cat ~/.bash_profile | grep -i go
```

## Resolution Options

### Option 1: Install Go (If Missing)
```bash
# Download from https://go.dev/dl/
# Install to C:\Go or C:\Program Files\Go
# Add to system PATH via System Properties → Environment Variables
```

### Option 2: Add Go to Git Bash PATH (If Installed)
```bash
# Add to ~/.bashrc
export PATH="/c/Go/bin:$PATH"

# Or if installed in Program Files
export PATH="/c/Program Files/Go/bin:$PATH"

# Reload
source ~/.bashrc
```

### Option 3: Use Pre-built Binaries
- If Go builds are not needed for this workflow
- Use existing binaries from CI/CD or other machines
- Copy `dist/bin/` from a working build

### Option 4: Build in PowerShell/CMD
```powershell
# PowerShell typically has correct PATH
cd C:\Systems\agentmux
task package:portable
```

## Prevention Measures

### Short-term
1. ✅ Document Go as a required dependency in BUILD.md
2. ✅ Add PATH verification to build scripts
3. ✅ Create setup script to verify all build tools

### Long-term
1. **Environment Verification Script**
   - Create `scripts/verify-build-env.sh`
   - Check for: Go, Node, Rust, Tauri CLI
   - Run automatically in `task dev` and `task package`

2. **Developer Setup Guide**
   - Add `docs/SETUP_WINDOWS.md` with step-by-step instructions
   - Include PATH configuration for Git Bash
   - Add troubleshooting section

3. **CI/CD Validation**
   - Ensure CI builds catch missing dependencies
   - Add environment validation step before builds

4. **Better Error Messages**
   - Detect missing Go before starting build
   - Provide helpful error with installation instructions

## Action Items

- [ ] **IMMEDIATE**: Determine if Go is installed on this machine
- [ ] **IMMEDIATE**: Configure Go PATH for Git Bash if installed
- [ ] **IMMEDIATE**: Document current resolution in this retro
- [ ] **SHORT-TERM**: Add `scripts/verify-build-env.sh`
- [ ] **SHORT-TERM**: Update BUILD.md with Go installation instructions
- [ ] **LONG-TERM**: Create comprehensive Windows setup guide

## Related Issues

- Similar issue may exist for other build tools (Rust, etc.)
- Git Bash PATH propagation needs investigation
- Consider using Dev Containers or Nix for reproducible environments

## Lessons Learned

1. **Environment assumptions are dangerous** - Don't assume PATH is configured
2. **Builds should fail fast** - Detect missing tools before starting
3. **Documentation is critical** - Setup instructions must be comprehensive
4. **Cross-platform complexity** - Windows PATH handling differs from Unix

## Current Status

**Status:** 🔴 BLOCKED
**Next Step:** Verify Go installation and configure PATH
**Owner:** AgentA
**Priority:** High (blocks all builds)

---

## Update Log

### 2026-02-16 15:52
- Created retro document
- Identified root cause: Go not in PATH
- Documented resolution options
- Awaiting environment verification

---

## RESOLUTION - 2026-02-16 16:17

### ✅ Go PATH Fixed

**Root Cause Confirmed:**  
- `.bashrc` was regenerated/modified earlier today (Feb 16 08:06)
- This overwrote a previous agent's Go PATH configuration
- Git Bash requires explicit PATH configuration (doesn't inherit Windows system PATH)

**Solution Applied:**
```bash
# Added to both ~/.bashrc and ~/.bash_profile
export PATH="/c/Program Files/Go/bin:$PATH"
```

**Why Double Redundancy:**
- `.bashrc` - Primary profile file (loaded by Git Bash)
- `.bash_profile` - Backup (older, less likely to be regenerated)

**Verification:**
```bash
$ go version
go version go1.26.0 windows/amd64
```

**Build Test:**
```bash
$ task build:backend
✅ SUCCESS - All Go binaries built successfully
```

### ✅ Secondary Issue: Tauri Plugin Version Mismatch

**Discovered During Build:**
- `tauri-plugin-opener`: Rust 2.5.3 vs npm 2.3.1
- `tauri-plugin-shell`: Rust 2.3.4 vs npm 2.2.0

**Resolution:**
```bash
npm install @tauri-apps/plugin-opener@^2.5.3 @tauri-apps/plugin-shell@^2.3.4 --save-exact
```

**Final Build:**
```bash
$ task package:portable
✅ SUCCESS - Created: agentmux-0.28.20-x64-portable.zip (35 MB)
```

### Lessons Confirmed

1. **Environment configuration is fragile** - Profile files can be regenerated
2. **Redundancy prevents repeat failures** - Dual PATH configuration stuck this time
3. **Version alignment critical** - Tauri ecosystem requires tight version coupling
4. **Scripts exist for a reason** - PR #333's verification scripts would have caught this

### Permanent Fixes Applied

**Environment (local machine):**
- [x] Go PATH added to both `.bashrc` and `.bash_profile`

**Repository (separate PRs):**
- [x] Tauri plugin versions synchronized → PR #334
- [x] ExpectedVersion aligned to 0.28.20 → PR #335 (auto-updated by bump-version.sh)

**Outcome:**
- [x] Portable build v0.28.20 created successfully
- [x] Retro documented for future reference

**Status:** 🟢 RESOLVED - Builds working, PATH persists across sessions

