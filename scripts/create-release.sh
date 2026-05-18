#!/usr/bin/env bash
# create-release.sh - Create and push a new release
# Usage: ./scripts/create-release.sh <version> [--dry-run]

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

error() { echo -e "${RED}Error: $1${NC}" >&2; exit 1; }
info() { echo -e "${GREEN}$1${NC}"; }
warn() { echo -e "${YELLOW}$1${NC}"; }
section() { echo -e "\n${BLUE}==> $1${NC}"; }

if [[ $# -lt 1 ]]; then
    error "Usage: $0 <version> [--dry-run]"
fi

VERSION="$1"
DRY_RUN=false

if [[ $# -ge 2 && "$2" == "--dry-run" ]]; then
    DRY_RUN=true
    warn "DRY RUN MODE - No changes will be committed"
fi

if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    error "Version must be semantic (e.g., 1.2.3)"
fi

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$WORKSPACE_ROOT"

if [[ -n $(git status --porcelain) ]]; then
    error "Working directory must be clean before creating a release"
fi

CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)
if [[ "$CURRENT_BRANCH" != "main" ]]; then
    warn "Current branch is '$CURRENT_BRANCH', not 'main'"
    read -p "Continue? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

if git rev-parse "v$VERSION" >/dev/null 2>&1; then
    error "Tag v$VERSION already exists"
fi

section "Pre-flight checks"

info "✓ Running tests..."
cargo test --quiet || error "Tests failed"

info "✓ Running clippy..."
cargo clippy --quiet -- -D warnings || error "Clippy checks failed"

info "✓ Running cargo fmt check..."
cargo fmt --check || error "Code not formatted (run 'cargo fmt')"

info "✓ Running security audit..."
cargo audit || warn "Security audit found issues (review before proceeding)"

info "✓ Checking for outdated dependencies..."
cargo outdated --exit-code 1 >/dev/null 2>&1 || warn "Some dependencies are outdated"

section "Building release binary"

cargo build --release --quiet || error "Release build failed"

BINARY_SIZE=$(stat -f%z target/release/file-relay 2>/dev/null || stat -c%s target/release/file-relay 2>/dev/null || echo "unknown")
info "✓ Binary size: $BINARY_SIZE bytes"

section "Extracting changelog"

if [[ ! -f "CHANGELOG.md" ]]; then
    error "CHANGELOG.md not found"
fi

CHANGELOG_ENTRY=$(awk "/## \[$VERSION\]/,/## \[/" CHANGELOG.md | sed '/## \[/d' | sed '/^$/d' | head -n -1)

if [[ -z "$CHANGELOG_ENTRY" ]]; then
    error "No changelog entry found for version $VERSION. Update CHANGELOG.md first."
fi

info "Changelog entry:"
echo "$CHANGELOG_ENTRY"
echo

section "Creating release commit and tag"

if [[ "$DRY_RUN" == true ]]; then
    info "[DRY RUN] Would commit: chore: release v$VERSION"
    info "[DRY RUN] Would create tag: v$VERSION"
    info "[DRY RUN] Would push to origin"
else
    git add -A
    git commit -m "chore: release v$VERSION" || error "Commit failed"

    git tag -a "v$VERSION" -m "Release v$VERSION

$CHANGELOG_ENTRY" || error "Tag creation failed"

    info "✓ Created commit and tag v$VERSION"

    section "Pushing to remote"

    git push origin "$CURRENT_BRANCH" || error "Push failed"
    git push origin "v$VERSION" || error "Tag push failed"

    info "✓ Pushed to origin"
fi

section "Summary"

info "Release v$VERSION ready!"
info ""
info "Next steps:"
info "  1. Monitor CI/CD: https://github.com/file-network/relay-server/actions"
info "  2. Verify Docker image: ghcr.io/file-network/relay-server:$VERSION"
info "  3. Check GitHub Release (auto-generated): https://github.com/file-network/relay-server/releases/tag/v$VERSION"
info "  4. Monitor staging deployment: kubectl -n file-relay get pods"
info "  5. Approve production canary: Review metrics after 5 minutes"
info ""

if [[ "$DRY_RUN" == true ]]; then
    warn "DRY RUN completed - no changes were made"
fi
