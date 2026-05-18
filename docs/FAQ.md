# Frequently Asked Questions (FAQ)

**Last Updated**: 2026-05-18

---

## General

### What is FILE Relay Server?

FILE Relay Server is a stateless, production-ready relay for MPQUIC (Multi-Path QUIC) peer-to-peer connections. It enables peers to communicate through NAT and firewalls without maintaining connection state, while providing DoS protection via Computational Amortized Proof of Work (CAPoW).

### Why do I need a relay server?

**NAT Traversal**: Most peers are behind NAT/firewalls and cannot directly connect. A relay enables communication.

**Privacy**: The relay cannot decrypt packets (end-to-end encrypted) and doesn't log plaintext IPs (GDPR-compliant hashing).

**DoS Protection**: CAPoW prevents attackers from overwhelming the relay with connection requests.

### Is the relay stateful or stateless?

**Stateless**. The relay:
- Does not store persistent state
- Does not maintain session history
- Can be killed and restarted without data loss
- Scales horizontally without coordination

The only "state" is ephemeral:
- Active peer registry (in-memory)
- Replay cache (5-minute TTL)
- Rate limiters (per-boot)

### What does "zero-trust" mean?

The relay:
- Cannot decrypt packets (E2E encrypted with MPQUIC)
- Cannot impersonate peers (Noise XX authentication, pending event loop)
- Cannot log plaintext IPs (HMAC-SHA256 hashing)
- Cannot replay proofs (5-minute cache)
- Assumes all input is malicious (input validation, rate limiting)

---

## Deployment

### What are the minimum requirements?

**CPU**: 2+ cores (4+ recommended)  
**Memory**: 512 MB minimum, 2 GB recommended  
**Network**: Public IP with UDP port 8080 accessible  
**OS**: Linux (Ubuntu 22.04+, Debian 12+), macOS 13+

### How many peers can one relay handle?

**Default**: 1,000 concurrent peers per instance

**Tunable**: Increase `--max-peers` or scale horizontally with Kubernetes HPA.

**Real capacity** depends on:
- Packet forwarding rate
- CAPoW verification rate (CPU-bound)
- Network bandwidth
- Memory (each peer ~0.5 MB)

**Recommended**: Scale horizontally at 800-900 peers (90% capacity).

### Should I run one big relay or many small relays?

**Many small relays** (horizontal scaling):
- ✅ Better fault tolerance
- ✅ Load balancing
- ✅ Geographic distribution
- ✅ Easier capacity planning

**One big relay** (vertical scaling):
- ❌ Single point of failure
- ❌ Hard capacity limit
- ❌ Expensive (large instance)

**Best practice**: Start with 3 replicas, auto-scale to 10 based on load.

### Can I run this behind a CDN?

**No**. The relay uses **UDP** (QUIC), which CDNs like Cloudflare do not support in the same way as HTTP.

**Use instead**:
- **DDoS protection**: Cloudflare Spectrum (UDP proxy)
- **Load balancing**: Kubernetes LoadBalancer or cloud provider LB
- **Geographic distribution**: Deploy relays in multiple regions

### Do I need a domain name?

**No**. Clients connect via IP address and UDP port:
```
udp://relay.example.com:8080
udp://203.0.113.10:8080  # Works the same
```

**But recommended** for:
- Easier IP changes (update DNS)
- Human-readable names
- TLS health check endpoint (future)

---

## Security

### Is CAPoW secure?

**Yes**. CAPoW (Computational Amortized Proof of Work) uses **MTP-Argon2id**:
- Memory-hard (resistant to ASICs/GPUs)
- 100ms timeout (prevents CPU exhaustion)
- Replay prevention (5-minute cache)
- Rate limiting backup (10 req/s per IP)

**Attack cost**: ~$0.01 per connection attempt (assuming attacker uses cloud)

### What if an attacker has a botnet?

**Distributed attacks** from many IPs are harder to stop with per-IP rate limiting.

**Mitigations**:
1. **CAPoW**: Each bot must compute proof (slows attack)
2. **Rate limiting**: 10 req/s per IP (per-bot limit)
3. **Monitoring**: Alert on high rejection rate (>50/s = likely attack)
4. **DDoS protection**: Deploy behind Cloudflare Spectrum or AWS Shield
5. **Horizontal scaling**: More relays = harder to overwhelm

**Bottom line**: CAPoW + rate limiting raises attack cost significantly, but determined attackers with large botnets can still cause issues. Use network-level DDoS protection for those scenarios.

### Are IP addresses logged?

**No plaintext IPs**. All IPs are hashed with **HMAC-SHA256** before logging:
```
Original IP: 203.0.113.42
Logged as:   a3f5b8c1d2e4f678  (first 16 hex chars of HMAC)
```

**GDPR-compliant**:
- No PII in logs
- Cannot reverse-engineer original IP
- Per-boot secret (historical logs can't be correlated across restarts)

**If you need** plaintext IPs:
- Check firewall/load balancer logs (outside relay)
- Not supported by design (GDPR)

### What about Noise XX authentication?

**Status**: Design complete, blocked by connection event loop implementation.

**When implemented**:
- Mutual authentication (relay ↔ peer)
- Forward secrecy (past sessions stay secret)
- Prevents MITM attacks

**Until then**:
- E2E encryption still protects payload
- CAPoW prevents DoS
- IP hashing protects privacy

**Not a vulnerability**, just a missing feature.

---

## Operations

### How do I monitor the relay?

**Prometheus Metrics**: `http://relay:8081/metrics`
- 7 metrics (active peers, packets forwarded, CAPoW stats, rate limiting)
- Scrape every 30 seconds

**Grafana Dashboard**: Pre-configured with 4 panels
- Active peers (gauge with thresholds)
- Packet rate (per pod)
- CAPoW rate (verified vs rejected)
- Rate limiting (requests/s)

**Alerts**: 12 production alerts via Prometheus + Alertmanager
- 4 critical (server down, capacity, attacks)
- 8 warning (high load, timeouts, etc.)

**See**: [Deployment Guide - Monitoring](deployment/DEPLOYMENT-GUIDE.md#monitoring-setup)

### What should I alert on?

**Critical** (immediate action):
- `FileRelayServerDown` — Server not responding
- `FileRelayCapacityReached` — At max peer capacity
- `FileRelayCAPoWAttack` — >50 rejections/s (likely attack)
- `FileRelayRateLimitAttack` — >100 limits/s (likely attack)

**Warning** (investigate soon):
- `FileRelayHighPeerCount` — >900 peers (capacity planning)
- `FileRelayHighCAPoWRejectionRate` — >10 rejections/s
- `FileRelayHighPeerTimeouts` — >5 timeouts/s (network quality)

**See**: [Operations Runbook](deployment/OPERATIONS-RUNBOOK.md) for response procedures.

### How do I scale the relay?

**Horizontal scaling** (Kubernetes):
```bash
# Manual
kubectl scale deployment file-relay-server --replicas=5

# Auto-scaling (HPA already configured)
# Scales 3-10 replicas based on CPU/memory/active_peers
```

**Vertical scaling** (increase per-instance capacity):
```bash
# Increase max-peers
kubectl set env deployment/file-relay-server MAX_PEERS=2000
```

**Recommendation**: Horizontal scaling is preferred (better fault tolerance).

### What if a relay crashes?

**Kubernetes** restarts it automatically:
- Liveness probe fails → pod killed → new pod started
- PodDisruptionBudget ensures min 2 replicas always available
- No data loss (stateless)
- Peers reconnect to another replica

**Docker** (with `--restart unless-stopped`):
- Container crashes → Docker restarts it
- No orchestration (manual intervention needed)

**Bare metal**:
- Use systemd for automatic restart
- No built-in health checks
- Manual monitoring required

---

## Performance

### What is the latency?

**Relay latency**: ~1-5 ms (packet forwarding only)

**Total latency** (peer A → relay → peer B):
- Network latency (A → relay): varies
- Relay processing: ~1-5 ms
- Network latency (relay → B): varies

**Relay does not add significant latency** (just forwards packets).

### What is the throughput?

**Per relay instance**: 
- Limited by network bandwidth (not CPU/memory)
- Typical: 1-10 Gbps depending on instance type
- Bottleneck is usually peer bandwidth, not relay

**Per peer**:
- Limited by peer's own bandwidth
- Relay does not throttle (no QoS)

### How much CPU does CAPoW verification use?

**Worker pool**: 25% of CPU cores, minimum 2 workers

**Examples**:
- 4 cores → 1 worker
- 8 cores → 2 workers
- 16 cores → 4 workers

**Per verification**: ~50-100 ms of CPU time (Argon2id)

**Impact**: CAPoW is CPU-bound, but worker pool prevents starvation of packet forwarding.

### How much memory per peer?

**~0.5 MB per peer**:
- Peer registry entry: ~100 bytes
- Rate limiter: ~48 bytes (per IP, not per peer)
- Replay cache: ~48 bytes per proof (5-minute TTL)
- OS networking buffers: ~300 KB per peer

**Total for 1,000 peers**: ~512 MB

**Recommendation**: 2 GB RAM for 1,000 peers (comfortable headroom).

---

## Troubleshooting

### High CAPoW rejection rate

**Symptoms**: `file_relay_capow_rejected_total` increasing rapidly

**Causes**:
1. **DoS attack** — Malicious peers sending invalid proofs
2. **Client misconfiguration** — Clients using wrong difficulty
3. **Clock skew** — Client/server clocks out of sync

**Actions**:
```bash
# Check rejection rate
curl http://relay:8081/metrics | grep capow_rejected

# Review logs
kubectl logs deployment/file-relay-server | grep "CAPoW rejected"

# If attack: alert security team, monitor rate limiting
# If clients: verify client CAPoW implementation
# If clock: check NTP sync on clients
```

### Peers can't connect

**Symptoms**: Clients timeout when registering

**Causes**:
1. **Port not open** — UDP 8080 blocked by firewall
2. **Wrong IP/domain** — Clients connecting to wrong address
3. **Capacity reached** — Relay at max peers (1000)
4. **Rate limited** — Client IP hitting 10 req/s limit

**Actions**:
```bash
# Check server is running
curl http://relay:8081/health

# Check capacity
curl http://relay:8081/ready

# Test UDP connectivity
nc -vzu relay-ip 8080

# Check firewall rules
# (cloud provider console or iptables)
```

### High memory usage

**Symptoms**: Memory usage growing unbounded

**Causes**:
1. **Many peers** — More peers = more memory (expected)
2. **Replay cache growth** — Should be bounded (5-minute TTL)
3. **Memory leak** — Unlikely (Rust), but possible

**Actions**:
```bash
# Check peer count
curl http://relay:8081/metrics | grep active_peers

# Expected memory: ~500 MB + (active_peers * 0.5 MB)

# If OOM: increase memory limits
kubectl set resources deployment/file-relay-server \
  --limits=memory=4Gi --requests=memory=2Gi

# Or scale horizontally
kubectl scale deployment/file-relay-server --replicas=5
```

---

## Development

### How do I contribute?

**See**: [CONTRIBUTING.md](../CONTRIBUTING.md) for full guidelines.

**Quick start**:
1. Fork the repository
2. Create feature branch (`git checkout -b feature/my-feature`)
3. Make changes, add tests
4. Run checks (`cargo fmt`, `cargo clippy`, `cargo test`)
5. Submit pull request

### How do I run tests locally?

```bash
# All tests
cargo test

# Relay server tests only
cargo test --package relay_server

# With output
cargo test -- --nocapture

# Specific test
cargo test test_rate_limiter_burst_capacity
```

### How do I build for production?

```bash
# Debug build (fast, not optimized)
cargo build

# Release build (optimized)
cargo build --release

# Docker image
docker build -t file-relay -f deploy/Dockerfile .

# Binary location
./target/release/file-relay --help
```

---

## Licensing & Legal

### What license is this?

**MIT License** — permissive, commercial use allowed.

**You can**:
- Use commercially
- Modify
- Distribute
- Sublicense
- Private use

**You must**:
- Include copyright notice
- Include license text

**You cannot**:
- Hold author liable

**See**: [LICENSE](../LICENSE)

### Can I use this for my business?

**Yes**. MIT License allows commercial use without royalties.

### Do I need to open-source my modifications?

**No**. MIT License does not require you to share modifications (unlike GPL).

**But please consider**:
- Contributing improvements upstream (pull requests welcome)
- Reporting bugs (helps everyone)
- Sharing use cases (helps roadmap)

---

## Roadmap

### When will the event loop be implemented?

**Status**: Pending (all infrastructure ready).

**Blocking**: Connection event loop implementation is the prerequisite.

**When done**:
- Noise XX authentication active
- Real packet forwarding (currently infrastructure only)
- End-to-end tests possible
- Performance benchmarks possible

**Priority**: High (required for production use).

### Will there be WebSocket support?

**Maybe**. WebSocket would enable browser clients.

**Trade-offs**:
- ✅ Browser compatibility
- ❌ Extra protocol complexity
- ❌ Performance overhead
- ❌ More attack surface

**Decision**: After event loop is stable, evaluate demand.

### Will there be post-quantum cryptography?

**Planned**. Noise XX supports post-quantum KEMs (Key Encapsulation Mechanisms).

**Timeline**: After Noise XX is implemented and stable.

**Options**: Kyber, NTRU, or NIST-standardized PQC algorithms.

---

## Support

**Questions not answered here?**

- **GitHub Discussions**: https://github.com/file-network/relay-server/discussions
- **Issues**: https://github.com/file-network/relay-server/issues
- **Security**: security@file-network.example
- **Email**: dev@file-network.example

---

**Have a question that should be added to this FAQ?** [Open an issue](https://github.com/file-network/relay-server/issues/new) or [submit a pull request](../CONTRIBUTING.md).
