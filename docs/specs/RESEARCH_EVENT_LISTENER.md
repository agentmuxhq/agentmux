# AgentMux Event Listener - Research Summary

## Primary User Story

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  REACTIVE AGENT COMMUNICATION - KEY USE CASE                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  1. AgentX opens a PR                                                       │
│          │                                                                  │
│          ▼                                                                  │
│  2. ReagentX reviews PR, requests changes                                   │
│          │                                                                  │
│          ▼                                                                  │
│  3. GitHub Event: pull_request_review (action: changes_requested)           │
│          │                                                                  │
│          ▼                                                                  │
│  4. Event routed to AgentMux via webhook infrastructure                      │
│          │                                                                  │
│          ▼                                                                  │
│  5. Terminal injection → directs AgentX to address the PR comments          │
│                                                                             │
│  Result: Reactive, autonomous agent response to code review feedback        │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Key GitHub Event:** `pull_request_review` with `action: changes_requested`

**Goal:** When ReagentX requests changes on a PR, the original agent (AgentX) is automatically notified via terminal injection and prompted to address the review comments.

---

## Critical Challenge: Agent-to-Pane Mapping

Multiple agents run simultaneously in different terminal panes. The system needs to route events to the **correct pane**.

### Mapping Options

| Approach | How It Works | Pros | Cons |
|----------|--------------|------|------|
| **Branch-based** | PR branch `agent2/feature` → pane for agent2 | Simple, uses existing convention | Requires branch naming discipline |
| **Registration** | Agent registers pane ID + identity on startup | Explicit, flexible | Requires agent cooperation |
| **CWD-based** | Pane working directory determines agent | Automatic | Fragile if agent changes dirs |
| **Hybrid** | Branch name + fallback to registration | Best of both | More complex |

### Proposed Mapping Table (DynamoDB)

```
AgentMuxAgentRegistry
├── agent_id (PK)     : "agent2"
├── pane_id           : "pane-abc123"
├── workspace_path    : "C:/Code/agent-workspaces/agent2"
├── active_branches   : ["agent2/feature-x", "agent2/fix-bug"]
├── github_user       : "Agent2-asaf"
├── last_heartbeat    : "2025-12-24T10:00:00Z"
└── connection_id     : "ws-connection-xyz"
```

### Event Routing Flow

```
GitHub Event: pull_request_review on branch "agent2/fix-auth"
       │
       ▼
Lambda extracts branch name → "agent2/fix-auth"
       │
       ▼
Parse agent ID from branch prefix → "agent2"
       │
       ▼
Lookup AgentMuxAgentRegistry → pane_id = "pane-abc123"
       │
       ▼
Route to correct WebSocket connection → inject into pane
```

### Registration Trigger Options

1. **wsh hook** - Agent calls `wsh register-agent agent2` on terminal startup
2. **Claude Code hook** - Hook fires on session start, registers automatically
3. **Environment variable** - `WAVEMUX_AGENT_ID=agent2` detected by terminal
4. **AgentMux integration** - AgentMux MCP server registers pane when agent connects

---

## Project Overview

**AgentMux** (v0.12.19) - Terminal multiplexer forked from Wave Terminal
- **Location:** Runs on **gamerlove** (to avoid crashes during development on claudius)
- **Tech:** Electron 38.1.2 + TypeScript/React frontend + Go backend (agentmuxsrv)
- **Current version:** 0.12.19

---

## Specs Found

| Spec | Location | Description |
|------|----------|-------------|
| **SPEC_REACTIVE_AGENT_COMMUNICATION.md** | agentmux root (GitHub main) | **KEY SPEC** - 35KB comprehensive webhook shell injection spec |
| **SPEC_AGENTMUX_INTEGRATION.md** | agentmux root | Agent communication hub with MCP server |
| **wps-events.md** | agentmux/aiprompts/ | WPS (Wave PubSub) internal event system guide |
| **Pulse webhooks.md** | pulse/docs/04-features/github-integration/ | GitHub webhook handler spec (1445 lines) |
| **ReAgent ARCHITECTURE.md** | dev-tools/packages/reagent-worker/ | GitHub router Lambda architecture |

---

## AWS Infrastructure (DEPLOYED)

### CloudFormation Stack
- **Stack Name:** `agentmux-webhook-prod`
- **Status:** CREATE_COMPLETE (deployed 2025-10-29)
- **Region:** us-east-1

### Lambda
| Resource | Value |
|----------|-------|
| Function | `agentmux-webhook-router-prod` |
| Runtime | Python 3.12 |
| Handler | `lambda_function.lambda_handler` |
| Memory | 256 MB |
| Timeout | 30 seconds |

### API Gateway

**HTTP API (Webhooks)**
| Resource | Value |
|----------|-------|
| Name | `agentmux-webhook-http-prod` |
| Endpoint | `https://m6jrh0uo28.execute-api.us-east-1.amazonaws.com` |
| Routes | `/webhook` (POST), `/health` (GET), `/admin/*` |

**WebSocket API (Real-time)**
| Resource | Value |
|----------|-------|
| Name | `agentmux-webhook-ws-prod` |
| Endpoint | `wss://oft9nfu83k.execute-api.us-east-1.amazonaws.com` |
| Routes | `$connect`, `$disconnect`, `$default`, `register`, `heartbeat` |

### DynamoDB Tables

| Table | Purpose |
|-------|---------|
| `AgentMuxWebhookConfig-prod` | Webhook routing configuration |
| `AgentMuxConnections-prod` | Active WebSocket connections |

---

## KEY SPEC: Reactive Agent Communication

From **SPEC_REACTIVE_AGENT_COMMUNICATION.md** (35KB):

### Architecture Overview

```
External Sources (GitHub, CI/CD, etc.)
              │
              ▼
    ┌─────────────────────────┐
    │   AWS Lambda Router     │
    │  (AgentMuxWebhookRouter) │
    └───────────┬─────────────┘
                │
    ┌───────────┴───────────┐
    │                       │
    ▼                       ▼
HTTP API             WebSocket API
(webhook ingress)    (real-time delivery)
    │                       │
    └───────────┬───────────┘
                │
                ▼
    ┌─────────────────────────┐
    │     AgentMux Backend     │
    │    (webhookinjector)    │
    └───────────┬─────────────┘
                │
                ▼
    Terminal Pane (Shell Injection)
```

### Key Components

1. **Persistent Terminal Identity**
   - Each terminal pane gets a unique, persistent ID
   - Survives app restarts
   - Used for webhook routing

2. **Webhook Configuration**
   - Stored in DynamoDB (`AgentMuxWebhookConfig-prod`)
   - Maps external events to terminal panes
   - Supports filters, transformations

3. **Shell Injection**
   - Commands injected directly into terminal input
   - ANSI formatting for visibility
   - Configurable injection modes (echo, execute, notify)

4. **WebSocket Connection**
   - Real-time event delivery
   - Connection tracking in DynamoDB
   - Heartbeat mechanism for connection health

### Implementation Status

| Component | Status | Location |
|-----------|--------|----------|
| Lambda Function | ✅ Deployed | `infra/lambda/` |
| HTTP API | ✅ Deployed | AWS API Gateway |
| WebSocket API | ✅ Deployed | AWS API Gateway |
| DynamoDB Tables | ✅ Deployed | AWS DynamoDB |
| CDK Infrastructure | ✅ Complete | `infra/cdk/` |
| Go Backend Integration | ⏳ Pending | `pkg/webhookinjector/` |
| Frontend UI | ⏳ Pending | `frontend/app/` |

---

## Existing GitHub Router (shared-infrastructure)

Separate from AgentMux webhook system:

- **Function:** `infrastructure-github-router-function`
- **Endpoint:** `https://github-router.asaf.cc/webhook`
- **Repository:** a5af/shared-infrastructure (deployed via CDK)
- **Consumers:**
  1. **Pulse** → DynamoDB (all events)
  2. **ReAgent** → SQS queue (PR events for code review)

---

## WPS (Wave PubSub) - Internal Events

Internal event system for AgentMux components:

- **Broker pattern** for async internal communication
- **Location:** `pkg/wps/` (Go)
- **Features:** Scopes, persistence, wildcards
- **Events:** blockclose, connchange, waveobjupdate, etc.

### Key Events
```go
Event_BlockClose       = "blockclose"
Event_ConnChange       = "connchange"
Event_WaveObjUpdate    = "waveobjupdate"
Event_ControllerStatus = "controllerstatus"
Event_WaveAIRateLimit  = "waveai:ratelimit"
```

---

## infra/ Folder Contents (GitHub main)

Located at `agentmux/infra/`:

| File/Folder | Description |
|-------------|-------------|
| `README.md` | Infrastructure overview |
| `AWS_RESOURCES.md` | Deployed resource reference |
| `DEPLOYMENT.md` | Deployment instructions |
| `DEPLOYMENT_OUTPUTS.md` | CDK output values |
| `DEPLOYMENT_SUCCESS.md` | Deployment verification |
| `SECRETS_SETUP.md` | Secret configuration |
| `TEST_RESULTS.md` | Integration test results |
| `cdk/` | CDK infrastructure code |
| `lambda/` | Lambda function code |
| `scripts/` | Deployment/test scripts |

---

## Next Steps

### Immediate (AWS is ready)
1. **Sync local worktree** - Pull main to get `infra/` and `SPEC_REACTIVE_AGENT_COMMUNICATION.md`
2. **Implement Go backend** - `pkg/webhookinjector/` module
3. **Frontend UI** - Webhook configuration panel

### Backend Integration Points
- WebSocket client in agentmuxsrv to connect to `wss://oft9nfu83k.execute-api.us-east-1.amazonaws.com`
- Shell injection via existing pty infrastructure
- WPS event publishing for UI updates

### Frontend Integration Points
- Configuration modal for webhook setup
- Event log/history view
- Connection status indicator

---

## Quick Commands

```bash
# Test webhook endpoint
curl -X POST https://m6jrh0uo28.execute-api.us-east-1.amazonaws.com/webhook \
  -H "Content-Type: application/json" \
  -d '{"event": "test", "data": {"message": "hello"}}'

# Check health
curl https://m6jrh0uo28.execute-api.us-east-1.amazonaws.com/health

# View Lambda logs
aws logs tail /aws/lambda/agentmux-webhook-router-prod --follow
```

---

## References

- **Main Spec:** SPEC_REACTIVE_AGENT_COMMUNICATION.md (on GitHub main)
- **AgentMux Spec:** SPEC_AGENTMUX_INTEGRATION.md
- **WPS Guide:** aiprompts/wps-events.md
- **Pulse Webhooks:** pulse/docs/04-features/github-integration/webhooks.md
