# Dual-Mode Routing: LAN Discovery + AgentBus Cloud

**Date**: 2026-02-13
**Status**: Design Documentation
**Related**: SPEC_LAN_DISCOVERY_AND_LINKING.md

---

## Overview

AgentMux supports **dual-mode backend-to-backend communication**:
1. **LAN Discovery** - Fast, local, peer-to-peer (no internet required)
2. **AgentBus Cloud** - Global, internet-based, central server

Both systems share the same routing infrastructure but use different transport mechanisms. The router intelligently selects the best available route based on availability and performance.

---

## Architecture Comparison

### AgentBus (Cloud Mode)

**Architecture**: Client ↔ Central Server ↔ Client

```
┌─────────────┐                    ┌──────────────────┐                    ┌─────────────┐
│  Backend A  │    HTTPS Polling   │  AgentBus Server │    HTTPS Polling   │  Backend B  │
│  (Home)     │◄──────────────────►│  api.waveterm.dev│◄──────────────────►│  (Office)   │
│             │                    │                  │                    │             │
│  AgentX     │                    │  Message Queue   │                    │  AgentY     │
└─────────────┘                    └──────────────────┘                    └─────────────┘
```

**Characteristics:**
- **Transport**: HTTPS polling every 30 seconds
- **Discovery**: Manual configuration via `agentmux.json`
- **Requires**: Internet connectivity + central server
- **Latency**: 50-200ms + polling delay
- **Use Case**: Remote work, different locations, cross-network
- **Auth**: Bearer token (OAuth)
- **Bandwidth**: Metered (cloud provider costs)

**Configuration File** (`~/.wave/agentmux.json`):
```json
{
  "url": "https://api.waveterm.dev/central",
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
}
```

**Existing Implementation**:
- ✅ Already implemented
- Location: `pkg/reactive/poller.go`, `pkg/wcloud/wcloud.go`
- Polling client: `GET /reactive/pending/{agent_id}`
- Acknowledgment: `POST /reactive/ack`

---

### LAN Discovery (Local Mode)

**Architecture**: Peer ↔ Peer (Direct)

```
┌─────────────┐                                           ┌─────────────┐
│  Backend A  │         mDNS Discovery + WebSocket        │  Backend B  │
│  (Laptop)   │◄─────────────────────────────────────────►│  (Desktop)  │
│             │         ws://192.168.1.100:51234          │             │
│  AgentX     │                                           │  AgentY     │
└─────────────┘                                           └─────────────┘
       │                                                         │
       └─────────────────── Same LAN Subnet ───────────────────┘
```

**Characteristics:**
- **Transport**: WebSocket (persistent connection)
- **Discovery**: Automatic via mDNS/Bonjour or UDP broadcast
- **Requires**: Same local network only
- **Latency**: <10ms (local network)
- **Use Case**: Home/office LAN, no internet needed
- **Auth**: Pairing codes + shared secret
- **Bandwidth**: Free (local network)

**Configuration File** (`~/.wave/lan-peers.json`):
```json
{
  "peers": [
    {
      "backend_id": "uuid-of-peer",
      "hostname": "desktop-main",
      "addresses": ["192.168.1.100"],
      "ws_port": 51234,
      "http_port": 51235,
      "reactive_port": 9999,
      "shared_secret": "<encrypted>",
      "paired_at": "2026-02-13T10:30:00Z",
      "last_seen": "2026-02-13T10:35:00Z",
      "auto_reconnect": true
    }
  ]
}
```

**New Implementation** (To be built):
- 🔨 Phase 1: mDNS discovery
- 🔨 Phase 2: Pairing protocol
- 🔨 Phase 3: WebSocket routing
- See: `LAN_DISCOVERY_IMPLEMENTATION_ROADMAP.md`

---

## Shared Infrastructure

Both systems **reuse** the following components:

### 1. RPC Router (`pkg/wshutil/wshrouter.go`)

**Current Implementation** (Supports local routing):
```go
type WshRouter struct {
  routes map[string]*Route
  mu     sync.RWMutex
}

type Route struct {
  RouteId   string
  RpcClient *RpcClient
  Local     bool
}

func (r *WshRouter) RegisterRoute(routeId string, client *RpcClient, local bool) {
  r.mu.Lock()
  defer r.mu.Unlock()
  r.routes[routeId] = &Route{
    RouteId:   routeId,
    RpcClient: client,
    Local:     local,
  }
}
```

**Enhanced Implementation** (Supports LAN + Cloud routing):
```go
type Route struct {
  RouteId   string
  RpcClient *RpcClient
  Local     bool
  PeerInfo  *PeerInfo        // NEW: For LAN peers
  CloudInfo *CloudPeerInfo   // NEW: For AgentBus peers
}

type PeerInfo struct {
  BackendID   string
  Transport   string  // "lan" or "cloud"
  Connection  *websocket.Conn  // For LAN
  CloudURL    string           // For AgentBus
  LastPing    time.Time
  Priority    int     // Lower = higher priority
}

func (r *WshRouter) RouteMessage(targetAgent string, message interface{}) error {
  // Parse: "AgentName@BackendID"
  agentName, backendID := parseAgentAddress(targetAgent)

  // 1. Local routing (same process)
  if backendID == "" || backendID == getCurrentBackendID() {
    return r.routeLocal(agentName, message)
  }

  // 2. LAN peer routing (fast path)
  if peer := getLANPeer(backendID); peer != nil {
    return r.routeViaLAN(peer, agentName, message)
  }

  // 3. Cloud routing (fallback)
  if cloudPeer := getCloudPeer(backendID); cloudPeer != nil {
    return r.routeViaCloud(cloudPeer, agentName, message)
  }

  return fmt.Errorf("backend not reachable: %s", backendID)
}
```

### 2. Message Format (RPC Protocol)

**Unified Message Structure**:
```go
type AgentMessage struct {
  MessageID   string      `json:"message_id"`
  FromBackend string      `json:"from_backend"`
  FromAgent   string      `json:"from_agent"`
  ToBackend   string      `json:"to_backend"`
  ToAgent     string      `json:"to_agent"`
  Method      string      `json:"method"`  // "mux" or "ject"
  Payload     interface{} `json:"payload"`
  Timestamp   int64       `json:"timestamp"`
  Transport   string      `json:"transport"`  // "lan", "cloud", or "local"
}
```

**Same format for both LAN and Cloud**, only transport layer differs.

### 3. Agent Registry (`pkg/reactive/handler.go`)

**Existing Agent Registration**:
```go
type Handler struct {
  agentToBlock map[string]string      // agent_id → block_id
  blockToAgent map[string]string      // block_id → agent_id
  agentInfo    map[string]*AgentRegistration
}

func (h *Handler) RegisterAgent(agentID, blockID, tabID string) error {
  h.agentToBlock[agentID] = blockID
  h.blockToAgent[blockID] = agentID

  h.agentInfo[agentID] = &AgentRegistration{
    AgentID:      agentID,
    BlockID:      blockID,
    TabID:        tabID,
    RegisteredAt: time.Now(),
  }

  return nil
}
```

**Used by both LAN and Cloud** for local agent lookup. When a message arrives (via LAN or Cloud), the handler resolves which terminal block should receive it.

### 4. API Consistency

**Frontend API** (Same for both):
```typescript
// Send message to agent (router auto-selects transport)
await RpcApi.SendMessage({
  to: "AgentX@backend-uuid-123",  // Target agent
  message: "Hello from peer!",
  method: "mux"
});

// Router internally:
// - Checks if backend-uuid-123 is in lan-peers.json → Use LAN
// - Else checks if in agentmux.json → Use Cloud
// - Else error: backend not found
```

**Backend API** (Same for both):
```go
// Inject message to local agent (from LAN or Cloud)
err := reactiveHandler.InjectMessage(agentID, message)
```

---

## Routing Priority & Fallback

### Priority Order

```
1. LOCAL (same process)
   - Latency: <1ms
   - Always preferred if target is on same backend

2. LAN (local network)
   - Latency: <10ms
   - Preferred over cloud if both available
   - Zero cost bandwidth

3. CLOUD (internet)
   - Latency: 50-200ms + polling delay
   - Used when LAN unavailable
   - Fallback for remote backends
```

### Routing Decision Tree

```
Message to: "AgentX@BackendY"

                    ┌─────────────────┐
                    │ Is BackendY     │
                    │ local?          │
                    └────────┬────────┘
                             │
                   ┌─────────┴─────────┐
                   │ YES               │ NO
                   ▼                   ▼
            ┌────────────┐      ┌────────────────┐
            │ Route via  │      │ Is BackendY in │
            │ Local RPC  │      │ lan-peers.json?│
            └────────────┘      └────────┬───────┘
                                         │
                               ┌─────────┴─────────┐
                               │ YES               │ NO
                               ▼                   ▼
                        ┌────────────┐      ┌────────────────┐
                        │ Is LAN     │      │ Is BackendY in │
                        │ peer alive?│      │ agentmux.json? │
                        └─────┬──────┘      └────────┬───────┘
                              │                      │
                    ┌─────────┴─────────┐  ┌─────────┴─────────┐
                    │ YES               │  │ YES               │ NO
                    ▼                   ▼  ▼                   ▼
             ┌────────────┐      ┌────────────────┐    ┌─────────┐
             │ Route via  │      │ Route via      │    │ ERROR:  │
             │ LAN WS     │      │ AgentBus Cloud │    │ Backend │
             └────────────┘      └────────────────┘    │ not     │
                                                        │ found   │
                                                        └─────────┘
```

### Fallback Configuration

**Config File** (`~/.wave/agent-routing.json`):
```json
{
  "prefer_lan": true,           // Prefer LAN over cloud if both available
  "fallback_to_cloud": true,    // Use cloud if LAN fails
  "lan_timeout_ms": 5000,       // How long to wait for LAN before trying cloud
  "retry_failed_lan": true,     // Retry LAN on next message if previous failed
  "auto_discover_lan": true,    // Auto-discover LAN peers on startup
  "cloud_enabled": true         // Enable AgentBus cloud routing
}
```

### Intelligent Fallback Example

```go
func (r *WshRouter) RouteWithFallback(target string, message interface{}) error {
  config := loadRoutingConfig()

  // Try LAN first (if configured)
  if config.PreferLAN {
    if err := r.tryLAN(target, message); err == nil {
      return nil  // Success!
    }
    log.Printf("LAN routing failed: %v", err)
  }

  // Fallback to cloud (if configured)
  if config.FallbackToCloud {
    if err := r.tryCloud(target, message); err == nil {
      log.Printf("Routed via cloud fallback")
      return nil  // Success!
    }
    log.Printf("Cloud routing also failed: %v", err)
  }

  return fmt.Errorf("all routing attempts failed for %s", target)
}
```

---

## Example Scenarios

### Scenario 1: Home Office (LAN Only)

**Setup**:
- Laptop: `192.168.1.50` (Backend A)
- Desktop: `192.168.1.100` (Backend B)
- No internet connection
- Both on same WiFi

**Flow**:
```
User on Laptop: "mux to AgentX@desktop Hello!"

1. Router checks: Is "desktop" local? NO
2. Router checks: Is "desktop" in lan-peers.json? YES
   - backend_id: uuid-of-desktop
   - address: 192.168.1.100
   - ws_port: 51234
3. Router sends via WebSocket:
   ws://192.168.1.100:51234/ws?authkey=<shared_secret>
4. Desktop Backend receives, routes to local AgentX
5. Latency: ~8ms (LAN only)
```

**Config Files Used**:
- ✅ `lan-peers.json` (has Desktop)
- ❌ `agentmux.json` (not used, no internet)

---

### Scenario 2: Remote Work (Cloud Only)

**Setup**:
- Home Laptop: `98.123.45.67` (Backend A)
- Office Desktop: `203.45.67.89` (Backend B)
- Different networks, internet available
- No LAN connection

**Flow**:
```
User on Laptop: "mux to AgentY@office-desktop Task completed"

1. Router checks: Is "office-desktop" local? NO
2. Router checks: Is "office-desktop" in lan-peers.json? NO
3. Router checks: Is "office-desktop" in agentmux.json? YES
   - backend_id: uuid-of-office-desktop
   - cloud_url: https://api.waveterm.dev/central
4. Router sends via AgentBus:
   POST https://api.waveterm.dev/central/reactive/inject
   Body: { "to": "uuid-of-office-desktop", "agent": "AgentY", ... }
5. Office Desktop polls AgentBus, receives message
6. Office Desktop routes to local AgentY
7. Latency: ~120ms + polling delay (up to 30s)
```

**Config Files Used**:
- ❌ `lan-peers.json` (empty, different networks)
- ✅ `agentmux.json` (has Office Desktop)

---

### Scenario 3: Hybrid Office (Both LAN + Cloud)

**Setup**:
- Laptop: `192.168.1.50` (Backend A, on office WiFi)
- Office Desktop: `192.168.1.100` (Backend B, on same WiFi)
- Home Server: `Remote IP` (Backend C, only reachable via cloud)
- Internet available

**Flow 1** (Message to Office Desktop - LAN preferred):
```
User on Laptop: "mux to AgentX@office-desktop Data ready"

1. Router checks: Is "office-desktop" in lan-peers.json? YES
2. Router checks: Is LAN peer alive? YES (last_seen: 2s ago)
3. Router sends via LAN WebSocket (fast!)
4. Latency: ~7ms
```

**Flow 2** (Message to Home Server - Cloud required):
```
User on Laptop: "mux to AgentZ@home-server Backup started"

1. Router checks: Is "home-server" in lan-peers.json? NO
2. Router checks: Is "home-server" in agentmux.json? YES
3. Router sends via AgentBus Cloud
4. Latency: ~95ms + polling delay
```

**Config Files Used**:
- ✅ `lan-peers.json` (has Office Desktop)
- ✅ `agentmux.json` (has Home Server)
- Router intelligently picks best route for each target!

---

### Scenario 4: Network Failure & Automatic Fallback

**Setup**:
- Both backends have LAN AND Cloud configured
- LAN connection becomes unstable (WiFi issues)
- Cloud connection still available

**Flow**:
```
User: "mux to AgentX@desktop Important message"

1. Router tries LAN first (preferred)
2. WebSocket connection fails (timeout after 5s)
3. Router automatically falls back to Cloud
4. Message delivered via AgentBus
5. User sees notification: "Routed via cloud (LAN unavailable)"
6. Later: LAN recovers, router switches back automatically
```

**Routing Config** (`agent-routing.json`):
```json
{
  "prefer_lan": true,
  "fallback_to_cloud": true,
  "lan_timeout_ms": 5000,
  "retry_failed_lan": true
}
```

---

## Configuration File Structure

### Overview

```
~/.wave/
├── wave-endpoints.json      # Local backend (multi-window)
├── lan-peers.json           # LAN peers (same network)
├── agentmux.json            # Cloud peers (internet)
└── agent-routing.json       # Routing preferences
```

### 1. Local Backend Discovery (`wave-endpoints.json`)

**Purpose**: Multi-window shared backend (PR #290)

```json
{
  "ws_endpoint": "127.0.0.1:51234",
  "web_endpoint": "127.0.0.1:51235",
  "auth_key": "uuid-auth-key"
}
```

**Scope**: Single host only
**Created by**: First AgentMux instance on host
**Used by**: Additional windows to find existing backend

---

### 2. LAN Peers (`lan-peers.json`)

**Purpose**: Discovered and paired backends on local network

```json
{
  "peers": [
    {
      "backend_id": "uuid-1",
      "hostname": "desktop-main",
      "addresses": ["192.168.1.100", "fe80::1"],
      "ws_port": 51234,
      "http_port": 51235,
      "reactive_port": 9999,
      "shared_secret": "<encrypted>",
      "paired_at": "2026-02-13T10:30:00Z",
      "last_seen": "2026-02-13T10:35:00Z",
      "trust_level": "full",
      "auto_reconnect": true,
      "agent_count": 3,
      "tags": ["office", "always-on"]
    },
    {
      "backend_id": "uuid-2",
      "hostname": "laptop-2",
      "addresses": ["192.168.1.50"],
      "ws_port": 51236,
      "http_port": 51237,
      "reactive_port": 9998,
      "shared_secret": "<encrypted>",
      "paired_at": "2026-02-13T09:15:00Z",
      "last_seen": "2026-02-13T10:33:00Z",
      "trust_level": "full",
      "auto_reconnect": false,
      "agent_count": 1,
      "tags": ["temporary"]
    }
  ],
  "discovery_enabled": true,
  "auto_pair_same_subnet": false
}
```

**Scope**: Local network (same subnet)
**Created by**: LAN discovery + pairing process
**Updated**: On peer discovery/departure

---

### 3. Cloud Peers (`agentmux.json`)

**Purpose**: AgentBus cloud configuration for remote backends

```json
{
  "url": "https://api.waveterm.dev/central",
  "ws_url": "wss://wsapi.waveterm.dev/",
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "backend_id": "uuid-of-this-backend",
  "poll_interval_seconds": 30,
  "enabled": true
}
```

**Scope**: Internet (anywhere)
**Created by**: User configuration or OAuth flow
**Updated**: Token refresh

---

### 4. Routing Preferences (`agent-routing.json`)

**Purpose**: Control routing behavior and fallback

```json
{
  "prefer_lan": true,
  "fallback_to_cloud": true,
  "lan_timeout_ms": 5000,
  "cloud_timeout_ms": 10000,
  "retry_failed_lan": true,
  "retry_interval_ms": 60000,
  "auto_discover_lan": true,
  "auto_pair_same_subnet": false,
  "cloud_enabled": true,
  "debug_routing": false
}
```

**Scope**: Local backend only
**Created by**: User settings or defaults
**Updated**: Via settings UI

---

## Implementation Reuse Matrix

| Component | LAN | Cloud | Implementation Status | Shared Code |
|-----------|-----|-------|----------------------|-------------|
| **RPC Router** | ✅ | ✅ | ✅ Exists | ✅ 100% shared |
| **Message Format** | ✅ | ✅ | ✅ Exists | ✅ 100% shared |
| **Agent Registry** | ✅ | ✅ | ✅ Exists | ✅ 100% shared |
| **Auth Validation** | ✅ | ✅ | ✅ Exists | ✅ 80% shared (different secrets) |
| **Discovery** | mDNS/UDP | Manual | 🔨 Need LAN | ❌ 0% shared (different methods) |
| **Transport** | WebSocket | HTTPS Poll | ✅ Exists | ✅ 50% shared (WebSocket code exists) |
| **Connection Pool** | ✅ | ✅ | 🔨 Enhance | ✅ 70% shared (pool pattern exists) |
| **Pairing/Config** | Codes | OAuth | 🔨 Need LAN | ❌ 0% shared (different flows) |
| **Health Check** | Ping/Pong | Poll ACK | ✅ Exists | ✅ 60% shared (pattern similar) |
| **Frontend API** | ✅ | ✅ | ✅ Exists | ✅ 100% shared |

**Legend**:
- ✅ Exists: Already implemented
- 🔨 Need: Needs to be built
- % Shared: How much code can be reused

---

## Code Integration Points

### 1. Enhanced Router Initialization

**File**: `cmd/server/main-server.go`

```go
func main() {
  // ... existing startup ...

  // Initialize routing
  router := wshutil.DefaultRouter

  // Register local routes (existing)
  registerLocalRoutes(router)

  // NEW: Register LAN peer routes
  if !*noLANDiscovery {
    lanPeers := landiscovery.LoadPeers()
    for _, peer := range lanPeers {
      router.RegisterPeerRoute(peer.BackendID, peer, "lan")
    }
  }

  // NEW: Register cloud routes
  if cloudConfig := loadCloudConfig(); cloudConfig != nil {
    cloudPeers := reactive.GetCloudPeers(cloudConfig)
    for _, peer := range cloudPeers {
      router.RegisterPeerRoute(peer.BackendID, peer, "cloud")
    }
  }

  // ... rest of startup ...
}
```

### 2. Unified Routing Method

**File**: `pkg/wshutil/wshrouter.go`

```go
func (r *WshRouter) RouteMessage(target string, msg interface{}) error {
  agentName, backendID := parseTarget(target)

  // Priority 1: Local
  if backendID == "" || backendID == r.localBackendID {
    return r.routeLocal(agentName, msg)
  }

  // Priority 2: LAN
  if route := r.findRoute("lan:" + backendID); route != nil {
    return r.routeViaTransport(route, agentName, msg)
  }

  // Priority 3: Cloud
  if route := r.findRoute("cloud:" + backendID); route != nil {
    return r.routeViaTransport(route, agentName, msg)
  }

  return fmt.Errorf("backend not reachable: %s", backendID)
}
```

### 3. Transport Abstraction

**File**: `pkg/wshutil/transport.go` (NEW)

```go
type Transport interface {
  Send(agentID string, message interface{}) error
  Close() error
  HealthCheck() error
}

type LANTransport struct {
  conn      *websocket.Conn
  peerInfo  *landiscovery.PeerInfo
}

func (t *LANTransport) Send(agentID string, msg interface{}) error {
  return t.conn.WriteJSON(msg)
}

type CloudTransport struct {
  httpClient *http.Client
  cloudURL   string
  token      string
}

func (t *CloudTransport) Send(agentID string, msg interface{}) error {
  req := &reactive.InjectionRequest{
    AgentID: agentID,
    Message: msg,
  }
  resp, err := t.httpClient.Post(
    t.cloudURL + "/reactive/inject",
    "application/json",
    json.Marshal(req),
  )
  return err
}
```

---

## Performance Comparison

| Metric | Local | LAN | Cloud |
|--------|-------|-----|-------|
| **Latency** | <1ms | 5-15ms | 50-200ms |
| **Polling Delay** | - | - | +0-30s |
| **Total Latency** | <1ms | 5-15ms | 50-230ms |
| **Bandwidth** | Free | Free | Metered |
| **Requires Internet** | No | No | Yes |
| **Setup Complexity** | Zero | Zero | Manual |
| **Firewall Issues** | None | Minimal | Possible |

**Best Practices**:
- Use LAN for frequent, latency-sensitive messaging
- Use Cloud for infrequent cross-network communication
- Enable fallback for reliability

---

## Security Considerations

### LAN Security
- Pairing codes (60s TTL)
- Shared secret via ECDH
- TLS for WebSocket connections
- Same subnet restriction

### Cloud Security
- OAuth bearer tokens
- HTTPS only
- Central server ACLs
- Token rotation

### Shared Security
- Message authentication (HMAC)
- Replay attack prevention (nonces)
- Rate limiting
- Audit logging

---

## Migration Path

### Phase 1: Current State (Cloud Only)
```
Backends use AgentBus for all cross-host messaging
Config: agentmux.json only
```

### Phase 2: Add LAN Discovery (Dual Mode)
```
Backends auto-discover LAN peers
Config: agentmux.json + lan-peers.json
Router prefers LAN, falls back to Cloud
```

### Phase 3: Enhanced Routing (Intelligent)
```
Router learns best routes based on performance
Config: + agent-routing.json
Automatic failover and load balancing
```

---

## Testing Strategy

### Unit Tests
```go
func TestRouterPriority(t *testing.T) {
  // Test: Local > LAN > Cloud
}

func TestLANFallbackToCloud(t *testing.T) {
  // Test: LAN fails → Cloud succeeds
}

func TestCloudOnlyWhenNoLAN(t *testing.T) {
  // Test: No LAN config → Use Cloud
}
```

### Integration Tests
```bash
# Test 1: LAN routing
./agentmuxsrv --lan-enabled &  # Backend A
./agentmuxsrv --lan-enabled &  # Backend B
# Send message A→B, verify uses LAN

# Test 2: Cloud routing
./agentmuxsrv --cloud-config=cloud.json &  # Backend A (remote)
./agentmuxsrv --cloud-config=cloud.json &  # Backend B (remote)
# Send message A→B, verify uses Cloud

# Test 3: Hybrid routing
./agentmuxsrv --lan-enabled --cloud-config=cloud.json &
# Send to LAN peer → Uses LAN
# Send to Cloud peer → Uses Cloud
```

---

## Future Enhancements

1. **Adaptive Routing**
   - Learn latency patterns
   - Auto-switch to faster route

2. **Load Balancing**
   - Multiple routes to same backend
   - Round-robin or least-latency

3. **Hybrid Transport**
   - Try LAN and Cloud simultaneously
   - Use whichever responds first

4. **Mesh Networking**
   - Multi-hop routing (A → B → C)
   - Decentralized discovery

---

## Summary

**Key Insight**: LAN Discovery and AgentBus Cloud are **complementary systems** that share infrastructure but use different transports.

**Shared Components**:
- ✅ RPC Router (100% reuse)
- ✅ Message Format (100% reuse)
- ✅ Agent Registry (100% reuse)
- ✅ Frontend API (100% reuse)

**Different Components**:
- ❌ Discovery (mDNS vs Manual)
- ❌ Transport (WebSocket vs HTTPS Polling)
- ❌ Auth (Pairing vs OAuth)

**Routing Priority**: Local > LAN > Cloud

**Best Use Cases**:
- **LAN**: Fast local communication (home/office)
- **Cloud**: Remote cross-network communication
- **Both**: Hybrid office with automatic fallback

**Implementation Effort**:
- LAN Discovery: 4 weeks (new code)
- Cloud Integration: Already exists ✅
- Unified Router: 1 week (enhancement)

---

**End of Documentation**
