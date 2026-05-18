# FILE Relay Server - Automation Scripts

This directory contains automation scripts for release management and operations.

---

## Directory Structure

```
scripts/
├── README.md                      # This file
├── bump-version.sh                # Version bumping
├── create-release.sh              # Release workflow
├── validate-release.sh            # Pre-release validation
├── check-dependencies.sh          # Dependency health check
└── ops/
    ├── health-check.sh            # Comprehensive health check
    ├── capacity-calculator.sh     # Resource sizing calculator
    ├── backup-metrics.sh          # Metrics snapshot
    ├── peer-cleanup.sh            # Manual peer cleanup
    └── metric-diff.sh             # Compare metric snapshots
```

---

## Release Management Scripts

### bump-version.sh

Bump version across all workspace crates.

**Usage**:
```bash
./scripts/bump-version.sh <major|minor|patch|X.Y.Z>
```

**Examples**:
```bash
# Bump patch version (0.1.0 → 0.1.1)
./scripts/bump-version.sh patch

# Bump minor version (0.1.1 → 0.2.0)
./scripts/bump-version.sh minor

# Set specific version
./scripts/bump-version.sh 1.0.0
```

**What it does**:
1. Updates `version` in all `Cargo.toml` files
2. Updates `Cargo.lock`
3. Updates `CHANGELOG.md` with release date and version
4. Prints next steps (commit, tag, push)

**Does NOT**:
- Commit changes (you review first)
- Create git tags (use `create-release.sh` for that)
- Push to remote

---

### create-release.sh

Create and push a new release.

**Usage**:
```bash
./scripts/create-release.sh <version> [--dry-run]
```

**Examples**:
```bash
# Dry run (no commits/tags created)
./scripts/create-release.sh 0.2.0 --dry-run

# Real release
./scripts/create-release.sh 0.2.0
```

**What it does**:
1. Runs pre-flight checks (tests, lints, security audit)
2. Builds release binary
3. Extracts changelog entry for this version
4. Creates commit with message `chore: release v<version>`
5. Creates annotated git tag `v<version>`
6. Pushes commit and tag to origin
7. Prints next steps (monitor CI/CD)

**Prerequisites**:
- Clean working directory (no uncommitted changes)
- Version exists in `CHANGELOG.md`
- All tests passing
- On `main` branch (or confirm otherwise)

---

### validate-release.sh

Validate release readiness.

**Usage**:
```bash
./scripts/validate-release.sh
```

**What it checks**:
- Working directory clean
- On main branch
- Code formatted (`cargo fmt --check`)
- No clippy warnings (`cargo clippy -- -D warnings`)
- All tests passing (`cargo test`)
- No security vulnerabilities (`cargo audit`)
- All required files exist (README, CHANGELOG, LICENSE, etc.)
- Docker image builds successfully

**Exit codes**:
- 0: All checks passed, ready to release
- 1: One or more checks failed

**Use before**: Running `create-release.sh`

---

### check-dependencies.sh

Check dependency health.

**Usage**:
```bash
./scripts/check-dependencies.sh [--json]
```

**Output formats**:
- **Human-readable** (default): Colored summary with recommendations
- **JSON**: Machine-readable for CI/CD integration

**Examples**:
```bash
# Human-readable output
./scripts/check-dependencies.sh

# JSON output (for CI/CD)
./scripts/check-dependencies.sh --json > dependency-report.json
```

**What it checks**:
- Security vulnerabilities (`cargo audit`)
- Outdated dependencies (`cargo outdated`)
- Total dependency count
- Recommendations for updates

**Exit codes**:
- 0: No vulnerabilities found
- 1: Vulnerabilities detected

---

## Operations Scripts

### ops/health-check.sh

Comprehensive health check for deployed relay.

**Usage**:
```bash
./scripts/ops/health-check.sh [relay-host] [--verbose]
```

**Examples**:
```bash
# Check localhost
./scripts/ops/health-check.sh

# Check remote relay
./scripts/ops/health-check.sh relay.example.com

# Verbose output (show curl commands)
./scripts/ops/health-check.sh relay.example.com --verbose
```

**What it checks**:
- ✓ Endpoint availability (/health, /ready, /metrics)
- ✓ Metrics validation (all 7 metrics present)
- ✓ Capacity status (peer count vs max)
- ✓ Security health (CAPoW rejection rate, rate limiting)
- ✓ Network connectivity (UDP 8080, TCP 8081)

**Output**:
- Colored output (✓ success, ⚠ warning, ✗ error)
- Summary (passed/warned/failed counts)

**Exit codes**:
- 0: All checks passed or warnings only
- 1: One or more checks failed

**Use cases**:
- Post-deployment validation
- Scheduled health monitoring
- Incident triage

---

### ops/capacity-calculator.sh

Calculate resource requirements for target peer count.

**Usage**:
```bash
./scripts/ops/capacity-calculator.sh <target-peers> [--json]
```

**Examples**:
```bash
# Calculate for 1,000 peers
./scripts/ops/capacity-calculator.sh 1000

# Calculate for 5,000 peers (JSON output)
./scripts/ops/capacity-calculator.sh 5000 --json
```

**Output**:
- Single-instance requirements (not recommended)
- **Horizontal scaling recommendation** (preferred)
  - Instance count
  - Peers per instance (1000)
  - Per-instance CPU/memory
- Cloud instance recommendations (AWS, GCP, DigitalOcean)
- Docker Compose configuration snippet
- Kubernetes configuration snippet

**Calculations**:
- Memory: 512 MB base + 0.5 MB per peer + 20% headroom
- CPU: 100m base + 50m per 100 peers + 20% headroom
- Network: 0.5 Mbps per peer (forwarding only)

**Use cases**:
- Capacity planning
- Cost estimation
- Infrastructure sizing

---

### ops/backup-metrics.sh

Backup current metrics snapshot for analysis.

**Usage**:
```bash
./scripts/ops/backup-metrics.sh [relay-host] [output-dir]
```

**Examples**:
```bash
# Backup from localhost to default directory
./scripts/ops/backup-metrics.sh

# Backup from remote relay to custom directory
./scripts/ops/backup-metrics.sh relay.example.com /var/backups/metrics
```

**Output files**:
- `metrics-<timestamp>.txt` — Raw Prometheus text format
- `metrics-<timestamp>.json` — Structured JSON with derived metrics

**Derived metrics**:
- CAPoW success rate (%)
- Packets per peer (average)

**Use cases**:
- Pre/post deployment comparison
- Incident investigation
- Performance analysis
- Historical trend analysis

---

### ops/peer-cleanup.sh

Trigger manual peer cleanup (testing/debugging only).

**Usage**:
```bash
./scripts/ops/peer-cleanup.sh [relay-host]
```

**What it does**:
1. Fetches current metrics (peer count, timeouts)
2. Warns that automatic cleanup runs every 60s
3. Prompts for confirmation
4. Waits 5 seconds for next cleanup cycle
5. Fetches metrics again
6. Reports peers cleaned

**Note**: The relay automatically cleans up peers every 60 seconds. This script is for manual verification only.

**Use cases**:
- Debugging cleanup logic
- Verifying timeout behavior
- Testing cleanup performance

---

### ops/metric-diff.sh

Compare two metric snapshots.

**Usage**:
```bash
./scripts/ops/metric-diff.sh <snapshot1.json> <snapshot2.json>
```

**Prerequisites**: Requires `jq` (install with `brew install jq` or `apt install jq`)

**Example**:
```bash
# Create two snapshots
./scripts/ops/backup-metrics.sh relay.example.com
# ... wait some time ...
./scripts/ops/backup-metrics.sh relay.example.com

# Compare
./scripts/ops/metric-diff.sh \
  metrics-backups/metrics-20260518-100000.json \
  metrics-backups/metrics-20260518-103000.json
```

**Output**:
- Side-by-side comparison (snapshot1 vs snapshot2)
- Difference (absolute)
- Change percentage
- Rates (per second, if time difference available)
- Health assessment (CAPoW, rate limiting)

**Color coding**:
- 🔵 Blue: Gauge (neutral)
- 🟢 Green: Increasing good counters (packets forwarded)
- 🔴 Red: Increasing error counters (rejections, rate limits)

**Use cases**:
- Before/after deployment comparison
- Attack detection (sudden increase in rejections)
- Performance regression testing

---

## Quick Reference

| Task | Script | Example |
|------|--------|---------|
| Bump version | `bump-version.sh` | `./scripts/bump-version.sh patch` |
| Create release | `create-release.sh` | `./scripts/create-release.sh 0.2.0` |
| Validate before release | `validate-release.sh` | `./scripts/validate-release.sh` |
| Check dependencies | `check-dependencies.sh` | `./scripts/check-dependencies.sh` |
| Health check | `ops/health-check.sh` | `./scripts/ops/health-check.sh relay.example.com` |
| Calculate capacity | `ops/capacity-calculator.sh` | `./scripts/ops/capacity-calculator.sh 5000` |
| Backup metrics | `ops/backup-metrics.sh` | `./scripts/ops/backup-metrics.sh` |
| Compare snapshots | `ops/metric-diff.sh` | `./scripts/ops/metric-diff.sh snap1.json snap2.json` |

---

## CI/CD Integration

### GitHub Actions

```yaml
# .github/workflows/release.yml
- name: Validate release
  run: ./scripts/validate-release.sh

- name: Check dependencies
  run: ./scripts/check-dependencies.sh --json > dependency-report.json

- name: Create release
  run: ./scripts/create-release.sh ${{ github.ref_name }}
```

### Kubernetes CronJob

```yaml
apiVersion: batch/v1
kind: CronJob
metadata:
  name: relay-health-check
spec:
  schedule: "*/5 * * * *"  # Every 5 minutes
  jobTemplate:
    spec:
      template:
        spec:
          containers:
          - name: health-check
            image: curlimages/curl:latest
            command:
            - sh
            - -c
            - |
              curl -sL https://raw.githubusercontent.com/file-network/relay-server/main/scripts/ops/health-check.sh | sh -s -- file-relay-server.file-relay.svc.cluster.local
```

---

## Prerequisites

### All Scripts

- Bash 4.0+
- Standard Unix utilities (`grep`, `awk`, `sed`, `date`)

### Release Scripts

- Rust 1.83+ (`cargo`)
- Git 2.30+

### Operations Scripts

- `curl` (for fetching metrics)
- `nc` (netcat, for network checks)
- `jq` (for `metric-diff.sh` only)
- `bc` (for calculations)

### Optional

- Docker (for Docker image validation)
- Kubernetes CLI (`kubectl`, for K8s health checks)

---

## Troubleshooting

### "Permission denied"

Scripts must be executable:
```bash
chmod +x scripts/*.sh
chmod +x scripts/ops/*.sh
```

### "curl: command not found"

Install curl:
```bash
# Ubuntu/Debian
sudo apt install curl

# macOS (usually pre-installed)
brew install curl
```

### "jq: command not found"

Install jq (required for `metric-diff.sh` only):
```bash
# Ubuntu/Debian
sudo apt install jq

# macOS
brew install jq
```

### "bc: command not found"

Install bc:
```bash
# Ubuntu/Debian
sudo apt install bc

# macOS
brew install bc
```

---

## Contributing

Found a bug or have an idea for a new script? See [CONTRIBUTING.md](../CONTRIBUTING.md).

**Script conventions**:
- Bash shebang: `#!/usr/bin/env bash`
- Strict mode: `set -euo pipefail`
- Help message on wrong usage
- Colored output (green=success, red=error, yellow=warning, blue=info)
- Exit codes: 0=success, 1=error

---

## Further Reading

- [Operations Runbook](../docs/deployment/OPERATIONS-RUNBOOK.md) — Manual procedures
- [Observability Guide](../docs/deployment/OBSERVABILITY-GUIDE.md) — Monitoring best practices
- [Performance Tuning](../docs/deployment/PERFORMANCE-TUNING.md) — Optimization guide

---

**Questions?** [Open an issue](https://github.com/file-network/relay-server/issues).
