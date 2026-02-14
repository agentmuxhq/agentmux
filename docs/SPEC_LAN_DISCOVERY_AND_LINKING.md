# Specification: Local Network Discovery & Backend Linking

**Date**: 2026-02-13
**Status**: Design Draft
**Dependencies**: Multi-Window Shared Backend (PR #290)

---

## Overview

Design a system where AgentMux instances on the same local network can discover each other and establish backend-to-backend communication **without internet access**. This complements the existing cloud-based agentbus system for over-the-internet linking.

---

## Goals

1. **Zero-Config LAN Discovery**: AgentMux instances automatically discover each other on the same network
2. **Offline-First**: Works without internet connectivity
3. **Secure Pairing**: Cryptographic verification before establishing trust
4. **Backend-to-Backend Messaging**: Agents on different hosts can communicate directly
5. **Dual Mode**: Support both LAN discovery (local) and cloud linking (internet)

---

## Current Architecture Foundation

### What We Have

**From codebase analysis:**

1. **Multi-Window Shared Backend** (`wave-endpoints.json`)
   - Single backend per host
   - Multiple frontends connect to one backend
   - Endpoints saved to `~/.wave/wave-endpoints.json`
   - Health checking via HTTP

2. **Reactive Server** (0.0.0.0:PORT)
   - Optional HTTP API on all interfaces
   - Agent discovery endpoints (`/wave/reactive/agents`)
   - Injection endpoints (`/wave/reactive/inject`)
   - Currently used for Docker access

3. **Auth System**
   - UUID-based auth keys
   - Header or query param validation
   - Memory-only storage (not persisted)

4. **Cloud Integration** (AgentBus)
   - Polling client for cross-host messaging
   - Config via `~/.wave/agentmux.json`
   - Central server at `api.waveterm.dev`

5. **RPC Router**
   - Routes messages to agents/blocks
   - Supports dynamic route registration
   - Already handles remote connections

---

## Architecture: LAN Discovery

### Layer 1: Network Discovery (mDNS + UDP Broadcast)

**Primary Method: mDNS/DNS-SD (Bonjour)**

```
Service Type: _agentmux._tcp.local.
Instance Name: <hostname>-<random-suffix>
Port: <reactive-server-port>
TXT Records:
  - version=0.27.4
  - backend_id=<uuid>
  - hostname=<hostname>
  - ws_port=<websocket-port>
  - http_port=<http-port>
  - auth_required=true
  - pairing_enabled=true
```

**Fallback Method: UDP Broadcast**

```
Protocol: JSON over UDP
Broadcast Port: 51888 (default)
Packet Structure:
{
  "type": "discover",
  "backend_id": "<uuid>",
  "hostname": "<hostname>",
  "ws_port": 51234,
  "http_port": 51235,
  "reactive_port": 9999,
  "version": "0.27.4",
  "timestamp": 1707865234
}
```

**Response Packet:**
```json
{
  "type": "announce",
  "backend_id": "<uuid>",
  "hostname": "<hostname>",
  "ws_port": 51236,
  "http_port": 51237,
  "reactive_port": 9998,
  "version": "0.27.4",
  "timestamp": 1707865235
}
```

### Layer 2: Secure Pairing

**Challenge-Response Protocol**

```
Step 1: Discovery
  AgentMux-A discovers AgentMux-B via mDNS/UDP

Step 2: User Confirmation (Optional)
  User on Host A: "Link with <hostname-B>?"
  User on Host B: "Accept link from <hostname-A>?"

  Or: Auto-accept with pairing code

Step 3: Pairing Code Generation
  Code Format: XXXX-XXXX (8 digits, human-readable)
  Valid For: 60 seconds
  Displayed in both UIs

Step 4: Cryptographic Handshake
  A → B: { "pairing_request": { "backend_id": "...", "challenge": "<random>" } }
  B → A: { "pairing_response": { "backend_id": "...", "challenge_response": "...", "shared_secret": "<encrypted>" } }
  A → B: { "pairing_confirm": { "shared_secret_hash": "..." } }

Step 5: Trust Establishment
  Both sides store:
  - Paired backend ID
  - Shared secret for auth
  - Network address
  - Pairing timestamp
```

**Storage Format** (`~/.wave/lan-peers.json`):

```json
{
  "peers": [
    {
      "backend_id": "uuid-of-peer",
      "hostname": "laptop-2",
      "addresses": ["192.168.1.100"],
      "ws_port": 51236,
      "http_port": 51237,
      "reactive_port": 9998,
      "shared_secret": "<encrypted>",
      "paired_at": "2026-02-13T10:30:00Z",
      "last_seen": "2026-02-13T10:35:00Z",
      "trust_level": "full",
      "auto_reconnect": true
    }
  ]
}
```

### Layer 3: Backend-to-Backend Communication

**Connection Types**

1. **Direct WebSocket** (Preferred)
   ```
   ws://<peer-ip>:<ws_port>/ws?authkey=<shared_secret>
   ```

2. **HTTP Reactive API** (Fallback)
   ```
   POST http://<peer-ip>:<reactive_port>/wave/reactive/inject
   Headers:
     X-AgentMux-Peer: <backend_id>
     X-Shared-Secret: <secret>
   Body: { "agent_id": "...", "message": "..." }
   ```

**Message Routing**

```
┌──────────────┐                    ┌──────────────┐
│  Host A      │                    │  Host B      │
│              │                    │              │
│  Agent-X ────┼────┐               │  Agent-Y     │
│              │    │               │              │
│  Backend-A   │    │  LAN Link     │  Backend-B   │
│  (Router)    │◄───┼───────────────┼─►(Router)    │
│              │    │               │              │
└──────────────┘    │               └──────────────┘
                    │
         Message: "mux" from Agent-X to Agent-Y@Host-B
                    │
                    ▼
         Backend-A resolves "Host-B" → Peer lookup
         Backend-A sends via WebSocket to Backend-B
         Backend-B routes to local Agent-Y
```

**RPC Extension**

Extend existing RPC router to handle remote backends:

```go
// pkg/wshutil/wshrouter.go

type Route struct {
  RouteId   string
  RpcClient *wshutil.RpcClient
  Local     bool
  PeerInfo  *PeerInfo  // NEW: For remote backends
}

type PeerInfo struct {
  BackendID   string
  Hostname    string
  Address     string
  WSPort      int
  Connection  *websocket.Conn  // Persistent connection
  LastPing    time.Time
}

func (r *WshRouter) RegisterPeerRoute(backendID string, peer *PeerInfo) {
  // Register route for all agents on remote backend
  // Messages to "AgentName@BackendID" route through peer connection
}
```

---

## Implementation Plan

### Phase 1: LAN Discovery Service (Week 1)

**New Files:**
- `pkg/landiscovery/mdns.go` - mDNS service advertisement
- `pkg/landiscovery/udp.go` - UDP broadcast discovery
- `pkg/landiscovery/discovery.go` - Discovery manager

**Features:**
- [ ] mDNS service registration on startup
- [ ] UDP broadcast listener on port 51888
- [ ] Periodic announce packets (every 30s)
- [ ] Peer cache with TTL (5 minutes)
- [ ] Network interface enumeration

**Go Dependencies:**
```go
import (
  "github.com/hashicorp/mdns"  // mDNS library
  "net"                         // UDP sockets
)
```

### Phase 2: Pairing Protocol (Week 2)

**New Files:**
- `pkg/landiscovery/pairing.go` - Pairing handshake
- `pkg/landiscovery/crypto.go` - Key exchange
- `pkg/landiscovery/storage.go` - Peer persistence

**Features:**
- [ ] Pairing code generation (XXXX-XXXX)
- [ ] Challenge-response protocol
- [ ] Shared secret derivation (ECDH or similar)
- [ ] Peer storage in `lan-peers.json`
- [ ] Trust management (trust levels, revocation)

**HTTP Endpoints:**
- `POST /wave/lan/pair-request` - Initiate pairing
- `POST /wave/lan/pair-response` - Accept pairing
- `POST /wave/lan/pair-confirm` - Finalize pairing
- `GET /wave/lan/peers` - List paired backends
- `DELETE /wave/lan/peers/{backend_id}` - Unpair

### Phase 3: Backend-to-Backend Routing (Week 3)

**Modified Files:**
- `pkg/wshutil/wshrouter.go` - Add peer routing
- `pkg/web/ws.go` - Handle peer connections
- `cmd/server/main-server.go` - Initialize LAN discovery

**Features:**
- [ ] Persistent WebSocket connections to peers
- [ ] Message routing via `AgentName@BackendID` syntax
- [ ] Automatic reconnection on disconnect
- [ ] Peer health monitoring (ping/pong)
- [ ] Fallback to HTTP if WebSocket fails

**RPC Extensions:**
```go
// Send message to agent on remote backend
RpcApi.SendMessage("AgentX@backend-uuid-123", "Hello from LAN peer!")

// Route resolution:
// 1. Check if "backend-uuid-123" is local → Direct delivery
// 2. Check if it's a known peer → Route via LAN connection
// 3. Check if it's in cloud config → Route via AgentBus
// 4. Return error: "Backend not found"
```

### Phase 4: Frontend UI (Week 4)

**New Frontend Components:**
- `frontend/app/view/lanpeers/` - LAN peers view
- `frontend/app/modal/pairing-modal.tsx` - Pairing dialog

**Features:**
- [ ] "LAN Peers" widget showing discovered backends
- [ ] Pairing dialog with code entry
- [ ] Visual indicator for connected/disconnected peers
- [ ] Trust management UI (unpair, block)
- [ ] Network status indicator

**UI Mockup:**
```
┌─ LAN Peers ──────────────────────────────┐
│                                          │
│ 🟢 laptop-2 (192.168.1.100)            │
│    Last seen: 5s ago                    │
│    Agents: 3                            │
│    [Disconnect] [Settings]              │
│                                          │
│ 🔴 desktop-main (192.168.1.50)         │
│    Last seen: 2m ago (offline)          │
│    Agents: 1                            │
│    [Reconnect] [Remove]                 │
│                                          │
│ [📡 Discover New Peers]                 │
└──────────────────────────────────────────┘
```

---

## Dual-Mode Architecture: LAN + Cloud

**Routing Priority:**

```
Message to AgentX@BackendY:
  1. Check if BackendY is local (same process)
     → Route via local RPC

  2. Check if BackendY is in lan-peers.json
     → Route via LAN connection (WebSocket/HTTP)

  3. Check if BackendY is in agentmux.json (cloud config)
     → Route via AgentBus polling/webhook

  4. Return error: "Backend not reachable"
```

**Configuration Files:**

```
~/.wave/
├── wave-endpoints.json      # Local backend endpoints (multi-window)
├── lan-peers.json           # LAN-discovered peers
├── agentmux.json            # Cloud linking config (agentbus)
└── agent-routing.json       # NEW: Routing preferences

agent-routing.json:
{
  "prefer_lan": true,          // Prefer LAN over cloud if both available
  "fallback_to_cloud": true,   // Use cloud if LAN fails
  "auto_discover_lan": true,   // Auto-discover on startup
  "pairing_auto_accept": false // Require user confirmation
}
```

---

## Security Considerations

### Threat Model

**Threats:**
1. **Network Sniffing**: Attacker on LAN intercepts messages
2. **Impersonation**: Attacker pretends to be legitimate AgentMux instance
3. **MITM**: Attacker intercepts pairing handshake
4. **Denial of Service**: Flood discovery port with fake announces

**Mitigations:**

1. **Encrypted Connections**
   - Use TLS for WebSocket connections (wss://)
   - Shared secret derived via ECDH (Elliptic Curve Diffie-Hellman)
   - Rotate shared secrets periodically

2. **Pairing Code Verification**
   - Short-lived codes (60s TTL)
   - Out-of-band confirmation (users must see code on both screens)
   - Rate limiting on pairing attempts

3. **Message Authentication**
   - HMAC signatures on all inter-backend messages
   - Replay attack prevention (nonces/timestamps)

4. **Discovery Flood Protection**
   - Rate limit announce packets (max 1/second per IP)
   - Ignore duplicate backend IDs
   - Blocklist for misbehaving IPs

---

## Testing Strategy

### Unit Tests

```go
// pkg/landiscovery/mdns_test.go
func TestMDNSServiceRegistration(t *testing.T)
func TestMDNSServiceDiscovery(t *testing.T)

// pkg/landiscovery/pairing_test.go
func TestPairingCodeGeneration(t *testing.T)
func TestChallengeResponse(t *testing.T)
func TestSharedSecretDerivation(t *testing.T)

// pkg/landiscovery/routing_test.go
func TestRemoteBackendRouting(t *testing.T)
func TestFallbackToCloud(t *testing.T)
```

### Integration Tests

**Scenario 1: Two-Host Discovery**
```bash
# Terminal 1 (Host A)
$ ./agentmuxsrv --lan-discovery-enabled

# Terminal 2 (Host B on same network)
$ ./agentmuxsrv --lan-discovery-enabled

# Expected: Both backends discover each other within 5 seconds
```

**Scenario 2: Pairing**
```bash
# Host A
$ curl localhost:51235/wave/lan/pair-request \
  -d '{"target_backend": "uuid-of-B"}'
# Output: {"pairing_code": "1234-5678"}

# Host B
$ curl localhost:51236/wave/lan/pair-response \
  -d '{"pairing_code": "1234-5678", "accept": true}'
# Output: {"status": "paired"}
```

**Scenario 3: Message Routing**
```bash
# Host A - Send message to agent on Host B
$ wshrpc call inject --agent-id "AgentX@backend-uuid-B" \
  --message "Hello from Host A"

# Host B - Check if AgentX received message
$ wshrpc call get-agent-mailbox --agent-id "AgentX"
# Output: [{"from": "backend-uuid-A", "message": "Hello from Host A"}]
```

---

## Performance Considerations

### Discovery Overhead

**mDNS Traffic:**
- Initial announce: ~200 bytes
- Periodic announce: ~200 bytes every 30s
- Query response: ~300 bytes
- **Total bandwidth:** ~10 KB/hour per instance

**UDP Broadcast Traffic:**
- Announce packet: ~150 bytes every 30s
- **Total bandwidth:** ~5 KB/hour per instance

**Negligible impact** on network performance.

### Connection Pooling

```go
type PeerConnectionPool struct {
  connections map[string]*websocket.Conn  // backend_id → connection
  maxConns    int                         // Limit to 10 peers
  keepAlive   time.Duration               // 30s ping interval
}

func (p *PeerConnectionPool) GetOrCreate(backendID string) (*websocket.Conn, error) {
  // Reuse existing connection or create new one
  // Close least-recently-used if at maxConns limit
}
```

### Message Batching

For high-frequency messaging:
- Batch multiple messages into single WebSocket frame
- Flush batch every 100ms or 10 messages (whichever comes first)
- Reduces overhead for rapid agent-to-agent communication

---

## Comparison: LAN vs Cloud

| Feature | LAN Discovery | Cloud (AgentBus) |
|---------|---------------|------------------|
| **Requires Internet** | ❌ No | ✅ Yes |
| **Latency** | <10ms (local network) | 50-200ms (internet) |
| **Setup** | Zero-config (auto-discover) | Manual config (agentmux.json) |
| **Bandwidth** | Free (LAN) | Metered (cloud provider) |
| **Cross-Network** | ❌ Same subnet only | ✅ Anywhere |
| **Firewall Issues** | Minimal | Possible (NAT, corporate) |
| **Security** | Pairing code + shared secret | OAuth + bearer token |
| **Use Case** | Home/office LAN | Remote work, multi-site |

**Recommendation:** Use LAN for local, Cloud for remote. System auto-selects best route.

---

## Open Questions

1. **mDNS Library Choice?**
   - Option A: `github.com/hashicorp/mdns` (mature, well-tested)
   - Option B: `github.com/grandcat/zeroconf` (active development)
   - **Recommendation:** Option A (hashicorp/mdns)

2. **Default Reactive Port?**
   - Current: User must set `WAVEMUX_REACTIVE_PORT`
   - Proposed: Auto-assign and advertise via mDNS
   - **Recommendation:** Auto-assign random port, save to endpoints file

3. **Pairing UX?**
   - Option A: Always require user confirmation
   - Option B: Auto-pair on same subnet with notification
   - Option C: Config option (paranoid vs convenient mode)
   - **Recommendation:** Option C (default: paranoid)

4. **IPv6 Support?**
   - Should we support IPv6 LAN discovery?
   - **Recommendation:** Yes, listen on both IPv4 and IPv6

5. **Discovery Scope?**
   - Should we discover across VLANs/subnets?
   - Requires router support for mDNS reflector
   - **Recommendation:** Same subnet only (simpler, more secure)

---

## Success Metrics

- [ ] Discovery time < 5 seconds on same LAN
- [ ] Pairing completion < 30 seconds (including user input)
- [ ] Message latency < 50ms between LAN peers
- [ ] Zero configuration required for 80% of users
- [ ] Automatic reconnection within 10 seconds of network change
- [ ] Support for 10+ simultaneous peers without performance degradation

---

## References

- **RFC 6762** - Multicast DNS (mDNS)
- **RFC 6763** - DNS-Based Service Discovery (DNS-SD)
- **hashicorp/mdns** - https://github.com/hashicorp/mdns
- **Current codebase** - See agent analysis output above
- **Multi-Window Shared Backend** - PR #290, commit 26fba66

---

## Next Steps

1. **Review & Approve Spec** - Team discussion on design choices
2. **Create GitHub Issues** - Break down into implementable tasks
3. **Prototype mDNS Discovery** - Proof of concept for network discovery
4. **Security Review** - Cryptographic pairing protocol validation
5. **User Testing** - Early feedback on pairing UX

---

**End of Specification**
