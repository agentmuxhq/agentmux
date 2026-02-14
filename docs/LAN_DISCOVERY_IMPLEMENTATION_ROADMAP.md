# LAN Discovery - Implementation Roadmap

**Based on**: SPEC_LAN_DISCOVERY_AND_LINKING.md
**Current Status**: Design Phase

---

## What We Already Have ✅

### 1. Multi-Window Shared Backend (PR #290)
**Location**: `src-tauri/src/sidecar.rs`, `src-tauri/src/lib.rs`

**Capabilities:**
- ✅ Single backend per host
- ✅ Multiple frontends share one backend
- ✅ `wave-endpoints.json` file with endpoints + auth key
- ✅ HTTP health checking for backend discovery
- ✅ Automatic backend reuse across windows

**Reusable for LAN:**
- Discovery pattern (file-based → network-based)
- Health check mechanism
- Auth key sharing model

### 2. Reactive Server
**Location**: `pkg/web/web.go` (RunReactiveServer)

**Capabilities:**
- ✅ HTTP server on `0.0.0.0:PORT` (all interfaces)
- ✅ Agent discovery endpoints (`/wave/reactive/agents`)
- ✅ Injection endpoints (`/wave/reactive/inject`)
- ✅ No auth required (SkipAuth design)

**Reusable for LAN:**
- Already listens on all interfaces
- Already has agent discovery API
- Just needs mDNS advertisement

### 3. RPC Router & Agent Registry
**Location**: `pkg/wshutil/wshrouter.go`, `pkg/reactive/handler.go`

**Capabilities:**
- ✅ Dynamic route registration
- ✅ Agent-to-block mapping
- ✅ Message routing infrastructure
- ✅ Webhook-based agent registration

**Reusable for LAN:**
- Extend router to support remote backends
- Add peer routes (AgentName@BackendID)
- Connection pooling for peer WebSockets

### 4. Cloud Integration (AgentBus)
**Location**: `pkg/reactive/poller.go`, `pkg/wcloud/wcloud.go`

**Capabilities:**
- ✅ Cross-host messaging via polling
- ✅ Config file (`agentmux.json`)
- ✅ Bearer token authentication
- ✅ Pending message queue

**Reusable for LAN:**
- Dual-mode routing (LAN + Cloud)
- Config file pattern
- Auth token pattern

---

## What We Need to Build 🔨

### Phase 1: Network Discovery (Week 1)

**NEW: mDNS Service Registration**
```go
// pkg/landiscovery/mdns.go
package landiscovery

import "github.com/hashicorp/mdns"

func RegisterMDNSService(port int, backendID string) (*mdns.Server, error) {
  service, err := mdns.NewMDNSService(
    hostname + "-" + backendID[:8],  // Instance name
    "_agentmux._tcp",                // Service type
    "",                               // Domain (local)
    "",                               // Host
    port,                            // Port
    []net.IP{getLocalIP()},          // IPs
    []string{
      "version=0.27.4",
      "backend_id=" + backendID,
    },
  )

  server, err := mdns.NewServer(&mdns.Config{Zone: service})
  return server, err
}

func DiscoverMDNSPeers() ([]*PeerInfo, error) {
  entries := make(chan *mdns.ServiceEntry, 10)
  mdns.Lookup("_agentmux._tcp", entries)

  var peers []*PeerInfo
  for entry := range entries {
    peers = append(peers, parseMDNSEntry(entry))
  }
  return peers, nil
}
```

**NEW: UDP Broadcast Discovery**
```go
// pkg/landiscovery/udp.go
package landiscovery

const BroadcastPort = 51888

func StartUDPListener() {
  conn, _ := net.ListenUDP("udp", &net.UDPAddr{Port: BroadcastPort})

  for {
    buf := make([]byte, 1024)
    n, addr, _ := conn.ReadFromUDP(buf)

    var packet DiscoveryPacket
    json.Unmarshal(buf[:n], &packet)

    if packet.Type == "discover" {
      // Respond with announce
      sendAnnounce(conn, addr)
    } else if packet.Type == "announce" {
      // Add to peer cache
      addPeer(packet)
    }
  }
}

func BroadcastDiscover() {
  conn, _ := net.DialUDP("udp", nil, &net.UDPAddr{
    IP: net.IPv4(255, 255, 255, 255),
    Port: BroadcastPort,
  })

  packet := DiscoveryPacket{
    Type: "discover",
    BackendID: getBackendID(),
    Hostname: getHostname(),
    WSPort: getWSPort(),
    HTTPPort: getHTTPPort(),
  }

  json.NewEncoder(conn).Encode(packet)
}
```

**MODIFY: main-server.go**
```go
// cmd/server/main-server.go

func main() {
  // ... existing startup code ...

  // NEW: Start LAN discovery
  if !*noLANDiscovery {
    go landiscovery.StartDiscoveryService(webPort, wsPort, backendID)
  }

  // ... rest of startup ...
}
```

### Phase 2: Pairing Protocol (Week 2)

**NEW: Pairing Handler**
```go
// pkg/landiscovery/pairing.go

func GeneratePairingCode() string {
  // Generate 8-digit code: XXXX-XXXX
  rand1 := rand.Intn(10000)
  rand2 := rand.Intn(10000)
  return fmt.Sprintf("%04d-%04d", rand1, rand2)
}

func InitiatePairing(targetBackendID string) (*PairingSession, error) {
  code := GeneratePairingCode()
  challenge := generateChallenge()

  session := &PairingSession{
    Code: code,
    Challenge: challenge,
    TargetBackendID: targetBackendID,
    ExpiresAt: time.Now().Add(60 * time.Second),
  }

  activePairings.Store(code, session)
  return session, nil
}

func AcceptPairing(code string) (*PairingResult, error) {
  session, ok := activePairings.Load(code)
  if !ok {
    return nil, errors.New("invalid pairing code")
  }

  // Derive shared secret via ECDH
  sharedSecret := deriveSharedSecret(session.Challenge)

  // Store peer
  peer := &PeerInfo{
    BackendID: session.TargetBackendID,
    SharedSecret: sharedSecret,
    PairedAt: time.Now(),
  }

  savePeer(peer)
  return &PairingResult{Peer: peer}, nil
}
```

**NEW: HTTP Endpoints**
```go
// pkg/landiscovery/httphandler.go

func HandlePairRequest(w http.ResponseWriter, r *http.Request) {
  var req PairRequestBody
  json.NewDecoder(r.Body).Decode(&req)

  session, err := InitiatePairing(req.TargetBackendID)
  if err != nil {
    http.Error(w, err.Error(), 400)
    return
  }

  json.NewEncoder(w).Encode(map[string]string{
    "pairing_code": session.Code,
    "expires_in": "60",
  })
}

func HandlePairResponse(w http.ResponseWriter, r *http.Request) {
  var req PairResponseBody
  json.NewDecoder(r.Body).Decode(&req)

  result, err := AcceptPairing(req.PairingCode)
  if err != nil {
    http.Error(w, err.Error(), 400)
    return
  }

  json.NewEncoder(w).Encode(map[string]string{
    "status": "paired",
    "backend_id": result.Peer.BackendID,
  })
}
```

**NEW: Peer Storage**
```go
// pkg/landiscovery/storage.go

type LANPeersFile struct {
  Peers []PeerInfo `json:"peers"`
}

func LoadPeers() ([]*PeerInfo, error) {
  path := filepath.Join(wavebase.GetWaveDataDir(), "lan-peers.json")
  data, err := os.ReadFile(path)
  if err != nil {
    return []*PeerInfo{}, nil  // Empty on first run
  }

  var file LANPeersFile
  json.Unmarshal(data, &file)
  return file.Peers, nil
}

func SavePeer(peer *PeerInfo) error {
  peers, _ := LoadPeers()
  peers = append(peers, peer)

  file := LANPeersFile{Peers: peers}
  data, _ := json.MarshalIndent(file, "", "  ")

  path := filepath.Join(wavebase.GetWaveDataDir(), "lan-peers.json")
  return os.WriteFile(path, data, 0600)
}
```

### Phase 3: Backend-to-Backend Routing (Week 3)

**MODIFY: RPC Router**
```go
// pkg/wshutil/wshrouter.go

type Route struct {
  RouteId   string
  RpcClient *RpcClient
  Local     bool
  PeerInfo  *landiscovery.PeerInfo  // NEW
}

func (r *WshRouter) RegisterPeerRoute(backendID string, peer *landiscovery.PeerInfo) error {
  // Create WebSocket connection to peer
  conn, err := connectToPeer(peer)
  if err != nil {
    return err
  }

  // Wrap in RPC client
  rpcClient := &RpcClient{
    Conn: conn,
    PeerInfo: peer,
  }

  // Register route for all agents on this backend
  routeID := "peer:" + backendID
  r.RegisterRoute(routeID, rpcClient, false)

  return nil
}

func (r *WshRouter) RouteMessage(agentID string, message interface{}) error {
  // Parse agentID: "AgentName@BackendID"
  parts := strings.Split(agentID, "@")
  if len(parts) == 2 {
    agentName := parts[0]
    backendID := parts[1]

    // Check if it's a LAN peer
    if peer := landiscovery.GetPeer(backendID); peer != nil {
      routeID := "peer:" + backendID
      return r.SendToRoute(routeID, agentName, message)
    }

    // Check if it's a cloud peer
    if cloudPeer := reactive.GetCloudPeer(backendID); cloudPeer != nil {
      return reactive.SendViaCloud(backendID, agentName, message)
    }

    return errors.New("backend not found: " + backendID)
  }

  // Local agent (no "@")
  return r.SendToLocal(agentID, message)
}
```

**NEW: Peer Connection Manager**
```go
// pkg/landiscovery/connections.go

type ConnectionPool struct {
  connections sync.Map  // backendID → *websocket.Conn
  maxConns    int
}

func (p *ConnectionPool) GetOrCreate(peer *PeerInfo) (*websocket.Conn, error) {
  if conn, ok := p.connections.Load(peer.BackendID); ok {
    return conn.(*websocket.Conn), nil
  }

  // Create new connection
  wsURL := fmt.Sprintf("ws://%s:%d/ws?authkey=%s",
    peer.Addresses[0], peer.WSPort, peer.SharedSecret)

  conn, _, err := websocket.DefaultDialer.Dial(wsURL, nil)
  if err != nil {
    return nil, err
  }

  // Start keepalive
  go keepAlive(conn, peer)

  p.connections.Store(peer.BackendID, conn)
  return conn, nil
}

func keepAlive(conn *websocket.Conn, peer *PeerInfo) {
  ticker := time.NewTicker(30 * time.Second)
  for range ticker.C {
    if err := conn.WriteControl(websocket.PingMessage, []byte{}, time.Now().Add(10*time.Second)); err != nil {
      log.Printf("Peer %s disconnected: %v", peer.BackendID, err)
      conn.Close()
      connectionPool.connections.Delete(peer.BackendID)
      return
    }
  }
}
```

### Phase 4: Frontend UI (Week 4)

**NEW: LAN Peers Widget**
```tsx
// frontend/app/view/lanpeers/lanpeers-view.tsx

export function LANPeersView() {
  const [peers, setPeers] = useState<PeerInfo[]>([]);
  const [discovering, setDiscovering] = useState(false);

  useEffect(() => {
    loadPeers();
  }, []);

  async function loadPeers() {
    const resp = await RpcApi.GetLANPeers();
    setPeers(resp.peers);
  }

  async function discoverNewPeers() {
    setDiscovering(true);
    await RpcApi.DiscoverLANPeers();
    setTimeout(loadPeers, 5000);  // Wait 5s for discovery
    setDiscovering(false);
  }

  return (
    <div className="lan-peers">
      <h2>LAN Peers</h2>

      {peers.map(peer => (
        <PeerCard key={peer.backend_id} peer={peer} />
      ))}

      <button onClick={discoverNewPeers} disabled={discovering}>
        {discovering ? "Discovering..." : "📡 Discover New Peers"}
      </button>
    </div>
  );
}

function PeerCard({ peer }: { peer: PeerInfo }) {
  const isOnline = Date.now() - peer.last_seen < 60000;

  return (
    <div className={`peer-card ${isOnline ? 'online' : 'offline'}`}>
      <div className="peer-status">
        {isOnline ? '🟢' : '🔴'}
      </div>
      <div className="peer-info">
        <h3>{peer.hostname}</h3>
        <p>{peer.addresses[0]}</p>
        <p>Last seen: {formatTimestamp(peer.last_seen)}</p>
        <p>Agents: {peer.agent_count}</p>
      </div>
      <div className="peer-actions">
        {isOnline ? (
          <button onClick={() => disconnectPeer(peer)}>Disconnect</button>
        ) : (
          <button onClick={() => reconnectPeer(peer)}>Reconnect</button>
        )}
        <button onClick={() => unpairPeer(peer)}>Remove</button>
      </div>
    </div>
  );
}
```

**NEW: Pairing Modal**
```tsx
// frontend/app/modal/pairing-modal.tsx

export function PairingModal({ targetBackend }: { targetBackend: PeerInfo }) {
  const [pairingCode, setPairingCode] = useState<string>("");
  const [status, setStatus] = useState<"init" | "waiting" | "success" | "error">("init");

  async function initiatePairing() {
    setStatus("waiting");
    const resp = await RpcApi.InitiatePairing(targetBackend.backend_id);
    setPairingCode(resp.pairing_code);

    // Wait for other side to accept
    const result = await RpcApi.WaitForPairingConfirmation(resp.session_id);

    if (result.success) {
      setStatus("success");
    } else {
      setStatus("error");
    }
  }

  return (
    <div className="pairing-modal">
      <h2>Pair with {targetBackend.hostname}</h2>

      {status === "init" && (
        <button onClick={initiatePairing}>Start Pairing</button>
      )}

      {status === "waiting" && (
        <div className="pairing-code-display">
          <p>Enter this code on {targetBackend.hostname}:</p>
          <h1 className="code">{pairingCode}</h1>
          <p className="expires">Expires in 60 seconds</p>
        </div>
      )}

      {status === "success" && (
        <div className="success">
          ✅ Successfully paired with {targetBackend.hostname}!
        </div>
      )}

      {status === "error" && (
        <div className="error">
          ❌ Pairing failed. Please try again.
        </div>
      )}
    </div>
  );
}
```

---

## Files to Create (Summary)

### Go Backend
```
pkg/landiscovery/
  ├── mdns.go             # mDNS service registration/discovery
  ├── udp.go              # UDP broadcast discovery
  ├── discovery.go        # Main discovery manager
  ├── pairing.go          # Pairing protocol
  ├── crypto.go           # Key exchange
  ├── storage.go          # Peer persistence
  ├── connections.go      # Connection pooling
  └── httphandler.go      # HTTP API endpoints
```

### Frontend
```
frontend/app/view/lanpeers/
  ├── lanpeers-view.tsx   # LAN peers widget
  ├── peer-card.tsx       # Individual peer display
  └── lanpeers.scss       # Styling

frontend/app/modal/
  └── pairing-modal.tsx   # Pairing dialog
```

### Config
```
~/.wave/
  └── lan-peers.json      # NEW: Paired backends storage
```

---

## Dependencies to Add

### Go Modules
```bash
go get github.com/hashicorp/mdns
```

### Tauri Frontend
No new dependencies needed (uses existing RpcApi)

---

## Testing Checklist

- [ ] mDNS service registers on startup
- [ ] mDNS discovery finds peers within 5 seconds
- [ ] UDP broadcast works as fallback
- [ ] Pairing code generation and validation
- [ ] Shared secret derivation
- [ ] Peer storage and loading
- [ ] WebSocket connection to peer
- [ ] Message routing via `AgentName@BackendID`
- [ ] Automatic reconnection on disconnect
- [ ] Frontend displays discovered peers
- [ ] Pairing modal flow works end-to-end

---

## Estimated Effort

| Phase | Duration | Complexity |
|-------|----------|------------|
| Phase 1: Discovery | 1 week | Medium |
| Phase 2: Pairing | 1 week | High (crypto) |
| Phase 3: Routing | 1 week | Medium |
| Phase 4: Frontend | 1 week | Low |
| **Total** | **4 weeks** | **Medium-High** |

---

## Next Immediate Steps

1. ✅ **Spec approved** (this document)
2. 🔨 **Prototype mDNS discovery** - Validate hashicorp/mdns library
3. 🔨 **Test UDP broadcast** - Ensure network permissions work
4. 🔨 **Design pairing UX** - Mockups for approval
5. 🔨 **Security review** - Crypto protocol validation

---

**Ready to implement!**
