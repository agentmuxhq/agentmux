# WaveTerm Fork Version History

This document tracks the version history of the a5af/waveterm fork.

## Latest Version: 0.12.14-fork

**Main Worktree:** `D:/Code/waveterm` (branch: `feature/high-contrast-terminal-borders`)
**This Worktree:** `D:/Code/agent-workspaces/agentx/waveterm` (branch: `agentx/merge-upstream-v0.12.0`)
**Base:** Upstream v0.12.0 + custom features

---

## Version History (Latest First)

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

## Worktree Structure

This repository uses **git worktrees** for parallel development:

1. **Main Worktree:** `D:/Code/waveterm` (v0.12.10, latest features)
   - Active development on `feature/high-contrast-terminal-borders`
   - Has all the latest bug fixes and features

2. **Agent Worktree:** `D:/Code/agent-workspaces/agentx/waveterm` (v0.12.3)
   - Agent workspace on `agentx/merge-upstream-v0.12.0`
   - Has version management scripts and documentation updates

To sync this worktree with latest from main worktree:
```bash
# Fetch changes from main worktree's branch into this branch
git fetch . feature/high-contrast-terminal-borders:agentx/merge-upstream-v0.12.0

# Or cherry-pick specific commits
git cherry-pick <commit-hash>

# Or merge the branch
git merge feature/high-contrast-terminal-borders
```

---

## Upstream Version Tracking

- **Upstream repository:** https://github.com/wavetermdev/waveterm
- **Latest Upstream:** v0.12.0 (wavetermdev/waveterm)
- **Fork repository:** https://github.com/a5af/waveterm
- **Latest Fork:** v0.12.10-fork (main worktree)
- **This Worktree:** v0.12.3-fork
- **Commits Ahead of Upstream:** 15+ commits with custom features

---

## Key Fork Features

1. **Multi-instance support** - Multiple WaveTerm instances can run simultaneously
2. **Shared config** - Config shared across instances
3. **High-contrast borders** - Visual improvements for terminal blocks
4. **Enhanced packaging** - Fixed dist folder inclusion
5. **Development docs** - Added BUILD.md and CLAUDE.md guides
6. **Version management** - Automated version bump scripts
7. **Crash reporting** - Comprehensive error tracking
8. **Portable mode** - Persistent settings across instances

---

## How to Use This File

1. **Before starting work:** Check the current version and worktree location above
2. **After significant changes:** Use `./bump-version.sh` or `./bump-version.ps1` to bump and update this file
3. **For new agents:** Read this file first to understand what work has been done and which worktree to use
4. **Check worktrees:** Use `git worktree list` to see all development locations

### Version Bump Instructions

**Windows:**
```powershell
./bump-version.ps1 patch -Message "Fix multi-instance bug"
./bump-version.ps1 minor -Message "Add new terminal borders feature"
./bump-version.ps1 0.12.11 -Message "Next release"
```

**macOS/Linux:**
```bash
./bump-version.sh patch --message "Fix multi-instance bug"
./bump-version.sh minor --message "Add new terminal borders feature"
./bump-version.sh 0.12.11 --message "Next release"
```

The bump scripts automatically:
- ✅ Update `package.json` and `package-lock.json`
- ✅ Update this `VERSION_HISTORY.md` with date, agent, and changes
- ✅ Create git commit with version message
- ✅ Create git tag (e.g., `v0.12.11-fork`)
- ✅ Track which agent made the change

---

## Notes for New Agents

- Always check this file first to understand current version state and worktree locations
- Fork versions append `-fork` to differentiate from upstream
- Version number should be >= upstream base version
- The **main worktree** at `D:/Code/waveterm` has the latest code (v0.12.10)
- This **agent worktree** at `D:/Code/agent-workspaces/agentx/waveterm` has documentation and tooling updates (v0.12.3)
- Use `git worktree list` to see all active worktrees
- Document all major changes in this file when bumping versions
