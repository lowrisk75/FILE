# FILE Relay Server - Testing

Comprehensive testing framework covering unit tests, integration tests, load tests, and performance benchmarks.

---

## Overview

FILE Relay Server uses a multi-layered testing approach:

1. **Unit Tests** — Test individual functions and modules in isolation
2. **Integration Tests** — Test component interactions and end-to-end flows
3. **Load Tests** — Validate performance and capacity under realistic load
4. **Regression Tests** — Detect performance regressions across versions
5. **Security Tests** — Validate DoS protection and security controls

---

## Test Coverage

### Current Status

```
Relay Server:
  Unit tests:         48 passing
  Integration tests:  27 passing
  Total test coverage: 85%+

Load Testing:
  Framework:          Complete
  Scenarios:          7 defined
  CI integration:     Ready
```

---

## Quick Start

### 1. Unit and Integration Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_capow_verification

# Run with coverage
cargo tarpaulin --out Html
open tarpaulin-report.html
```

**Location**: `relay-server/tests/`

**Documentation**: See inline test documentation in source files

---

### 2. Load Tests

```bash
cd testing/load

# Install dependencies
pip install -r requirements.txt

# Run quick test (10 peers, 30s)
./relay_load_test.py --host localhost --peers 10 --duration 30

# Run capacity test (1000 peers, 60s)
./relay_load_test.py --host localhost --peers 1000 --duration 60 --packet-rate 5.0

# Save baseline
./relay_load_test.py \
  --host localhost \
  --peers 500 \
  --duration 120 \
  --seed 42 \
  --output baseline-v0.1.0.json
```

**Documentation**: [testing/load/README.md](load/README.md)

---

### 3. Regression Tests

```bash
cd testing/load

# Compare baselines
./compare_baselines.py baseline-v0.1.0.json baseline-v0.2.0.json

# Fail on regression (for CI)
./compare_baselines.py baseline.json current.json --fail-on-regression
```

**Documentation**: [testing/load/README.md#regression-testing](load/README.md#regression-testing)

---

## Test Pyramid

```
                      ╱╲
                     ╱  ╲
                    ╱    ╲
                   ╱ Load ╲         < 10 scenarios, slow, realistic
                  ╱  Tests ╲
                 ╱──────────╲
                ╱            ╲
               ╱              ╲
              ╱  Integration  ╲     < 30 tests, medium speed
             ╱      Tests      ╲
            ╱────────────────────╲
           ╱                      ╲
          ╱       Unit Tests       ╲  < 50+ tests, fast, isolated
         ╱──────────────────────────╲
```

**Philosophy**:
- **Many unit tests**: Fast, cheap, catch bugs early
- **Some integration tests**: Validate component interactions
- **Few load tests**: Expensive, realistic, validate production readiness

---

## Testing Strategies

### 1. Test-Driven Development (TDD)

**Process**:
1. Write failing test
2. Implement minimal code to pass
3. Refactor
4. Repeat

**Example** (adding new metric):

```rust
#[test]
fn test_new_metric_incremented() {
    let metrics = Metrics::new();
    metrics.increment_new_metric();
    assert_eq!(metrics.get_new_metric(), 1);
}
```

**When to use**: New features, bug fixes

---

### 2. Property-Based Testing

**Concept**: Generate random inputs, assert properties always hold.

**Example** (CAPoW verification):

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_capow_verification_never_panics(
        peer_id in prop::array::uniform32(any::<u8>()),
        proof in prop::collection::vec(any::<u8>(), 0..1024)
    ) {
        // Should never panic, even with random input
        let _ = verify_capow(&peer_id, &proof);
    }
}
```

**When to use**: Security-critical code, parsers, validation

---

### 3. Fuzz Testing

**Concept**: Generate malformed inputs to find crashes and vulnerabilities.

**Setup** (with cargo-fuzz):

```bash
cargo install cargo-fuzz
cargo fuzz init
cargo fuzz add fuzz_capow

# Run fuzzer
cargo fuzz run fuzz_capow
```

**Example target**:

```rust
#[fuzz_target]
fn fuzz_capow(data: &[u8]) {
    if data.len() >= 32 {
        let (peer_id, proof) = data.split_at(32);
        let _ = verify_capow(peer_id, proof);
    }
}
```

**When to use**: Security-critical parsers (CAPoW, message parsing)

---

### 4. Chaos Engineering

**Concept**: Inject failures to test resilience.

**Scenarios**:
- Kill random pods (test recovery)
- Inject network latency (test timeout handling)
- Exhaust resources (test graceful degradation)

**Tools**:
- **Chaos Mesh**: Kubernetes-native chaos engineering
- **Litmus**: Cloud-native chaos engineering

**Example** (Chaos Mesh):

```yaml
apiVersion: chaos-mesh.org/v1alpha1
kind: PodChaos
metadata:
  name: relay-pod-kill
spec:
  action: pod-kill
  mode: one
  selector:
    namespaces:
      - file-relay
    labelSelectors:
      app: file-relay-server
  scheduler:
    cron: "*/5 * * * *"  # Every 5 minutes
```

**When to use**: Pre-production validation, quarterly disaster recovery drills

---

## Test Scenarios

### Unit Tests (relay-server/tests/)

| Test | Purpose | Duration |
|------|---------|----------|
| `test_capow_verification` | Validate CAPoW proof verification | < 1ms |
| `test_rate_limiting` | Validate per-IP rate limiting | < 1ms |
| `test_ip_hashing` | Validate GDPR-compliant IP hashing | < 1ms |
| `test_metrics_atomicity` | Validate atomic metric updates | < 1ms |
| `test_graceful_shutdown` | Validate clean shutdown | 100ms |

**Total**: 48 tests, ~200ms total runtime

---

### Integration Tests (relay-server/tests/integration/)

| Test | Purpose | Duration |
|------|---------|----------|
| `test_peer_registration_flow` | End-to-end registration | 50ms |
| `test_packet_forwarding` | Packet relay through server | 100ms |
| `test_peer_timeout_cleanup` | Zombie peer removal | 5s |
| `test_concurrent_registrations` | Race conditions | 200ms |
| `test_metrics_endpoint` | Prometheus scraping | 10ms |

**Total**: 27 tests, ~10s total runtime

---

### Load Tests (testing/load/)

| Scenario | Purpose | Duration | When to Run |
|----------|---------|----------|-------------|
| Smoke Test | Quick validation | 30s | After every deploy |
| Capacity Test | Max peer count | 60s | Before production |
| Throughput Test | Max packets/s | 60s | Performance work |
| Churn Test | Stability under churn | 120s | Before production |
| Stress Test | Overload behavior | 300s | Before production |
| Soak Test | Memory leaks | 3600s | Quarterly |
| Baseline Benchmark | Regression detection | 120s | Every release |

---

## CI/CD Integration

### GitHub Actions Workflow

```yaml
name: Test

on: [push, pull_request]

jobs:
  unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run tests
        run: cargo test --all-features

  integration-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Run integration tests
        run: cargo test --test '*' -- --ignored

  load-tests:
    runs-on: ubuntu-latest
    if: github.event_name == 'pull_request'
    steps:
      - uses: actions/checkout@v3

      - name: Start relay server
        run: |
          cargo build --release
          ./target/release/relay-server &
          sleep 5

      - name: Run load test
        run: |
          pip install -r testing/load/requirements.txt
          ./testing/load/relay_load_test.py \
            --host localhost \
            --peers 100 \
            --duration 60 \
            --output results.json

      - name: Compare baseline
        run: |
          ./testing/load/compare_baselines.py \
            testing/load/baselines/baseline-main.json \
            results.json \
            --fail-on-regression

  security-audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Security audit
        run: cargo audit
```

---

## Performance Baselines

### Expected Performance (1 vCPU, 512 MB RAM)

| Metric | Light Load | Medium Load | Heavy Load |
|--------|-----------|-------------|------------|
| **Peers** | 100 | 500 | 1000 |
| **Throughput** | 100 pkt/s | 2,500 pkt/s | 10,000 pkt/s |
| **Latency (p95)** | < 10ms | < 20ms | < 50ms |
| **CPU Usage** | 5-10% | 30-40% | 70-80% |
| **Memory Usage** | 128 MB | 256 MB | 384 MB |
| **Packet Delivery** | > 99.9% | > 99.5% | > 99% |

**Baseline files**: `testing/load/baselines/`

---

## Troubleshooting Tests

### Unit Tests Failing

```bash
# Run with verbose output
cargo test -- --nocapture

# Run specific test with debug
RUST_LOG=debug cargo test test_name -- --nocapture

# Check for race conditions
cargo test -- --test-threads=1
```

### Load Tests Failing

**High rejection rate**:
- Check CAPoW difficulty (too high?)
- Check rate limiting (too strict?)
- Check server capacity (overloaded?)

**Low delivery rate**:
- Check network (packet loss?)
- Check CPU (throttled?)
- Check memory (OOM?)

**Connection refused**:
- Is server running? `curl http://localhost:8081/health`
- Check firewall rules
- Check port binding

### Baseline Comparison Failing

```bash
# Check what changed
./testing/load/compare_baselines.py baseline.json current.json

# If legitimate change, update baseline
cp current.json baseline-vX.Y.Z.json
git add baseline-vX.Y.Z.json
git commit -m "Update performance baseline for vX.Y.Z"
```

---

## Best Practices

### 1. Test Isolation

- Each test should be independent
- No shared state between tests
- Clean up after each test

### 2. Descriptive Names

```rust
// Bad
#[test]
fn test1() { ... }

// Good
#[test]
fn test_capow_verification_rejects_invalid_proof() { ... }
```

### 3. Test One Thing

```rust
// Bad (tests multiple things)
#[test]
fn test_registration() {
    // Test CAPoW verification
    // Test rate limiting
    // Test metrics
}

// Good (focused)
#[test]
fn test_registration_increments_peer_count() { ... }

#[test]
fn test_registration_enforces_rate_limit() { ... }
```

### 4. Arrange-Act-Assert Pattern

```rust
#[test]
fn test_example() {
    // Arrange (setup)
    let relay = RelayServer::new();
    let peer_id = [0u8; 32];

    // Act (execute)
    let result = relay.register(peer_id);

    // Assert (verify)
    assert!(result.is_ok());
}
```

### 5. Test Edge Cases

- Empty inputs
- Maximum values
- Boundary conditions
- Error paths

---

## Further Reading

- **Load Testing Guide**: [testing/load/README.md](load/README.md)
- **Performance Tuning**: [docs/deployment/PERFORMANCE-TUNING.md](../docs/deployment/PERFORMANCE-TUNING.md)
- **Capacity Planning**: [docs/deployment/CAPACITY-PLANNING.md](../docs/deployment/CAPACITY-PLANNING.md)
- **Rust Testing Book**: https://doc.rust-lang.org/book/ch11-00-testing.html

---

**Questions?** [Open an issue](https://github.com/file-network/relay-server/issues) or consult the [FAQ](../docs/FAQ.md).
