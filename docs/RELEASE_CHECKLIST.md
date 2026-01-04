# Release Checklist

**CRITICAL:** Follow this checklist in order to prevent releasing old code or broken builds.

## Pre-Release (Development)

### ✅ Step 1: Fix All Bugs FIRST
```bash
# Make all necessary fixes
git add <files>
git commit -m "fix: Description of all fixes"

# Verify all changes are committed
git status  # Should show "nothing to commit, working tree clean"
```

**⚠️ DO NOT bump version until ALL fixes are committed!**

### ✅ Step 2: Run All Tests
```bash
# Run full test suite
npm test

# Run version consistency tests specifically
npm test -- version.test.ts

# Check test results - ALL must pass
```

### ✅ Step 3: Bump Version
```bash
# Only bump AFTER all fixes are committed
./bump-version.sh patch --message "Description of changes"

# The script will:
# - Update package.json and package-lock.json
# - Update VERSION_HISTORY.md
# - Create a commit with the version bump
# - Create a git tag
#
# DO NOT:
# - Commit more fixes after this
# - Build before verifying git status
```

### ✅ Step 4: Verify Git State
```bash
# Check that version bump commit is the latest
git log --oneline -3

# Ensure no uncommitted changes
git status  # Must show "nothing to commit, working tree clean"

# Verify all fixes are in the version bump commit or earlier
```

---

## Build Phase

### ✅ Step 5: Clean Build Environment
```bash
# Remove old binaries
rm -rf dist/bin/wsh-*.exe make/

# Rebuild backend with new version
task build:backend

# Verify binary versions match
ls -lh dist/bin/wsh-*.exe
# Should show files like: wsh-0.12.14-windows.x64.exe
```

### ✅ Step 6: Build Package
```bash
# Build portable package
npx electron-builder --config electron-builder.config.cjs --win zip -p never

# Wait for build to complete (takes ~2-3 minutes)
```

### ✅ Step 7: Verify Package Contents
```bash
# Check wsh binaries are in the package
unzip -l make/Wave-win32-x64-*.zip | grep wsh

# Should show:
# - wsh-{VERSION}-windows.x64.exe
# - wsh-{VERSION}-windows.arm64.exe

# Run package verification
bash scripts/verify-package.sh

# All checks must pass ✅
```

---

## Release Phase

### ✅ Step 8: Push Changes
```bash
# Push branch and tags to GitHub
git push origin <branch-name> --tags

# Verify push succeeded
git log origin/<branch-name> --oneline -3
```

### ✅ Step 9: Create GitHub Release
```bash
# Get GitHub App token
export GH_TOKEN=$(bash ~/.config/gh/github-apps/generate-token-a5af.sh)

# Create release with package
gh release create v{VERSION}-fork \
  --repo a5af/waveterm \
  --title "Wave Terminal v{VERSION}-fork - Title" \
  --notes-file release-notes.md \
  make/Wave-win32-x64-{VERSION}.zip

# Get release URL
```

---

## Post-Release Verification

### ✅ Step 10: Test Downloaded Package
```bash
# Download from GitHub releases
# Extract to a test location
# Run Wave.exe

# Verify:
# 1. Title bar shows correct version
# 2. No "undefined" in instance label
# 3. Shell integration works (wsh commands)
```

### ✅ Step 11: Document Issues
If problems found:
1. DO NOT delete the release
2. Document the issue
3. Start over from Step 1 with a NEW version number
4. Never reuse a version number

---

## Common Mistakes to Avoid

### ❌ NEVER Do This
1. **Bump version before committing fixes**
   ```bash
   # ❌ WRONG
   ./bump-version.sh patch
   git add fixes.ts
   git commit -m "fix bug"
   ```

2. **Release with uncommitted changes**
   ```bash
   # ❌ WRONG
   ./bump-version.sh patch
   # ... make more changes ...
   # ... build and release without committing
   ```

3. **Skip package verification**
   ```bash
   # ❌ WRONG
   npm run build
   # ... immediately create release without verification
   ```

4. **Build with old binaries**
   ```bash
   # ❌ WRONG
   ./bump-version.sh patch
   # ... skip task build:backend ...
   npm run build  # Uses old wsh binaries!
   ```

### ✅ ALWAYS Do This
1. **Commit all fixes FIRST, then bump version**
   ```bash
   # ✅ CORRECT
   git add fixes.ts
   git commit -m "fix bug"
   ./bump-version.sh patch
   ```

2. **Verify git state before building**
   ```bash
   # ✅ CORRECT
   ./bump-version.sh patch
   git status  # Must be clean
   git log -1  # Must show version bump commit
   ```

3. **Rebuild binaries after version bump**
   ```bash
   # ✅ CORRECT
   ./bump-version.sh patch
   rm -rf dist/bin/wsh-*.exe
   task build:backend
   ```

4. **Test package before releasing**
   ```bash
   # ✅ CORRECT
   npx electron-builder ...
   bash scripts/verify-package.sh
   # ... extract and test manually ...
   # ... THEN create release
   ```

---

## Version History Tracking

After each release, update `VERSION_HISTORY.md` to include:
- What was fixed
- What was tested
- Any known issues
- Links to commits and PRs

This helps future agents understand what changed and why.
