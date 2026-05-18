# FILE Relay Server - Documentation Index

**Last Updated**: 2026-05-18  
**Version**: 0.1.0

Welcome to the FILE Relay Server documentation hub. This index provides quick access to all documentation resources.

---

## Quick Links

| Document | Purpose | Audience |
|----------|---------|----------|
| [README](../README.md) | Project overview (French) | All |
| [Relay Server README](../crates/relay_server/README.md) | Quick start guide | Users |
| [FAQ](FAQ.md) | Frequently asked questions | All |
| [CHANGELOG](../CHANGELOG.md) | Version history | All |

---

## Getting Started

### For Users

1. **First time deploying?**
   - Read: [Deployment Guide](deployment/DEPLOYMENT-GUIDE.md)
   - Choose: [Examples](../examples/README.md) (Docker, Kubernetes, or systemd)
   - Monitor: [Operations Runbook](deployment/OPERATIONS-RUNBOOK.md)

2. **Need to compare with alternatives?**
   - Read: [Comparison Guide](COMPARISON.md) (vs Tailscale DERP, LibP2P, TURN, WebRTC)

3. **Have questions?**
   - Check: [FAQ](FAQ.md) (30+ questions answered)
   - Search: [GitHub Issues](https://github.com/file-network/relay-server/issues)

### For Developers

1. **First time contributing?**
   - Read: [Contributing Guide](../CONTRIBUTING.md)
   - Setup: [Development Guide](DEVELOPMENT.md)
   - Follow: [Code Standards](../CONTRIBUTING.md#code-standards)

2. **Understanding the architecture?**
   - Read: [System Overview](architecture/SYSTEM-OVERVIEW.md) (diagrams + explanations)
   - Review: [Implementation Summary](security/Implementation-Summary-2026-05-18.md)

3. **Running tests?**
   - Review: [Test Coverage Report](tests/Test-Coverage-Report-2026-05-18.md)
   - Run: `cargo test` (see [Development Guide](DEVELOPMENT.md#running-tests))

### For Operators

1. **Deploying to production?**
   - Read: [Deployment Guide](deployment/DEPLOYMENT-GUIDE.md)
   - Use: [Examples](../examples/README.md)
   - Review: [Performance Tuning](deployment/PERFORMANCE-TUNING.md)

2. **Responding to incidents?**
   - Follow: [Operations Runbook](deployment/OPERATIONS-RUNBOOK.md)
   - Check: [Troubleshooting](FAQ.md#troubleshooting)

3. **Optimizing performance?**
   - Read: [Performance Tuning Guide](deployment/PERFORMANCE-TUNING.md)
   - Monitor: [Metrics](deployment/DEPLOYMENT-GUIDE.md#monitoring-setup)

### For Security Reviewers

1. **Auditing security?**
   - Read: [Security Policy](../SECURITY.md)
   - Review: [Supply Chain Audit](security/Supply-Chain-Audit-2026-05-18.md)
   - Check: [Implementation Summary](security/Implementation-Summary-2026-05-18.md)

2. **Reporting vulnerabilities?**
   - Follow: [Responsible Disclosure](../SECURITY.md#reporting-a-vulnerability)

---

## Documentation by Category

### User Documentation

| Document | Description | Length |
|----------|-------------|--------|
| [FAQ](FAQ.md) | 30+ common questions covering deployment, security, operations | 14 KB |
| [Comparison Guide](COMPARISON.md) | FILE Relay vs alternatives (Tailscale DERP, LibP2P, TURN, WebRTC) | 20 KB |
| [Examples](../examples/README.md) | Docker, Kubernetes, systemd deployment examples | 8 KB |

### Deployment Documentation

| Document | Description | Length |
|----------|-------------|--------|
| [Deployment Guide](deployment/DEPLOYMENT-GUIDE.md) | Complete deployment procedures (local, Docker, Kubernetes) | 14 KB |
| [Operations Runbook](deployment/OPERATIONS-RUNBOOK.md) | Incident response playbooks + maintenance schedules | 21 KB |
| [Performance Tuning](deployment/PERFORMANCE-TUNING.md) | OS, application, K8s, network tuning for production | 18 KB |

### Developer Documentation

| Document | Description | Length |
|----------|-------------|--------|
| [Development Guide](DEVELOPMENT.md) | Setup, building, testing, contributing workflow | 16 KB |
| [Contributing Guide](../CONTRIBUTING.md) | Contribution guidelines, code standards, PR process | 11 KB |
| [System Overview](architecture/SYSTEM-OVERVIEW.md) | Complete architecture with diagrams | 22 KB |

### Security Documentation

| Document | Description | Length |
|----------|-------------|--------|
| [Security Policy](../SECURITY.md) | Security features, best practices, disclosure policy | 9 KB |
| [Supply Chain Audit](security/Supply-Chain-Audit-2026-05-18.md) | cargo-audit scan results + risk assessment | 29 KB |
| [Implementation Summary](security/Implementation-Summary-2026-05-18.md) | Architecture decisions + security findings | 24 KB |

### Testing Documentation

| Document | Description | Length |
|----------|-------------|--------|
| [Test Coverage Report](tests/Test-Coverage-Report-2026-05-18.md) | 75 tests across 6 suites, 100% infrastructure coverage | 29 KB |

### Release Documentation

| Document | Description | Length |
|----------|-------------|--------|
| [CHANGELOG](../CHANGELOG.md) | Version history following Keep a Changelog format | 7 KB |

---

## Quick Reference Cards

### Deployment Quick Start

```bash
# Docker Compose
cd examples/docker
docker-compose -f docker-compose.production.yaml up -d

# Kubernetes (Helm)
helm install file-relay-server . -f examples/kubernetes/production-values.yaml

# Systemd (bare metal)
sudo cp examples/systemd/file-relay.service /etc/systemd/system/
sudo systemctl enable --now file-relay
```

### Monitoring Quick Start

```bash
# Health check
curl http://localhost:8081/health

# Readiness check (capacity check)
curl http://localhost:8081/ready

# Prometheus metrics
curl http://localhost:8081/metrics
```

### Troubleshooting Quick Reference

| Problem | Check | Solution |
|---------|-------|----------|
| Peers can't connect | `curl http://relay:8081/ready` | Check capacity, firewall |
| High CAPoW rejection | `curl http://relay:8081/metrics \| grep capow_rejected` | Check for DoS attack |
| High memory usage | `curl http://relay:8081/metrics \| grep active_peers` | Scale horizontally |
| Service won't start | `journalctl -u file-relay` | Check logs for errors |

---

## Automation Scripts

Located in `scripts/`:

| Script | Purpose | Usage |
|--------|---------|-------|
| [bump-version.sh](../scripts/bump-version.sh) | Bump version across workspace | `./scripts/bump-version.sh <major\|minor\|patch\|X.Y.Z>` |
| [create-release.sh](../scripts/create-release.sh) | Create and push release | `./scripts/create-release.sh <version> [--dry-run]` |
| [validate-release.sh](../scripts/validate-release.sh) | Pre-release validation | `./scripts/validate-release.sh` |
| [check-dependencies.sh](../scripts/check-dependencies.sh) | Dependency health check | `./scripts/check-dependencies.sh [--json]` |

---

## External Resources

### Community

- **GitHub Repository**: https://github.com/file-network/relay-server
- **GitHub Issues**: https://github.com/file-network/relay-server/issues
- **GitHub Discussions**: https://github.com/file-network/relay-server/discussions

### Related Projects

- **MPQUIC**: Multi-Path QUIC implementation
- **Noise Framework**: Cryptographic protocol framework
- **Tailscale DERP**: Alternative relay solution
- **LibP2P**: Decentralized networking stack

### Standards

- **RFC 5766**: TURN (relay protocol)
- **RFC 9000**: QUIC transport protocol
- **Keep a Changelog**: CHANGELOG format
- **Semantic Versioning**: Version numbering

---

## Documentation Statistics

**Total documentation**: ~205 KB across 15+ documents

| Category | Documents | Total Size |
|----------|-----------|------------|
| User | 3 | 42 KB |
| Deployment | 3 | 53 KB |
| Developer | 3 | 49 KB |
| Security | 3 | 62 KB |
| Testing | 1 | 29 KB |
| Other | 3 | 30 KB |

**Test coverage**: 75 tests, 100% infrastructure coverage

**CI/CD**: 24 jobs across 3 workflows (ci.yml, cd.yml, security.yml)

---

## Documentation Roadmap

**Completed** ✅:
- All user-facing documentation
- Complete deployment guides
- Security audit and policy
- Test coverage report
- Architecture diagrams
- Performance tuning guide
- Development guide
- Comparison with alternatives
- Example configurations
- Release automation scripts

**Pending** (blocked by event loop implementation):
- End-to-end testing guide
- Load testing guide
- Performance benchmark results
- Real-world deployment case studies

---

## Contributing to Documentation

Documentation contributions are welcome! See [CONTRIBUTING.md](../CONTRIBUTING.md) for guidelines.

**Documentation standards**:
- Markdown format (GitHub-flavored)
- Line length: 100 characters max (exceptions for code blocks, URLs)
- Headings: Title case for H1/H2, sentence case for H3+
- Code blocks: Always specify language for syntax highlighting
- Examples: Prefer real, working examples over pseudo-code
- Tone: Professional, clear, concise

**File naming**:
- Uppercase for top-level docs: `README.md`, `CONTRIBUTING.md`, `FAQ.md`
- Kebab-case for subdirectory docs: `deployment-guide.md`, `system-overview.md`
- Date-stamped for versioned reports: `Supply-Chain-Audit-2026-05-18.md`

---

## Getting Help

1. **Check documentation first**:
   - [FAQ](FAQ.md) — Most common questions
   - [Troubleshooting](deployment/OPERATIONS-RUNBOOK.md#troubleshooting) — Common issues

2. **Search existing issues**:
   - [GitHub Issues](https://github.com/file-network/relay-server/issues)

3. **Ask the community**:
   - [GitHub Discussions](https://github.com/file-network/relay-server/discussions)

4. **Report bugs**:
   - [New Issue](https://github.com/file-network/relay-server/issues/new) (use templates)

5. **Security issues**:
   - Email: security@file-network.example (see [SECURITY.md](../SECURITY.md))

---

## License

All documentation is licensed under [MIT License](../LICENSE), same as the code.

---

**Last updated**: 2026-05-18  
**Documentation version**: Matches code version 0.1.0  
**Maintainer**: FILE Network Team
