# Webhook Delivery - Phase 2 Implementation

**Status:** ✅ Implemented and Ready for Testing

This package implements Phase 2 of the AgentMux reactive agent communication system, providing WebSocket client integration for webhook-based command injection into terminals.

---

## Overview

The webhook delivery system connects AgentMux terminals to the AWS-hosted webhook infrastructure, enabling real-time command injection from external webhooks (GitHub, CI/CD, monitoring systems, etc.).

## Architecture

```
GitHub/CI/CD Webhook
    │
    ├─> AWS API Gateway (HTTP)
    │       └─> Lambda (webhook-router)
    │               └─> DynamoDB (subscriptions)
    │                       └─> API Gateway (WebSocket)
    │                               │
    │                               └─> AgentMux WebSocket Client (this package)
    │                                       └─> Block Controller
    │                                               └─> Terminal PTY
```

---

## Components

### 1. Configuration (`webhookconfig.go`)

Manages webhook integration configuration stored in `~/.config/waveterm/webhook-config.json`:

```json
{
  "version": "1.0",
  "workspaceId": "agent2-workspace",
  "authToken": "generated-hmac-token",
  "cloudEndpoint": "wss://oft9nfu83k.execute-api.us-east-1.amazonaws.com/prod",
  "enabled": true,
  "terminals": ["terminal-uuid-1", "terminal-uuid-2"]
}
```

**Key Functions:**
- `ReadWebhookConfig()` - Load configuration from disk
- `WriteWebhookConfig(config)` - Save configuration to disk
- `config.IsTerminalSubscribed(terminalId)` - Check subscription status
- `config.Validate()` - Validate configuration

### 2. WebSocket Client (`client.go`)

Manages WebSocket connection to AWS API Gateway with automatic reconnection:

**Features:**
- Exponential backoff reconnection (1s → 2m)
- Ping/pong keepalive (50s interval)
- Automatic message routing
- Thread-safe connection management

**Connection Flow:**
1. Parse cloud endpoint URL
2. Add query parameters: `?workspaceId=<id>&token=<token>`
3. Establish WebSocket connection
4. Start read/write/ping loops
5. Handle incoming webhook events
6. Automatic reconnection on disconnection

**Key Functions:**
- `NewWebhookClient(config, handler)` - Create client
- `client.Start()` - Begin connection loop
- `client.Stop()` - Graceful shutdown
- `client.IsConnected()` - Connection status

### 3. Integration Layer (`integration.go`)

Integrates webhook delivery with AgentMux block controllers:

**Responsibilities:**
- Initialize webhook service on startup
- Subscribe to block controller lifecycle events
- Maintain terminal ID → block ID mapping
- Inject commands into terminal PTYs
- Handle webhook events and route to terminals

**Key Functions:**
- `InitializeWebhookService()` - Initialize global service
- `GetWebhookService()` - Get service instance
- `ShutdownWebhookService()` - Graceful shutdown
- `service.RegisterTerminal(terminalId, blockId)` - Manual registration
- `service.GetStatus()` - Service status

---

## Integration Points

### Main Server (`cmd/server/main-server.go`)

**Startup** (line ~424):
```go
// Initialize webhook service for reactive agent communication
go func() {
    defer func() {
        panichandler.PanicHandler("InitWebhookService", recover())
    }()
    if err := webhookdelivery.InitializeWebhookService(); err != nil {
        log.Printf("warning: failed to initialize webhook service: %v\n", err)
    }
}()
```

**Shutdown** (line ~75):
```go
// Shutdown webhook service
webhookdelivery.ShutdownWebhookService()
```

---

## Configuration Setup

### 1. Generate Authentication Token

```bash
# Get the DEFAULT_AUTH_SECRET from AWS Secrets Manager
DEFAULT_SECRET=$(aws secretsmanager get-secret-value \
  --secret-id services/prod \
  --profile Agent2 \
  --query SecretString \
  --output text | jq -r '.agentmux.DEFAULT_AUTH_SECRET')

# Generate token for workspace
WORKSPACE_ID="agent2-workspace"
AUTH_TOKEN=$(echo -n "$WORKSPACE_ID" | openssl dgst -sha256 -hmac "$DEFAULT_SECRET" | sed 's/^.* //')

echo "Auth Token: $AUTH_TOKEN"
```

### 2. Create Configuration File

Create `~/.config/waveterm/webhook-config.json`:

```json
{
  "version": "1.0",
  "workspaceId": "agent2-workspace",
  "authToken": "<generated-token-from-above>",
  "cloudEndpoint": "wss://oft9nfu83k.execute-api.us-east-1.amazonaws.com/prod",
  "enabled": true,
  "terminals": []
}
```

### 3. Restart AgentMux

The webhook service will automatically initialize on startup if `enabled: true`.

---

## Usage

### Register a Terminal for Webhook Delivery

Terminals need to be subscribed to receive webhook events:

**Option 1: Via Configuration File**

Edit `webhook-config.json` and add terminal UUIDs:

```json
{
  "terminals": ["block-uuid-1", "block-uuid-2"]
}
```

**Option 2: Via Service API** (Future: Add RPC command)

```go
service := webhookdelivery.GetWebhookService()
service.RegisterTerminal("block-uuid-1", "block-uuid-1")
```

### Register Subscription in AWS

Use the HTTP API to register a webhook subscription:

```bash
curl -X POST https://m6jrh0uo28.execute-api.us-east-1.amazonaws.com/register \
  -H "Content-Type: application/json" \
  -d '{
    "workspaceId": "agent2-workspace",
    "terminalId": "block-uuid-1",
    "provider": "github",
    "eventType": "pull_request",
    "filters": {
      "action": ["opened", "synchronize", "closed"]
    },
    "commandTemplate": "echo \"PR #{{pull_request.number}} {{action}} by {{pull_request.user.login}}\""
  }'
```

---

## Event Flow

### 1. Webhook Received by AWS

1. GitHub sends webhook to `POST /webhook/github`
2. Lambda validates HMAC signature
3. Lambda extracts event data (PR number, action, user, etc.)
4. Lambda queries DynamoDB for matching subscriptions
5. Lambda finds subscription for `terminalId=block-uuid-1`
6. Lambda renders command template with event data

### 2. Command Delivery to AgentMux

1. Lambda sends WebSocket message to connected client
2. AgentMux client receives message in `readLoop()`
3. Client parses `WebhookEvent` JSON
4. Client calls `handleWebhookEvent(event)`
5. Service checks terminal subscription
6. Service looks up block ID from terminal map
7. Service calls `injectCommand(blockId, command)`

### 3. Command Injection into Terminal

1. `injectCommand()` creates `BlockInputUnion`
2. Adds newline to command for execution
3. Calls `blockcontroller.SendInputToBlock(blockId, input)`
4. Block controller forwards to shell controller
5. Shell controller writes to PTY stdin
6. Command executes in terminal

---

## Monitoring

### Check Service Status

```go
service := webhookdelivery.GetWebhookService()
if service != nil {
    status := service.GetStatus()
    fmt.Printf("Webhook Service Status:\n")
    fmt.Printf("  Enabled: %v\n", status["enabled"])
    fmt.Printf("  Connected: %v\n", status["connected"])
    fmt.Printf("  Workspace: %v\n", status["workspaceId"])
    fmt.Printf("  Endpoint: %v\n", status["endpoint"])
    fmt.Printf("  Terminals: %v\n", status["terminalCount"])
}
```

### Logs

```bash
# AgentMux logs (stdout/stderr)
[WebhookService] Initializing webhook service
[WebhookService] Webhook service initialized successfully
[WebhookClient] Starting webhook client for workspace: agent2-workspace
[WebhookClient] Connecting to: wss://...
[WebhookClient] Connected successfully
[WebhookService] Registered terminal block-uuid-1 -> block block-uuid-1
[WebhookClient] Received webhook event: provider=github, type=pull_request, terminalId=block-uuid-1
[WebhookService] Injecting command into block block-uuid-1: echo "PR #123 opened by user"
[WebhookService] Command injected successfully
```

---

## Error Handling

### Connection Failures

- **Automatic reconnection** with exponential backoff
- Starts at 1 second delay
- Increases by 2x each attempt
- Caps at 2 minutes
- Continues indefinitely until connected

### Authentication Errors

- Invalid token → Connection refused (401)
- Check `authToken` in configuration
- Regenerate token with correct workspace ID

### Command Injection Errors

- Block not found → Log error, ignore event
- Terminal not subscribed → Log warning, ignore event
- PTY closed → Log error, block controller handles

---

## Security

### Authentication

- HMAC-SHA256 token generation
- Token = HMAC(workspaceId, DEFAULT_AUTH_SECRET)
- Token validated by AWS Lambda on WebSocket connection

### Connection Security

- TLS/WSS encrypted WebSocket connection
- AWS API Gateway handles TLS termination
- No plaintext credentials in transit

### Isolation

- Each workspace has unique authentication token
- Commands only delivered to subscribed terminals
- Workspace isolation enforced by token validation

---

## Future Enhancements

### Phase 3: Frontend UI

- [ ] Configuration modal in React
- [ ] Subscription list view
- [ ] Terminal webhook assignment UI
- [ ] GitHub webhook setup wizard

### Additional Features

- [ ] RPC commands for configuration management
- [ ] Terminal metadata for persistent terminal IDs
- [ ] Event filtering UI
- [ ] Webhook delivery metrics
- [ ] Rate limiting per terminal
- [ ] Command queuing for disconnected terminals

---

## Testing

### Unit Tests (Future)

```go
func TestWebhookConfig(t *testing.T)
func TestWebSocketClient(t *testing.T)
func TestCommandInjection(t *testing.T)
```

### Integration Testing

1. Start AgentMux with webhook config enabled
2. Open terminal and note block UUID
3. Register subscription via HTTP API
4. Trigger GitHub webhook
5. Verify command appears in terminal

---

## Dependencies

- **gorilla/websocket** - WebSocket client library
- **pkg/blockcontroller** - Terminal PTY management
- **pkg/wconfig** - Configuration file handling
- **pkg/wps** - Event bus for block lifecycle events

---

## Files

- `webhookconfig.go` - Configuration types and I/O
- `client.go` - WebSocket client with reconnection
- `integration.go` - AgentMux block controller integration
- `README.md` - This file

---

**Last Updated:** October 29, 2025
**Status:** Ready for testing with deployed AWS infrastructure
