# Phase 17: Final Go→Rust Migration
**Version:** 0.20.2 → 0.21.0
**Lead:** AgentA
**Status:** In Progress
**Date:** 2026-02-09

---

## Executive Summary

Complete the final 7.1% of Go→Rust backend migration to achieve 100% coverage. Agent3's Phases 9-16 brought us to 92.9% coverage. This phase ports the remaining 3 packages: wshutil, genconn, and wsl.

**Current State:**
- ✅ 92.9% Go coverage (Agent3 Phases 9-16)
- ✅ 59 Rust backend modules created
- ⏳ 3 packages remaining (3,422 Go lines → ~1,600 Rust lines)

---

## Packages to Port

### Priority 1: wshutil (~1,200 Rust lines)

**Go Source:** `pkg/wshutil/*.go` (2,754 lines)
**Target:** `src-tauri/src/backend/wshutil/`
**Complexity:** High (WebSocket RPC transport layer)

**What's Already Done:**
- ✅ `rpc/router.rs` - RPC routing + engine (Agent3)
- ✅ `rpc/engine.rs` - Command dispatch

**What's Missing:**
| File | Go Lines | Purpose | Est. Rust Lines |
|------|----------|---------|-----------------|
| wshutil.go | 589 | WS connection setup, lifecycle | ~250 |
| wshproxy.go | 287 | Remote proxy | ~150 |
| wshadapter.go | 160 | Transport adapters | ~100 |
| wshmultiproxy.go | 156 | Broadcast proxy | ~100 |
| wshcmdreader.go | 171 | CLI stdin reader | ~100 |
| wshevent.go | 67 | Event types | ~50 |
| wshrpcio.go | 59 | RPC I/O helpers | ~50 |

**Dependencies:**
- `tokio-tungstenite` (already in Cargo.toml ✅)
- `serde_json` (already in Cargo.toml ✅)
- Depends on: rpc/router.rs, rpc/engine.rs

**Implementation Strategy:**
1. Create `src-tauri/src/backend/wshutil/mod.rs`
2. Port core WshRpc struct (connection lifecycle)
3. Port proxy types (single + multi)
4. Port transport adapters
5. Port CLI stdin reader
6. Add integration tests

---

### Priority 2: genconn (~300 Rust lines)

**Go Source:** `pkg/remote/genconn/*.go` (457 lines)
**Target:** `src-tauri/src/backend/remote/genconn.rs` (already exists with traits)
**Complexity:** Medium (needs SSH library decision)

**What's Already Done:**
- ✅ `genconn.rs` - Traits, mocks, helpers (537 lines by Agent3)
- ✅ ShellClient trait
- ✅ CommandSpec struct
- ✅ MockShellClient for testing

**What's Missing:**
| Component | Go Lines | Purpose | Est. Rust Lines |
|-----------|----------|---------|-----------------|
| SSHShellClient | ~230 | SSH connection via russh or system ssh | ~150 |
| WSLShellClient | ~227 | WSL connection via wsl.exe | ~150 |

**Decision Required: SSH Implementation**
- **Option A:** Add `russh` crate (~25KB compiled)
  - Pros: Pure Rust, type-safe, async
  - Cons: New dependency, more code

- **Option B:** Use `std::process::Command` to shell out to system `ssh`
  - Pros: Zero deps, simpler, works everywhere
  - Cons: Requires system ssh binary, less control

**Recommendation:** Option B (interim) - shell out to system ssh
- Faster to implement
- Proven pattern (wsl.exe already uses std::process)
- Can upgrade to russh in Phase 18 if needed

**Implementation Strategy:**
1. Implement SSHShellClient using std::process::Command
2. Implement WSLShellClient using std::process::Command + wsl.exe
3. Add cfg(windows) gates for WSLShellClient
4. Add integration tests (mock mode on CI, real mode locally)

---

### Priority 3: wsl (~100 Rust lines)

**Go Source:** `pkg/wsl/*.go` (211 lines)
**Target:** `src-tauri/src/backend/wslconn.rs` (already exists with wire types)
**Complexity:** Low (simple system command parsing)

**What's Already Done:**
- ✅ `wslconn.rs` - Wire types (305 lines by Agent3)
- ✅ WslName, RemoteInfo, ConnStateFields

**What's Missing:**
| Function | Go Lines | Purpose | Est. Rust Lines |
|----------|----------|---------|-----------------|
| registered_distros() | ~80 | Parse `wsl.exe --list` | ~40 |
| default_distro() | ~60 | Get default distro | ~30 |
| get_distro() | ~71 | Validate distro name | ~30 |

**Implementation Strategy:**
1. Add functions to wslconn.rs
2. Use `std::process::Command` to call `wsl.exe --list`
3. Parse output (UTF-16 on Windows)
4. Add `#[cfg(windows)]` gates
5. Non-Windows: return error
6. Add unit tests with mock output

---

## Version Strategy

**Bump to 0.21.0** (minor version) when Phase 17 completes:
- Significant milestone: 100% Go→Rust migration complete
- Breaking API change: Go backend fully removed
- Update all files: package.json, Cargo.toml, tauri.conf.json

---

## Testing Strategy

### Backend Tests
- Run `backend-test` crate tests (need to fix Cargo.toml first)
- Add integration tests for each new module
- Mock external dependencies (SSH, WSL) for CI

### Manual Testing
- Test SSH connections to remote servers
- Test WSL connections on Windows
- Test WS RPC proxying end-to-end
- Verify CLI stdin handling

### Cross-Platform
- Windows: Full testing (native WSL, SSH via Git Bash)
- macOS/Linux: SSH only (WSL functions return error)

---

## Implementation Phases

### Phase 17.1: wsl utilities (1-2 hours)
- ✅ Lowest complexity
- ✅ Standalone (no external deps)
- ✅ Can test immediately on Windows

**Tasks:**
1. Add registered_distros() to wslconn.rs
2. Add default_distro() to wslconn.rs
3. Add get_distro() to wslconn.rs
4. Add cfg(windows) gates
5. Add unit tests
6. Manual test on Windows

### Phase 17.2: genconn implementations (3-4 hours)
- Medium complexity
- Depends on wsl utilities for WSLShellClient

**Tasks:**
1. Implement SSHShellClient (std::process ssh)
2. Implement WSLShellClient (calls wsl utilities)
3. Add cfg(windows) for WSLShellClient
4. Add integration tests
5. Manual test SSH + WSL connections

### Phase 17.3: wshutil transport (6-8 hours)
- Highest complexity
- Core RPC transport layer

**Tasks:**
1. Port WshRpc struct (connection lifecycle)
2. Port WshProxy + WshMultiProxy
3. Port transport adapters
4. Port CLI stdin reader
5. Port event types + I/O helpers
6. Integration tests
7. End-to-end RPC testing

---

## Success Criteria

- [ ] All 180 remaining Go files ported to Rust
- [ ] 100% Go→Rust coverage achieved
- [ ] All backend tests pass
- [ ] Cross-platform testing complete (Windows verified)
- [ ] Version bumped to 0.21.0
- [ ] PR merged with ReAgent approval
- [ ] Go pkg/ directory archived or deleted

---

## Risks & Mitigation

**Risk 1: SSH library choice impacts timeline**
- Mitigation: Start with std::process::Command (faster), upgrade to russh in Phase 18 if needed

**Risk 2: WebSocket RPC complexity**
- Mitigation: Leverage existing rpc/router.rs + rpc/engine.rs from Agent3

**Risk 3: Cross-platform testing gaps**
- Mitigation: Windows testing mandatory, macOS/Linux best-effort (cfg gates protect)

**Risk 4: Breaking changes to frontend**
- Mitigation: Keep RPC API compatible, frontend shouldn't notice backend language change

---

## Timeline Estimate

| Phase | Complexity | Est. Time | Dependencies |
|-------|-----------|-----------|--------------|
| 17.1: wsl | Low | 1-2 hours | None |
| 17.2: genconn | Medium | 3-4 hours | 17.1 |
| 17.3: wshutil | High | 6-8 hours | 17.2 |
| **Total** | - | **10-14 hours** | - |

**Target Completion:** 2026-02-10 (tomorrow)

---

## Post-Phase 17 (Future Work)

### Phase 18: Cleanup & Optimization
- Remove Go pkg/ directory
- Upgrade SSH to russh (if needed)
- Performance optimization
- Dead code elimination
- Clippy warnings cleanup

### Phase 19: Production Hardening
- Error handling improvements
- Logging + telemetry
- Crash recovery
- Memory leak testing

---

**Plan Status:** Ready to Execute
**Next Action:** Start Phase 17.1 (wsl utilities)
