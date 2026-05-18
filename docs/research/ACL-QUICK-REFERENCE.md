# ACL Quick Reference Card
## FILE Relay Server Access Control — Actionable Patterns

**Date**: 2026-05-18  
**For**: Quick lookup during implementation  
**Full Report**: [ACL-PATTERNS-RESEARCH-2026-05-18.md](ACL-PATTERNS-RESEARCH-2026-05-18.md)

---

## 1. Authentication Mechanisms (Choose One)

### Option A: CAPoW-Only (Public Relay) ⭐⭐⭐⭐⭐

**Use Case**: Public relay, open access, privacy-first

```rust
// Client-side: Generate CAPoW proof
let proof = capow::generate_proof(&challenge, difficulty).await?;

// Server-side: Verify proof
if !capow::verify_proof(&proof, &challenge, difficulty).await? {
    return Err(Error::InvalidCapow);
}
```

**Pros**: Stateless, scales horizontally, no user accounts  
**Cons**: Cannot ban users, no quotas, no billing

### Option B: Time-Limited HMAC Tokens (Managed Relay) ⭐⭐⭐⭐

**Use Case**: Managed relay with user accounts and quotas

```rust
// Backend: Issue token (24h expiry)
fn issue_token(peer_id: &PeerId, secret: &[u8]) -> String {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).unwrap();
    mac.update(peer_id.as_bytes());
    mac.update(&timestamp.to_le_bytes());
    let hmac = mac.finalize().into_bytes();
    
    format!("{}:{}", timestamp, hex::encode(hmac))
}

// Relay: Validate token
fn validate_token(token: &str, peer_id: &PeerId, secret: &[u8]) -> Result<(), Error> {
    let (timestamp_str, hmac_hex) = token.split_once(':').ok_or(Error::InvalidToken)?;
    let timestamp = timestamp_str.parse::<u64>()?;
    
    // Check expiry (24 hours)
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    if now - timestamp > 86400 {
        return Err(Error::TokenExpired);
    }
    
    // Verify HMAC
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).unwrap();
    mac.update(peer_id.as_bytes());
    mac.update(&timestamp.to_le_bytes());
    mac.verify_slice(&hex::decode(hmac_hex)?)?;
    
    Ok(())
}
```

**Pros**: Stateless, user management, quotas, billing  
**Cons**: Requires token-issuing backend, cannot revoke tokens before expiry

### Option C: OAuth 2.0 (Enterprise) ⭐⭐

**Use Case**: Enterprise with existing OAuth infrastructure

**Complexity**: Requires authorization server, token validation endpoint  
**Recommendation**: ❌ **Overkill**—use Option B (HMAC tokens) instead

---

## 2. Rate Limiting (DashMap + Token Bucket)

### Per-IP Rate Limiting ⭐⭐⭐⭐⭐

**Limits**: 10 req/s sustained, 50 burst

```rust
use dashmap::DashMap;
use governor::{Quota, RateLimiter as Governor};

struct RateLimiter {
    per_ip: DashMap<IpAddr, Governor>,
}

impl RateLimiter {
    fn new() -> Self {
        Self {
            per_ip: DashMap::new(),
        }
    }
    
    async fn check(&self, ip: IpAddr) -> Result<(), Error> {
        let limiter = self.per_ip.entry(ip).or_insert_with(|| {
            Governor::direct(Quota::per_second(10u32).allow_burst(50u32.try_into().unwrap()))
        });
        
        limiter.check().map_err(|_| Error::RateLimitExceeded)?;
        Ok(())
    }
    
    // Cleanup old entries (run every 5 minutes)
    fn cleanup(&self) {
        self.per_ip.retain(|_, limiter| {
            // Keep if used in last 5 minutes
            limiter.check().is_ok()
        });
    }
}
```

### Per-User Rate Limiting (Optional)

**Limits**: Max 10 peers per user, 1 GB/day bandwidth

```rust
struct UserQuota {
    user_id: UserId,
    current_peers: AtomicUsize,
    bandwidth_used: AtomicU64,  // bytes
    reset_at: AtomicU64,         // unix timestamp
}

impl UserQuota {
    fn check_peer_limit(&self) -> Result<(), Error> {
        if self.current_peers.load(Ordering::Relaxed) >= 10 {
            return Err(Error::QuotaExceeded);
        }
        Ok(())
    }
    
    fn check_bandwidth(&self, bytes: u64) -> Result<(), Error> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let reset_at = self.reset_at.load(Ordering::Relaxed);
        
        // Reset daily quota
        if now >= reset_at {
            self.bandwidth_used.store(0, Ordering::Relaxed);
            self.reset_at.store(now + 86400, Ordering::Relaxed);
        }
        
        let used = self.bandwidth_used.fetch_add(bytes, Ordering::Relaxed);
        if used > 1_000_000_000 {  // 1 GB
            return Err(Error::BandwidthQuotaExceeded);
        }
        
        Ok(())
    }
}
```

---

## 3. GDPR Compliance (IP Hashing)

### HMAC-SHA256 IP Anonymization ⭐⭐⭐⭐⭐

**Requirement**: Never log full IP addresses (GDPR violation)

```rust
use hmac::{Hmac, Mac};
use sha2::Sha256;

struct IpHasher {
    secret: [u8; 32],  // Rotate monthly
}

impl IpHasher {
    fn hash_ip(&self, ip: IpAddr) -> [u8; 32] {
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.secret).unwrap();
        mac.update(ip.to_string().as_bytes());
        mac.finalize().into_bytes().into()
    }
}

// Usage in logging
tracing::info!(
    ip_hash = %hex::encode(ip_hasher.hash_ip(src_addr.ip())),
    peer_id = %hex::encode(peer_id),
    event = "registration_success"
);
```

**Key Rotation**:
```rust
// Rotate HMAC secret monthly (keep old secret for 30 days for log correlation)
struct IpHasherWithRotation {
    current: IpHasher,
    previous: Option<IpHasher>,
    rotation_at: Instant,
}
```

---

## 4. Transport Security (Noise XX)

### Mutual Authentication Before Registration ⭐⭐⭐⭐⭐

**Requirement**: SEC-C06 critical finding—no plaintext registration

```rust
use snow::{Builder, params::NoiseParams};

// Server-side: Accept Noise handshake
async fn accept_noise_handshake(stream: &mut QuicStream) -> Result<(PeerId, Session), Error> {
    let params: NoiseParams = "Noise_XX_25519_ChaChaPoly_BLAKE2s".parse()?;
    let builder = Builder::new(params.clone());
    let static_key = load_server_private_key()?;
    let mut noise = builder
        .local_private_key(&static_key)
        .build_responder()?;
    
    // Handshake: -> e
    let mut buf = vec![0u8; 65535];
    let len = stream.read(&mut buf).await?;
    noise.read_message(&buf[..len], &mut [])?;
    
    // Handshake: <- e, ee, s, es
    let len = noise.write_message(&[], &mut buf)?;
    stream.write_all(&buf[..len]).await?;
    
    // Handshake: -> s, se
    let len = stream.read(&mut buf).await?;
    noise.read_message(&buf[..len], &mut [])?;
    
    // Extract peer identity
    let peer_pubkey = noise.get_remote_static().ok_or(Error::NoRemoteStatic)?;
    let peer_id = PeerId::from_ed25519(peer_pubkey)?;
    
    // Convert to transport mode
    let session = noise.into_transport_mode()?;
    
    Ok((peer_id, session))
}
```

---

## 5. Peer ID Validation

### Ed25519 Public Key Verification ⭐⭐⭐⭐⭐

**Requirement**: SEC-C05 critical finding—no validation

```rust
fn validate_peer_id(peer_id: &[u8]) -> Result<PeerId, Error> {
    // Check length (Ed25519 public key = 32 bytes)
    if peer_id.len() != 32 {
        return Err(Error::InvalidPeerIdLength);
    }
    
    // Verify Ed25519 public key is valid (on curve)
    let public_key = ed25519_dalek::PublicKey::from_bytes(peer_id)
        .map_err(|_| Error::InvalidPeerIdFormat)?;
    
    Ok(PeerId::from(public_key))
}

// Duplicate detection (same peer_id from different IP)
fn check_duplicate_peer(
    peers: &DashMap<PeerId, PeerInfo>,
    peer_id: &PeerId,
    src_addr: SocketAddr,
) -> Result<(), Error> {
    if let Some(existing) = peers.get(peer_id) {
        if existing.addr != src_addr {
            // Same peer_id from different IP (possible attack or legitimate roaming)
            tracing::warn!(
                peer_id = %hex::encode(peer_id),
                existing_ip = %existing.addr.ip(),
                new_ip = %src_addr.ip(),
                "Duplicate peer_id from different IP"
            );
            
            // Option 1: Reject (strict)
            // return Err(Error::DuplicatePeerId);
            
            // Option 2: Update (allow roaming)
            drop(existing);
            peers.insert(*peer_id, PeerInfo {
                addr: src_addr,
                last_seen: Instant::now(),
            });
        }
    }
    
    Ok(())
}
```

---

## 6. Admin Access Control

### Level 1: Metrics Endpoint Security

#### Option A: Network Isolation (Recommended) ⭐⭐⭐⭐⭐

**Kubernetes NetworkPolicy**:
```yaml
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

**Pros**: Simple, standard practice  
**Cons**: Assumes trusted internal network

#### Option B: HTTP Basic Auth (Multi-Network) ⭐⭐⭐

```rust
use axum::{
    middleware::{self, Next},
    http::{Request, StatusCode, header::AUTHORIZATION},
};
use base64::{engine::general_purpose::STANDARD, Engine};

async fn auth_middleware<B>(
    State(config): State<Arc<Config>>,
    req: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    let auth_header = req.headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    if !auth_header.starts_with("Basic ") {
        return Err(StatusCode::UNAUTHORIZED);
    }
    
    let encoded = &auth_header[6..];
    let decoded = STANDARD.decode(encoded).map_err(|_| StatusCode::UNAUTHORIZED)?;
    let credentials = String::from_utf8(decoded).map_err(|_| StatusCode::UNAUTHORIZED)?;
    
    let (username, password) = credentials
        .split_once(':')
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    if username == config.metrics_username && password == config.metrics_password {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}
```

**Prometheus Config**:
```yaml
scrape_configs:
  - job_name: 'file-relay'
    static_configs:
      - targets: ['relay:8081']
    basic_auth:
      username: 'prometheus'
      password_file: /etc/prometheus/metrics_password
```

---

## 7. P2P ACLs (Client-Side Filtering)

### Recommended Approach: Client Enforces ⭐⭐⭐⭐⭐

**Rationale**: Relay doesn't know application context (who should talk to whom)

```rust
// Client-side filtering
struct FileClient {
    allowed_peers: HashSet<PeerId>,
}

impl FileClient {
    async fn on_packet(&self, from_peer: PeerId, data: Vec<u8>) {
        // Check whitelist
        if !self.allowed_peers.contains(&from_peer) {
            tracing::warn!(
                peer_id = %hex::encode(from_peer),
                "Dropped packet from unknown peer"
            );
            return;
        }
        
        // Process packet
        self.handle_packet(data).await;
    }
}
```

**Pros**: Relay stays stateless, privacy-preserving, simple  
**Cons**: Wastes relay bandwidth (forwards unwanted packets)

### Alternative: Server-Side P2P ACLs (Only if Spam Proven) ⭐⭐

**Use Case**: Spam is rampant, bandwidth waste is significant

```rust
struct PeerAcl {
    peer_id: PeerId,
    allowed_peers: HashSet<PeerId>,  // Whitelist (max 1000 entries)
}

async fn handle_relay(packet: RelayPacket, state: Arc<State>) -> Result<()> {
    // Lookup ACL
    if let Some(acl) = state.peer_acls.get(&packet.dest_peer_id) {
        let src_peer = state.find_peer_by_addr(src_addr)?;
        
        if !acl.allowed_peers.is_empty() && !acl.allowed_peers.contains(&src_peer) {
            tracing::debug!(
                src = %hex::encode(src_peer),
                dest = %hex::encode(packet.dest_peer_id),
                "Blocked by ACL"
            );
            return Err(Error::BlockedByAcl);
        }
    }
    
    // Forward packet
    let dest_info = state.peers.get(&packet.dest_peer_id).ok_or(Error::UnknownPeer)?;
    state.endpoint.send_to(&packet.payload, dest_info.addr).await?;
    Ok(())
}
```

**Trade-off**: Relay learns social graph (privacy leak)

---

## 8. Metrics and Monitoring

### Critical Metrics ⭐⭐⭐⭐⭐

```rust
struct Metrics {
    // Counters
    registrations_total: AtomicU64,
    registrations_failed: AtomicU64,
    auth_failures_total: AtomicU64,
    packets_forwarded_total: AtomicU64,
    capow_verifications_total: AtomicU64,
    
    // Gauges
    active_peers: AtomicU64,
    rate_limit_exceeded_total: AtomicU64,
    
    // Histograms
    capow_verification_duration_seconds: Histogram,
    packet_forward_duration_seconds: Histogram,
}
```

**Prometheus Queries**:
```promql
# Auth failure rate (alert if >5%)
rate(relay_auth_failures_total[5m]) / rate(relay_registrations_total[5m]) > 0.05

# P99 CAPoW latency (alert if >200ms)
histogram_quantile(0.99, rate(relay_capow_verification_duration_seconds_bucket[5m])) > 0.2

# Active peers capacity (alert if >900/1000)
relay_active_peers > 900
```

---

## 9. Multi-Tenancy Isolation

### Option A: Tenant ID in Peer ID ⭐⭐⭐

**Use Case**: Small deployments (2-10 tenants)

```rust
// Encode tenant ID in peer_id
struct PeerId {
    tenant_id: [u8; 16],
    random: [u8; 16],
}

fn validate_relay(src: PeerId, dest: PeerId) -> Result<(), Error> {
    if src.tenant_id != dest.tenant_id {
        return Err(Error::CrossTenantNotAllowed);
    }
    Ok(())
}
```

**Pros**: Stateless, simple  
**Cons**: Reduces peer_id entropy, tenant ID visible in packets

### Option B: Separate Relay Instances ⭐⭐⭐⭐⭐

**Use Case**: Enterprise B2B (10-100 tenants)

**Kubernetes**:
```yaml
# Deploy one relay per tenant namespace
apiVersion: apps/v1
kind: Deployment
metadata:
  name: file-relay
  namespace: tenant-acme
spec:
  replicas: 3
  template:
    spec:
      containers:
        - name: relay
          image: file-relay:latest
          env:
            - name: TENANT_ID
              value: "acme"
```

**Pros**: Strong isolation, tenant-specific configs  
**Cons**: Higher cost (more pods)

---

## Decision Tree

```
Start here
    │
    ├─ Public relay (open access)?
    │   └─ Use CAPoW-only + per-IP rate limiting
    │
    ├─ Managed relay (user accounts)?
    │   └─ Use HMAC tokens + per-user quotas
    │
    ├─ Enterprise B2B?
    │   └─ Separate relay instances per tenant
    │
    ├─ Need P2P ACLs?
    │   ├─ No spam yet? → Client-side filtering
    │   └─ Spam proven? → Server-side ACLs
    │
    └─ Need SOC 2?
        └─ Add admin access controls + audit logging
```

---

## Critical Checklist

Before deploying FILE relay to production:

- [ ] **SEC-C06**: Noise XX handshake before registration
- [ ] **SEC-C01**: CAPoW verification enabled
- [ ] **SEC-C05**: Peer ID validation (32 bytes, Ed25519)
- [ ] **SEC-C03**: Per-IP rate limiting (10/s sustained, 50 burst)
- [ ] **SEC-C04**: IP address hashing (HMAC-SHA256)
- [ ] **SEC-C02**: Packet forwarding implemented
- [ ] **Level 1**: Metrics endpoint secured (network isolation or Basic Auth)
- [ ] **Monitoring**: Prometheus metrics + Grafana dashboard
- [ ] **Logging**: Structured JSON logs with hashed IDs
- [ ] **GDPR**: Privacy policy updated (IP hashing, retention limits)

---

**Quick Reference Completed**: 2026-05-18  
**Full Research**: [ACL-PATTERNS-RESEARCH-2026-05-18.md](ACL-PATTERNS-RESEARCH-2026-05-18.md)  
**Executive Summary**: [ACL-RESEARCH-EXECUTIVE-SUMMARY.md](ACL-RESEARCH-EXECUTIVE-SUMMARY.md)
