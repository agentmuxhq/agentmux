# AgentMux Component Architecture

## High-Level Block Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              USER INTERFACE                                  │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                         Electron Shell                                │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐ │   │
│  │  │   Window    │  │  Menu Bar   │  │   Tray      │  │   Dialogs   │ │   │
│  │  │  Manager    │  │             │  │             │  │             │ │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘ │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                    │                                         │
│                            IPC Bridge (preload.ts)                          │
│                                    │                                         │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                     React Frontend (Renderer)                         │   │
│  │  ┌───────────────────────────────────────────────────────────────┐   │   │
│  │  │                      Tab Management                            │   │   │
│  │  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐           │   │   │
│  │  │  │    Tab 1    │  │    Tab 2    │  │    Tab N    │           │   │   │
│  │  │  └─────────────┘  └─────────────┘  └─────────────┘           │   │   │
│  │  └───────────────────────────────────────────────────────────────┘   │   │
│  │                                │                                      │   │
│  │  ┌───────────────────────────────────────────────────────────────┐   │   │
│  │  │                    Layout Engine (FlexLayout)                  │   │   │
│  │  │  ┌─────────────────────┬─────────────────────┐                │   │   │
│  │  │  │       BLOCK         │       BLOCK         │                │   │   │
│  │  │  │   ┌───────────┐     │   ┌───────────┐     │                │   │   │
│  │  │  │   │ BlockFrame│     │   │ BlockFrame│     │                │   │   │
│  │  │  │   │ (Header)  │     │   │ (Header)  │     │                │   │   │
│  │  │  │   ├───────────┤     │   ├───────────┤     │                │   │   │
│  │  │  │   │ Block View│     │   │ Block View│     │                │   │   │
│  │  │  │   │ (term)    │     │   │ (preview) │     │                │   │   │
│  │  │  │   └───────────┘     │   └───────────┘     │                │   │   │
│  │  │  └─────────────────────┴─────────────────────┘                │   │   │
│  │  └───────────────────────────────────────────────────────────────┘   │   │
│  │                                │                                      │   │
│  │  ┌───────────────────────────────────────────────────────────────┐   │   │
│  │  │                    State Management (Jotai)                    │   │   │
│  │  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐           │   │   │
│  │  │  │   Atoms     │  │    WOS      │  │  RPC Client │           │   │   │
│  │  │  │ (Local UI)  │  │ (Wave Obj)  │  │ (TabRpc)    │           │   │   │
│  │  │  └─────────────┘  └─────────────┘  └─────────────┘           │   │   │
│  │  └───────────────────────────────────────────────────────────────┘   │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                            WebSocket (ws://)
                                    │
┌─────────────────────────────────────────────────────────────────────────────┐
│                           GO BACKEND (agentmuxsrv)                           │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                         RPC Server (wshserver)                        │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                   │   │
│  │  │  SetMeta    │  │ ConnEnsure  │  │ FileOps     │                   │   │
│  │  │  Command    │  │ Command     │  │ Commands    │                   │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘                   │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                    │                                         │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                      Object Store (waveobj/wstore)                    │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                   │   │
│  │  │   Blocks    │  │    Tabs     │  │  Workspace  │                   │   │
│  │  │  (meta,oid) │  │  (blockids) │  │  (tabids)   │                   │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘                   │   │
│  │                         │ SQLite                                      │   │
│  │                         ▼                                             │   │
│  │                  ~/.waveterm/waveterm.db                              │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                    │                                         │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                    Block Controllers                                  │   │
│  │  ┌─────────────────────────────────────────────────────────────────┐ │   │
│  │  │                   Shell Controller                               │ │   │
│  │  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │ │   │
│  │  │  │  PTY Mgmt   │  │   Input     │  │   Output    │             │ │   │
│  │  │  │  (shellexec)│  │  Handler    │  │  Handler    │             │ │   │
│  │  │  └─────────────┘  └─────────────┘  └─────────────┘             │ │   │
│  │  │                          │                                      │ │   │
│  │  │                    OSC Handlers                                 │ │   │
│  │  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │ │   │
│  │  │  │  OSC 7      │  │  OSC 0/2    │  │  OSC 16162  │             │ │   │
│  │  │  │  (CWD)      │  │  (Title)    │  │  (Wave SI)  │             │ │   │
│  │  │  └─────────────┘  └─────────────┘  └─────────────┘             │ │   │
│  │  └─────────────────────────────────────────────────────────────────┘ │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                    │                                         │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                    Connection Manager                                 │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                   │   │
│  │  │   Local     │  │    SSH      │  │    WSL      │                   │   │
│  │  │  Connector  │  │  Connector  │  │  Connector  │                   │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘                   │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                   PTY
                                    │
┌─────────────────────────────────────────────────────────────────────────────┐
│                          SHELL ENVIRONMENT                                   │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                    Shell Integration Scripts                          │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                   │   │
│  │  │ zsh_zshrc   │  │ bash_bashrc │  │ pwsh_wave   │                   │   │
│  │  │    .sh      │  │    .sh      │  │   pwsh.sh   │                   │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘                   │   │
│  │                          │                                            │   │
│  │          Sends: OSC 7 (CWD), OSC 16162 (Shell Integration)           │   │
│  │                          │                                            │   │
│  │  ┌─────────────────────────────────────────────────────────────────┐ │   │
│  │  │                         wsh CLI                                  │ │   │
│  │  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │ │   │
│  │  │  │  wsh run    │  │  wsh edit   │  │  wsh view   │             │ │   │
│  │  │  │             │  │             │  │             │             │ │   │
│  │  │  └─────────────┘  └─────────────┘  └─────────────┘             │ │   │
│  │  └─────────────────────────────────────────────────────────────────┘ │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Component Interactions

### 1. Electron Main Process → Frontend (IPC)

```
emain/emain.ts
    │
    ├── Window lifecycle events (close, minimize, maximize)
    ├── Native menu actions
    ├── File dialog results
    └── System events (sleep, wake, network)
              │
              ▼
preload/index.ts (contextBridge)
              │
              ▼
frontend/app/store/global.ts (atoms)
```

### 2. Frontend → Backend (WebSocket RPC)

```
React Component (e.g., blockframe.tsx)
    │
    ├── useWaveObjectValue(blockId) - Subscribe to block changes
    └── RpcApi.SetMetaCommand() - Update block metadata
              │
              ▼
frontend/app/store/wshclientapi.ts (Generated)
              │
              ▼
WebSocket (localhost:port)
              │
              ▼
pkg/wshrpc/wshserver/wshserver.go
              │
              ├── SetMetaCommand → wstore.UpdateObjectMeta()
              └── Broadcasts changes → Frontend re-renders
```

### 3. Terminal Input/Output Flow

```
User Types in Terminal
    │
    ▼
frontend/app/view/term/term.tsx
    │
    ├── xterm.js captures keystrokes
    └── Sends to backend via WebSocket
              │
              ▼
pkg/blockcontroller/shellcontroller.go
    │
    ├── ShellInputCh receives input
    └── Writes to PTY (shellexec)
              │
              ▼
Shell Process (bash/zsh/pwsh)
    │
    ├── Executes commands
    └── Outputs to stdout/stderr
              │
              ▼
pkg/blockcontroller/shellcontroller.go
    │
    ├── Reads PTY output
    └── Writes to block file (term data)
              │
              ▼
frontend/app/view/term/termwrap.ts
    │
    ├── Receives data chunks
    └── Feeds to xterm.js for rendering
```

### 4. OSC 16162 Shell Integration Flow

```
Shell Integration Script (prompt hook)
    │
    ├── Detects agent env var (WAVEMUX_AGENT_ID)
    └── Sends: printf '\033]16162;E;{"WAVEMUX_AGENT_ID":"AgentA"}\007'
              │
              ▼
PTY Output → termwrap.ts OSC handler
              │
              ▼
handleOsc16162Command() [case "E"]
    │
    └── RpcApi.SetMetaCommand({ meta: { "cmd:env": {...} } })
              │
              ▼
Backend wshserver.go SetMetaCommand
    │
    └── wstore.UpdateObjectMeta()
              │
              ▼
Frontend receives WOS update
    │
    ▼
blockframe.tsx re-renders
    │
    ├── detectAgentFromEnv(blockEnv) returns "AgentA"
    ├── viewName = "AgentA" (title bar)
    └── agentColor = "#1e3a5f" (header background)
```

### 5. Configuration Flow

```
~/.waveterm/config/settings.json
    │
    ▼
pkg/wconfig/watcher.go (file watcher)
    │
    └── GetFullConfig() returns merged settings
              │
              ▼
Frontend via RPC or direct read
    │
    ▼
frontend/app/store/global.ts → atoms.fullConfigAtom
    │
    ▼
Components read via useAtomValue(atoms.fullConfigAtom)
```

---

## Key Files by Layer

### Electron Main Process
| File | Purpose |
|------|---------|
| `emain/emain.ts` | Main entry, window management |
| `emain/preload.ts` | IPC bridge to renderer |

### Frontend (React/TypeScript)
| File | Purpose |
|------|---------|
| `frontend/app/block/blockframe.tsx` | Block container, header, agent detection |
| `frontend/app/block/block.tsx` | Block component, focus handling |
| `frontend/app/block/autotitle.ts` | Agent detection from env/path |
| `frontend/app/view/term/termwrap.ts` | OSC handlers, xterm.js wrapper |
| `frontend/app/store/global.ts` | Jotai atoms, state management |

### Backend (Go)
| File | Purpose |
|------|---------|
| `cmd/server/main-server.go` | Backend entry point |
| `pkg/blockcontroller/shellcontroller.go` | PTY management, shell I/O |
| `pkg/wshrpc/wshserver/wshserver.go` | RPC command handlers |
| `pkg/waveobj/waveobj.go` | Object model (Block, Tab, etc.) |
| `pkg/wstore/wstore.go` | SQLite persistence layer |

### Shell Integration
| File | Purpose |
|------|---------|
| `pkg/util/shellutil/shellintegration/zsh_zshrc.sh` | Zsh hooks |
| `pkg/util/shellutil/shellintegration/bash_bashrc.sh` | Bash hooks |
| `pkg/util/shellutil/shellintegration/pwsh_wavepwsh.sh` | PowerShell hooks |

### wsh CLI
| File | Purpose |
|------|---------|
| `cmd/wsh/main-wsh.go` | CLI entry point |
| `cmd/wsh/cmd/*.go` | Subcommands (run, edit, view, etc.) |

---

## Data Flow Summary

```
User Action → React → WebSocket RPC → Go Backend → Database/PTY
                                           │
                                           ▼
                                    State Change
                                           │
                                           ▼
          React ← WebSocket Update ← Go Backend
               │
               ▼
         UI Re-render
```
