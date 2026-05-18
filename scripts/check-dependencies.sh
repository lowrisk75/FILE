#!/usr/bin/env bash
# check-dependencies.sh - Comprehensive dependency health check
# Usage: ./scripts/check-dependencies.sh [--json]

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

info() { echo -e "${GREEN}$1${NC}"; }
warn() { echo -e "${YELLOW}$1${NC}"; }
error() { echo -e "${RED}$1${NC}"; }
section() { echo -e "\n${BLUE}==> $1${NC}"; }

JSON_OUTPUT=false
if [[ $# -ge 1 && "$1" == "--json" ]]; then
    JSON_OUTPUT=true
fi

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$WORKSPACE_ROOT"

if [[ "$JSON_OUTPUT" == false ]]; then
    section "Dependency Health Check"
fi

TEMP_DIR=$(mktemp -d)
trap 'rm -rf "$TEMP_DIR"' EXIT

AUDIT_FILE="$TEMP_DIR/audit.json"
OUTDATED_FILE="$TEMP_DIR/outdated.json"
TREE_FILE="$TEMP_DIR/tree.txt"

if [[ "$JSON_OUTPUT" == false ]]; then
    info "Running cargo audit..."
fi
cargo audit --json > "$AUDIT_FILE" 2>/dev/null || true

if [[ "$JSON_OUTPUT" == false ]]; then
    info "Checking for outdated dependencies..."
fi
cargo outdated --format json > "$OUTDATED_FILE" 2>/dev/null || true

if [[ "$JSON_OUTPUT" == false ]]; then
    info "Generating dependency tree..."
fi
cargo tree --depth 3 > "$TREE_FILE" 2>/dev/null || true

VULNERABILITIES=$(jq '.vulnerabilities.count' "$AUDIT_FILE" 2>/dev/null || echo "0")
WARNINGS=$(jq '.warnings | length' "$AUDIT_FILE" 2>/dev/null || echo "0")
OUTDATED=$(jq '[.crates[] | select(.latest_version != .current_version)] | length' "$OUTDATED_FILE" 2>/dev/null || echo "0")

TOTAL_DEPS=$(grep -c '^[^ ]' "$TREE_FILE" 2>/dev/null || echo "0")

if [[ "$JSON_OUTPUT" == true ]]; then
    cat <<EOF
{
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "vulnerabilities": $VULNERABILITIES,
  "warnings": $WARNINGS,
  "outdated": $OUTDATED,
  "total_dependencies": $TOTAL_DEPS,
  "audit_details": $(cat "$AUDIT_FILE"),
  "outdated_details": $(cat "$OUTDATED_FILE")
}
EOF
else
    section "Summary"
    echo
    echo "Total dependencies:    $TOTAL_DEPS"
    echo "Security vulnerabilities: $VULNERABILITIES"
    echo "Security warnings:     $WARNINGS"
    echo "Outdated dependencies: $OUTDATED"
    echo

    if [[ $VULNERABILITIES -gt 0 ]]; then
        error "CRITICAL: $VULNERABILITIES security vulnerabilities found!"
        echo
        jq -r '.vulnerabilities.list[] | "  • \(.advisory.id): \(.advisory.title)\n    Package: \(.package.name) \(.package.version)\n    Fixed in: \(.versions.patched[])"' "$AUDIT_FILE"
        echo
    fi

    if [[ $WARNINGS -gt 0 ]]; then
        warn "$WARNINGS security warnings found"
        echo
        jq -r '.warnings[] | "  • \(.kind): \(.package.name) \(.package.version)\n    \(.advisory.title)"' "$AUDIT_FILE"
        echo
    fi

    if [[ $OUTDATED -gt 0 ]]; then
        warn "$OUTDATED dependencies are outdated"
        echo
        jq -r '[.crates[] | select(.latest_version != .current_version)] | .[] | "  • \(.name): \(.current_version) → \(.latest_version)"' "$OUTDATED_FILE" | head -n 10

        if [[ $OUTDATED -gt 10 ]]; then
            echo "  ... and $((OUTDATED - 10)) more"
        fi
        echo
    fi

    section "Recommendations"
    echo

    if [[ $VULNERABILITIES -gt 0 ]]; then
        error "Update vulnerable dependencies immediately:"
        echo "  cargo update"
        echo "  cargo audit fix"
        echo
    fi

    if [[ $OUTDATED -gt 0 ]]; then
        warn "Consider updating outdated dependencies:"
        echo "  cargo update"
        echo
    fi

    if [[ $VULNERABILITIES -eq 0 && $WARNINGS -eq 0 && $OUTDATED -eq 0 ]]; then
        info "✓ All dependencies are up-to-date and secure!"
        echo
    fi

    section "Detailed Reports"
    echo
    echo "Full audit report:    $AUDIT_FILE"
    echo "Outdated packages:    $OUTDATED_FILE"
    echo "Dependency tree:      $TREE_FILE"
    echo
fi

if [[ $VULNERABILITIES -gt 0 ]]; then
    exit 1
fi
