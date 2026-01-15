# WaveMux Version History

This document tracks the version history of the a5af/wavemux fork (formerly a5af/waveterm).

## Latest Version: 0.15.8

**Base:** Upstream waveterm v0.12.0 + extensive custom features

---

## Version History (Latest First)

### v0.15.8 (2026-01-15)
- **Agent:** AgentA
- **Changes:**
  - Fix: disable hostname-based agent detection for local terminals (#127)
  - Local terminals no longer auto-detect agent from hostname patterns
  - SSH connections still use hostname-based detection
  - Explicit `agent-workspaces` directory pattern works for all connections
  - Env vars (WAVEMUX_AGENT_ID) take highest priority

### v0.15.5 (2026-01-14)
- **Agent:** AgentX
- **Changes:**
  - Fix: Claude activity display - no duplicate, bold in header (#126)
  - Fix: per-pane agent identification + build system fixes (#125)
  - Fix: re-enable hardware acceleration by default (#123)

### v0.15.4 (2026-01-13)
- **Agent:** AgentX
- **Changes:**
  - Feat: add AgentY to default agent colors (#122)
  - Feat: Display Claude activity summaries in pane title bar (#121)
  - Feat: per-pane agent colors via shell environment variables (#120)
  - Fix: improve agent detection path matching with trailing slash (#119)

### v0.15.0 - v0.15.3 (2026-01-12)
- **Agent:** AgentX
- **Changes:**
  - Feat: Add agent colors to terminal pane headers (#103)
  - Feat: environment variable-based agent detection (#102)
  - Disable Dependabot - causing too many blockers (#118)
  - Sync missing aiprompts files from upstream waveterm

### v0.14.0 (2026-01-09)
- **Agent:** Agent2
- **Changes:**
  - Removed Storybook (unused dev tool, ~36MB savings)
  - Removed Storybook references from Dependabot config
  - Fixed gamerlove startup failures (reverted to simple 1-terminal layout)
  - Disabled hardware acceleration for Windows Sandbox/RDP compatibility
  - Added console window with verbose startup logging
  - Multiple dependency updates (xterm, monaco, react-hook-form, etc.)

### v0.13.3 - v0.13.6 (2026-01-08)
- **Changes:**
  - Various hardware acceleration and startup fixes
  - Window size calculation debugging
  - Layout fixes for gamerlove

### v0.12.14-fork (2025-10-20)
- **Location:** Main worktree (`D:/Code/waveterm`)
- **Branch:** `feature/high-contrast-terminal-borders`
- **Agent:** agentx
- **Changes:**
  - **P0 FIX:** Cross-platform wsh binary exclusions (breaks macOS/Linux builds)
  - **P1 FIX:** Updater IPC handler crash when auto-update disabled
  - Added RELEASE_CHECKLIST.md with comprehensive workflow guide
  - Enhanced bump-version.sh to prevent releasing old code
  - Documented correct release workflow to prevent v0.12.13 issue recurrence

### v0.12.13-fork (2025-10-20)
- **Location:** Main worktree (`D:/Code/waveterm`)
- **Branch:** `feature/high-contrast-terminal-borders`
- **Agent:** agentx
- **Changes:**
  - Fix title bar instance number parsing bug (was showing "undefined")
  - Add comprehensive app name and instance tests
  - **NOTE:** This version was released BEFORE instance parsing fix was committed
  - **ISSUE:** Users downloaded old code under new version number
  - **RESOLUTION:** v0.12.14 includes all fixes with corrected workflow

### v0.12.12-fork (2025-10-20)
- **Location:** Main worktree (`D:/Code/waveterm`)
- **Branch:** `feature/high-contrast-terminal-borders`
- **Changes:**
  - Package verification and version consistency fixes

### v0.12.11-fork (2025-10-20)
- **Changes:**
  - Version management improvements and documentation

### v0.12.10-fork (2025-10-19)
- **Location:** Main worktree (`D:/Code/waveterm`)
- **Branch:** `feature/high-contrast-terminal-borders`
- **Changes:**
  - Fix waveConfigDirName undefined error
  - Add smoke tests for configuration

### v0.12.9-fork (2025-10-19)
- **Changes:**
  - Fix waveDirName undefined error
  - Add more configuration tests

### v0.12.8-fork (2025-10-19)
- **Changes:**
  - Implement portable multi-instance mode with persistent settings

### v0.12.7-fork (2025-10-19)
- **Changes:**
  - UI improvements: hard corners, better borders
  - Fix settings persistence issues
  - Optimize build size: remove heavy artifacts

### v0.12.6-fork (2025-10-19)
- **Changes:**
  - Add comprehensive crash reporting system

### v0.12.3-fork (2025-10-19)
- **Location:** This worktree (`D:/Code/agent-workspaces/agentx/waveterm`)
- **Agent:** agentx
- **Branch:** `agentx/merge-upstream-v0.12.0`
- **Changes:**
  - Add high-contrast white borders to unselected terminal blocks
  - Fix electron-builder packaging bug (upgrade to v26.1.0)
  - Document critical electron-builder files configuration bug
  - Add build investigation spec and artifact verification
  - Added multi-instance development support
  - Added comprehensive documentation (BUILD.md, CLAUDE.md)
  - **Added version management scripts (bump-version.sh/ps1) and this VERSION_HISTORY.md**

### v0.12.2-fork
- Multi-instance support improvements
- Multi-instance dialog

### v0.12.1-fork
- Inherit main install profile
- Initial multi-instance support with shared config

### v0.12.0-fork
- Initial merge from upstream v0.12.0

---

## Development Setup

**Primary Development Machine:** area54 (192.168.1.26)
**Repository:** `C:\Systems\wavemux`

### Agent Workspaces

Agents work on feature branches from `main`:
- `agenta/feature-name` - AgentA (area54)
- `agentx/feature-name` - AgentX (claudius)
- `agentg/feature-name` - AgentG (gamerlove)

---

## Upstream Version Tracking

- **Upstream repository:** https://github.com/wavetermdev/waveterm
- **Base Upstream Version:** v0.12.0
- **Fork repository:** https://github.com/a5af/wavemux
- **Latest Fork:** v0.15.8
- **Commits Ahead of Upstream:** 100+ commits with custom features

---

## Key Fork Features

1. **Per-pane agent identification** - Terminal panes show agent identity (AgentA, AgentX, etc.)
2. **Agent color borders** - Colored borders indicate which agent owns a pane
3. **Claude activity display** - Shows Claude Code activity summaries in title bar
4. **Environment-based agent detection** - WAVEMUX_AGENT_ID and AGENTMUX_AGENT_ID env vars
5. **OSC 16162 shell integration** - Shell can send agent identity via escape sequences
6. **Multi-instance support** - Multiple WaveMux instances can run simultaneously
7. **Portable mode** - Persistent settings across instances
8. **High-contrast borders** - Visual improvements for terminal blocks
9. **Version management** - Automated version bump scripts

---

## Version Bump Instructions

```bash
# Bump patch version (0.15.8 -> 0.15.9)
./bump-version.sh patch --message "Fix description"

# Bump minor version (0.15.8 -> 0.16.0)
./bump-version.sh minor --message "New feature"
```

The bump scripts automatically:
- ✅ Update `package.json` and `package-lock.json`
- ✅ Create git commit with version message
- ✅ Create git tag (e.g., `v0.15.9-fork`)

---

## Notes for Agents

- Always check this file first to understand current version state
- Create feature branches from `main`: `git checkout -b agentX/feature-name`
- Open PRs against `main` branch (it's protected, requires PR)
- Run `task build:backend` after Go changes
- Run `task dev` for development with hot reload
- Run `task package` only for final release builds
