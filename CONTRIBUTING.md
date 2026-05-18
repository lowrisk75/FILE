# Contributing to FILE Relay Server

Thank you for your interest in contributing! This document provides guidelines for contributing to the FILE Relay Server project.

---

## Table of Contents

1. [Code of Conduct](#code-of-conduct)
2. [Getting Started](#getting-started)
3. [Development Workflow](#development-workflow)
4. [Coding Standards](#coding-standards)
5. [Testing Requirements](#testing-requirements)
6. [Pull Request Process](#pull-request-process)
7. [Issue Guidelines](#issue-guidelines)

---

## Code of Conduct

This project adheres to a code of conduct adapted from the [Contributor Covenant](https://www.contributor-covenant.org/). By participating, you are expected to uphold this code.

### Our Standards

**Positive behaviors:**
- Using welcoming and inclusive language
- Being respectful of differing viewpoints
- Gracefully accepting constructive criticism
- Focusing on what is best for the community
- Showing empathy towards other community members

**Unacceptable behaviors:**
- Harassment, trolling, or derogatory comments
- Publishing others' private information without permission
- Other conduct which could reasonably be considered inappropriate

**Enforcement:** Violations may result in temporary or permanent ban from the project. Report issues to: security@file-network.example

---

## Getting Started

### Prerequisites

- **Rust**: 1.83+ ([install](https://rustup.rs/))
- **Git**: For version control
- **Docker**: For testing deployment (optional)
- **kubectl**: For Kubernetes testing (optional)

### Fork and Clone

```bash
# Fork the repository on GitHub first, then:
git clone https://github.com/YOUR_USERNAME/relay-server.git
cd relay-server

# Add upstream remote
git remote add upstream https://github.com/file-network/relay-server.git
```

### Build and Test

```bash
# Install dependencies
cargo build

# Run tests
cargo test

# Run relay server locally
cargo run -p relay_server -- --bind 127.0.0.1:8080
```

---

## Development Workflow

### Branching Strategy

- **`main`**: Production-ready code, protected branch
- **`develop`**: Integration branch for features
- **`feature/*`**: Feature branches (e.g., `feature/add-websocket-support`)
- **`bugfix/*`**: Bug fix branches (e.g., `bugfix/fix-rate-limiter-overflow`)
- **`hotfix/*`**: Urgent production fixes

### Creating a Feature Branch

```bash
# Update your fork
git checkout develop
git pull upstream develop

# Create feature branch
git checkout -b feature/your-feature-name

# Make changes, commit
git add .
git commit -m "feat: add your feature"

# Push to your fork
git push origin feature/your-feature-name
```

### Keeping Your Fork Updated

```bash
# Fetch upstream changes
git fetch upstream

# Merge into your local branch
git checkout develop
git merge upstream/develop

# Push to your fork
git push origin develop
```

---

## Coding Standards

### Rust Style

We follow the [Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/):

```bash
# Format code before committing
cargo fmt --all

# Check for common mistakes
cargo clippy --all-targets --all-features -- -D warnings
```

### Code Quality Requirements

- **No compiler warnings** in release mode
- **All clippy warnings resolved**
- **Code formatted with `rustfmt`**
- **No `unsafe` code without detailed justification**
- **Meaningful variable names** (no single-letter except loop counters)

### Comments

- Document **why**, not **what** (code should be self-documenting)
- Use `///` for public API documentation
- Use `//` for inline comments (sparingly)
- Add `// SAFETY:` comments for all `unsafe` blocks

**Good**:
```rust
// SECURITY: Replay prevention to stop DoS attacks via cached proofs
let cache_entry = CacheEntry::new();
```

**Bad**:
```rust
// Create a new cache entry
let cache_entry = CacheEntry::new();
```

### Error Handling

- Use `Result<T, E>` for recoverable errors
- Use `panic!` only for unrecoverable errors
- Provide context with error messages
- Log errors before returning them

**Good**:
```rust
let config = Config::load().context("Failed to load configuration")?;
```

**Bad**:
```rust
let config = Config::load().unwrap();
```

---

## Testing Requirements

### Test Coverage

All contributions must include tests:

- **Unit tests**: For individual functions/modules
- **Integration tests**: For feature interactions
- **Edge cases**: Boundary conditions, error paths

### Running Tests

```bash
# All tests
cargo test

# Specific package
cargo test --package relay_server

# Specific test file
cargo test --test rate_limiter_tests

# With output
cargo test -- --nocapture --test-threads=1
```

### Writing Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_allows_burst() {
        let mut limiter = RateLimiter::new(50.0, 10.0);
        
        // Should allow 50 requests immediately
        for _ in 0..50 {
            assert!(limiter.try_consume());
        }
        
        // 51st should be denied
        assert!(!limiter.try_consume());
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        // Async test using tokio runtime
    }
}
```

### Test Naming

- `test_<feature>_<scenario>` (e.g., `test_rate_limiter_refill`)
- Descriptive assertion messages
- One logical assertion per test

---

## Pull Request Process

### Before Submitting

1. **Run all checks**:
   ```bash
   cargo fmt --all --check
   cargo clippy --all-targets --all-features
   cargo test --all
   cargo audit
   ```

2. **Update documentation** if needed:
   - README.md
   - API docs (`cargo doc`)
   - Deployment guides

3. **Write a clear commit message**:
   ```
   feat: add WebSocket support for relay protocol
   
   - Implement WebSocket handler in main.rs
   - Add WS-specific rate limiting
   - Update docs with WS configuration
   
   Closes #123
   ```

### Commit Message Format

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types**:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `style`: Formatting (no code change)
- `refactor`: Code restructuring (no behavior change)
- `perf`: Performance improvement
- `test`: Adding tests
- `chore`: Build/tooling changes
- `security`: Security fix

**Examples**:
```
feat(rate-limiter): add exponential backoff
fix(metrics): correct packet counter overflow
docs: update deployment guide with Terraform
security: patch RUSTSEC-2026-0123 in maxminddb
```

### Creating a Pull Request

1. **Push your branch** to your fork
2. **Open a PR** on GitHub against `develop` (not `main`)
3. **Fill out the PR template** completely
4. **Link related issues** (e.g., "Closes #123")
5. **Request review** from maintainers

### PR Template

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix (non-breaking change)
- [ ] New feature (non-breaking change)
- [ ] Breaking change (fix or feature causing existing functionality to break)
- [ ] Documentation update

## Testing
- [ ] All tests pass locally
- [ ] Added new tests for this change
- [ ] Manual testing performed

## Checklist
- [ ] Code follows style guidelines
- [ ] Self-review completed
- [ ] Comments added for complex logic
- [ ] Documentation updated
- [ ] No new compiler warnings
- [ ] cargo audit passes
```

### Review Process

1. **Automated checks** must pass (CI/CD)
2. **One maintainer approval** required
3. **All review comments addressed**
4. **Squash and merge** (clean commit history)

**Review timeline**:
- Simple fixes: 1-2 days
- Features: 3-5 days
- Large refactors: 1 week+

---

## Issue Guidelines

### Before Creating an Issue

1. **Search existing issues** (open and closed)
2. **Check documentation** for solutions
3. **Try latest version** (bug may be fixed)

### Issue Types

We use issue templates for:
- 🐛 Bug reports
- ✨ Feature requests
- 📖 Documentation improvements
- 🔒 Security vulnerabilities (use security@file-network.example instead)

### Bug Report Template

```markdown
**Describe the bug**
Clear description of what's wrong

**To Reproduce**
1. Start relay server with '...'
2. Send CAPoW proof '...'
3. Observe error '...'

**Expected behavior**
What you expected to happen

**Environment:**
- OS: [e.g., Ubuntu 22.04]
- Rust version: [e.g., 1.83.0]
- Relay version: [e.g., 0.1.0]

**Logs**
```
[paste relevant logs]
```

**Additional context**
Any other relevant information
```

### Feature Request Template

```markdown
**Problem Statement**
What problem does this solve?

**Proposed Solution**
How should it work?

**Alternatives Considered**
What other approaches did you consider?

**Additional Context**
Any relevant examples or use cases
```

---

## Specific Contribution Areas

### Security

Security contributions are highly valued:

- **Review code** for vulnerabilities
- **Run security scanners** (cargo-audit, clippy)
- **Report vulnerabilities** privately to security@file-network.example
- **Improve security documentation**

**Do not** open public issues for security vulnerabilities.

### Performance

Performance improvements welcome:

- **Benchmark before/after** (use `cargo bench`)
- **Profile with flamegraph** (show hotspots)
- **Measure impact** (CPU, memory, throughput)
- **Document trade-offs** (complexity vs. speed)

### Documentation

Documentation improvements always appreciated:

- **Fix typos/grammar**
- **Clarify confusing sections**
- **Add examples**
- **Improve code comments**

Small docs PRs can skip issue creation.

### Testing

Expand test coverage:

- **Add edge cases**
- **Test error paths**
- **Improve integration tests**
- **Add performance benchmarks**

---

## Getting Help

**Questions?**
- GitHub Discussions (preferred)
- Slack: #file-relay-dev
- Email: dev@file-network.example

**Stuck?**
- Tag `@maintainers` in your PR
- Ask in GitHub Discussions
- Join community calls (monthly)

---

## Recognition

Contributors are recognized in:
- **CHANGELOG.md** (for each release)
- **README.md** (significant contributions)
- **GitHub Contributors** page

---

## License

By contributing, you agree that your contributions will be licensed under the project's MIT License.

---

**Thank you for contributing to FILE Relay Server!** 🎉
