# Access Control & ACLs for FILE Relay Server

> ⚠️ **ADVANCED OPTIONS - NOT FOR DEFAULT DEPLOYMENT**
>
> This document describes **OPTIONAL** access control mechanisms for enterprise/managed relay deployments.
>
> **The default FILE Relay is stateless and requires NONE of these features.**
>
> - ✅ **Public relay**: Use QUICK-START.md (CAPoW only, no ACLs, no database)
> - ⚠️ **Managed relay**: Read this doc for API keys + quotas (adds database)
> - ⚠️ **Enterprise relay**: Read this doc for multi-tenancy (complex)
>
> **If you're deploying a public relay: you can skip this entire document.**

---

**Status**: Design document (not yet implemented)  
**Priority**: LOW (optional, only if needed for managed/enterprise deployments)  
**Estimated Effort**: 2 weeks (if implementing advanced options)

---

## Overview

FILE Relay Server IS and WILL REMAIN a **"dumb pipe"** with no peer-level access control **by default**.

This document describes **optional extensions** for enterprise use cases that **diverge from the zero-trust stateless philosophy**.

**Core FILE Relay** (what you should implement first):
- ✅ Stateless (no database)
- ✅ CAPoW for DoS protection
- ✅ Per-IP rate limiting
- ✅ IP hashing for GDPR
- ✅ Zero peer authentication (beyond proof-of-work)

**Advanced options** (this document):
1. Current state (what exists today)
2. ACL architecture options (5 levels)
3. Implementation strategies (API keys, P2P ACLs, multi-tenancy)
4. Trade-offs and recommendations

---

## Current State: Zero-Trust Architecture

### What Exists Today

**Network-level protection**:
- ✅ CAPoW (prevents DoS)
- ✅ Per-IP rate limiting (10 req/s sustained, 50 burst)
- ✅ Max peers capacity (1000 per instance)
- ✅ IP address hashing (GDPR compliance)

**What does NOT exist**:
- ❌ Peer-to-peer ACLs (any peer can send to any peer)
- ❌ Peer authentication (beyond CAPoW proof of work)
- ❌ Peer authorization (no whitelist/blacklist)
- ❌ Multi-tenancy isolation
- ❌ Admin access control (metrics endpoint is public on port 8081)

### Design Philosophy: "Dumb Pipe"

The relay is **intentionally opaque**:
- Doesn't inspect packet contents
- Doesn't know about "users" or "accounts"
- Doesn't maintain session state beyond peer registry
- Forwards packets blindly based on peer_id

**Why?**
- **Performance**: No crypto operations, no DB lookups
- **Privacy**: Cannot decrypt or log packet contents
- **Simplicity**: Stateless, easy to scale horizontally
- **Zero-trust**: Clients handle authentication/encryption end-to-end

---

## ACL Levels

There are 5 distinct ACL levels to consider:

```
┌─────────────────────────────────────────────────────────┐
│ Level 5: Application ACLs (Client handles)              │
│ ↓                                                        │
│ Level 4: Peer-to-Peer ACLs (Relay enforces)            │
│ ↓                                                        │
│ Level 3: Relay Registration ACLs (Who can register?)    │
│ ↓                                                        │
│ Level 2: Network ACLs (Firewall, security groups)      │
│ ↓                                                        │
│ Level 1: Admin ACLs (Metrics, health endpoints)        │
└─────────────────────────────────────────────────────────┘
```

---

## Level 1: Admin ACLs (Metrics & Management)

**Problem**: Currently, metrics endpoint (`http://relay:8081/metrics`) is accessible to anyone on the network.

### Threat Model

| Threat | Impact | Likelihood |
|--------|--------|------------|
| Information disclosure | Medium (peer count, packet rate exposed) | High |
| DoS via metrics scraping | Low (HTTP is lightweight) | Low |

### Solution 1: Network Isolation (Recommended)

**Restrict port 8081 to trusted networks only**:

```yaml
# Kubernetes NetworkPolicy
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: file-relay-metrics-policy
spec:
  podSelector:
    matchLabels:
      app: file-relay
  ingress:
    - from:
      - namespaceSelector:
          matchLabels:
            name: monitoring  # Only Prometheus namespace
      ports:
        - protocol: TCP
          port: 8081
```

**AWS Security Group**:
```hcl
resource "aws_security_group_rule" "metrics_ingress" {
  type              = "ingress"
  from_port         = 8081
  to_port           = 8081
  protocol          = "tcp"
  cidr_blocks       = ["10.0.0.0/8"]  # Internal VPC only
  security_group_id = aws_security_group.relay.id
}
```

**Advantages**:
- ✅ Simple (no code changes)
- ✅ Standard practice (firewall rules)
- ✅ Works with existing monitoring tools

**Disadvantages**:
- ❌ Assumes trusted internal network
- ❌ Doesn't work for multi-tenant scenarios

### Solution 2: HTTP Basic Auth

**Add authentication to metrics endpoint**:

```rust
use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::Response,
};
use base64::{engine::general_purpose::STANDARD, Engine};

async fn auth_middleware<B>(
    State(config): State<Arc<Config>>,
    req: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    if !auth_header.starts_with("Basic ") {
        return Err(StatusCode::UNAUTHORIZED);
    }
    
    let encoded = &auth_header[6..];
    let decoded = STANDARD.decode(encoded)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    let credentials = String::from_utf8(decoded)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    
    let (username, password) = credentials
        .split_once(':')
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    if username == config.metrics_username && password == config.metrics_password {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

// In main.rs
let app = Router::new()
    .route("/metrics", get(metrics_handler))
    .route("/health", get(health_handler))
    .route_layer(middleware::from_fn_with_state(
        config.clone(),
        auth_middleware,
    ));
```

**Prometheus scrape config**:
```yaml
scrape_configs:
  - job_name: 'file-relay'
    static_configs:
      - targets: ['relay:8081']
    basic_auth:
      username: 'prometheus'
      password: 'secret-from-k8s-secret'
```

**Advantages**:
- ✅ Standard HTTP auth
- ✅ Works across networks
- ✅ Prometheus supports it natively

**Disadvantages**:
- ❌ Credentials management (secrets rotation)
- ❌ Adds complexity to deployment

**Recommendation**: Use **Solution 1 (Network Isolation)** for most deployments. Use Solution 2 only if metrics must be exposed across untrusted networks.

---

## Level 2: Network ACLs (Firewall)

**Problem**: Which IP addresses can access the relay UDP port (8080)?

### Current State

Port 8080 is typically **open to 0.0.0.0/0** (entire internet) because:
- Peers are behind NAT (dynamic IPs)
- Geographic distribution (unknown source IPs)

### Options

#### Option A: Open to Internet (Current, Recommended)

**Configuration**:
```hcl
# AWS Security Group
resource "aws_security_group_rule" "relay_udp_ingress" {
  type              = "ingress"
  from_port         = 8080
  to_port           = 8080
  protocol          = "udp"
  cidr_blocks       = ["0.0.0.0/0"]  # Open to world
  security_group_id = aws_security_group.relay.id
}
```

**Protection**:
- ✅ CAPoW prevents DoS
- ✅ Per-IP rate limiting
- ✅ Cloud provider DDoS protection (AWS Shield, GCP Cloud Armor)

**Use case**: Public relay for general use.

#### Option B: IP Whitelist (Enterprise)

**Configuration**:
```hcl
# Only allow specific corporate IP ranges
cidr_blocks = [
  "203.0.113.0/24",  # Office A
  "198.51.100.0/24", # Office B
]
```

**Use case**: Enterprise private relay, known IP ranges.

#### Option C: VPN/Tailscale Only (Private)

**Configuration**:
- Relay deployed inside VPN (WireGuard, Tailscale)
- Only VPN clients can reach relay

**Use case**: Internal use only, not public relay.

**Recommendation**: **Option A (Open)** for public relay + CAPoW. Option B/C for private enterprise deployments.

---

## Level 3: Relay Registration ACLs

**Problem**: Which peers are allowed to register with the relay?

### Current State

**Any peer can register** if:
1. They have valid CAPoW proof (100ms computation)
2. They're not rate-limited (10 req/s per IP)
3. Relay has capacity (< 1000 peers)

**No concept of**:
- User accounts
- API keys
- Peer whitelists
- Tenant isolation

### Option A: CAPoW-Only (Current, Recommended for Public)

**How it works**:
- Peer fetches challenge from `/challenge`
- Peer generates CAPoW proof (~100ms CPU)
- Relay verifies proof and registers peer

**Advantages**:
- ✅ No user management
- ✅ No database
- ✅ Scales horizontally
- ✅ Privacy-preserving (no identity)

**Disadvantages**:
- ❌ Cannot ban specific peers
- ❌ Cannot prioritize premium users
- ❌ Cannot enforce quotas per user

**Use case**: Public relay, open access.

### Option B: API Key Authentication

**How it works**:
1. User signs up, gets API key (e.g., via web dashboard)
2. Peer includes API key in REGISTER packet
3. Relay validates API key against database/cache
4. If valid, registers peer

**Implementation**:

```rust
// REGISTER packet format with API key
struct RegisterPacketV2 {
    packet_type: u8,        // 0x01
    peer_id: [u8; 32],
    api_key: [u8; 32],      // New: API key
    capow_proof: MtpProof,
}

// Validation
async fn handle_register_v2(packet: RegisterPacketV2, state: Arc<State>) -> Result<()> {
    // 1. Validate API key
    let user = state.api_key_cache.get(&packet.api_key)
        .ok_or(Error::InvalidApiKey)?;
    
    // 2. Check user quota
    if user.current_peers >= user.max_peers {
        return Err(Error::QuotaExceeded);
    }
    
    // 3. Verify CAPoW (still required for DoS protection)
    if !verify_capow(&packet.capow_proof).await? {
        return Err(Error::InvalidCapow);
    }
    
    // 4. Register peer
    state.peers.insert(packet.peer_id, PeerInfo {
        addr: src_addr,
        user_id: user.id,  // Track which user owns this peer
        last_seen: Instant::now(),
    });
    
    Ok(())
}
```

**API Key Storage**:

Option 1: **Redis** (distributed)
```rust
// On registration: check Redis
let user: Option<User> = redis_conn.get(format!("api_key:{}", hex::encode(&api_key))).await?;
```

Option 2: **PostgreSQL** (persistent)
```sql
CREATE TABLE users (
    id UUID PRIMARY KEY,
    api_key BYTEA UNIQUE NOT NULL,
    max_peers INT NOT NULL DEFAULT 10,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_users_api_key ON users(api_key);
```

**Advantages**:
- ✅ Can ban users (revoke API key)
- ✅ Can enforce per-user quotas
- ✅ Can track usage for billing
- ✅ Can prioritize premium users

**Disadvantages**:
- ❌ Requires database/cache (Redis/PostgreSQL)
- ❌ Adds latency (DB lookup on registration)
- ❌ More complex deployment
- ❌ Privacy concerns (user tracking)

**Use case**: Enterprise/managed relay with billing.

### Option C: Peer Whitelist (Invite-Only)

**How it works**:
1. Admin adds peer_id to whitelist (via API or config file)
2. Relay only accepts REGISTER from whitelisted peer_ids
3. All others are rejected

**Implementation**:

```rust
struct Config {
    allowed_peers: HashSet<PeerId>,  // Loaded from file or API
}

async fn handle_register(packet: RegisterPacket, config: Arc<Config>) -> Result<()> {
    // Check whitelist
    if !config.allowed_peers.contains(&packet.peer_id) {
        return Err(Error::PeerNotWhitelisted);
    }
    
    // Continue with normal registration...
}
```

**Whitelist sources**:

Option 1: **Static file** (`allowed_peers.txt`)
```
# allowed_peers.txt (one peer_id per line)
abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789
1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef
```

Option 2: **Redis** (dynamic updates)
```rust
let is_allowed: bool = redis_conn.sismember("allowed_peers", &peer_id).await?;
```

**Advantages**:
- ✅ Strong access control
- ✅ Simple to understand
- ✅ No user accounts needed

**Disadvantages**:
- ❌ Manual peer management
- ❌ Doesn't scale to many peers
- ❌ Hard to distribute whitelist across relay instances

**Use case**: Small private relay, trusted group only.

**Recommendation**:
- **Public relay**: Option A (CAPoW-only)
- **Managed/SaaS relay**: Option B (API keys)
- **Private relay**: Option C (Whitelist)

---

## Level 4: Peer-to-Peer ACLs

**Problem**: Should peer A be allowed to send packets to peer B?

### Current State

**Any peer can send to any peer** that's registered.

Example:
```
Peer A (malicious) → RELAY packet to Peer B (victim)
Relay forwards packet (doesn't check if allowed)
```

### Threat Model

| Threat | Impact | Current Mitigation |
|--------|--------|-------------------|
| Spam/harassment | Medium | None (clients must filter) |
| Bandwidth exhaustion | Low | Rate limiting (per sender IP) |
| Privacy leak (peer enumeration) | Low | peer_ids are not enumerable |

### Option A: No P2P ACLs (Current, Recommended)

**Design**: Relay forwards all packets. Clients handle filtering.

**Client-side filtering**:
```rust
// Client code
let allowed_peers: HashSet<PeerId> = load_contacts();

client.on_packet(|from_peer, data| {
    if !allowed_peers.contains(&from_peer) {
        warn!("Dropped packet from unknown peer: {}", from_peer);
        return; // Ignore
    }
    
    handle_packet(data);
});
```

**Advantages**:
- ✅ Simple relay (no state)
- ✅ Fast (no ACL lookups)
- ✅ Privacy (relay doesn't know social graph)

**Disadvantages**:
- ❌ Wastes relay bandwidth (forwards unwanted packets)
- ❌ Cannot prevent spam at network level

**Use case**: Default for most deployments.

### Option B: Server-Side P2P ACLs

**Design**: Relay maintains ACL table per peer.

**ACL Table Structure**:
```rust
struct PeerAcl {
    peer_id: PeerId,
    allowed_destinations: HashSet<PeerId>,  // Whitelist
    blocked_destinations: HashSet<PeerId>,  // Blacklist
}

// In-memory storage
peers_acls: DashMap<PeerId, PeerAcl>
```

**How it works**:
1. Peer A registers and uploads ACL: "I only accept packets from B, C, D"
2. Relay stores ACL
3. Peer X tries to send to A
4. Relay checks ACL: Is X in A's allowed list?
5. If yes: forward. If no: drop.

**Implementation**:

```rust
async fn handle_relay(packet: RelayPacket, state: Arc<State>) -> Result<()> {
    let dest_peer = packet.dest_peer_id;
    
    // Lookup destination peer
    let dest_info = state.peers.get(&dest_peer)
        .ok_or(Error::UnknownPeer)?;
    
    // Check ACL (NEW)
    if let Some(acl) = state.peer_acls.get(&dest_peer) {
        let src_peer = state.find_peer_by_addr(src_addr)?;
        
        // Check whitelist
        if !acl.allowed_destinations.is_empty() 
            && !acl.allowed_destinations.contains(&src_peer) {
            return Err(Error::Blocked);  // Drop packet
        }
        
        // Check blacklist
        if acl.blocked_destinations.contains(&src_peer) {
            return Err(Error::Blocked);
        }
    }
    
    // Forward packet
    state.endpoint.send_to(&packet.payload, dest_info.addr).await?;
    Ok(())
}
```

**ACL Upload Mechanism**:

New packet type: **ACL_UPDATE** (0x05)
```
┌──────────────────────────────────────────────────────┐
│ Type (1) │ Peer ID (32) │ ACL (variable)           │
├──────────┼──────────────┼──────────────────────────┤
│   0x05   │  [peer_id]   │  [allowed_peer_ids...]   │
└──────────────────────────────────────────────────────┘
```

**Advantages**:
- ✅ Stops spam at relay (saves bandwidth)
- ✅ Stronger privacy control
- ✅ Can enforce platform-wide bans

**Disadvantages**:
- ❌ Relay maintains state (ACL table)
- ❌ ACL lookups add latency (~100µs per packet)
- ❌ Privacy concern (relay knows social graph)
- ❌ More complex

**Use case**: Managed relay with user accounts, anti-spam requirements.

**Recommendation**: **Option A (No P2P ACLs)** for public relay. Option B only if spam is a proven problem.

---

## Level 5: Application-Level ACLs

**Problem**: Authorization within the application (e.g., "who can send messages in this chat room?").

### Responsibility: CLIENT-SIDE

The relay **should not** handle application logic. This includes:
- Chat room permissions
- File sharing permissions
- Group membership
- Feature access control

**Why?**
- Relay is transport layer (OSI Layer 4/5)
- Application is Layer 7
- Mixing concerns breaks zero-trust model

**How clients should handle it**:

```rust
// Example: Chat application with room permissions
struct ChatClient {
    relay_client: RelayClient,
    room_acls: HashMap<RoomId, RoomAcl>,
}

struct RoomAcl {
    room_id: RoomId,
    members: HashSet<PeerId>,
    moderators: HashSet<PeerId>,
}

impl ChatClient {
    async fn handle_message(&self, from_peer: PeerId, msg: ChatMessage) {
        // Application-level ACL check
        let room_acl = self.room_acls.get(&msg.room_id).unwrap();
        
        if !room_acl.members.contains(&from_peer) {
            warn!("Peer {} not in room {}", from_peer, msg.room_id);
            return; // Ignore message
        }
        
        // Further checks (e.g., is sender banned, is room read-only, etc.)
        
        // Display message
        self.display_message(msg);
    }
}
```

**Recommendation**: Always handle application ACLs client-side, not in relay.

---

## Multi-Tenancy Isolation

**Problem**: Multiple organizations/customers using the same relay. How to isolate them?

### Option A: Tenant ID in Peer ID

**Design**: Encode tenant ID in peer_id.

```
peer_id = tenant_id (16 bytes) || random (16 bytes)
```

**Relay enforcement**:
```rust
async fn handle_relay(packet: RelayPacket) -> Result<()> {
    let src_tenant = extract_tenant_id(src_peer_id);
    let dest_tenant = extract_tenant_id(packet.dest_peer_id);
    
    if src_tenant != dest_tenant {
        return Err(Error::CrossTenantNotAllowed);
    }
    
    // Forward packet
}
```

**Advantages**:
- ✅ Simple (no external state)
- ✅ Fast (tenant ID in packet)

**Disadvantages**:
- ❌ Reduces peer_id entropy (only 16 bytes random)
- ❌ Tenant ID visible in packets (privacy concern)

### Option B: Tenant-Specific Relay Instances

**Design**: Deploy separate relay instance per tenant.

```
Tenant A → relay-a.example.com:8080
Tenant B → relay-b.example.com:8080
```

**Kubernetes**:
```yaml
# Namespace per tenant
apiVersion: v1
kind: Namespace
metadata:
  name: tenant-a

---
# Deployment per tenant
apiVersion: apps/v1
kind: Deployment
metadata:
  name: file-relay
  namespace: tenant-a
spec:
  replicas: 3
  template:
    spec:
      containers:
        - name: relay
          image: file-relay:latest
          env:
            - name: TENANT_ID
              value: "tenant-a"
```

**Advantages**:
- ✅ Strong isolation (separate processes, namespaces)
- ✅ Easy to scale per tenant
- ✅ Tenant-specific configs (rate limits, etc.)

**Disadvantages**:
- ❌ Higher infrastructure cost (more pods)
- ❌ More complex to manage

**Recommendation**: **Option B (Separate Instances)** for enterprise SaaS. Option A for lightweight multi-tenancy.

---

## Implementation Roadmap

### Phase 1: Network & Admin ACLs (1 week)

**Goal**: Secure metrics endpoint and document firewall best practices.

**Deliverables**:
- [ ] Add NetworkPolicy examples to Kubernetes manifests
- [ ] Document firewall rules (AWS/GCP security groups)
- [ ] Add HTTP Basic Auth option for metrics endpoint
- [ ] Update SECURITY-HARDENING-CHECKLIST.md

### Phase 2: API Key Authentication (2 weeks)

**Goal**: Enable user-based access control for managed relay.

**Deliverables**:
- [ ] Design API key schema (PostgreSQL or Redis)
- [ ] Implement REGISTER_V2 packet with API key
- [ ] Add API key validation middleware
- [ ] Create admin API for key management (CRUD)
- [ ] Update CLIENT-INTEGRATION-GUIDE.md

### Phase 3: P2P ACLs (Optional, 2 weeks)

**Goal**: Server-side spam prevention.

**Deliverables**:
- [ ] Design ACL_UPDATE packet format
- [ ] Implement ACL table (in-memory, DashMap)
- [ ] Add ACL checks to RELAY handler
- [ ] ACL persistence (Redis for multi-instance)
- [ ] Metrics (ACL hits, blocks)

---

## Security Considerations

### API Key Storage

**Never store plaintext API keys**. Use:

```rust
// Hash API key before storage
use argon2::{Argon2, PasswordHasher};

let api_key_hash = Argon2::default()
    .hash_password(api_key.as_bytes(), salt)?
    .to_string();

// Store hash in database
db.execute("INSERT INTO users (api_key_hash) VALUES (?)", [api_key_hash])?;
```

### ACL Injection

**Problem**: Attacker uploads malicious ACL to DoS relay.

Example:
```rust
// Malicious ACL with 1 million entries
let malicious_acl = (0..1_000_000)
    .map(|_| PeerId::generate())
    .collect::<HashSet<_>>();
```

**Mitigation**:
```rust
const MAX_ACL_SIZE: usize = 1000;  // Cap at 1000 entries

if acl.allowed_destinations.len() > MAX_ACL_SIZE {
    return Err(Error::AclTooLarge);
}
```

---

## Monitoring & Metrics

### New Metrics for ACLs

```rust
struct AclMetrics {
    api_key_validations_total: AtomicU64,     // Total API key checks
    api_key_invalid_total: AtomicU64,         // Invalid API key attempts
    acl_blocks_total: AtomicU64,              // P2P packets blocked by ACL
    acl_updates_total: AtomicU64,             // ACL_UPDATE packets processed
}
```

**Prometheus queries**:
```promql
# API key failure rate (should be < 1%)
rate(file_relay_api_key_invalid_total[5m]) / rate(file_relay_api_key_validations_total[5m])

# ACL block rate
rate(file_relay_acl_blocks_total[5m])
```

---

## FAQ

### Do I need ACLs?

**For public relay**: No. CAPoW + rate limiting is sufficient.

**For enterprise relay**: Yes. Use API keys + quotas.

**For private relay**: Yes. Use whitelist or VPN.

### Does adding ACLs break zero-trust?

Partially. API keys mean the relay knows user identity. But:
- Packet contents still encrypted end-to-end
- Relay still can't decrypt payloads
- Just adds authentication layer

### Can I use OAuth2 instead of API keys?

Yes, but it's more complex:
1. User authenticates via OAuth2 (Google, GitHub, etc.)
2. Your backend issues short-lived JWT
3. Client includes JWT in REGISTER packet
4. Relay validates JWT signature

**Trade-off**: More secure (expiring tokens) but adds auth service dependency.

---

## Research Foundation

This ACCESS-CONTROL.md design is informed by comprehensive industry research:

### 📚 Research Reports

- **[ACL Patterns Research (2026-05-18)](../research/ACL-PATTERNS-RESEARCH-2026-05-18.md)** — Full research report (9,800 words)
  - TURN/STUN RFCs (5766, 8656)
  - Tailscale DERP architecture
  - LibP2P authentication patterns
  - WireGuard key management
  - Signal relay security
  - Zero-trust architectures
  - GDPR compliance
  - SOC 2 requirements

- **[Executive Summary](../research/ACL-RESEARCH-EXECUTIVE-SUMMARY.md)** — Key findings and recommendations

### Key Validation

Research confirms FILE's 5-level ACL design aligns with industry best practices:

| FILE Level | Industry Analog | Sources |
|------------|----------------|---------|
| Level 1: Admin ACLs | Prometheus, Grafana | Standard practice |
| Level 2: Network ACLs | TURN servers, Tailscale DERP | RFC 5766, RFC 8656 |
| Level 3: Registration ACLs | Coturn REST API, Tailscale keys | Time-limited tokens |
| Level 4: P2P ACLs | LibP2P client filtering, Signal | Zero-trust design |
| Level 5: Application ACLs | Universal (no relay enforcement) | Separation of concerns |

**Architectural Verdict**: ✅ **Design is sound**. Focus on implementation quality.

---

## References

- [THREAT-MODEL.md](THREAT-MODEL.md) — Security analysis
- [PROTOCOL-SPECIFICATION.md](PROTOCOL-SPECIFICATION.md) — Packet formats
- [SECURITY-HARDENING-CHECKLIST.md](../deployment/SECURITY-HARDENING-CHECKLIST.md)
- [ACL Research (2026-05-18)](../research/ACL-PATTERNS-RESEARCH-2026-05-18.md) — Industry standards validation

---

**Status**: Design document (implementation pending)  
**Research Completed**: 2026-05-18 (10+ hours, 30+ sources)  
**Last Updated**: 2026-05-18
