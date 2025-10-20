# PR: Upstream Merge v0.12.0 Specifications

**Branch:** `feature/add-upstream-merge-and-multi-instance-specs`
**Target:** `main`
**PR URL:** https://github.com/a5af/waveterm/compare/main...feature/add-upstream-merge-and-multi-instance-specs?expand=1

---

## Summary

Add comprehensive specifications for two major enhancements to the a5af/waveterm fork:

### 1. Upstream Merge v0.12.0 (SPEC_UPSTREAM_MERGE_V0.12.0.md)
- Strategy for merging 48 commits from upstream WaveTerm v0.12.0
- **Major AI features to integrate:**
  - Batch tool approval system
  - AI reasoning display (real-time streaming)
  - New AI tools: `read_dir`, native web search
  - AI response feedback and copy buttons
  - Google AI file summarization
- **Infrastructure updates:**
  - React 19 migration
  - Tailwind v4
  - Layout simplification
  - Tsunami framework (waveapps v2)
- **7-phase incremental merge plan** (3-5 days estimated)
- Detailed conflict analysis and resolution strategies
- Fork feature preservation plan (horizontal widget bar, pane title labels)

### 2. Multi-Instance Support (SPEC_MULTI_INSTANCE_PORTABLE_WINDOWS.md)
- Enable multiple simultaneous WaveTerm instances on Windows
- Instance-specific data directories via `--instance <id>` CLI flag
- Portable mode for USB drives via `--portable` flag
- Remove Electron single-instance lock
- 6-week implementation timeline

## Priority

**Upstream merge should be completed FIRST** before implementing multi-instance support to ensure we build on the latest stable base with new AI features.

## Testing Strategy

Both specs include:
- Detailed implementation plans
- Phase-by-phase testing approaches
- Risk assessments and mitigation strategies
- Rollback procedures

## Conflicts Expected

High-risk conflicts identified:
- Layout system changes (horizontal widget bar vs upstream layout simplification)
- Block management system (pane title labels vs upstream block changes)
- React 19 compatibility updates needed

## Next Steps

1. Review and approve this PR
2. Begin Phase 1: Pre-merge preparation
3. Execute 7-phase merge plan
4. After successful merge: implement multi-instance support

---

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>
