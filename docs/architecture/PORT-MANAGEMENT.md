# Port Management & Service Architecture

**Purpose**: Complete guide to port allocation, service discovery, and network architecture for FILE Relay Server deployments.

---

## Overview

FILE Relay Server uses **2 ports** by default:
- **8080/UDP** — Relay traffic (QUIC packets)
- **8081/TCP** — Metrics + health checks (HTTP)

This document covers:
1. Port allocation strategies
2. Service discovery patterns
3. Load balancing architecture
4. Multi-region deployments
5. Port security best practices

---

## Port Allocation

### Default Ports

| Port | Protocol | Purpose | Exposed To | Firewall |
|------|----------|---------|------------|----------|
| **8080** | UDP | Relay traffic (MPQUIC) | Internet (0.0.0.0/0) | Open |
| **8081** | TCP | Metrics (`/metrics`, `/health`, `/ready`) | Internal only | Restricted |

### Why These Ports?

**8080**:
- ✅ Common alternative HTTP port (familiar)
- ✅ Above 1024 (no root required)
- ✅ Not conflicting with common services
- ✅ Easy to remember

**8081**:
- ✅ Adjacent to 8080 (convention)
- ✅ Separate from relay traffic (security)
- ✅ Standard for metrics (Prometheus ecosystem)

### Configurable Ports

All ports are configurable via environment variables or CLI flags:

```bash
# CLI flags
file-relay-server \
  --bind 0.0.0.0:8080 \
  --health-port 8081

# Environment variables
BIND_ADDR=0.0.0.0:8080
HEALTH_PORT=8081
file-relay-server
```

**Use cases for custom ports**:
- Running multiple relay instances on same host
- Avoiding port conflicts with existing services
- Corporate firewall restrictions
- Non-standard deployment environments

---

## Port Conflicts & Resolution

### Common Conflicts

| Port | Conflicting Service | Resolution |
|------|-------------------|------------|
| 8080 | Jenkins, Tomcat, alternative HTTP | Change relay to 8082, 8090, or 9080 |
| 8081 | Alternative metrics endpoints | Change to 9081, 9091, or 8082 |

### Detecting Conflicts

**Before starting relay**:
```bash
# Check if port is in use
sudo lsof -i :8080
sudo lsof -i :8081

# Or with netstat
sudo netstat -tuln | grep 8080
sudo netstat -tuln | grep 8081

# Or with ss (modern)
sudo ss -tuln | grep 8080
```

**Kubernetes (check existing services)**:
```bash
kubectl get svc --all-namespaces -o wide | grep 8080
kubectl get svc --all-namespaces -o wide | grep 8081
```

### Dynamic Port Allocation

For test environments or multiple instances:

```rust
// Bind to port 0 = OS assigns random available port
let listener = UdpSocket::bind("0.0.0.0:0").await?;
let actual_port = listener.local_addr()?.port();
println!("Relay listening on port: {}", actual_port);
```

**Use case**: Integration tests, temporary instances.

---

## Service Discovery

### Problem

Clients need to find relay servers:
- What's the IP address?
- What port?
- Is it healthy?
- Which region is closest?

### Solution 1: Static Configuration (Simple)

**Client config file**:
```yaml
relay:
  url: udp://relay.example.com:8080
  backup_urls:
    - udp://relay-backup.example.com:8080
    - udp://relay-eu.example.com:8080
```

**Advantages**:
- ✅ Simple (no dependencies)
- ✅ Works offline
- ✅ Predictable

**Disadvantages**:
- ❌ Manual updates when relay changes
- ❌ No automatic failover
- ❌ No load balancing

**Use case**: Small deployments, single relay.

### Solution 2: DNS-Based Discovery

**DNS A records**:
```
relay.example.com.  300  IN  A  203.0.113.10
relay.example.com.  300  IN  A  203.0.113.11
relay.example.com.  300  IN  A  203.0.113.12
```

**Client resolution**:
```rust
use tokio::net::lookup_host;

let addrs: Vec<SocketAddr> = lookup_host("relay.example.com:8080")
    .await?
    .collect();

// Randomly pick one (simple load balancing)
let relay_addr = addrs.choose(&mut rand::thread_rng()).unwrap();
```

**Advantages**:
- ✅ Standard DNS (no special infrastructure)
- ✅ Automatic load balancing (DNS round-robin)
- ✅ TTL controls cache duration (5 min = 300s)

**Disadvantages**:
- ❌ DNS caching can delay updates
- ❌ No health checks (client might pick dead server)
- ❌ No geographic routing

**Use case**: Small-medium deployments, multiple replicas.

### Solution 3: SRV Records (Advanced DNS)

**DNS SRV records**:
```
_file-relay._udp.example.com. 300 IN SRV 10 50 8080 relay-1.example.com.
_file-relay._udp.example.com. 300 IN SRV 10 50 8080 relay-2.example.com.
_file-relay._udp.example.com. 300 IN SRV 20 50 8080 relay-backup.example.com.
```

**Format**: `priority weight port target`

**Client resolution**:
```rust
use trust_dns_resolver::Resolver;

let resolver = Resolver::from_system_conf()?;
let response = resolver.srv_lookup("_file-relay._udp.example.com")?;

for srv in response.iter() {
    println!("Relay: {}:{} (priority: {})", srv.target(), srv.port(), srv.priority());
}
```

**Advantages**:
- ✅ Includes port number (flexible)
- ✅ Priority-based failover
- ✅ Weight-based load balancing
- ✅ Standard protocol (RFC 2782)

**Disadvantages**:
- ❌ More complex DNS setup
- ❌ Not all clients support SRV
- ❌ Still no health checks

**Use case**: Enterprise deployments, complex routing.

### Solution 4: Consul/etcd Service Discovery

**Consul service registration**:
```hcl
service {
  name = "file-relay"
  port = 8080
  address = "10.0.1.10"
  
  check {
    name = "Relay health check"
    http = "http://10.0.1.10:8081/health"
    interval = "10s"
    timeout = "2s"
  }
}
```

**Client discovery**:
```rust
use consul::Client;

let consul = Client::new("http://consul.service.consul:8500")?;
let services = consul.catalog().service("file-relay", None).await?;

for service in services {
    if service.health == "passing" {
        println!("Healthy relay: {}:{}", service.address, service.port);
    }
}
```

**Advantages**:
- ✅ Automatic health checks
- ✅ Real-time updates (no DNS caching)
- ✅ Service metadata (region, version, etc.)
- ✅ Integrated with service mesh (Consul Connect)

**Disadvantages**:
- ❌ Requires Consul/etcd infrastructure
- ❌ More complex setup
- ❌ Another dependency

**Use case**: Microservices architecture, service mesh.

### Solution 5: Kubernetes Service Discovery

**Kubernetes Service**:
```yaml
apiVersion: v1
kind: Service
metadata:
  name: file-relay
  namespace: default
spec:
  type: LoadBalancer
  selector:
    app: file-relay
  ports:
    - name: relay-udp
      protocol: UDP
      port: 8080
      targetPort: 8080
    - name: metrics
      protocol: TCP
      port: 8081
      targetPort: 8081
```

**Client discovery (inside cluster)**:
```rust
// Kubernetes DNS (automatic)
let relay_addr = "file-relay.default.svc.cluster.local:8080";

// Or via environment variables (injected by K8s)
let relay_addr = env::var("FILE_RELAY_SERVICE_HOST")?;
let relay_port = env::var("FILE_RELAY_SERVICE_PORT_RELAY_UDP")?;
```

**Client discovery (outside cluster)**:
```bash
# Get external LoadBalancer IP
kubectl get svc file-relay -o jsonpath='{.status.loadBalancer.ingress[0].ip}'
# Output: 203.0.113.100

# Client uses: udp://203.0.113.100:8080
```

**Advantages**:
- ✅ Native Kubernetes integration
- ✅ Automatic load balancing (kube-proxy)
- ✅ Health checks via readiness probes
- ✅ Zero-config for in-cluster clients

**Disadvantages**:
- ❌ Kubernetes-specific (not portable)
- ❌ External clients need manual LoadBalancer IP lookup

**Use case**: Kubernetes-native deployments.

**Recommendation**:
- **Small deployments**: Solution 2 (DNS A records)
- **Kubernetes**: Solution 5 (K8s Service)
- **Microservices**: Solution 4 (Consul)
- **Enterprise multi-cloud**: Solution 3 (SRV records) + GeoDNS

---

## Load Balancing Architecture

### Layer 4 Load Balancing (UDP)

**Problem**: How to distribute traffic across multiple relay instances?

#### Option A: DNS Round-Robin (Simple)

**How it works**:
```
Client → DNS query for relay.example.com
DNS → Returns [10.0.1.10, 10.0.1.11, 10.0.1.12] (rotated)
Client → Connects to first IP in list
```

**Load distribution**: ~33% to each relay (random client selection)

**Advantages**:
- ✅ Zero infrastructure (DNS only)
- ✅ Stateless (no LB state)
- ✅ Works with UDP

**Disadvantages**:
- ❌ Uneven distribution (depends on DNS caching)
- ❌ No health checks
- ❌ No sticky sessions

#### Option B: Cloud Load Balancer (Recommended)

**AWS Network Load Balancer (NLB)**:
```hcl
resource "aws_lb" "relay" {
  name               = "file-relay-nlb"
  internal           = false
  load_balancer_type = "network"
  
  subnet_mapping {
    subnet_id = aws_subnet.public_a.id
  }
  subnet_mapping {
    subnet_id = aws_subnet.public_b.id
  }
}

resource "aws_lb_target_group" "relay_udp" {
  name     = "relay-udp"
  port     = 8080
  protocol = "UDP"
  vpc_id   = aws_vpc.main.id
  
  health_check {
    enabled  = true
    protocol = "HTTP"
    port     = 8081
    path     = "/health"
    interval = 30
  }
}
```

**GCP Network Load Balancer**:
```hcl
resource "google_compute_forwarding_rule" "relay_udp" {
  name                  = "relay-udp-lb"
  ip_protocol           = "UDP"
  port_range            = "8080"
  load_balancing_scheme = "EXTERNAL"
  backend_service       = google_compute_backend_service.relay.id
}

resource "google_compute_backend_service" "relay" {
  name          = "relay-backend"
  protocol      = "UDP"
  health_checks = [google_compute_health_check.relay.id]
  
  backend {
    group = google_compute_instance_group.relay.id
  }
}
```

**Advantages**:
- ✅ Automatic health checks
- ✅ Even load distribution
- ✅ Highly available (multi-AZ)
- ✅ Auto-scaling integration

**Disadvantages**:
- ❌ Cloud vendor lock-in
- ❌ Cost ($0.0225/hour for NLB on AWS)

**Use case**: Production deployments.

#### Option C: HAProxy (On-Premise)

**HAProxy config** (`/etc/haproxy/haproxy.cfg`):
```haproxy
frontend relay_udp
    bind *:8080 udp
    mode udp
    default_backend relay_servers

backend relay_servers
    mode udp
    balance roundrobin
    
    # Health check via HTTP (port 8081)
    option httpchk GET /health
    http-check expect status 200
    
    server relay1 10.0.1.10:8080 check port 8081
    server relay2 10.0.1.11:8080 check port 8081
    server relay3 10.0.1.12:8080 check port 8081
```

**Advantages**:
- ✅ On-premise (no cloud dependency)
- ✅ Flexible configuration
- ✅ Free and open-source

**Disadvantages**:
- ❌ Single point of failure (need HA setup with VRRP)
- ❌ Manual scaling
- ❌ Operational overhead

**Use case**: On-premise or hybrid deployments.

### Sticky Sessions (Connection Affinity)

**Problem**: Peer registers with relay A, then sends RELAY packet but LB routes to relay B (peer not registered).

**Solution**: Sticky sessions based on source IP.

**AWS NLB**:
```hcl
resource "aws_lb_target_group" "relay_udp" {
  stickiness {
    enabled = true
    type    = "source_ip"  # Route same IP to same target
  }
}
```

**HAProxy**:
```haproxy
backend relay_servers
    balance source  # Hash source IP for affinity
```

**Caveat**: NAT behind same IP → all peers route to same relay (uneven distribution).

**Alternative**: Share peer registry across relays (Redis) so any relay can handle any peer.

---

## Multi-Region Architecture

### Geographic Distribution

**Problem**: Latency from US client to EU relay = 100-150ms. How to reduce?

**Solution**: Deploy relays in multiple regions and route clients to nearest.

### Architecture Pattern

```
                    ┌─── relay-us-east (Virginia)
                    │
Client → GeoDNS ────┼─── relay-eu-west (Ireland)
                    │
                    └─── relay-ap-southeast (Singapore)
```

### GeoDNS Setup

**Route 53 (AWS)**:
```hcl
resource "aws_route53_record" "relay_geolocation" {
  zone_id = aws_route53_zone.main.zone_id
  name    = "relay.example.com"
  type    = "A"
  
  # US traffic → US relay
  geolocation_routing_policy {
    continent = "NA"
  }
  alias {
    name                   = aws_lb.relay_us_east.dns_name
    zone_id                = aws_lb.relay_us_east.zone_id
    evaluate_target_health = true
  }
}

resource "aws_route53_record" "relay_geolocation_eu" {
  zone_id = aws_route53_zone.main.zone_id
  name    = "relay.example.com"
  type    = "A"
  
  # EU traffic → EU relay
  geolocation_routing_policy {
    continent = "EU"
  }
  alias {
    name                   = aws_lb.relay_eu_west.dns_name
    zone_id                = aws_lb.relay_eu_west.zone_id
    evaluate_target_health = true
  }
}
```

**Cloudflare (Multi-cloud)**:
```yaml
# Cloudflare Load Balancer
pools:
  - name: us-east
    origins:
      - name: relay-us-1
        address: 203.0.113.10
        enabled: true
  
  - name: eu-west
    origins:
      - name: relay-eu-1
        address: 198.51.100.20
        enabled: true

rules:
  - name: geo-routing
    condition: geo.continent == "NA"
    pool: us-east
  
  - name: geo-routing-eu
    condition: geo.continent == "EU"
    pool: eu-west
```

**Advantages**:
- ✅ Lower latency (< 50ms p95)
- ✅ Fault tolerance (region failure)
- ✅ Compliance (data residency for GDPR)

**Disadvantages**:
- ❌ Higher infrastructure cost (3x relays)
- ❌ Complex DNS setup
- ❌ Cross-region peer communication not possible (peers in US can't relay to EU)

### Cross-Region Peer Communication

**Problem**: Peer A (US) wants to send to Peer B (EU), but they're registered on different relays.

**Solution 1: Shared Peer Registry (Redis Cluster)**

```
┌────────────────────────────────────────┐
│         Redis Cluster (Global)         │
│  {peer_A: relay_us, peer_B: relay_eu} │
└────────────────────────────────────────┘
           ↑                  ↑
           │                  │
    ┌──────┴──────┐    ┌─────┴──────┐
    │  Relay US   │    │  Relay EU  │
    └─────────────┘    └────────────┘
```

**How it works**:
1. Peer A (US) sends RELAY packet to Peer B
2. Relay US checks local registry: Peer B not found
3. Relay US queries Redis: Peer B on Relay EU (IP: 198.51.100.20)
4. Relay US forwards packet to Relay EU (UDP relay-to-relay)
5. Relay EU forwards to Peer B

**Disadvantages**:
- ❌ High latency (US → EU → peer = 150ms)
- ❌ Double bandwidth cost (relayed twice)

**Solution 2: Regional Relay Only (Recommended)**

**Design**: Peers can only communicate within same region.

**Client logic**:
```rust
// Client determines region and connects to regional relay
let client_region = detect_region(); // "us-east", "eu-west", "ap-southeast"
let relay_url = format!("udp://relay-{}.example.com:8080", client_region);

let client = RelayClient::new(relay_url).await?;
```

**Advantages**:
- ✅ Simple (no cross-relay forwarding)
- ✅ Low latency (same region)
- ✅ Lower cost (single relay hop)

**Disadvantages**:
- ❌ Peers in different regions can't communicate

**Use case**: Most applications (chat, gaming) where users are regionally clustered.

---

## Port Security Best Practices

### 1. Principle of Least Exposure

**Only expose necessary ports to necessary networks**:

| Port | Exposed To | Why |
|------|-----------|-----|
| 8080/UDP | Internet (0.0.0.0/0) | Peers must connect |
| 8081/TCP | Internal VPC only (10.0.0.0/8) | Metrics/health (sensitive) |

**AWS Security Group**:
```hcl
resource "aws_security_group_rule" "relay_udp_public" {
  type        = "ingress"
  from_port   = 8080
  to_port     = 8080
  protocol    = "udp"
  cidr_blocks = ["0.0.0.0/0"]  # Public
  security_group_id = aws_security_group.relay.id
}

resource "aws_security_group_rule" "metrics_internal" {
  type        = "ingress"
  from_port   = 8081
  to_port     = 8081
  protocol    = "tcp"
  cidr_blocks = ["10.0.0.0/8"]  # Internal VPC only
  security_group_id = aws_security_group.relay.id
}
```

### 2. DDoS Protection

**UDP port 8080 is vulnerable to amplification attacks**. Mitigations:

**AWS Shield Standard** (free):
- Automatic DDoS protection
- Layer 3/4 (network/transport)
- Always-on monitoring

**AWS Shield Advanced** ($3,000/month):
- Enhanced DDoS protection
- 24/7 DDoS Response Team
- Cost protection (refund for scaling costs during attack)

**GCP Cloud Armor**:
```hcl
resource "google_compute_security_policy" "relay_ddos" {
  name = "relay-ddos-protection"
  
  rule {
    action   = "rate_based_ban"
    priority = "1000"
    match {
      versioned_expr = "SRC_IPS_V1"
      config {
        src_ip_ranges = ["*"]
      }
    }
    rate_limit_options {
      conform_action = "allow"
      exceed_action  = "deny(429)"
      enforce_on_key = "IP"
      rate_limit_threshold {
        count        = 100
        interval_sec = 10
      }
    }
  }
}
```

### 3. Firewall Rules (Defense in Depth)

**Layer 1: Cloud Security Groups** (virtual firewall)
**Layer 2: OS Firewall** (iptables/firewalld)
**Layer 3: Application Rate Limiting** (relay code)

**Example: Linux iptables**:
```bash
# Allow UDP 8080 (relay)
iptables -A INPUT -p udp --dport 8080 -j ACCEPT

# Allow TCP 8081 from internal only
iptables -A INPUT -p tcp --dport 8081 -s 10.0.0.0/8 -j ACCEPT
iptables -A INPUT -p tcp --dport 8081 -j DROP

# Drop everything else (default deny)
iptables -P INPUT DROP
```

### 4. Port Scanning Defense

**Problem**: Attacker scans for open ports to find relay instances.

**Mitigation**:
- Port 8080 will respond (unavoidable for public relay)
- Port 8081 should **not** respond to external scans (firewall)

**Detection**:
```bash
# Monitor for port scan attempts
journalctl -u file-relay | grep "connection from" | awk '{print $NF}' | sort | uniq -c | sort -nr | head -10
```

---

## Service Mesh Integration

### Istio Example

**ServiceEntry** (register external relay):
```yaml
apiVersion: networking.istio.io/v1alpha3
kind: ServiceEntry
metadata:
  name: file-relay-external
spec:
  hosts:
    - relay.example.com
  ports:
    - number: 8080
      name: udp
      protocol: UDP
  location: MESH_EXTERNAL
  resolution: DNS
```

**VirtualService** (traffic routing):
```yaml
apiVersion: networking.istio.io/v1alpha3
kind: VirtualService
metadata:
  name: file-relay
spec:
  hosts:
    - file-relay
  udp:
    - match:
        - port: 8080
      route:
        - destination:
            host: file-relay
            port:
              number: 8080
          weight: 100
```

**Advantages**:
- ✅ Unified traffic management
- ✅ Automatic mTLS between services
- ✅ Observability (traces, metrics)

**Disadvantages**:
- ❌ Istio overhead (sidecar proxy)
- ❌ Complex setup
- ❌ UDP support limited in Istio

---

## Monitoring Port Health

### Prometheus Metrics

```promql
# Port open/closed status
probe_success{job="relay-health-check"}

# Connection attempts per port
rate(file_relay_connections_total{port="8080"}[5m])

# Failed connection attempts (port unreachable)
rate(file_relay_connection_errors_total{port="8080"}[5m])
```

### Health Check Endpoint

**`GET /health` on port 8081**:
```json
{
  "status": "healthy",
  "version": "0.1.0",
  "uptime_seconds": 86400,
  "ports": {
    "relay_udp": {
      "port": 8080,
      "status": "listening"
    },
    "metrics_http": {
      "port": 8081,
      "status": "listening"
    }
  }
}
```

---

## FAQ

### Can I run relay on port 443?

**Yes**, but:
- ❌ Requires root (ports < 1024)
- ❌ Conflicts with HTTPS (if hosting website on same server)
- ✅ Better firewall traversal (443 rarely blocked)

**Alternative**: Use capability binding (Linux):
```bash
sudo setcap CAP_NET_BIND_SERVICE=+eip /usr/local/bin/file-relay-server
```

### Can I use same port for UDP and TCP?

**Yes**. Ports are per-protocol. You can have:
- UDP 8080 (relay)
- TCP 8080 (different service)

But it's confusing. Better to use different ports.

### How many concurrent connections can one port handle?

**UDP has no "connections"** (connectionless). The limit is:
- OS buffer size (`net.core.rmem_max`, `net.core.wmem_max`)
- File descriptor limit (`ulimit -n`)

**Typical**: 1 relay instance can handle 1000+ peers on a single UDP port.

---

## References

- [DEPLOYMENT-GUIDE.md](../deployment/DEPLOYMENT-GUIDE.md)
- [SECURITY-HARDENING-CHECKLIST.md](../deployment/SECURITY-HARDENING-CHECKLIST.md)
- [Kubernetes Service Docs](https://kubernetes.io/docs/concepts/services-networking/service/)
- [RFC 2782: DNS SRV](https://tools.ietf.org/html/rfc2782)

---

**Last Updated**: 2026-05-18
