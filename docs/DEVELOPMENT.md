# FILE Relay Server - Development Guide

**Last Updated**: 2026-05-18  
**Version**: 0.1.0

---

## Table of Contents

1. [Development Environment Setup](#development-environment-setup)
2. [Project Structure](#project-structure)
3. [Building the Project](#building-the-project)
4. [Running Tests](#running-tests)
5. [Code Quality](#code-quality)
6. [Debugging](#debugging)
7. [Contributing Workflow](#contributing-workflow)
8. [Architecture Deep Dive](#architecture-deep-dive)
9. [Adding New Features](#adding-new-features)
10. [Performance Profiling](#performance-profiling)
11. [Troubleshooting](#troubleshooting)

---

## Development Environment Setup

### Prerequisites

**Required**:
- Rust 1.83+ (install via [rustup](https://rustup.rs/))
- Git 2.30+
- Unix-like environment (Linux, macOS, WSL2 on Windows)

**Optional but recommended**:
- Docker 20.10+ (for containerized testing)
- Kubernetes cluster (for testing K8s manifests)
- Prometheus + Grafana (for metrics testing)

### Installing Rust

```bash
# Install rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add to PATH (or restart terminal)
source $HOME/.cargo/env

# Verify installation
rustc --version
cargo --version
```

### Installing Development Tools

```bash
# cargo-watch: Auto-rebuild on file changes
cargo install cargo-watch

# cargo-audit: Security vulnerability scanning
cargo install cargo-audit

# cargo-outdated: Check for outdated dependencies
cargo install cargo-outdated

# cargo-tarpaulin: Code coverage (Linux only)
cargo install cargo-tarpaulin

# cargo-flamegraph: Performance profiling
cargo install flamegraph
```

### IDE Setup

**Visual Studio Code** (recommended):

1. Install extensions:
   - [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
   - [Even Better TOML](https://marketplace.visualstudio.com/items?itemName=tamasfe.even-better-toml)
   - [CodeLLDB](https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb) (debugging)

2. Workspace settings (`.vscode/settings.json`):
```json
{
  "rust-analyzer.check.command": "clippy",
  "rust-analyzer.check.allTargets": true,
  "rust-analyzer.cargo.features": "all",
  "editor.formatOnSave": true,
  "editor.rulers": [100]
}
```

**IntelliJ IDEA / CLion**:

1. Install [Rust plugin](https://plugins.jetbrains.com/plugin/8182-rust)
2. Enable "Run Clippy on the fly"
3. Set format on save: `Settings → Languages & Frameworks → Rust → Run cargo fmt on Save`

---

## Project Structure

```
FILE/
├── crates/
│   ├── relay_server/              # Main relay server crate
│   │   ├── src/
│   │   │   ├── main.rs            # Entry point, HTTP server, shutdown
│   │   │   ├── rate_limiter.rs    # Token bucket rate limiter
│   │   │   ├── metrics.rs         # Prometheus metrics (AtomicU64)
│   │   │   ├── gdpr.rs            # IP hashing (HMAC-SHA256)
│   │   │   └── lib.rs             # Shared types, CAPoW verification
│   │   ├── tests/                 # Integration tests
│   │   │   ├── health_check_tests.rs
│   │   │   ├── rate_limiter_tests.rs
│   │   │   ├── metrics_tests.rs
│   │   │   ├── gdpr_tests.rs
│   │   │   ├── capow_replay_tests.rs
│   │   │   └── shutdown_tests.rs
│   │   ├── Cargo.toml             # Dependencies
│   │   └── README.md              # Crate-specific README
│   └── (other crates...)          # Future: MPQUIC, crypto, etc.
│
├── deploy/
│   ├── Dockerfile                 # Multi-stage Docker build
│   ├── docker-compose.yaml        # Local dev stack
│   ├── kubernetes/                # Production K8s manifests
│   │   ├── relay-server-deployment.yaml
│   │   └── relay-server-monitoring.yaml
│   ├── prometheus/                # Prometheus config
│   ├── alertmanager/              # Alert routing
│   └── grafana/                   # Dashboards
│
├── docs/
│   ├── deployment/                # Deployment guides
│   ├── security/                  # Security audits
│   ├── tests/                     # Test reports
│   ├── architecture/              # Architecture docs
│   ├── FAQ.md                     # User FAQ
│   ├── COMPARISON.md              # vs alternatives
│   └── DEVELOPMENT.md             # This file
│
├── examples/
│   ├── docker/                    # Docker examples
│   ├── kubernetes/                # K8s examples
│   └── systemd/                   # Bare metal examples
│
├── scripts/
│   ├── bump-version.sh            # Version bump automation
│   ├── create-release.sh          # Release workflow
│   ├── validate-release.sh        # Pre-release checks
│   └── check-dependencies.sh      # Dependency health
│
├── .github/
│   ├── workflows/
│   │   ├── ci.yml                 # Continuous integration
│   │   ├── cd.yml                 # Continuous deployment
│   │   └── security.yml           # Security scanning
│   ├── ISSUE_TEMPLATE/
│   └── PULL_REQUEST_TEMPLATE.md
│
├── Cargo.toml                     # Workspace root
├── Cargo.lock                     # Dependency lock file
├── LICENSE                        # MIT License
├── CONTRIBUTING.md                # Contribution guidelines
├── SECURITY.md                    # Security policy
├── CHANGELOG.md                   # Version history
└── README.md                      # Project overview (French)
```

---

## Building the Project

### Quick Start

```bash
# Clone repository
git clone https://github.com/file-network/relay-server.git
cd relay-server

# Build all crates (debug mode)
cargo build

# Build relay server only (release mode)
cargo build --release --package relay_server

# Run relay server
./target/release/file-relay --help
```

### Build Profiles

| Profile | Optimizations | Debug Info | Build Time | Use Case |
|---------|---------------|------------|------------|----------|
| `dev` (default) | None | Full | Fast | Development |
| `release` | Max | None | Slow | Production |
| `test` | Some | Full | Medium | Testing |

```bash
# Debug build (fast, unoptimized)
cargo build

# Release build (slow, optimized)
cargo build --release

# Release with debug symbols (for profiling)
cargo build --release --profile release-with-debug
```

### Build Options

```bash
# Build with all features
cargo build --all-features

# Build without default features
cargo build --no-default-features

# Build specific crate
cargo build -p relay_server

# Build for different target
cargo build --target x86_64-unknown-linux-musl
```

### Cross-Compilation

```bash
# Install cross-compilation tool
cargo install cross

# Build for ARM64
cross build --target aarch64-unknown-linux-gnu --release

# Build for Windows
cross build --target x86_64-pc-windows-gnu --release
```

---

## Running Tests

### Test Hierarchy

```
tests/
├── Unit tests (embedded in src/)       # Test individual functions
├── Integration tests (tests/*.rs)      # Test component interactions
├── Doc tests (/// examples in code)    # Test documentation examples
└── End-to-end tests (not implemented)  # Test full system (blocked by event loop)
```

### Running Tests

```bash
# Run all tests
cargo test

# Run only relay_server tests
cargo test --package relay_server

# Run specific test file
cargo test --test health_check_tests

# Run specific test function
cargo test test_rate_limiter_burst_capacity

# Run with output (see println! statements)
cargo test -- --nocapture

# Run with multiple threads (default)
cargo test -- --test-threads=4

# Run single-threaded (for debugging)
cargo test -- --test-threads=1

# Run ignored tests
cargo test -- --ignored

# Run all tests including ignored
cargo test -- --include-ignored
```

### Test Coverage

```bash
# Generate coverage report (Linux only)
cargo tarpaulin --out Html --output-dir coverage/

# Open report
open coverage/index.html
```

### Benchmarking

```bash
# Run benchmarks (future: criterion.rs)
cargo bench

# Profile benchmarks
cargo bench -- --profile-time 10
```

---

## Code Quality

### Formatting

```bash
# Check formatting
cargo fmt --check

# Auto-format code
cargo fmt

# Format specific file
cargo fmt -- src/main.rs
```

**CI enforces** Rust standard formatting (no configuration).

### Linting (Clippy)

```bash
# Run clippy
cargo clippy

# Run clippy with all lints
cargo clippy -- -D warnings

# Run clippy on all targets (tests + benches)
cargo clippy --all-targets

# Fix auto-fixable lints
cargo clippy --fix
```

**CI enforces** `-D warnings` (treat warnings as errors).

### Security Audit

```bash
# Audit dependencies for vulnerabilities
cargo audit

# Audit with auto-fix (upgrade vulnerable deps)
cargo audit fix

# Generate audit report
cargo audit --json > audit-report.json
```

**CI runs** daily and on every PR.

### Dependency Checks

```bash
# Check for outdated dependencies
cargo outdated

# List dependency tree
cargo tree

# List licenses
cargo license
```

---

## Debugging

### Logging

Relay server uses `tracing` for structured logging.

```bash
# Set log level via environment variable
export RUST_LOG=debug
./target/debug/file-relay

# Filter by module
export RUST_LOG=relay_server::rate_limiter=trace
./target/debug/file-relay

# Multiple filters
export RUST_LOG=relay_server=debug,axum=info
./target/debug/file-relay
```

**Log levels**: `error`, `warn`, `info`, `debug`, `trace`

### Debugger (LLDB)

```bash
# Run with LLDB
rust-lldb ./target/debug/file-relay

# In LLDB:
(lldb) b main                # Set breakpoint
(lldb) r                     # Run
(lldb) bt                    # Backtrace
(lldb) p variable_name       # Print variable
(lldb) c                     # Continue
```

### VS Code Debugging

Add to `.vscode/launch.json`:

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug relay_server",
      "cargo": {
        "args": ["build", "--bin=file-relay"],
        "filter": {
          "name": "file-relay",
          "kind": "bin"
        }
      },
      "args": [],
      "cwd": "${workspaceFolder}",
      "env": {
        "RUST_LOG": "debug"
      }
    }
  ]
}
```

---

## Contributing Workflow

### 1. Fork and Clone

```bash
# Fork on GitHub (click "Fork" button)

# Clone your fork
git clone https://github.com/YOUR_USERNAME/relay-server.git
cd relay-server

# Add upstream remote
git remote add upstream https://github.com/file-network/relay-server.git
```

### 2. Create Feature Branch

```bash
# Fetch latest from upstream
git fetch upstream
git checkout main
git merge upstream/main

# Create feature branch
git checkout -b feature/my-feature
```

**Branch naming**:
- `feature/` — new features
- `fix/` — bug fixes
- `docs/` — documentation
- `refactor/` — code refactoring
- `test/` — test improvements

### 3. Make Changes

```bash
# Make changes
$EDITOR src/main.rs

# Format code
cargo fmt

# Run lints
cargo clippy

# Run tests
cargo test

# Commit changes
git add -A
git commit -m "feat: add new feature"
```

**Commit message format**:
```
<type>: <description>

[optional body]

[optional footer]
```

**Types**: `feat`, `fix`, `docs`, `test`, `refactor`, `perf`, `chore`

### 4. Push and PR

```bash
# Push to your fork
git push origin feature/my-feature

# Open PR on GitHub
# Fill out PR template
# Wait for CI checks
# Address review comments
```

---

## Architecture Deep Dive

### Core Components

**1. HTTP Health Server (Axum)**
- Port 8081, TCP
- Routes: `/health`, `/ready`, `/metrics`
- Async (Tokio)

**2. QUIC Listener (pending event loop)**
- Port 8080, UDP
- Receives: REGISTER, RELAY_PACKET, HEARTBEAT
- Async (Tokio)

**3. CAPoW Worker Pool**
- Dedicated thread pool (25% cores, min 2)
- Blocking (Argon2id verification)
- 100ms timeout per proof

**4. Shared State (DashMap)**
- Peer registry: `DashMap<PeerId, PeerInfo>`
- Rate limiters: `DashMap<IpHash, TokenBucket>`
- Replay cache: `DashMap<ProofHash, Instant>`
- Lock-free concurrent access

**5. Metrics (AtomicU64)**
- 7 metrics total (1 gauge, 6 counters)
- Lock-free increments
- Exposed via `/metrics` in Prometheus text format

### Data Structures

**Peer Registry**:
```rust
pub struct PeerInfo {
    pub socket_addr: SocketAddr,
    pub last_seen: Instant,
}
```

**Rate Limiter**:
```rust
pub struct TokenBucket {
    tokens: f64,              // Current tokens (atomic via Mutex)
    capacity: f64,            // Max tokens (burst)
    refill_rate: f64,         // Tokens per second
    last_refill: Instant,     // Last refill time
}
```

**Metrics**:
```rust
pub struct RelayMetrics {
    pub active_peers: AtomicU64,              // Gauge
    pub peers_registered_total: AtomicU64,    // Counter
    pub capow_verified_total: AtomicU64,      // Counter
    pub capow_rejected_total: AtomicU64,      // Counter
    pub rate_limited_total: AtomicU64,        // Counter
    pub packets_forwarded_total: AtomicU64,   // Counter
    pub peers_timeout_total: AtomicU64,       // Counter
}
```

### Critical Paths

**Registration Path**:
```
1. Receive REGISTER request (QUIC)
2. Hash IP (HMAC-SHA256)
3. Check rate limit (token bucket)
4. Verify CAPoW (worker pool, 100ms timeout)
5. Check replay cache (SHA-256 hash)
6. Validate peer_id (32-byte Ed25519)
7. Add to peer registry
8. Send REGISTER_ACK
```

**Forwarding Path** (pending event loop):
```
1. Receive RELAY_PACKET (QUIC)
2. Lookup target peer in registry
3. Forward packet to target
4. Increment packets_forwarded_total
5. Send RELAY_ACK
```

---

## Adding New Features

### Example: Add New Metric

1. **Define metric** in `src/metrics.rs`:
```rust
pub struct RelayMetrics {
    // ... existing metrics ...
    pub new_metric_total: AtomicU64,  // Add this
}
```

2. **Initialize** in `main.rs`:
```rust
let metrics = Arc::new(RelayMetrics {
    // ... existing ...
    new_metric_total: AtomicU64::new(0),
});
```

3. **Increment** where appropriate:
```rust
metrics.new_metric_total.fetch_add(1, Ordering::Relaxed);
```

4. **Expose** in `/metrics` handler:
```rust
let new_metric = self.metrics.new_metric_total.load(Ordering::Relaxed);
write!(f, "file_relay_new_metric_total {}\n", new_metric)?;
```

5. **Document** in `README.md` metrics table

6. **Test** in `tests/metrics_tests.rs`:
```rust
#[tokio::test]
async fn test_new_metric() {
    let metrics = Arc::new(RelayMetrics::new());
    metrics.new_metric_total.fetch_add(1, Ordering::Relaxed);
    assert_eq!(metrics.new_metric_total.load(Ordering::Relaxed), 1);
}
```

---

## Performance Profiling

### CPU Profiling (Linux)

```bash
# Install flamegraph
cargo install flamegraph

# Profile relay server (requires sudo for perf)
sudo cargo flamegraph --bin file-relay

# Opens flamegraph.svg in browser
```

### Memory Profiling (Linux)

```bash
# Install heaptrack
sudo apt install heaptrack

# Profile relay server
heaptrack ./target/release/file-relay

# Analyze results
heaptrack_gui heaptrack.file-relay.*.gz
```

### Benchmarking

```bash
# Add criterion dependency to Cargo.toml
[dev-dependencies]
criterion = "0.5"

# Create benches/my_benchmark.rs
# Run benchmarks
cargo bench
```

---

## Troubleshooting

### Build Errors

**Error**: `error: linker 'cc' not found`
```bash
# Ubuntu/Debian
sudo apt install build-essential

# macOS
xcode-select --install
```

**Error**: `could not find OpenSSL`
```bash
# Ubuntu/Debian
sudo apt install libssl-dev pkg-config

# macOS
brew install openssl
export PKG_CONFIG_PATH="/usr/local/opt/openssl/lib/pkgconfig"
```

### Test Failures

**Error**: `test_rate_limiter_sustained_rate failed`
- **Cause**: Timing-sensitive test, system under load
- **Fix**: Run single-threaded: `cargo test -- --test-threads=1`

**Error**: `Address already in use (port 8081)`
- **Cause**: Another process using port 8081
- **Fix**: `lsof -i :8081`, kill process, or change `METRICS_ADDR` env var

### Runtime Errors

**Error**: `Too many open files`
```bash
# Increase file descriptor limit
ulimit -n 65536
```

**Error**: `Permission denied (bind port 8080)`
```bash
# Use high port (>1024) or run with sudo (not recommended)
# Or use setcap to allow binding low ports:
sudo setcap 'cap_net_bind_service=+ep' ./target/release/file-relay
```

---

## Further Reading

- [CONTRIBUTING.md](../CONTRIBUTING.md) — Full contribution guidelines
- [Architecture Overview](architecture/SYSTEM-OVERVIEW.md) — System design
- [Deployment Guide](deployment/DEPLOYMENT-GUIDE.md) — Production deployment
- [FAQ](FAQ.md) — Frequently asked questions
- [Rust Book](https://doc.rust-lang.org/book/) — Learn Rust

---

**Questions?** Join our [GitHub Discussions](https://github.com/file-network/relay-server/discussions) or open an [issue](https://github.com/file-network/relay-server/issues).
