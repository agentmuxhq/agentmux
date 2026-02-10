# AgentMux — Project Status

**Date:** 2026-02-10
**Version:** 0.20.18
**Branch:** main @ `54349ca`

---

## Major Milestones

### Go Sidecar Eliminated (PR #238, AgentA)

The Go backend sidecar (`wavemuxsrv`) has been **completely removed**. AgentMux is now a single Rust binary with no external backend process. This was a massive refactor:

- Deleted `sidecar.rs`, `cmd/server/`, and 6 other Go cmd/ tools (~3,200 lines removed)
- Removed all `#[cfg(feature = "go-sidecar")]` / `#[cfg(feature = "rust-backend")]` feature gates (~68 instances)
- `portable-pty`, `interprocess`, and `notify` are now required dependencies (not feature-gated)
- `tauri.conf.json` no longer lists `agentmuxsrv` in `externalBin` — only `wsh` remains
- `state.rs` simplified: removed `BackendEndpoints`, `sidecar_child` fields
- All backend services now run in-process via `rust_backend::initialize()`

**Before:** Tauri app (Rust) ←WebSocket→ agentmuxsrv (Go) ←socket→ wsh (CLI)
**After:** Tauri app (Rust, single binary) ←Tauri IPC→ Rust backend (in-process) ←socket→ wsh (CLI)

### Rebrand: WaveMux → AgentMux (PR #239, AgentA)

Product renamed across all files — docs, configs, CDK stacks, shell integration, icons, logos. New AgentMux logo SVG added. App binary is now `agentmux`.

---

## Unified AI Pane

The core differentiating feature: one AI pane that combines multi-provider chat (Claude, GPT, Gemini, Perplexity) with coding agent capabilities (Claude Code, Gemini CLI, Codex CLI). No other terminal has this.

### Completed

#### Phase A-1: Type Foundation (PR #228)

Rust and TypeScript type definitions normalizing chat and agent backends into a common message format.

| File | Lines | Purpose |
|------|-------|---------|
| `backend/ai/unified.rs` | 780 | UnifiedMessage, UnifiedMessagePart, AgentBackendConfig, TokenUsage |
| `backend/ai/agent.rs` | 594 | AgentStatus state machine, AgentRegistry, IPC types |
| `backend/ai/adapters.rs` | 981 | AdapterEvent enum, chat/agent adapters, apply_adapter_event() |
| `unifiedai/unified-types.ts` | 530 | TypeScript equivalents, immutable applyAdapterEvent() for React |
| `unifiedai/adapter.ts` | 107 | BackendAdapter interfaces |

#### Phase A-2: Command Bridge (PR #234)

Tauri IPC commands and frontend state management connecting types to subprocess control.

| File | Lines | Purpose |
|------|-------|---------|
| `backend/ai/process.rs` | 615 | AgentProcess (spawn, stdin, signal, kill), NDJSON parser, backend discovery |
| `commands/agent.rs` | 421 | 6 Tauri commands: spawn/send/interrupt/kill/status/list |
| `unifiedai/agent-api.ts` | 139 | Typed Tauri invoke/listen wrappers |
| `unifiedai/useUnifiedAI.ts` | 303 | Jotai atoms + useUnifiedAI() React hook |

#### Phase A-3: UI Components (PR #237)

React view components rendering UnifiedMessage[] with all part types.

| File | Lines | Purpose |
|------|-------|---------|
| `unifiedai/unifiedai-model.ts` | 260 | ViewModel class — bridges block system to agent API |
| `unifiedai/unifiedai-view.tsx` | 420 | Message renderer, tool blocks, input, status bar, empty state |
| `unifiedai/unifiedai.scss` | 520 | Terminal-native styling (monospace, no bubbles) |

**Totals:** ~5,200 lines Rust (AI module) + ~2,400 lines TypeScript/SCSS = **~7,600 lines**
**Tests:** 133 passing (AI module)

#### Phase W-1: Enhanced NDJSON Adapter (PR #242)

Claude Code wrapper foundation — comprehensive spec written and first implementation phase complete.

| File | Lines | Purpose |
|------|-------|---------|
| `CLAUDE_CODE_WRAPPER_SPEC.md` | 818 | Complete technical spec covering 6 phases (W-1 through W-6) |
| `backend/ai/adapters.rs` | +246 | ClaudeCodeEvent outer wrapper, SessionStart/SessionEnd events |
| `backend/ai/process.rs` | +65 | parse_ndjson_line() tries outer event first, send_user_message() for multi-turn |
| `backend/ai/unified.rs` | +28 | Full wrapper config with -p mode, env vars (disable telemetry, OOM protection) |
| `unifiedai/unified-types.ts` | +26 | AdapterSessionStart, AdapterSessionEnd types |
| `unifiedai/unifiedai-model.ts` | +48 | sessionIdAtom, totalCostAtom, handleSessionEvent() |
| `unifiedai/unifiedai-view.tsx` | +4 | StatusBar cost display |

**Key implementation:**
- Expanded adapter to parse all 6 Claude Code `stream-json` event types (`system`, `stream_event`, `assistant`, `user`, `result`) instead of only inner stream events
- Added session tracking (`session_id` from system init, `total_cost_usd` from result event)
- Multi-turn stdin support via `--input-format stream-json` for follow-up messages within same subprocess
- Wrapper configuration: `-p --output-format stream-json --include-partial-messages --input-format stream-json`
- Environment variables: `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1`, `NODE_OPTIONS=--max-old-space-size=4096`

### Next: Claude Code Wrapper Phase W-2

Multi-turn conversation refinement — seamless follow-up prompts without subprocess restart. See `CLAUDE_CODE_WRAPPER_SPEC.md` for remaining phases (W-2 through W-6):
- **W-2:** Multi-turn stdin (structured messages, graceful error handling)
- **W-3:** Tool approval UI (approve/deny/edit for destructive operations)
- **W-4:** Session management (reconnect, history persistence)
- **W-5:** MCP pane awareness (Claude Code can see terminal scrollback, editor content)
- **W-6:** UI polish (streaming indicators, syntax highlighting)

### Future Phases

#### Phase B: MCP Server for Pane Awareness

Agent subprocesses can see all open panes (terminal scrollback, code editor content, web previews).

| Component | Purpose |
|-----------|---------|
| `backend/mcp/server.rs` | TCP listener + JSON-RPC handler |
| `backend/mcp/tools.rs` | agentmux_list_panes, agentmux_read_terminal, agentmux_screenshot |
| `backend/ai/orchestrator.rs` | MCP server lifecycle tied to agent spawn |

#### Phase C: Chat Backend Migration

Port AI chat orchestration to Rust. Currently using Vercel AI SDK; migrate to direct API calls.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────┐
│                    AgentMux App                       │
│                                                       │
│  ┌─── Tauri + Rust Backend (single binary) ────────┐ │
│  │ Window mgmt, menus, tray, crash handler          │ │
│  │ Storage (SQLite WaveStore + FileStore)            │ │
│  │ RPC engine + router                               │ │
│  │ Terminal PTY (portable-pty)                       │ │
│  │ Config system + file watching                     │ │
│  │ wsh IPC socket server                            │ │
│  │ Pub/Sub broker                                    │ │
│  │                                                  │ │
│  │ backend/ai/ ←── Unified AI Pane                  │ │
│  │   unified.rs, agent.rs, adapters.rs, process.rs  │ │
│  │ commands/agent.rs ←── 6 Tauri IPC commands       │ │
│  └──────────────────────────────────────────────────┘ │
│                                                       │
│  ┌─── Frontend (React/TypeScript) ─────────────────┐ │
│  │ Terminal (xterm.js), Code editor (Monaco)        │ │
│  │ Unified AI Pane (view: "unifiedai")              │ │
│  │ Web preview, layout system, landing page         │ │
│  │                                                  │ │
│  │ unifiedai/ ←── Types, hooks, API, view, model    │ │
│  └──────────────────────────────────────────────────┘ │
│                                                       │
│  ┌─── Agent Subprocesses ──────────────────────────┐ │
│  │ Claude Code (claude -p --output-format stream-json)│
│  │ Gemini CLI, Codex CLI                            │ │
│  │ NDJSON → AdapterEvent → UnifiedMessage           │ │
│  └──────────────────────────────────────────────────┘ │
│                                                       │
│  ┌─── wsh (Go CLI, bundled sidecar) ───────────────┐ │
│  │ Shell integration, remote connections            │ │
│  │ 8 platform/arch builds                           │ │
│  └──────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────┘
```

---

## Recent PRs

| PR | Description | Author | Status |
|----|-------------|--------|--------|
| #242 | Claude Code wrapper adapter and spec (Phase W-1) | Agent3 | Merged |
| #240 | go mod tidy after sidecar removal | AgentA | Merged |
| #239 | Rebrand WaveMux → AgentMux | AgentA | Merged |
| #238 | Remove Go sidecar — single Rust backend | AgentA | Merged |
| #237 | Unified AI pane UI components (Phase A-3) | Agent3 | Merged |
| #236 | Dark/light theme toggle | AgentA | Merged |
| #234 | Agent command bridge (Phase A-2) | Agent3 | Merged |
| #233 | Landing page | AgentA | Merged |
| #232 | Rename comms → agentbus | AgentA | Merged |
| #231 | Fix npm script names | AgentA | Merged |
| #230 | Remove Electron dead code | AgentA | Merged |
| #229 | Auto-updater | AgentA | Merged |
| #228 | Unified AI type foundation (Phase A-1) | Agent3 | Merged |

---

## Specs Written (uncommitted)

| File | Purpose |
|------|---------|
| `UNIFIED_AI_PANE_SPEC.md` | Full technical spec for the unified AI pane (Phases A-D) |
| `CLAUDE_CODE_WRAPPER_SPEC.md` | Polished UI wrapper for Claude Code (NEW) |
| `MESSAGING_PANE_SPEC.md` | Slack/Discord integration as panes |
| `EXTERNAL_AUTH_SPEC.md` | External browser OAuth flow |
| `PRICING_TIERS_PLAN.md` | Freemium pricing model |
| `PRO_FEATURES_SPEC.md` | Pro tier feature specifications |

---

## Key Decisions

1. **Single Rust binary** — No more Go sidecar. All backend services run in-process. Only `wsh` remains as a bundled Go binary for shell integration.

2. **Adapter pattern** — Both chat (SSE) and agent (NDJSON) backends produce `AdapterEvent` values, normalized into `UnifiedMessage` via a single state machine. One renderer for all backends.

3. **Per-pane state** — Each pane has its own agent instance, conversation, and status. Jotai `atomFamily` keyed by pane ID.

4. **Process group isolation** — Agent subprocesses get their own process group (`setpgid`) so SIGINT doesn't propagate to the parent app.

5. **Claude Code via `-p` mode** — Non-interactive mode with NDJSON streaming. Raw Claude Code UI is never shown to users.

---

## Agent Coordination

| Agent | Responsibility | Recent Work |
|-------|---------------|-------------|
| **Agent3** (me) | Unified AI Pane, Claude Code wrapper, specs | PR #228, #234, #237 |
| **AgentA** | Tauri migration, sidecar elimination, rebrand | PR #229-240 |
| **Agent1** | Planning | Wiki |
