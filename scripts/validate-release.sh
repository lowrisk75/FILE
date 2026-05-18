#!/usr/bin/env bash
# validate-release.sh - Validate release readiness
# Usage: ./scripts/validate-release.sh

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

error() { echo -e "${RED}✗ $1${NC}"; }
success() { echo -e "${GREEN}✓ $1${NC}"; }
warn() { echo -e "${YELLOW}⚠ $1${NC}"; }
section() { echo -e "\n${BLUE}==> $1${NC}"; }

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$WORKSPACE_ROOT"

CHECKS_PASSED=0
CHECKS_FAILED=0
CHECKS_WARNED=0

check() {
    if eval "$1" >/dev/null 2>&1; then
        success "$2"
        ((CHECKS_PASSED++))
        return 0
    else
        error "$2"
        ((CHECKS_FAILED++))
        return 1
    fi
}

warn_check() {
    if eval "$1" >/dev/null 2>&1; then
        success "$2"
        ((CHECKS_PASSED++))
    else
        warn "$2"
        ((CHECKS_WARNED++))
    fi
}

section "Repository Health"

check "[[ -z \$(git status --porcelain) ]]" "Working directory is clean"
check "[[ \$(git rev-parse --abbrev-ref HEAD) == 'main' ]]" "On main branch"
check "git diff-index --quiet HEAD --" "No uncommitted changes"

section "Code Quality"

check "cargo fmt --check" "Code is formatted (cargo fmt)"
check "cargo clippy --quiet -- -D warnings" "No clippy warnings"
check "cargo test --quiet" "All tests pass"

section "Security"

warn_check "cargo audit" "No security vulnerabilities (cargo audit)"
warn_check "! grep -r 'TODO.*SECURITY' crates/" "No pending security TODOs"
warn_check "! grep -r 'FIXME.*SECURITY' crates/" "No security FIXMEs"

section "Documentation"

check "[[ -f README.md ]]" "README.md exists"
check "[[ -f CHANGELOG.md ]]" "CHANGELOG.md exists"
check "[[ -f LICENSE ]]" "LICENSE exists"
check "[[ -f CONTRIBUTING.md ]]" "CONTRIBUTING.md exists"
check "[[ -f SECURITY.md ]]" "SECURITY.md exists"
check "grep -q '## \[Unreleased\]' CHANGELOG.md" "CHANGELOG.md has Unreleased section"

section "Deployment Artifacts"

check "[[ -f deploy/Dockerfile ]]" "Dockerfile exists"
check "[[ -f deploy/docker-compose.yaml ]]" "docker-compose.yaml exists"
check "[[ -f deploy/kubernetes/relay-server-deployment.yaml ]]" "Kubernetes deployment manifest exists"
check "[[ -f deploy/kubernetes/relay-server-monitoring.yaml ]]" "Kubernetes monitoring manifest exists"

section "CI/CD"

check "[[ -f .github/workflows/ci.yml ]]" "CI workflow exists"
check "[[ -f .github/workflows/cd.yml ]]" "CD workflow exists"
check "[[ -f .github/workflows/security.yml ]]" "Security workflow exists"

section "Build Verification"

check "cargo build --release --quiet" "Release build succeeds"

if [[ -f target/release/file-relay ]]; then
    BINARY_SIZE=$(stat -f%z target/release/file-relay 2>/dev/null || stat -c%s target/release/file-relay 2>/dev/null)
    if [[ $BINARY_SIZE -lt 20971520 ]]; then
        success "Binary size reasonable ($BINARY_SIZE bytes < 20 MB)"
        ((CHECKS_PASSED++))
    else
        warn "Binary size large ($BINARY_SIZE bytes >= 20 MB)"
        ((CHECKS_WARNED++))
    fi
fi

section "Docker Build Test"

if command -v docker >/dev/null 2>&1; then
    if docker build -t file-relay:test -f deploy/Dockerfile . >/dev/null 2>&1; then
        success "Docker image builds successfully"
        ((CHECKS_PASSED++))
        docker rmi file-relay:test >/dev/null 2>&1 || true
    else
        error "Docker build failed"
        ((CHECKS_FAILED++))
    fi
else
    warn "Docker not available, skipping image build test"
    ((CHECKS_WARNED++))
fi

section "Summary"

echo
echo "Checks passed:  $CHECKS_PASSED"
echo "Checks warned:  $CHECKS_WARNED"
echo "Checks failed:  $CHECKS_FAILED"
echo

if [[ $CHECKS_FAILED -eq 0 ]]; then
    success "Release validation passed! Ready to proceed."
    exit 0
else
    error "Release validation failed. Fix the issues above before releasing."
    exit 1
fi
