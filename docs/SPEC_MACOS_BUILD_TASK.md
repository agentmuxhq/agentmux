# Specification: macOS Build Task

**Author:** Claude
**Date:** 2026-02-14
**Status:** Draft
**Related:** PR #283 (Build System Standardization)

---

## Problem

Currently, `task package` builds using targets defined in `tauri.conf.json`:
```json
"targets": ["nsis"]  // Windows NSIS installer only
```

This works for Windows but **does not produce macOS bundles** (.app, .dmg). When run on macOS, Tauri builds the binary but skips bundling.

### Current Behavior on macOS
```bash
task package
# ✅ Builds backend binaries
# ✅ Builds frontend
# ✅ Compiles Tauri app
# ❌ Does NOT create .app bundle
# ❌ Does NOT create .dmg installer
```

### Why We Can't Just Change tauri.conf.json

The `targets` array in `tauri.conf.json` is **platform-agnostic**. Setting it to `["dmg", "app"]` would break Windows builds (and vice versa).

**Multi-platform repository requires platform-specific build tasks.**

---

## Solution

Add a new **platform-specific task** to `Taskfile.yml`:

```yaml
package:macos:
  desc: Package the application for macOS (creates .app and .dmg)
  platforms: [darwin]
  cmds:
    - task: build:backend
    - task: tauri:copy-sidecars
    - npx tauri build --bundles dmg,app {{.CLI_ARGS}}
  deps:
    - clean
    - npm:install
    - docsite:build:embedded
```

### Key Design Decisions

1. **Platform restriction**: `platforms: [darwin]` prevents accidental runs on Windows/Linux
2. **Bundle override**: `--bundles dmg,app` overrides `tauri.conf.json` targets via CLI
3. **Reuse dependencies**: Same `deps` as `package` task (clean, npm install, docs)
4. **Reuse build steps**: Same `build:backend` and `tauri:copy-sidecars` steps
5. **CLI args passthrough**: `{{.CLI_ARGS}}` allows `task package:macos -- --debug`

---

## Implementation Plan

### Step 1: Add Task to Taskfile.yml

Location: After existing `package` task (around line 150)

```yaml
    package:
        desc: Package the application for the current platform (Tauri).
        cmds:
            - task: build:backend
            - task: tauri:copy-sidecars
            - npx tauri build {{.CLI_ARGS}}
        deps:
            - clean
            - npm:install
            - docsite:build:embedded

    package:macos:
        desc: Package the application for macOS (creates .app and .dmg)
        platforms: [darwin]
        cmds:
            - task: build:backend
            - task: tauri:copy-sidecars
            - npx tauri build --bundles dmg,app {{.CLI_ARGS}}
        deps:
            - clean
            - npm:install
            - docsite:build:embedded
```

### Step 2: Update Documentation

**README.md** - Add macOS build instructions:
```markdown
### Building

**Development:**
```bash
task dev
```

**Production Installer:**
```bash
# Windows (NSIS installer)
task package

# macOS (.app and .dmg)
task package:macos
```

### Step 3: Verification

Test that the task:
1. ✅ Only runs on macOS (`platforms: [darwin]`)
2. ✅ Builds backend binaries correctly
3. ✅ Copies sidecars to Tauri binaries folder
4. ✅ Creates .app bundle at `src-tauri/target/release/bundle/macos/AgentMux.app`
5. ✅ Creates .dmg installer at `src-tauri/target/release/bundle/dmg/AgentMux_X.X.X_aarch64.dmg`

---

## Testing Plan

### Test 1: Clean Build
```bash
task clean
task package:macos
# Expected: .app and .dmg created successfully
```

### Test 2: Platform Restriction
```bash
# On Windows/Linux (if testing in VM):
task package:macos
# Expected: Error "task is not available on this platform"
```

### Test 3: Bundle Verification
```bash
task package:macos
ls -la src-tauri/target/release/bundle/macos/AgentMux.app
ls -la src-tauri/target/release/bundle/dmg/*.dmg
# Expected: Both exist and are valid
```

### Test 4: Installation Test
```bash
task package:macos
open src-tauri/target/release/bundle/dmg/AgentMux_*.dmg
# Manually:
# 1. Drag AgentMux.app to Applications
# 2. Launch from Applications
# 3. Verify backend starts correctly
# 4. Verify UI loads (no grey screen)
# 5. Verify multi-window support works
```

### Test 5: CLI Args Passthrough
```bash
task package:macos -- --debug
# Expected: Creates debug bundles with additional logging
```

---

## Alternative Approaches Considered

### Alternative 1: Conditional Logic in Existing Task
```yaml
package:
  cmds:
    - sh: |
        if [ "$(uname)" == "Darwin" ]; then
          npx tauri build --bundles dmg,app
        else
          npx tauri build
        fi
```
**Rejected:** Less discoverable, harder to test, platform detection fragile

### Alternative 2: Separate Config Files
Create `tauri.conf.macos.json` and `tauri.conf.windows.json`

**Rejected:**
- Duplicates most of config
- Harder to maintain consistency
- Requires `--config` flag on every build

### Alternative 3: Dynamic Config Override
```yaml
package:macos:
  cmds:
    - npx tauri build --config '{"bundle":{"targets":["dmg","app"]}}'
```
**Rejected:** JSON in YAML is ugly, error-prone, hard to read

---

## Future Considerations

### Linux Support
Add `package:linux` if needed:
```yaml
package:linux:
  desc: Package the application for Linux (creates .deb and .AppImage)
  platforms: [linux]
  cmds:
    - npx tauri build --bundles deb,appimage {{.CLI_ARGS}}
```

### Universal Binary (macOS)
For Apple Silicon + Intel universal binary:
```yaml
package:macos:universal:
  desc: Package universal macOS binary (arm64 + x86_64)
  platforms: [darwin]
  cmds:
    - npx tauri build --target universal-apple-darwin --bundles dmg,app
```

### CI/CD Integration
GitHub Actions can use platform-specific tasks:
```yaml
- name: Build (macOS)
  if: runner.os == 'macOS'
  run: task package:macos

- name: Build (Windows)
  if: runner.os == 'Windows'
  run: task package
```

---

## Success Criteria

✅ `task package:macos` creates working .app bundle
✅ `task package:macos` creates working .dmg installer
✅ Task only runs on macOS (enforced by `platforms: [darwin]`)
✅ DMG installs and launches without grey screen
✅ Backend starts and connects properly (multi-window architecture)
✅ Documentation updated in README.md
✅ No changes to `tauri.conf.json` (remains platform-agnostic)
✅ No breaking changes to existing `task package` (Windows builds unaffected)

---

## Implementation Checklist

- [ ] Add `package:macos` task to Taskfile.yml
- [ ] Update README.md with macOS build instructions
- [ ] Test clean build creates .app and .dmg
- [ ] Test .dmg installation and launch
- [ ] Verify backend connectivity (no grey screen)
- [ ] Test multi-window support works
- [ ] Verify `task package` still works on Windows (no regression)
- [ ] Update BUILD.md if it exists
- [ ] Optional: Add to CI/CD pipeline

---

## References

- PR #283: Build System Standardization
- PR #290: Multi-Window Shared Backend
- Tauri CLI docs: https://tauri.app/v1/guides/building/
- Task docs: https://taskfile.dev/usage/
