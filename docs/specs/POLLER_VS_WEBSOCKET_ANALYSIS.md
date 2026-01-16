# WaveMux Cross-Host Messaging: Poller vs WebSocket Analysis

**Date:** 2026-01-16
**Author:** AgentA
**Status:** Research Report

---

## Executive Summary

This report analyzes the current HTTP polling approach vs WebSocket for cross-host reactive messaging in WaveMux. The key finding: **WaveMux already has a sophisticated WebSocket infrastructure** that could be leveraged, but the complexity of extending it to AgentMux cloud relay makes polling the pragmatic choice for v1.

---

## Question 1: How Does the Poller Run?

### Current Architecture

The poller runs **inside wavemuxsrv** (the Go backend process), not as a separate OS process.

```
WaveMux.exe (Electron)
    └── spawns wavemuxsrv (Go backend)
            └── runs Poller goroutine
                    └── HTTP polls AgentMux every 5s
```

**Code Location:** `pkg/reactive/poller.go`

**Lifecycle:**
1. `wavemuxsrv` starts during WaveMux launch
2. `StartGlobalPoller()` called during server initialization
3. Poller runs as a background goroutine with `go p.pollLoop()`
4. Polls every 5 seconds using `time.Ticker`
5. Stops gracefully when WaveMux closes

**Pros of this approach:**
- No separate process to manage
- Shares memory/resources with backend
- Clean lifecycle management
- Access to local agent registry

---

## Question 2: Is WebSocket More Efficient?

### Yes, significantly.

| Metric | HTTP Polling (5s) | WebSocket |
|--------|-------------------|-----------|
| **Connections/hour** | 720 | 1 (persistent) |
| **Latency** | 0-5s (avg 2.5s) | ~50-100ms |
| **Bandwidth overhead** | High (headers each request) | Low (frames only) |
| **Server resources** | New connection each poll | Single long-lived connection |
| **Battery/CPU** | Constant activity | Event-driven, idle when quiet |

### Efficiency Comparison

**HTTP Polling (current):**
```
Every 5 seconds:
  → TCP handshake (3 packets)
  → TLS handshake (2-4 round trips)
  → HTTP request (~500 bytes headers)
  → HTTP response (~200 bytes minimum)
  → Connection teardown

Total: ~1KB+ per poll, 100-300ms latency per request
```

**WebSocket (alternative):**
```
Once at startup:
  → TCP + TLS handshake
  → WebSocket upgrade

Then for each message:
  → 2-14 bytes frame header + payload

Total: ~50 bytes per message, <100ms latency
```

### Real-World Impact

For 6 agents polling at 5s intervals:
- **Polling:** 720 requests/hour/agent = 4,320 requests/hour total
- **WebSocket:** 6 persistent connections, messages only when needed

---

## Question 3: How Difficult Would WebSocket Be?

### Difficulty: **Medium-High** (2-3 days of work)

### Required Changes

**1. AgentMux Server (Lambda → Always-On)**
```
Current:  Lambda functions (stateless, HTTP-triggered)
Required: Always-on server with WebSocket support

Options:
  a) AWS API Gateway WebSocket API + Lambda
  b) EC2/ECS with Node.js WebSocket server
  c) Cloudflare Workers with Durable Objects
```

**2. New AgentMux WebSocket Endpoints**
```go
// Pseudo-code for AgentMux WebSocket server
ws.on("connect", (conn) => {
    agentId := conn.query.agent_id
    token := conn.headers.authorization
    // Validate token
    // Register connection in map[agentId]Connection
})

ws.on("disconnect", (conn) => {
    // Remove from connection map
})

// When injection arrives:
func routeInjection(injection) {
    if conn := connections[injection.target_agent]; conn != nil {
        conn.send(injection)  // Direct push!
    } else {
        // Store in DynamoDB for later polling
    }
}
```

**3. WaveMux Poller → WebSocket Client**
```go
// Replace poller.go with wsclient.go
func (c *WSClient) Connect() error {
    conn, _, err := websocket.DefaultDialer.Dial(c.agentmuxWSURL, headers)
    c.conn = conn
    go c.readLoop()   // Handle incoming injections
    go c.pingLoop()   // Keep-alive
    return nil
}

func (c *WSClient) readLoop() {
    for {
        _, message, err := c.conn.ReadMessage()
        // Parse injection, deliver locally
        c.handler.InjectMessage(injection)
    }
}
```

**4. Reconnection Logic**
```go
func (c *WSClient) reconnectLoop() {
    backoff := []time.Duration{0, 2*time.Second, 5*time.Second, 10*time.Second, 30*time.Second}
    for attempt := 0; ; attempt++ {
        if err := c.Connect(); err == nil {
            return
        }
        time.Sleep(backoff[min(attempt, len(backoff)-1)])
    }
}
```

### Complexity Breakdown

| Component | Effort | Notes |
|-----------|--------|-------|
| AgentMux WebSocket server | 1 day | Need always-on infrastructure |
| Connection management | 0.5 day | Map of agent → connection |
| WaveMux WebSocket client | 0.5 day | Replace poller with WS client |
| Reconnection/resilience | 0.5 day | Backoff, heartbeat, error handling |
| Testing | 0.5 day | Multi-agent, disconnect scenarios |
| **Total** | **2-3 days** | |

### Infrastructure Cost Change

| Approach | Monthly Cost (estimate) |
|----------|------------------------|
| Lambda polling | ~$0.62 (6 agents, 5s poll) |
| API Gateway WebSocket | ~$1-3 (connection hours + messages) |
| EC2 t3.micro | ~$8-10 (always-on) |
| Cloudflare Workers | ~$5 (with Durable Objects) |

---

## Question 4: Existing WebSocket Infrastructure in WaveMux

### Yes! WaveMux has extensive WebSocket support.

**Architecture:**
```
Electron Frontend (TypeScript)
         ↓ WebSocket (/ws endpoint)
Go Backend (wavemuxsrv)
         ↓ RPC Router
Command Handlers + Event Broker
```

### Key Components

**1. WebSocket Server** (`pkg/web/ws.go`)
- HTTP → WebSocket upgrade at `/ws`
- Auth token validation
- Read/write loops with ping/pong
- Route-based message routing

**2. RPC System** (`pkg/wshutil/wshrpc.go`)
- Request/response correlation via ReqId/ResId
- Streaming support for large data
- 100+ defined RPC commands
- Bidirectional communication

**3. Pub/Sub Broker** (`pkg/wps/wps.go`)
- Event types: blockfile, connchange, waveobj:update, etc.
- Scope-based filtering with wildcards
- Route-targeted delivery
- Event history persistence

**4. Frontend Client** (`frontend/app/store/ws.ts`)
- `WSControl` class manages connection
- Reconnection with exponential backoff
- Message queue (5MB max)
- Ping/pong keep-alive (5s)

### Could We Reuse This?

**For local injection delivery: YES**

The current implementation already uses this:
```go
// In handler.go - InjectMessage uses the existing event system
result := p.handler.InjectMessage(InjectionRequest{...})
```

**For AgentMux cloud connection: PARTIALLY**

We could add a new "agentmux" route to the existing router:
```go
// Hypothetical: register AgentMux as a remote route
wshrouter.RegisterRoute("agentmux", agentmuxWSConn)

// Then injections could flow through existing RPC system
```

However, this would require:
- AgentMux to speak the WaveMux RPC protocol
- Managing a separate outbound WebSocket connection
- Coordinating auth differently (bearer token vs WaveMux auth)

---

## Recommendation

### Short-term (Current): Keep HTTP Polling

**Rationale:**
- Already working and tested
- Lambda is serverless (no infrastructure to maintain)
- Cost is minimal (~$0.62/month for 6 agents)
- 5-second latency is acceptable for agent-to-agent messaging

### Medium-term (If Needed): API Gateway WebSocket

**When to upgrade:**
- If we need sub-second message delivery
- If we have 50+ agents (polling becomes expensive)
- If we add real-time collaboration features

**Migration path:**
1. Add API Gateway WebSocket API to AgentMux
2. Keep Lambda for injection storage/retrieval
3. Add WebSocket notification when injection arrives
4. WaveMux connects via WebSocket, receives push notifications
5. Falls back to polling if WebSocket disconnects

### Hybrid Approach (Best of Both)

```
AgentMux Cloud
    ├── WebSocket: Push notifications ("you have a message")
    └── HTTP API: Fetch actual message content

WaveMux
    ├── WebSocket client: Receives push notifications
    ├── HTTP client: Fetches messages on notification
    └── Polling fallback: If WebSocket disconnects
```

This gives instant notifications without requiring WebSocket to handle message payload, keeping Lambda stateless.

---

## Conclusion

The current polling approach is **pragmatic and cost-effective** for the current scale. WebSocket would be more efficient but requires infrastructure changes to AgentMux (Lambda → always-on or API Gateway WebSocket).

**WaveMux already has all the WebSocket infrastructure needed** on the client side. The bottleneck is AgentMux cloud architecture, not WaveMux capability.

If real-time delivery becomes critical, the hybrid approach (WebSocket notifications + HTTP fetch) offers the best balance of efficiency and implementation simplicity.

---

## Appendix: Key Files

| File | Purpose |
|------|---------|
| `pkg/reactive/poller.go` | Current HTTP polling implementation |
| `pkg/reactive/handler.go` | Local injection delivery |
| `pkg/web/ws.go` | WebSocket server (existing) |
| `pkg/wshutil/wshrpc.go` | RPC message handling |
| `pkg/wps/wps.go` | Pub/sub event broker |
| `frontend/app/store/ws.ts` | Frontend WebSocket client |
