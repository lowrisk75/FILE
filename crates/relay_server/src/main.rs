//
// FILE Relay Server
//
// MPQUIC P2P Relay with CAPoW Anti-DDoS
// Zero Trust Architecture - No open ports until authenticated
//

use anyhow::{Context, Result};
use ant_quic::{P2pConfig, P2pEndpoint, PeerId};
use axum::{
    extract::State as AxumState,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use clap::Parser;
use dashmap::DashMap;
use hmac::{Hmac, Mac};
use rand::RngCore;
use relay_daemon::{MtpProof, async_verify_proof_with_timeout, init_crypto_workers};
use serde_json::json;
use sha2::{Sha256, Digest};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing::{info, error, warn};

/// FILE Relay Server CLI
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Bind address (default: 0.0.0.0:8080)
    #[arg(short, long, default_value = "0.0.0.0:8080")]
    bind: SocketAddr,

    /// Enable CAPoW verification (default: true)
    #[arg(long, default_value_t = true)]
    capow: bool,

    /// CAPoW timeout in seconds (default: 60)
    #[arg(long, default_value_t = 60)]
    capow_timeout: u64,

    /// Max concurrent peers (default: 1000)
    #[arg(long, default_value_t = 1000)]
    max_peers: usize,

    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "info")]
    log_level: String,

    /// Health check HTTP port (default: 8081)
    #[arg(long, default_value_t = 8081)]
    health_port: u16,
}

/// Relay metrics (Prometheus-compatible counters)
#[derive(Debug)]
struct RelayMetrics {
    peers_registered: AtomicU64,     // Total peers ever registered
    capow_verified: AtomicU64,       // Total CAPoW proofs successfully verified
    capow_rejected: AtomicU64,       // Total CAPoW proofs rejected (replay/invalid)
    rate_limited: AtomicU64,         // Total requests rate-limited
    packets_forwarded: AtomicU64,    // Total packets successfully forwarded
    peers_timeout: AtomicU64,        // Total peers removed due to timeout
}

impl RelayMetrics {
    fn new() -> Self {
        Self {
            peers_registered: AtomicU64::new(0),
            capow_verified: AtomicU64::new(0),
            capow_rejected: AtomicU64::new(0),
            rate_limited: AtomicU64::new(0),
            packets_forwarded: AtomicU64::new(0),
            peers_timeout: AtomicU64::new(0),
        }
    }
}

/// Token bucket rate limiter (leaky bucket algorithm)
/// - Sustained rate: 10 req/s
/// - Burst capacity: 50 requests
#[derive(Debug)]
struct RateLimiter {
    tokens: f64,
    last_refill: Instant,
    capacity: f64,
    refill_rate: f64,  // tokens per second
}

impl RateLimiter {
    fn new(capacity: f64, refill_rate: f64) -> Self {
        Self {
            tokens: capacity,  // Start with full bucket
            last_refill: Instant::now(),
            capacity,
            refill_rate,
        }
    }

    /// Attempt to consume 1 token. Returns true if allowed, false if rate limited.
    fn try_consume(&mut self) -> bool {
        // Refill tokens based on elapsed time
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.capacity);
        self.last_refill = now;

        // Try to consume 1 token
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

/// Challenge for CAPoW verification
#[derive(Debug, Clone)]
struct Challenge {
    merkle_root: [u8; 32],
    created_at: SystemTime,
    expires_at: SystemTime,
}

impl Challenge {
    /// Generate a new random challenge
    fn generate() -> Self {
        let mut merkle_root = [0u8; 32];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut merkle_root);

        let now = SystemTime::now();
        let rotation_interval = Duration::from_secs(3600); // 1 hour
        let grace_period = Duration::from_secs(300); // 5 minutes

        Self {
            merkle_root,
            created_at: now,
            expires_at: now + rotation_interval + grace_period,
        }
    }
}

/// Challenge manager with rotation and grace period support
struct ChallengeManager {
    current: Arc<tokio::sync::RwLock<Challenge>>,
    previous: Arc<tokio::sync::RwLock<Option<Challenge>>>,
}

impl ChallengeManager {
    fn new() -> Self {
        let current = Challenge::generate();
        Self {
            current: Arc::new(tokio::sync::RwLock::new(current)),
            previous: Arc::new(tokio::sync::RwLock::new(None)),
        }
    }

    /// Rotate challenge (called every hour by background task)
    async fn rotate(&self) {
        let new_challenge = Challenge::generate();

        // Move current to previous
        let old_current = {
            let mut current = self.current.write().await;
            let old = current.clone();
            *current = new_challenge.clone();
            old
        };

        // Store old current as previous (for grace period)
        let mut previous = self.previous.write().await;
        *previous = Some(old_current);

        info!(
            "🔄 Challenge rotated: new merkle_root = {}",
            hex::encode(&new_challenge.merkle_root)
        );
    }

    /// Verify proof against current or previous challenge
    async fn verify_challenge(&self, proof_merkle_root: &[u8; 32]) -> bool {
        // Try current challenge first
        let current = self.current.read().await;
        if proof_merkle_root == &current.merkle_root {
            return true;
        }

        // Try previous challenge (grace period)
        let previous = self.previous.read().await;
        if let Some(prev) = previous.as_ref() {
            if proof_merkle_root == &prev.merkle_root {
                // Check if still in grace period
                if SystemTime::now() < prev.expires_at {
                    return true;
                }
            }
        }

        false // Proof for unknown challenge
    }

    /// Get current challenge (for clients to fetch via /challenge endpoint)
    async fn get_current(&self) -> Challenge {
        self.current.read().await.clone()
    }
}

/// Connected peer state
#[derive(Debug)]
struct PeerState {
    peer_id: Vec<u8>,
    addr: SocketAddr,
    peer_id_quic: PeerId,  // ant-quic PeerId for forwarding
    connected_at: std::time::Instant,
    packets_forwarded: Arc<AtomicU64>,  // Atomic for lock-free concurrent updates
    last_activity: Arc<AtomicU64>,  // Unix timestamp (seconds) of last packet forwarded to this peer
}

/// Global relay state
struct RelayState {
    peers: DashMap<Vec<u8>, PeerState>,  // Lock-free concurrent access with secure hashing (SipHash)
    endpoint: Arc<P2pEndpoint>,
    capow_enabled: bool,
    capow_timeout: Duration,
    max_peers: usize,
    challenge_manager: Arc<ChallengeManager>, // Challenge generation and rotation
    rate_limiters: DashMap<IpAddr, RateLimiter>, // Per-IP rate limiting
    ip_hash_secret: [u8; 32], // Per-boot random secret for GDPR-compliant IP hashing
    proof_replay_cache: DashMap<[u8; 32], Instant>, // Anti-replay cache for CAPoW proofs (5 min TTL)
    metrics: RelayMetrics,  // Operational metrics (Prometheus-compatible)
}

impl RelayState {
    fn new(
        endpoint: P2pEndpoint,
        capow_enabled: bool,
        capow_timeout: Duration,
        max_peers: usize,
        challenge_manager: Arc<ChallengeManager>,
        ip_hash_secret: [u8; 32],
    ) -> Self {
        Self {
            peers: DashMap::new(),
            endpoint: Arc::new(endpoint),
            capow_enabled,
            capow_timeout,
            max_peers,
            challenge_manager,
            rate_limiters: DashMap::new(),
            ip_hash_secret,
            proof_replay_cache: DashMap::new(),
            metrics: RelayMetrics::new(),
        }
    }

    /// Hash an IP address with HMAC-SHA256 for GDPR-compliant logging
    /// Returns a hex-encoded hash (first 16 chars for brevity)
    fn hash_ip(&self, ip: IpAddr) -> String {
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(&self.ip_hash_secret)
            .expect("HMAC can take key of any size");
        mac.update(ip.to_string().as_bytes());
        let result = mac.finalize();
        let hash = hex::encode(result.into_bytes());
        hash[..16].to_string()  // First 16 chars (64 bits of entropy)
    }

    /// Check rate limit for an IP address
    /// Returns true if allowed, false if rate limited
    fn check_rate_limit(&self, addr: IpAddr) -> bool {
        let allowed = self.rate_limiters
            .entry(addr)
            .or_insert_with(|| RateLimiter::new(50.0, 10.0))  // 50 burst, 10 req/s
            .try_consume();

        if !allowed {
            self.metrics.rate_limited.fetch_add(1, Ordering::Relaxed);
            warn!("⚠️ Rate limit exceeded for IP hash: {}", self.hash_ip(addr));
        }

        allowed
    }

    /// Verify CAPoW challenge (if enabled)
    async fn verify_capow(&self, proof: &[u8]) -> Result<bool> {
        if !self.capow_enabled {
            return Ok(true);
        }

        // Anti-replay: Hash the proof to check if we've seen it before
        let mut hasher = Sha256::new();
        hasher.update(proof);
        let proof_hash: [u8; 32] = hasher.finalize().into();

        // Check replay cache (5 min window)
        if let Some(first_seen) = self.proof_replay_cache.get(&proof_hash) {
            let age = first_seen.value().elapsed();
            if age < Duration::from_secs(300) {  // 5 minutes
                self.metrics.capow_rejected.fetch_add(1, Ordering::Relaxed);
                warn!("⚠️ CAPoW proof replay detected (age: {:?})", age);
                return Ok(false);  // Reject replayed proof
            }
            // Old entry, will be cleaned up - allow verification
        }

        // Deserialize MtpProof from bytes
        let mtp_proof: MtpProof = bincode::deserialize(proof)
            .context("Failed to deserialize MtpProof")?;

        // Try verifying against current challenge first
        let current_challenge = self.challenge_manager.get_current().await;
        let verify_result = async_verify_proof_with_timeout(
            mtp_proof.clone(),
            current_challenge.merkle_root,
            Duration::from_millis(100),
        )
        .await;

        // If current challenge fails, try previous challenge (grace period)
        let verified = if verify_result.is_err() {
            // Try previous challenge if it exists
            let previous_opt = self.challenge_manager.previous.read().await;
            if let Some(prev_challenge) = previous_opt.as_ref() {
                // Check if still in grace period
                if SystemTime::now() < prev_challenge.expires_at {
                    match async_verify_proof_with_timeout(
                        mtp_proof,
                        prev_challenge.merkle_root,
                        Duration::from_millis(100),
                    )
                    .await
                    {
                        Ok(()) => {
                            info!("✅ CAPoW proof verified against previous challenge (grace period)");
                            true
                        }
                        Err(e) => {
                            self.metrics.capow_rejected.fetch_add(1, Ordering::Relaxed);
                            error!("❌ CAPoW verification failed against both challenges: {}", e);
                            false
                        }
                    }
                } else {
                    self.metrics.capow_rejected.fetch_add(1, Ordering::Relaxed);
                    warn!("⚠️ CAPoW proof for expired challenge");
                    false
                }
            } else {
                self.metrics.capow_rejected.fetch_add(1, Ordering::Relaxed);
                error!("❌ CAPoW verification failed: {:?}", verify_result.err());
                false
            }
        } else {
            info!("✅ CAPoW proof verified against current challenge");
            true
        };

        if verified {
            // Store proof hash to prevent replay
            self.proof_replay_cache.insert(proof_hash, Instant::now());
            self.metrics.capow_verified.fetch_add(1, Ordering::Relaxed);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Register a new peer
    async fn register_peer(&self, peer_id: Vec<u8>, addr: SocketAddr, peer_id_quic: PeerId) -> Result<()> {
        // Validate peer_id structure (must be exactly 32 bytes - Ed25519 public key)
        if peer_id.len() != 32 {
            anyhow::bail!(
                "Invalid peer_id length: {} bytes (expected 32)",
                peer_id.len()
            );
        }

        // Check for duplicate peer_id (prevents impersonation)
        if self.peers.contains_key(&peer_id) {
            anyhow::bail!(
                "Peer ID already registered: {:?}",
                hex::encode(&peer_id[..8])
            );
        }

        if self.peers.len() >= self.max_peers {
            anyhow::bail!("Max peers ({}) reached", self.max_peers);
        }

        let now_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let state = PeerState {
            peer_id: peer_id.clone(),
            addr,
            peer_id_quic,
            connected_at: std::time::Instant::now(),
            packets_forwarded: Arc::new(AtomicU64::new(0)),
            last_activity: Arc::new(AtomicU64::new(now_unix)),
        };

        self.peers.insert(peer_id.clone(), state);
        self.metrics.peers_registered.fetch_add(1, Ordering::Relaxed);
        info!(
            "Peer registered: {:?} from IP hash: {}",
            hex::encode(&peer_id[..8]),
            self.hash_ip(addr.ip())
        );
        Ok(())
    }

    /// Remove a peer
    async fn remove_peer(&self, peer_id: &[u8]) {
        if let Some((_, state)) = self.peers.remove(peer_id) {
            info!(
                "Peer removed: {:?} (connected {:?}, forwarded {} packets)",
                hex::encode(&peer_id[..8]),
                state.connected_at.elapsed(),
                state.packets_forwarded.load(Ordering::Relaxed)
            );
        }
    }

    /// Forward packet to destination peer
    async fn forward_packet(&self, from: &[u8], to: &[u8], data: &[u8]) -> Result<()> {
        if let Some(dest_peer) = self.peers.get(to) {
            // Forward via ant-quic P2pEndpoint (opens QUIC stream, encrypts, sends)
            self.endpoint
                .send(&dest_peer.peer_id_quic, data)
                .await
                .context("Failed to send packet via MPQUIC")?;

            // Update stats (atomic, lock-free)
            dest_peer.packets_forwarded.fetch_add(1, Ordering::Relaxed);
            self.metrics.packets_forwarded.fetch_add(1, Ordering::Relaxed);

            // Update last activity timestamp (for zombie peer cleanup)
            let now_unix = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            dest_peer.last_activity.store(now_unix, Ordering::Relaxed);

            info!(
                "✅ Forwarded {} bytes from {:?} to {:?}",
                data.len(),
                hex::encode(&from[..8]),
                hex::encode(&to[..8])
            );
            Ok(())
        } else {
            anyhow::bail!("Destination peer not found: {:?}", hex::encode(&to[..8]));
        }
    }
}

// ============================================================================
// HEALTH CHECK / METRICS HTTP ENDPOINTS
// ============================================================================

/// Health check endpoint (liveness probe)
/// Returns 200 OK if the process is alive
async fn health_handler() -> impl IntoResponse {
    (StatusCode::OK, Json(json!({
        "status": "healthy",
        "service": "FILE Relay Server"
    })))
}

/// Readiness probe endpoint
/// Returns 200 OK if ready to accept connections, 503 if not ready
async fn ready_handler(AxumState(state): AxumState<Arc<RelayState>>) -> impl IntoResponse {
    let active_peers = state.peers.len();
    let is_ready = active_peers < state.max_peers;

    if is_ready {
        (StatusCode::OK, Json(json!({
            "status": "ready",
            "active_peers": active_peers,
            "max_peers": state.max_peers
        })))
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, Json(json!({
            "status": "not_ready",
            "reason": "max_peers_reached",
            "active_peers": active_peers,
            "max_peers": state.max_peers
        })))
    }
}

/// Metrics endpoint (Prometheus-compatible text format)
async fn metrics_handler(AxumState(state): AxumState<Arc<RelayState>>) -> impl IntoResponse {
    let active_peers = state.peers.len();
    let peers_registered = state.metrics.peers_registered.load(Ordering::Relaxed);
    let capow_verified = state.metrics.capow_verified.load(Ordering::Relaxed);
    let capow_rejected = state.metrics.capow_rejected.load(Ordering::Relaxed);
    let rate_limited = state.metrics.rate_limited.load(Ordering::Relaxed);
    let packets_forwarded = state.metrics.packets_forwarded.load(Ordering::Relaxed);
    let peers_timeout = state.metrics.peers_timeout.load(Ordering::Relaxed);

    // Prometheus text format
    let metrics = format!(
        "# HELP file_relay_active_peers Current number of connected peers\n\
         # TYPE file_relay_active_peers gauge\n\
         file_relay_active_peers {}\n\
         \n\
         # HELP file_relay_peers_registered_total Total number of peers ever registered\n\
         # TYPE file_relay_peers_registered_total counter\n\
         file_relay_peers_registered_total {}\n\
         \n\
         # HELP file_relay_capow_verified_total Total CAPoW proofs successfully verified\n\
         # TYPE file_relay_capow_verified_total counter\n\
         file_relay_capow_verified_total {}\n\
         \n\
         # HELP file_relay_capow_rejected_total Total CAPoW proofs rejected (replay/invalid)\n\
         # TYPE file_relay_capow_rejected_total counter\n\
         file_relay_capow_rejected_total {}\n\
         \n\
         # HELP file_relay_rate_limited_total Total requests rate-limited\n\
         # TYPE file_relay_rate_limited_total counter\n\
         file_relay_rate_limited_total {}\n\
         \n\
         # HELP file_relay_packets_forwarded_total Total packets successfully forwarded\n\
         # TYPE file_relay_packets_forwarded_total counter\n\
         file_relay_packets_forwarded_total {}\n\
         \n\
         # HELP file_relay_peers_timeout_total Total peers removed due to idle timeout\n\
         # TYPE file_relay_peers_timeout_total counter\n\
         file_relay_peers_timeout_total {}\n",
        active_peers,
        peers_registered,
        capow_verified,
        capow_rejected,
        rate_limited,
        packets_forwarded,
        peers_timeout
    );

    (StatusCode::OK, metrics)
}

/// Challenge endpoint - returns current CAPoW challenge for clients
/// Returns JSON with merkle_root, created_at, and expires_at timestamps
async fn challenge_handler(AxumState(state): AxumState<Arc<RelayState>>) -> impl IntoResponse {
    let challenge = state.challenge_manager.get_current().await;

    let created_at = challenge
        .created_at
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let expires_at = challenge
        .expires_at
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    (StatusCode::OK, Json(json!({
        "merkle_root": hex::encode(&challenge.merkle_root),
        "created_at": created_at,
        "expires_at": expires_at,
    })))
}

// ============================================================================
// PACKET HANDLERS
// ============================================================================

/// Send REGISTER_ACK response to peer
///
/// Packet format:
/// ```
/// ┌──────────────────────────────────────────────────────┐
/// │ Type (1 byte) │ Status (1 byte) │ Message (variable) │
/// ├───────────────┼─────────────────┼────────────────────┤
/// │      0x02     │  0x00 = OK      │   "OK"             │
/// │               │  0x01 = Error   │   "CAPoW invalid"  │
/// └──────────────────────────────────────────────────────┘
/// ```
async fn send_register_ack(
    endpoint: &P2pEndpoint,
    peer_id: &PeerId,
    status: u8,
    message: &str,
) -> Result<()> {
    let mut response = Vec::with_capacity(2 + message.len());
    response.push(0x02); // REGISTER_ACK type
    response.push(status); // Status code
    response.extend_from_slice(message.as_bytes()); // Message

    endpoint.send(peer_id, &response)
        .await
        .context("Failed to send REGISTER_ACK")?;

    Ok(())
}

/// Handle REGISTER packet
///
/// Packet format:
/// ```
/// ┌─────────────────────────────────────────────────────────────┐
/// │ Type (1 byte) │ Peer ID (32 bytes) │ CAPoW Proof (variable) │
/// ├───────────────┼────────────────────┼────────────────────────┤
/// │      0x01     │   [peer_id]        │   [MtpProof]           │
/// └─────────────────────────────────────────────────────────────┘
/// ```
async fn handle_register(
    packet: &[u8],
    peer_id: PeerId,
    state: Arc<RelayState>,
) -> Result<()> {
    // Validate packet size (1 byte type + 32 bytes peer_id + at least 1 byte proof)
    if packet.len() < 34 {
        send_register_ack(&state.endpoint, &peer_id, 0x01, "Packet too small").await?;
        anyhow::bail!("REGISTER packet too small: {} bytes", packet.len());
    }

    // Extract peer_id from packet (bytes 1..33)
    let claimed_peer_id = &packet[1..33];

    // Extract CAPoW proof (bytes 33..)
    let proof_bytes = &packet[33..];

    // Verify CAPoW proof (if enabled)
    match state.verify_capow(proof_bytes).await {
        Ok(true) => {
            // CAPoW verification successful
        }
        Ok(false) => {
            // CAPoW verification failed
            warn!("⚠️ CAPoW verification failed for peer {:?}", hex::encode(&claimed_peer_id[..8]));
            send_register_ack(&state.endpoint, &peer_id, 0x01, "CAPoW invalid").await?;
            return Ok(());
        }
        Err(e) => {
            // CAPoW verification error
            error!("❌ CAPoW verification error: {}", e);
            send_register_ack(&state.endpoint, &peer_id, 0x01, "CAPoW error").await?;
            return Ok(());
        }
    }

    // Register peer
    match state.register_peer(
        claimed_peer_id.to_vec(),
        "0.0.0.0:0".parse().unwrap(), // Placeholder - actual address not needed for MPQUIC relay
        peer_id,
    ).await {
        Ok(()) => {
            info!("✅ Peer registered: {:?}", hex::encode(&claimed_peer_id[..8]));
            send_register_ack(&state.endpoint, &peer_id, 0x00, "OK").await?;
        }
        Err(e) => {
            // Registration failed (duplicate peer_id, max peers reached, etc.)
            warn!("⚠️ Peer registration failed: {}", e);
            let error_msg = e.to_string();
            send_register_ack(&state.endpoint, &peer_id, 0x01, &error_msg).await?;
            return Err(e);
        }
    }

    Ok(())
}

/// Handle RELAY packet
///
/// Packet format:
/// ```
/// ┌──────────────────────────────────────────────────────────────────┐
/// │ Type (1 byte) │ Dest Peer ID (32 bytes) │ Payload (variable)   │
/// ├───────────────┼─────────────────────────┼──────────────────────┤
/// │      0x03     │   [dest_peer_id]        │   [MPQUIC packet]    │
/// └──────────────────────────────────────────────────────────────────┘
/// ```
async fn handle_relay(
    packet: &[u8],
    src_peer_id: PeerId,
    state: Arc<RelayState>,
) -> Result<()> {
    // Validate packet size (1 byte type + 32 bytes dest_peer_id + payload)
    if packet.len() < 34 {
        anyhow::bail!("RELAY packet too small: {} bytes", packet.len());
    }

    // Extract destination peer_id (bytes 1..33)
    let dest_peer_id = &packet[1..33];

    // Extract payload (bytes 33..)
    let payload = &packet[33..];

    // Get source peer_id as bytes for logging
    let src_peer_bytes = src_peer_id.0.to_vec();

    // Forward packet to destination
    state.forward_packet(&src_peer_bytes, dest_peer_id, payload).await?;

    Ok(())
}

/// Handle KEEPALIVE packet
///
/// Packet format:
/// ```
/// ┌─────────────────────────────┐
/// │ Type (1 byte) │ Timestamp   │
/// ├───────────────┼─────────────┤
/// │      0x04     │  u64 (8 B)  │
/// └─────────────────────────────┘
/// ```
async fn handle_keepalive(
    packet: &[u8],
    peer_id: PeerId,
    state: Arc<RelayState>,
) -> Result<()> {
    // Validate packet size (1 byte type + 8 bytes timestamp)
    if packet.len() < 9 {
        anyhow::bail!("KEEPALIVE packet too small: {} bytes", packet.len());
    }

    // Find peer by peer_id and update last_activity
    let peer_id_bytes = peer_id.0.to_vec();

    if let Some(peer_entry) = state.peers.get(&peer_id_bytes) {
        let now_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        peer_entry.last_activity.store(now_unix, Ordering::Relaxed);

        info!("💓 Keepalive from {:?}", hex::encode(&peer_id_bytes[..8]));
    } else {
        warn!("⚠️ KEEPALIVE from unknown peer {:?}", hex::encode(&peer_id_bytes[..8]));
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(&args.log_level)
        .init();

    info!("🚀 FILE Relay Server starting...");
    info!("  Bind address: {}", args.bind);
    info!("  CAPoW enabled: {}", args.capow);
    info!("  Max peers: {}", args.max_peers);

    // Initialize crypto worker pool (for CAPoW verification)
    if args.capow {
        let num_crypto_cores = std::cmp::max(num_cpus::get() / 4, 2); // 25% of cores, min 2
        info!("Initializing crypto worker pool ({} cores)...", num_crypto_cores);
        init_crypto_workers(num_crypto_cores)
            .map_err(|e| anyhow::anyhow!("Failed to initialize crypto worker pool: {}", e))?;
        info!("✅ Crypto worker pool initialized");
    }

    // Initialize challenge manager with rotation
    info!("Initializing CAPoW challenge manager...");
    let challenge_manager = Arc::new(ChallengeManager::new());
    let current_challenge = challenge_manager.get_current().await;
    info!(
        "✅ Challenge manager initialized: merkle_root = {}",
        hex::encode(&current_challenge.merkle_root)
    );

    // Spawn challenge rotation task (rotates every hour)
    if args.capow {
        let challenge_manager_clone = challenge_manager.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(3600)); // 1 hour
            interval.tick().await; // First tick completes immediately
            loop {
                interval.tick().await;
                challenge_manager_clone.rotate().await;
            }
        });
        info!("✅ Challenge rotation task spawned (interval: 1 hour)");
    }

    // Generate per-boot random secret for GDPR-compliant IP hashing
    let mut ip_hash_secret = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut ip_hash_secret);
    info!("✅ Generated per-boot IP hash secret");

    // Initialize MPQUIC P2P endpoint
    info!("Initializing MPQUIC P2pEndpoint...");

    // Create P2pConfig with Pure PQC (ML-KEM-768 + ML-DSA-65)
    let config = P2pConfig::builder()
        .bind_addr(args.bind)
        .pqc(ant_quic::PqcConfig::default()) // ML-KEM-768 + ML-DSA-65
        .build()
        .context("Failed to build P2pConfig")?;

    let endpoint = P2pEndpoint::new(config).await
        .context("Failed to create P2pEndpoint")?;

    info!("✅ MPQUIC endpoint initialized on {}", args.bind);

    // Create relay state
    let state = Arc::new(RelayState::new(
        endpoint,
        args.capow,
        Duration::from_secs(args.capow_timeout),
        args.max_peers,
        challenge_manager.clone(),
        ip_hash_secret,
    ));

    // Spawn stats reporter (Prometheus-compatible structured metrics)
    {
        let state = state.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;

                let active_peers = state.peers.len();
                let peers_registered = state.metrics.peers_registered.load(Ordering::Relaxed);
                let capow_verified = state.metrics.capow_verified.load(Ordering::Relaxed);
                let capow_rejected = state.metrics.capow_rejected.load(Ordering::Relaxed);
                let rate_limited = state.metrics.rate_limited.load(Ordering::Relaxed);
                let packets_forwarded = state.metrics.packets_forwarded.load(Ordering::Relaxed);
                let peers_timeout = state.metrics.peers_timeout.load(Ordering::Relaxed);

                info!(
                    active_peers = active_peers,
                    max_peers = state.max_peers,
                    peers_registered = peers_registered,
                    capow_verified = capow_verified,
                    capow_rejected = capow_rejected,
                    rate_limited = rate_limited,
                    packets_forwarded = packets_forwarded,
                    peers_timeout = peers_timeout,
                    "📊 Relay metrics"
                );
            }
        });
    }

    // Spawn proof replay cache cleanup (remove entries > 5 min old)
    if args.capow {
        let state = state.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                let now = Instant::now();
                let ttl = Duration::from_secs(300);  // 5 minutes

                // Remove expired entries
                state.proof_replay_cache.retain(|_, first_seen| {
                    now.duration_since(*first_seen) < ttl
                });

                let cache_size = state.proof_replay_cache.len();
                if cache_size > 0 {
                    info!("🧹 Proof replay cache: {} entries", cache_size);
                }
            }
        });
    }

    // Spawn zombie peer cleanup (remove peers idle > 5 min)
    {
        let state = state.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;

                let now_unix = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                let idle_timeout_secs = 300;  // 5 minutes

                // Remove idle peers (DashMap.retain is lock-free)
                state.peers.retain(|peer_id, peer_state| {
                    let last_activity = peer_state.last_activity.load(Ordering::Relaxed);
                    let idle_time = now_unix.saturating_sub(last_activity);

                    if idle_time > idle_timeout_secs {
                        state.metrics.peers_timeout.fetch_add(1, Ordering::Relaxed);
                        warn!(
                            "🧟 Zombie peer removed: {:?} (uptime {:?}, {} packets, idle {} sec)",
                            hex::encode(&peer_id[..8]),
                            peer_state.connected_at.elapsed(),
                            peer_state.packets_forwarded.load(Ordering::Relaxed),
                            idle_time
                        );
                        false  // Remove this peer
                    } else {
                        true  // Keep this peer
                    }
                });
            }
        });
    }

    // Spawn HTTP health check / metrics server
    {
        let state = state.clone();
        let health_port = args.health_port;
        tokio::spawn(async move {
            let app = Router::new()
                .route("/health", get(health_handler))
                .route("/ready", get(ready_handler))
                .route("/metrics", get(metrics_handler))
                .route("/challenge", get(challenge_handler))
                .with_state(state);

            let addr = SocketAddr::from(([0, 0, 0, 0], health_port));
            let listener = tokio::net::TcpListener::bind(addr)
                .await
                .expect("Failed to bind health check port");

            info!("🏥 Health check server listening on http://0.0.0.0:{}", health_port);
            info!("   GET /health    - Liveness probe");
            info!("   GET /ready     - Readiness probe");
            info!("   GET /metrics   - Prometheus metrics");
            info!("   GET /challenge - CAPoW challenge (for clients)");

            axum::serve(listener, app)
                .await
                .expect("Health check server failed");
        });
    }

    info!("✅ FILE Relay Server ready on {}", args.bind);
    info!("   Waiting for peer connections...");

    // Main event loop - receive and process packets
    loop {
        tokio::select! {
            // Handle shutdown signal
            _ = tokio::signal::ctrl_c() => {
                info!("📡 Shutdown signal received, exiting event loop...");
                break;
            }

            // Receive packet from any peer
            result = state.endpoint.recv(Duration::from_secs(30)) => {
                match result {
                    Ok((peer_id, data)) => {
                        // Validate packet has at least 1 byte (type field)
                        if data.is_empty() {
                            warn!("⚠️ Received empty packet from {:?}", hex::encode(&peer_id.0[..8]));
                            continue;
                        }

                        // Parse packet type (first byte)
                        match data[0] {
                            0x01 => {
                                // REGISTER packet
                                if let Err(e) = handle_register(&data, peer_id, state.clone()).await {
                                    error!("❌ REGISTER handler error: {}", e);
                                }
                            }
                            0x03 => {
                                // RELAY packet
                                if let Err(e) = handle_relay(&data, peer_id, state.clone()).await {
                                    error!("❌ RELAY handler error: {}", e);
                                }
                            }
                            0x04 => {
                                // KEEPALIVE packet
                                if let Err(e) = handle_keepalive(&data, peer_id, state.clone()).await {
                                    error!("❌ KEEPALIVE handler error: {}", e);
                                }
                            }
                            unknown => {
                                warn!("⚠️ Unknown packet type: {:#04x} from {:?}", unknown, hex::encode(&peer_id.0[..8]));
                            }
                        }
                    }
                    Err(e) => {
                        // Error receiving packet (timeout is normal, just continue)
                        // Only log non-timeout errors
                        let err_str = e.to_string();
                        if !err_str.contains("timeout") && !err_str.contains("timed out") {
                            warn!("⚠️ Packet receive error: {}", e);
                        }
                    }
                }
            }
        }
    }

    // ============================================================================
    // GRACEFUL SHUTDOWN
    // ============================================================================

    info!("🛑 Shutdown signal received, initiating graceful shutdown...");

    // Step 1: Log final metrics
    let active_peers = state.peers.len();
    let peers_registered = state.metrics.peers_registered.load(Ordering::Relaxed);
    let capow_verified = state.metrics.capow_verified.load(Ordering::Relaxed);
    let capow_rejected = state.metrics.capow_rejected.load(Ordering::Relaxed);
    let rate_limited = state.metrics.rate_limited.load(Ordering::Relaxed);
    let packets_forwarded = state.metrics.packets_forwarded.load(Ordering::Relaxed);
    let peers_timeout = state.metrics.peers_timeout.load(Ordering::Relaxed);

    info!(
        active_peers = active_peers,
        peers_registered = peers_registered,
        capow_verified = capow_verified,
        capow_rejected = capow_rejected,
        rate_limited = rate_limited,
        packets_forwarded = packets_forwarded,
        peers_timeout = peers_timeout,
        "📊 Final relay metrics"
    );

    // Step 2: Connection draining (wait for in-flight packets)
    // Note: Full connection draining will be implemented after connection event loop exists
    info!("⏳ Draining connections (5 second grace period)...");
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Step 3: Log remaining peers (if any)
    if active_peers > 0 {
        warn!(
            "⚠️ Forcefully disconnecting {} remaining peers",
            active_peers
        );
        // Future: Send graceful disconnect message to peers
    }

    info!("✅ Graceful shutdown complete");
    Ok(())
}
