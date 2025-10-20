# WaveTerm AgentMux Integration Specification

**Version:** 1.0
**Date:** 2025-10-19
**Status:** Draft - Design Phase
**Target:** WaveTerm Fork v0.12.4+

---

## Executive Summary

Integrate cross-agent communication capabilities from AgentMux into WaveTerm fork, enabling Claude Code agents to communicate across different workspaces through WaveTerm's terminal interface via an MCP (Model Context Protocol) server.

**Core Concept:** WaveTerm becomes the **communication hub** for agents, with an MCP server tool that allows agents to send/receive messages and notifications through terminal injection.

---

## 1. Background

### AgentMux Architecture
From the AgentMux project:
- **Message Bus** - Real-time inter-agent messaging system
- **Agent Registry** - Tracks active agents and their metadata
- **Terminal Integration** - Embedded terminal with agent spawn capability
- **CLI Commands** - `agent list`, `agent info`, `bus send`, `bus listen`

### WaveTerm Capabilities
- **Multi-tab terminal** - Already supports multiple terminal sessions
- **Block system** - Can display rich content beyond plain terminal output
- **WebSocket/RPC** - Has existing client-server architecture (`wsh` protocol)
- **Plugin system** - Extensible via blocks and commands

### Integration Goal
Create a **shared message bus** that both AgentMux and WaveTerm can use, allowing agents in either application to communicate seamlessly.

---

## 2. Architecture

### 2.1 Component Overview

```
┌─────────────────────────────────────────────────────────┐
│                   Agent Workspace                        │
│  ┌──────────────────────────────────────────────────┐  │
│  │  Claude Code Agent (agent1)                      │  │
│  │  ├─ MCP Client                                   │  │
│  │  └─ Tools Available:                             │  │
│  │     ├─ waveterm_send_message                     │  │
│  │     ├─ waveterm_broadcast                        │  │
│  │     └─ waveterm_list_agents                      │  │
│  └──────────────────────────────────────────────────┘  │
│                      │                                   │
│                      │ MCP Protocol (stdio/websocket)    │
│                      ▼                                   │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│                WaveTerm Application                      │
│  ┌──────────────────────────────────────────────────┐  │
│  │  MCP Server (embedded in WaveTerm)               │  │
│  │  ├─ Agent Registry                               │  │
│  │  ├─ Message Router                               │  │
│  │  └─ Terminal Injector                            │  │
│  └──────────────────────────────────────────────────┘  │
│                      │                                   │
│                      │ Internal API                      │
│                      ▼                                   │
│  ┌──────────────────────────────────────────────────┐  │
│  │  Terminal Sessions (blocks)                      │  │
│  │  ├─ Tab 1: agent1 workspace                      │  │
│  │  ├─ Tab 2: agent2 workspace                      │  │
│  │  └─ Tab 3: agent3 workspace                      │  │
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│          Shared Message Bus (Optional)                   │
│  ┌──────────────────────────────────────────────────┐  │
│  │  AgentMux Compatibility Layer                    │  │
│  │  ├─ WebSocket Server                             │  │
│  │  ├─ Message Persistence (SQLite)                 │  │
│  │  └─ Agent Status Tracking                        │  │
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

### 2.2 Data Flow

**Sending a Message:**
```
Agent1 (Claude Code)
  → MCP Tool: waveterm_send_message(to="agent2", message="Deploy complete")
  → WaveTerm MCP Server
  → Message Router
  → Terminal Injector
  → Agent2's terminal tab receives:
     "[AGENT-MSG] agent1: Deploy complete"
```

**Broadcasting:**
```
Agent1 (Claude Code)
  → MCP Tool: waveterm_broadcast(message="Build finished", type="success")
  → WaveTerm MCP Server
  → Message Router (all registered agents)
  → Terminal Injector (multiple tabs)
  → All agent terminals receive:
     "[BROADCAST] agent1: Build finished"
```

---

## 3. MCP Server Specification

### 3.1 Server Location
- **Embedded in WaveTerm**: Part of the Electron main process
- **Path**: `waveterm/pkg/mcp-server/` (Go) OR `waveterm/emain/mcp-server/` (TypeScript)
- **Launch**: Auto-starts when WaveTerm launches
- **Communication**: stdio or WebSocket

### 3.2 MCP Tools

#### Tool: `waveterm_send_message`
**Purpose:** Send a direct message to a specific agent

**Parameters:**
```typescript
{
  to: string;           // Target agent ID (e.g., "agent2", "agent3")
  message: string;      // Message content
  priority?: "low" | "normal" | "high" | "urgent";
  metadata?: Record<string, any>;
}
```

**Returns:**
```typescript
{
  success: boolean;
  messageId: string;
  deliveredAt: string;  // ISO timestamp
}
```

**Example Usage:**
```typescript
// Agent sends message via MCP
await use_mcp_tool({
  server_name: "waveterm",
  tool_name: "waveterm_send_message",
  arguments: {
    to: "agent2",
    message: "Database migration complete. Ready for deploy.",
    priority: "high"
  }
});
```

#### Tool: `waveterm_broadcast`
**Purpose:** Broadcast message to all connected agents

**Parameters:**
```typescript
{
  message: string;
  type?: "info" | "success" | "warning" | "error";
  metadata?: Record<string, any>;
}
```

**Returns:**
```typescript
{
  success: boolean;
  messageId: string;
  recipients: string[];  // Agent IDs that received it
}
```

#### Tool: `waveterm_list_agents`
**Purpose:** Get list of active agents

**Parameters:**
```typescript
{
  includeInactive?: boolean;  // Include agents that disconnected <5min ago
}
```

**Returns:**
```typescript
{
  agents: Array<{
    id: string;
    workspace: string;
    status: "active" | "idle" | "busy" | "offline";
    lastSeen: string;
    metadata: Record<string, any>;
  }>;
}
```

#### Tool: `waveterm_register_agent`
**Purpose:** Register agent with the hub (auto-called on MCP connection)

**Parameters:**
```typescript
{
  agentId: string;
  workspace: string;
  metadata?: Record<string, any>;
}
```

---

## 4. Terminal Injection

### 4.1 Message Display Format

When messages arrive, inject them into the terminal using ANSI escape codes:

```bash
# Direct message
\033[36m[AGENT-MSG]\033[0m \033[33magent1\033[0m: Deploy complete

# Broadcast
\033[35m[BROADCAST]\033[0m \033[33magent1\033[0m: Build finished

# System notification
\033[32m[SYSTEM]\033[0m Agent agent2 joined the workspace
```

### 4.2 Injection Mechanism

**Option A: Block Integration (Recommended)**
- Create a new block type: `AgentMessageBlock`
- Inject as a rich block with:
  - Sender avatar/icon
  - Timestamp
  - Message content
  - Action buttons (Reply, Dismiss)

**Option B: Terminal Text Injection**
- Use existing terminal write mechanism
- Inject ANSI-formatted text directly into pty
- Simpler but less rich

### 4.3 Implementation Location
- **File**: `waveterm/pkg/wshutil/wshutil.go` or `waveterm/frontend/app/block/`
- **Method**: Extend `SendCommand` or create `InjectMessage` RPC

---

## 5. Code Reusability with AgentMux

### 5.1 Shared Modules

Create a **shared NPM package** or **Go module** that both projects can use:

**Package Name:** `@agentmux/message-bus-core`

**Exports:**
```typescript
// Types
export type AgentMessage = {
  id: string;
  from: string;
  to: string | null;  // null for broadcasts
  message: string;
  timestamp: string;
  type: MessageType;
  metadata?: Record<string, any>;
};

export type AgentInfo = {
  id: string;
  workspace: string;
  status: AgentStatus;
  lastSeen: Date;
  metadata?: Record<string, any>;
};

// Core classes
export class MessageRouter {
  register(agent: AgentInfo): void;
  unregister(agentId: string): void;
  send(message: AgentMessage): Promise<void>;
  broadcast(message: AgentMessage): Promise<void>;
  getAgents(): AgentInfo[];
}

export class MessageStore {
  save(message: AgentMessage): Promise<void>;
  getHistory(agentId: string, limit?: number): Promise<AgentMessage[]>;
  clear(): Promise<void>;
}
```

### 5.2 Reusable Components

**From AgentMux → WaveTerm:**
- Message routing logic
- Agent registry/discovery
- Message persistence (SQLite schema)
- WebSocket server (for external integrations)

**Shared Between Both:**
- Message types/interfaces
- Agent status enums
- Message priority handling
- Heartbeat/keepalive logic

### 5.3 Integration Points

```
agentmux/
  ├── cli/           # AgentMux CLI
  └── src/
      └── services/
          ├── messageService.ts     ← Uses @agentmux/message-bus-core
          └── agentRegistry.ts      ← Uses @agentmux/message-bus-core

waveterm/
  ├── pkg/mcp-server/
  │   ├── server.go
  │   ├── router.go      ← Go port of MessageRouter
  │   └── registry.go    ← Go port of AgentRegistry
  └── emain/
      └── agent-bus.ts   ← TypeScript wrapper using @agentmux/message-bus-core
```

---

## 6. Implementation Phases

### Phase 1: MCP Server Foundation (Week 1)
- [ ] Create MCP server skeleton in WaveTerm
- [ ] Implement basic agent registry
- [ ] Add stdio/WebSocket communication
- [ ] Test MCP connection from Claude Code

**Deliverable:** Agents can connect to WaveTerm via MCP

### Phase 2: Core Messaging (Week 2)
- [ ] Implement `waveterm_send_message` tool
- [ ] Implement `waveterm_broadcast` tool
- [ ] Implement `waveterm_list_agents` tool
- [ ] Add message routing logic

**Deliverable:** Agents can send/receive messages

### Phase 3: Terminal Integration (Week 3)
- [ ] Build terminal injection mechanism
- [ ] Create agent message block type (or text injection)
- [ ] Add ANSI formatting for messages
- [ ] Handle message arrival in active terminal tabs

**Deliverable:** Messages appear in WaveTerm terminals

### Phase 4: Shared Module Extraction (Week 4)
- [ ] Extract common code into `@agentmux/message-bus-core`
- [ ] Refactor AgentMux to use shared module
- [ ] Refactor WaveTerm to use shared module
- [ ] Create Go bindings for shared logic

**Deliverable:** Both apps use same core logic

### Phase 5: Advanced Features (Week 5-6)
- [ ] Message persistence (SQLite)
- [ ] Message history retrieval
- [ ] Agent status tracking (active/idle/busy)
- [ ] WebSocket server for external tools
- [ ] Agent discovery (automatic registration)

**Deliverable:** Full-featured message bus

### Phase 6: AgentMux Compatibility (Week 7)
- [ ] Bridge WaveTerm MCP server with AgentMux bus
- [ ] Allow AgentMux agents to message WaveTerm agents
- [ ] Test cross-application communication
- [ ] Document integration setup

**Deliverable:** Seamless AgentMux ↔ WaveTerm communication

---

## 7. Configuration

### 7.1 WaveTerm Settings

Add new settings to WaveTerm config:

```json
{
  "agent-bus:enabled": true,
  "agent-bus:port": 9876,
  "agent-bus:protocol": "websocket",
  "agent-bus:persistence": true,
  "agent-bus:heartbeat-interval": 30000,
  "agent-bus:auto-register": true,
  "agent-bus:inject-messages": true,
  "agent-bus:message-format": "rich-block"
}
```

### 7.2 MCP Server Configuration

```json
{
  "mcpServers": {
    "waveterm": {
      "command": "waveterm-mcp-server",
      "args": ["--port", "9876"],
      "env": {
        "WAVETERM_WORKSPACE": "agent1"
      }
    }
  }
}
```

---

## 8. User Experience

### 8.1 Setup Flow

1. **Install WaveTerm Fork** with MCP support
2. **Configure Claude Code** to use WaveTerm MCP server:
   ```bash
   # In Claude Code settings
   Add MCP server: waveterm
   ```
3. **Use tools** in conversation:
   ```
   User: Send a message to agent2 saying "Ready for review"
   Claude: I'll send that message using the WaveTerm message bus.
   [Uses waveterm_send_message tool]
   ```

### 8.2 Agent Workflow

**Scenario: Multi-agent deployment**

**Agent1 (backend):**
```
User: Deploy the API
Agent1: Deploying... [work happens]
Agent1: [Uses waveterm_broadcast("API deployed, run migrations")]
```

**Agent2 (database):**
```
[Receives in terminal]
> [BROADCAST] agent1: API deployed, run migrations

User: Run the migrations
Agent2: Running migrations... [work happens]
Agent2: [Uses waveterm_send_message(to="agent3", message="DB ready")]
```

**Agent3 (frontend):**
```
[Receives in terminal]
> [AGENT-MSG] agent2: DB ready

User: Deploy the frontend
Agent3: Deploying frontend...
```

---

## 9. Technical Considerations

### 9.1 Security
- **Authentication**: Agents must authenticate with MCP server
- **Authorization**: Only allow registered agents to send messages
- **Rate limiting**: Prevent message spam
- **Message validation**: Sanitize content before injection

### 9.2 Performance
- **Message queue**: Use in-memory queue for fast delivery
- **Async injection**: Don't block sender waiting for injection
- **Batch updates**: Coalesce multiple messages if terminal busy
- **TTL**: Expire old messages after configurable timeout

### 9.3 Reliability
- **Reconnection**: Auto-reconnect if MCP connection drops
- **Message buffering**: Queue messages if recipient offline
- **Delivery confirmation**: Track message delivery status
- **Error handling**: Graceful degradation if injection fails

---

## 10. Testing Strategy

### 10.1 Unit Tests
- Message routing logic
- Agent registry operations
- Message persistence
- Terminal injection formatting

### 10.2 Integration Tests
- MCP tool invocation from Claude Code
- Multi-agent message flow
- Cross-tab message delivery
- AgentMux ↔ WaveTerm bridge

### 10.3 E2E Tests
- Full agent workflow (3+ agents)
- Message broadcast to all tabs
- Agent disconnect/reconnect
- Message history persistence

---

## 11. Future Enhancements

### 11.1 Rich Messages
- Markdown rendering in messages
- Code snippet formatting
- File attachments
- Inline actions (buttons, forms)

### 11.2 Agent Collaboration
- Shared workspace state
- File sharing between agents
- Screen sharing / terminal sharing
- Agent handoff protocols

### 11.3 External Integrations
- Slack/Discord notifications
- GitHub webhook triggers
- CI/CD pipeline integration
- Monitoring/alerting systems

---

## 12. Success Metrics

- **Adoption**: % of agents using message bus
- **Reliability**: Message delivery rate (target: >99.9%)
- **Performance**: Message latency (target: <100ms)
- **User satisfaction**: NPS score for cross-agent communication
- **Code reuse**: % of shared code between AgentMux/WaveTerm

---

## 13. References

- **AgentMux README**: `D:/Code/projects/agentmux/README.md`
- **WaveTerm Docs**: `https://docs.waveterm.dev`
- **MCP Specification**: `https://spec.modelcontextprotocol.io`
- **Related Specs**:
  - AgentMux WaveTerm UI Redesign: `agentmux/docs/SPEC_WAVETERM_UI_REDESIGN.md`
  - AgentMux Architecture: `agentmux/docs/ARCHITECTURE.md`

---

**Status:** Ready for review and implementation planning
**Next Steps:** Review with team, prioritize phases, begin Phase 1
