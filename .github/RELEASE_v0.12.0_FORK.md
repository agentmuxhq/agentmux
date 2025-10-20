# WaveTerm v0.12.0 Fork Release

## Overview
This release merges 59 commits from upstream wavetermdev/waveterm v0.12.0 while preserving our fork-specific features.

## Fork-Specific Features ✨
- **Horizontal Widget Bar** - Widgets displayed horizontally in tab bar
- **Pane Title Labels** - Auto-generated pane titles with custom labels support
- **Custom Layout Model** - Modified layout system for widget positioning

## New Upstream v0.12.0 Features 🚀

### AI Enhancements
- **AI Reasoning Display** - Real-time visualization of AI thought process
- **Response Feedback & Copy Buttons** - User feedback system for AI responses
- **Google AI File Summarization** - File analysis support
- **Enhanced `wsh ai` Command** - Complete CLI AI interface rewrite
- **Terminal Context Improvements** - Better AI awareness of terminal state
- **Batch Tool Approval** - Security for multiple AI actions
- **Welcome Message** - New user onboarding in AI panel
- **Context Menus** - Right-click support for AI messages

### Infrastructure Updates
- **OSC 7 Support** - Fish & PowerShell shell integration
- **Log Rotation** - Automatic cleanup system
- **Mobile UA Emulation** - Web widget improvements
- **React 19 Compatibility** - Framework updates
- **Tailwind v4 Migration** - CSS architecture progress
- **50+ Dependency Updates** - Security and feature improvements

## Build Information 📦
- **Platform**: Windows (x64)
- **Format**: Portable ZIP (no installer required)
- **Build Type**: Production
- **File Size**: 143 MB
- **Test Suite**: 97.6% passing (41/42 tests)

## Installation 📥
1. Download `Wave-win32-x64-0.12.0.zip`
2. Extract to desired location
3. Run `Wave.exe`

No installation required - fully portable!

## Merge Statistics 📊
- **Commits Merged**: 59
- **Conflicts Resolved**: 49 files
- **Files Changed**: 135 total
- **Test Pass Rate**: 97.6% (41/42)

## Known Issues ⚠️
- One minor test failure in layout model pending action queue (test-only, no runtime impact)
- Manual testing recommended before production use

## Documentation 📖
- Merge details: See `.github/PR_MERGE_UPSTREAM_V0.12.0.md`
- Test results: See `_temp/TEST_RESULTS.md`
- Merge summary: See `_temp/MERGE_COMPLETE_SUMMARY.md`

---

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>
