#!/usr/bin/env bash
# bump-version.sh - Bump version across all workspace crates
# Usage: ./scripts/bump-version.sh <major|minor|patch|X.Y.Z>

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

error() { echo -e "${RED}Error: $1${NC}" >&2; exit 1; }
info() { echo -e "${GREEN}$1${NC}"; }
warn() { echo -e "${YELLOW}$1${NC}"; }

if [[ $# -ne 1 ]]; then
    error "Usage: $0 <major|minor|patch|X.Y.Z>"
fi

VERSION_ARG="$1"
WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "$WORKSPACE_ROOT"

if [[ ! -f "Cargo.toml" ]]; then
    error "Cargo.toml not found in $WORKSPACE_ROOT"
fi

CURRENT_VERSION=$(grep -m1 '^version = "' Cargo.toml | sed 's/version = "\(.*\)"/\1/')

if [[ -z "$CURRENT_VERSION" ]]; then
    error "Could not detect current version from Cargo.toml"
fi

info "Current version: $CURRENT_VERSION"

if [[ "$VERSION_ARG" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    NEW_VERSION="$VERSION_ARG"
elif [[ "$VERSION_ARG" =~ ^(major|minor|patch)$ ]]; then
    IFS='.' read -r major minor patch <<< "$CURRENT_VERSION"
    case "$VERSION_ARG" in
        major)
            NEW_VERSION="$((major + 1)).0.0"
            ;;
        minor)
            NEW_VERSION="$major.$((minor + 1)).0"
            ;;
        patch)
            NEW_VERSION="$major.$minor.$((patch + 1))"
            ;;
    esac
else
    error "Invalid version argument. Use 'major', 'minor', 'patch', or semantic version (e.g., 1.2.3)"
fi

info "New version: $NEW_VERSION"

if [[ -n $(git status --porcelain) ]]; then
    warn "Working directory has uncommitted changes. Commit or stash before bumping version."
    exit 1
fi

if git rev-parse "v$NEW_VERSION" >/dev/null 2>&1; then
    error "Tag v$NEW_VERSION already exists"
fi

info "Updating Cargo.toml files..."

find . -name Cargo.toml -type f | while read -r cargo_file; do
    if grep -q '^version = "' "$cargo_file"; then
        sed -i.bak "s/^version = \".*\"/version = \"$NEW_VERSION\"/" "$cargo_file"
        rm "${cargo_file}.bak"
        info "  Updated $cargo_file"
    fi
done

info "Updating Cargo.lock..."
cargo check --quiet 2>/dev/null || true

info "Updating CHANGELOG.md..."
if [[ -f "CHANGELOG.md" ]]; then
    TODAY=$(date +%Y-%m-%d)
    sed -i.bak "s/## \[Unreleased\]/## [Unreleased]\n\n---\n\n## [$NEW_VERSION] - $TODAY/" CHANGELOG.md

    PREV_TAG=$(git describe --tags --abbrev=0 2>/dev/null || echo "")
    if [[ -n "$PREV_TAG" ]]; then
        sed -i.bak "s|\[Unreleased\]:.*|\[Unreleased\]: https://github.com/file-network/relay-server/compare/v$NEW_VERSION...HEAD\n[$NEW_VERSION]: https://github.com/file-network/relay-server/compare/$PREV_TAG...v$NEW_VERSION|" CHANGELOG.md
    else
        sed -i.bak "s|\[Unreleased\]:.*|\[Unreleased\]: https://github.com/file-network/relay-server/compare/v$NEW_VERSION...HEAD\n[$NEW_VERSION]: https://github.com/file-network/relay-server/releases/tag/v$NEW_VERSION|" CHANGELOG.md
    fi

    rm CHANGELOG.md.bak
    info "  Updated CHANGELOG.md"
else
    warn "CHANGELOG.md not found, skipping"
fi

info ""
info "Version bumped from $CURRENT_VERSION to $NEW_VERSION"
info ""
info "Next steps:"
info "  1. Review changes: git diff"
info "  2. Commit changes: git add -A && git commit -m \"chore: bump version to $NEW_VERSION\""
info "  3. Create tag: git tag -a v$NEW_VERSION -m \"Release v$NEW_VERSION\""
info "  4. Push: git push origin main --tags"
info ""
info "Or run: ./scripts/create-release.sh $NEW_VERSION"
