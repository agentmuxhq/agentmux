# Cross-Host Reactive Messaging & Event Ingestion

**Author:** AgentA
**Date:** 2026-01-16
**Status:** Draft
**Last Updated:** 2026-01-16

## Overview

Extend the AgentMux reactive messaging system to support:
1. **Cross-host agent communication** - Agents on different machines can inject messages into each other's terminals
2. **Async event ingestion** - External events (GitHub webhooks, CI/CD, monitoring alerts) can route messages to agents

## Current Architecture

### AgentMux (Localhost)
```
┌─────────────────────────────────────────────────────────────┐
│ AgentMux Instance (area54)                                   │
│                                                             │
│  ┌─────────┐   OSC 16162   ┌──────────────┐                │
│  │ Agent1  │──────────────►│ termwrap.ts  │                │
│  │ (Claude)│               │              │                │
│  └─────────┘               │   register   │                │
│                            │      ▼       │                │
│  ┌─────────┐   OSC 16162   │ ┌──────────┐ │   inject       │
│  │ Agent2  │──────────────►│ │ reactive │◄├───────────────┐│
│  │ (Claude)│               │ │ backend  │ │               ││
│  └─────────┘               │ └──────────┘ │               ││
│                            └──────────────┘               ││
│                                                           ││
│  ┌─────────┐                                              ││
│  │ AgentA  │──── curl localhost:port/wave/reactive/inject─┘│
│  │ (Claude)│                                               │
│  └─────────┘                                               │
└─────────────────────────────────────────────────────────────┘
```

**Limitation:** Injection only works within the same AgentMux instance.

### AgentMux (Cloud)
```
┌────────────────────────────────────────────────────────────────┐
│ AgentMux Lambda (agentmux.asaf.cc)                             │
│                                                                │
│  ┌─────────────────┐                                           │
│  │ HTTP API        │◄──── Bearer token auth                    │
│  │ /mcp            │                                           │
│  └────────┬────────┘                                           │
│           │                                                    │
│           ▼                                                    │
│  ┌─────────────────┐    ┌─────────────────┐                   │
│  │ Messages Table  │    │ Agents Table    │                   │
│  │ (DynamoDB)      │    │ (DynamoDB)      │                   │
│  └─────────────────┘    └─────────────────┘                   │
└────────────────────────────────────────────────────────────────┘
```

**Current tools:** send_message, read_messages, list_agents, broadcast_message, delete_messages

## Infrastructure Decision: Consolidate into AgentMux

### Background

During infrastructure exploration, found a separate `agentmux-webhook-prod` CloudFormation stack deployed Oct 2025:
- `AgentMuxConnections-prod` table (empty)
- `AgentMuxWebhookConfig-prod` table (empty)
- `agentmux-webhook-router-prod` Lambda (unused)
- HTTP + WebSocket API Gateways

**Decision:** Discard the separate agentmux stack and build all cloud functionality in AgentMux.

### Rationale

| Factor | Separate Stack | Consolidated AgentMux |
|--------|----------------|----------------------|
| Tables with data | 0 items | Active (agents, messages) |
| Agent registry | None | ✅ Already exists |
| Auth model | Custom HMAC | Bearer token (established) |
| Maintenance | Two stacks | Single stack |
| Code reuse | Separate Python | Extend existing Node.js |

### Action Items

1. ~~Delete `agentmux-webhook-prod` CloudFormation stack~~ ✅ Deleted
2. Add `/reactive/*` endpoints to `agentmux-server` Lambda
3. Add `/webhook/*` endpoints to `agentmux-server` Lambda
4. Port webhook validation logic from Python to Node.js

### Active Infrastructure (AgentMux)

| Resource | Purpose | Status |
|----------|---------|--------|
| `agentmux-server` | Main Lambda - MCP API + new endpoints | Active |
| `agentmux-agents-prod` | Agent registry | Active |
| `agentmux-messages-prod` | Async messages (GSI: `to_agent`+`timestamp`) | Active |
| `agentmux-injections-prod` | Cross-host injection queue | **To create** |

## Proposed Architecture

### Phase 1: Cross-Host Reactive Relay

```
┌─────────────────┐         ┌─────────────────┐         ┌─────────────────┐
│ AgentMux (area54)│         │ AgentMux Lambda │         │ AgentMux (claud.)│
│                 │         │                 │         │                 │
│ AgentA ─────────┼──inject─┼►Pending Queue   │         │                 │
│                 │         │  (DynamoDB)     │◄──poll──┼─ AgentX         │
│                 │         │                 │         │                 │
└─────────────────┘         └─────────────────┘         └─────────────────┘
```

**Flow:**
1. AgentA calls `inject_terminal` targeting AgentX
2. agentmux-client detects AgentX is not local
3. Injection request is sent to AgentMux Lambda
4. Lambda stores in `agentmux-injections-prod` table
5. AgentMux (claudius) polls for pending injections
6. AgentMux executes local injection to AgentX's terminal

### Phase 2: Real-Time Delivery (WebSocket)

```
┌─────────────────┐         ┌─────────────────┐         ┌─────────────────┐
│ AgentMux (area54)│         │ Bastion Host    │         │ AgentMux (claud.)│
│                 │   WS    │ (wss://8443)    │   WS    │                 │
│ AgentA ─────────┼────────►│                 │────────►│ AgentX          │
│                 │         │ WebSocket Hub   │         │                 │
│                 │◄────────┤                 │◄────────┤                 │
└─────────────────┘         └─────────────────┘         └─────────────────┘
```

**Benefits over polling:**
- Sub-second delivery latency
- Reduced Lambda invocations
- Bidirectional communication

**Note:** Bastion already has port 8443 open for this purpose.

### Phase 3: Event Ingestion (Webhooks)

```
┌──────────────┐    ┌─────────────────┐    ┌─────────────────┐
│ GitHub       │    │ AgentMux Lambda │    │ AgentMux         │
│ Webhook      │───►│ /webhook/github │───►│ (target agent)  │
└──────────────┘    └─────────────────┘    └─────────────────┘

┌──────────────┐    ┌─────────────────┐    ┌─────────────────┐
│ CI/CD        │    │ AgentMux Lambda │    │ AgentMux         │
│ (Jenkins/GH) │───►│ /webhook/ci     │───►│ (target agent)  │
└──────────────┘    └─────────────────┘    └─────────────────┘

┌──────────────┐    ┌─────────────────┐    ┌─────────────────┐
│ Monitoring   │    │ AgentMux Lambda │    │ AgentMux         │
│ (PagerDuty)  │───►│ /webhook/alert  │───►│ (on-call agent) │
└──────────────┘    └─────────────────┘    └─────────────────┘
```

## Data Models

### New DynamoDB Table: `agentmux-injections-prod`

```typescript
interface InjectionRecord {
  id: string;                    // UUID
  target_agent: string;          // Agent ID to receive injection
  source_agent: string;          // Agent ID that sent it
  message: string;               // Message content
  priority: 'normal' | 'urgent'; // Delivery priority
  status: 'pending' | 'delivered' | 'failed' | 'expired';
  created_at: string;            // ISO timestamp
  delivered_at?: string;         // When delivered
  ttl: number;                   // DynamoDB TTL (epoch seconds)
  metadata?: {
    source_type: 'agent' | 'webhook' | 'system';
    webhook_type?: string;       // github, ci, alert, etc.
    original_event?: object;     // Raw webhook payload
  };
}

// GSI: target_agent-status-index
// Partition: target_agent
// Sort: created_at
// Filter: status = 'pending'
```

### New DynamoDB Table: `agentmux-webhooks-prod`

```typescript
interface WebhookConfig {
  id: string;                    // Webhook endpoint ID
  type: 'github' | 'gitlab' | 'ci' | 'alert' | 'custom';
  secret: string;                // HMAC secret for validation
  target_agent: string;          // Default agent to receive events
  routing_rules?: RoutingRule[]; // Optional event-based routing
  enabled: boolean;
  created_at: string;
}

interface RoutingRule {
  match: {
    event_type?: string;         // e.g., 'pull_request', 'push'
    branch?: string;             // e.g., 'main', 'feature/*'
    action?: string;             // e.g., 'opened', 'merged'
  };
  target_agent: string;          // Route to this agent
  message_template?: string;     // Optional custom message format
}
```

## API Design

### Lambda Endpoints

#### POST /reactive/inject
Relay injection to remote AgentMux instance.

```typescript
// Request
{
  target_agent: string;
  message: string;
  source_agent: string;
  priority?: 'normal' | 'urgent';
}

// Response
{
  success: boolean;
  injection_id: string;
  status: 'queued' | 'delivered';  // delivered if target is connected via WS
}
```

#### GET /reactive/pending/{agent_id}
Poll for pending injections (used by AgentMux).

```typescript
// Response
{
  injections: [{
    id: string;
    message: string;
    source_agent: string;
    priority: string;
    created_at: string;
  }];
}
```

#### POST /reactive/ack
Acknowledge injection delivery.

```typescript
// Request
{
  injection_ids: string[];
}
```

#### POST /webhook/{type}
Receive external webhooks.

```typescript
// GitHub example
POST /webhook/github
X-Hub-Signature-256: sha256=...
X-GitHub-Event: pull_request

{
  action: "opened",
  pull_request: { ... },
  repository: { ... }
}

// Response
{
  success: boolean;
  injection_id?: string;
  routed_to?: string;
}
```

### AgentMux Polling Service

```go
// pkg/reactive/poller.go

type Poller struct {
    agentmuxURL string
    agentmuxToken string
    pollInterval time.Duration
    handler *Handler
}

func (p *Poller) Start() {
    ticker := time.NewTicker(p.pollInterval)
    for range ticker.C {
        p.pollAndInject()
    }
}

func (p *Poller) pollAndInject() {
    // Get registered agents
    agents := p.handler.ListAgents()

    for _, agent := range agents {
        // Poll for pending injections
        pending := p.fetchPending(agent.AgentID)

        for _, injection := range pending {
            // Execute local injection
            p.handler.InjectMessage(InjectionRequest{
                TargetAgentID: agent.AgentID,
                Message: injection.Message,
                SourceAgent: injection.SourceAgent,
            })

            // Acknowledge delivery
            p.ackDelivery(injection.ID)
        }
    }
}
```

## Implementation Plan

### Phase 1: Cross-Host Polling (MVP)

**Note:** Leverages existing `agentmux-server` Lambda - just add new endpoints.

| Task | Owner | Details |
|------|-------|---------|
| Add `agentmux-injections-prod` DynamoDB table | AgentA/X | New table with GSI on target_agent |
| Add `/reactive/inject` to agentmux-server | AgentX | Store injection request, return ID |
| Add `/reactive/pending/{agent}` to agentmux-server | AgentX | Query pending injections for agent |
| Add `/reactive/ack` to agentmux-server | AgentX | Mark injections as delivered |
| Implement AgentMux polling service | AgentA | New `pkg/reactive/poller.go` |
| Update inject_terminal MCP tool | AgentA | Route to Lambda if target not local |
| Integration testing | Both | Cross-host injection test |

**Deliverables:**
- Cross-host injection works via polling
- 5-10 second delivery latency (configurable poll interval)

### Phase 2: WebSocket Real-Time (Optional)

**Note:** `AgentMuxConnections-prod` table already exists.

| Task | Owner | Details |
|------|-------|---------|
| Design WebSocket protocol | AgentA | Message format, heartbeat, reconnect |
| Implement bastion WebSocket hub | AgentX | Node.js on port 8443 (already open) |
| Implement AgentMux WebSocket client | AgentA | Connect on startup, auto-reconnect |
| Fallback to polling when WS unavailable | AgentA | Graceful degradation |
| Testing and hardening | Both | Connection drops, reconnect, load |

**Deliverables:**
- Sub-second delivery latency
- Graceful fallback to polling

### Phase 3: Webhook Ingestion

**Note:** Build webhook endpoints in `agentmux-server`. Port validation logic from `agentmux/infra/lambda/webhook-router/handler.py`.

| Task | Owner | Details |
|------|-------|---------|
| Add `agentmux-webhooks-prod` DynamoDB table | AgentX | Subscription storage |
| Add `/webhook/{provider}` to agentmux-server | AgentX | GitHub, GitLab, CI, alerts |
| Port webhook signature validation | AgentX | HMAC-SHA256 from Python → Node.js |
| Port template engine | AgentX | `{{path.to.value}}` rendering |
| Port filter matching | AgentX | Wildcards, lists, exact match |
| Create subscription management API | AgentX | `/webhook/subscribe`, `/webhook/unsubscribe` |
| Documentation and examples | Both | Setup guides for GitHub, etc. |

**Code to Port** (from `agentmux/infra/lambda/webhook-router/handler.py`):
- `validate_webhook_signature()` - GitHub HMAC validation
- `render_command_template()` - Template engine
- `matches_filters()` - Filter matching
- `extract_event_type()` - Provider-specific event extraction

**Deliverables:**
- GitHub webhook integration
- Extensible to other webhook sources
- Configurable routing rules

## Security Considerations

### Authentication
- **Agent-to-Lambda:** Existing Bearer token auth
- **AgentMux-to-Lambda:** Same Bearer token (via env var)
- **Webhooks:** HMAC signature validation per webhook type

### Authorization
- Agents can only inject to agents they have permission to message
- Consider adding ACL rules: `agent_a can_inject agent_b`

### Rate Limiting
- Lambda: API Gateway throttling (if added)
- AgentMux: Max injections per second per agent

### Message Sanitization
- All messages sanitized before injection (existing code)
- Webhook payloads filtered to safe fields only

## Configuration

### AgentMux Settings

```json
// ~/.config/waveterm/settings.json
{
  "reactive": {
    "enabled": true,
    "crossHost": {
      "enabled": true,
      "pollIntervalMs": 5000,
      "agentmuxUrl": "https://agentmux.asaf.cc",
      "agentmuxToken": "${AGENTMUX_TOKEN}"
    }
  }
}
```

### Environment Variables

| Variable | Description |
|----------|-------------|
| `AGENTMUX_URL` | AgentMux Lambda URL (required for cross-host) |
| `AGENTMUX_TOKEN` | Bearer token for auth (required for cross-host) |
| `WAVEMUX_REACTIVE_POLL_INTERVAL` | Polling interval (e.g., "5s", "30s") |

## Cost Estimates

### Lambda Invocation Costs

The poller calls the Lambda function at a configurable interval (default 5s). Cost depends on:
- Number of AgentMux instances polling
- Number of registered agents per instance
- Poll frequency

**AWS Lambda Pricing (us-east-1, 2026):**
- $0.20 per 1M requests
- $0.0000166667 per GB-second (512MB = ~$0.00000833/s)

**Cost Calculation:**

| Poll Interval | Polls/Hour | Polls/Day | Polls/Month | Monthly Cost (1 agent) |
|---------------|------------|-----------|-------------|------------------------|
| 5s | 720 | 17,280 | 518,400 | ~$0.10 |
| 10s | 360 | 8,640 | 259,200 | ~$0.05 |
| 30s | 120 | 2,880 | 86,400 | ~$0.02 |
| 60s | 60 | 1,440 | 43,200 | ~$0.01 |

**Multi-Agent Example (3 AgentMux instances, 2 agents each = 6 agents polling):**

| Poll Interval | Monthly Requests | Monthly Cost |
|---------------|------------------|--------------|
| 5s | 3,110,400 | ~$0.62 |
| 30s | 518,400 | ~$0.10 |

**DynamoDB Costs:**
- Read capacity: ~0.5 RCU per poll (negligible with on-demand)
- Write capacity: Only on actual injections (rare)
- Storage: ~$0.25/GB (minimal for injection queue)

**Recommendation:**
- Default 5s is fine for development/small teams (~$0.62/month for 6 agents)
- For large deployments, increase to 30s+ or implement WebSocket (Phase 2)
- Set `WAVEMUX_REACTIVE_POLL_INTERVAL=30s` to reduce costs by 6x

## Open Questions

1. **WebSocket vs Polling:** Is sub-second latency needed? Polling is simpler.
2. **Webhook routing:** Should routing rules be stored in DynamoDB or config file?
3. **Message retention:** How long to keep pending injections? (Currently: 1 hour TTL)
4. **Agent discovery:** Should agents auto-register with AgentMux for cross-host routing?
5. **Offline agents:** Queue messages for offline agents? For how long?

## References

- [AgentMux Reactive Handler](../pkg/reactive/handler.go)
- [AgentMux Infrastructure](https://github.com/a5af/agentmux/tree/main/infrastructure)
- [AWS WebSocket API Best Practices](https://www.cloudthat.com/resources/blog/scalable-real-time-communication-using-aws-websocket-apis-and-aws-lambda)
- [Serverless GitHub Webhooks](https://www.serverless.com/blog/serverless-github-webhook-slack)
- [MCP Streamable HTTP Spec](https://modelcontextprotocol.io/docs/concepts/transports)
