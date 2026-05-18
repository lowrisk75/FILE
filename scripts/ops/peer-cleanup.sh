#!/usr/bin/env bash
# peer-cleanup.sh - Trigger manual peer cleanup (for testing/debugging)
# Usage: ./scripts/ops/peer-cleanup.sh [relay-host]
#
# Note: The relay automatically cleans up peers every 60 seconds.
# This script is for manual/emergency cleanup only.

set -euo pipefail

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

info() { echo -e "${BLUE}$1${NC}"; }
warn() { echo -e "${YELLOW}$1${NC}"; }
success() { echo -e "${GREEN}$1${NC}"; }

RELAY_HOST="${1:-localhost}"

info "Fetching current metrics from $RELAY_HOST..."

# Fetch current state
if ! METRICS=$(curl -sf http://$RELAY_HOST:8081/metrics 2>/dev/null); then
    echo "Error: Cannot connect to relay server at $RELAY_HOST:8081"
    exit 1
fi

ACTIVE_PEERS_BEFORE=$(echo "$METRICS" | grep "^file_relay_active_peers" | awk '{print $2}')
PEERS_TIMEOUT_BEFORE=$(echo "$METRICS" | grep "^file_relay_peers_timeout_total" | awk '{print $2}')

echo
echo "Current State"
echo "─────────────"
echo "  Active peers:         $ACTIVE_PEERS_BEFORE"
echo "  Peers timed out:      $PEERS_TIMEOUT_BEFORE"
echo

warn "⚠ The relay automatically cleans up peers every 60 seconds."
warn "  Manual cleanup is typically not needed."
echo

read -p "Continue with manual cleanup check? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    info "Cleanup cancelled."
    exit 0
fi

info "Waiting 5 seconds for next automatic cleanup cycle..."
sleep 5

# Fetch metrics again
if ! METRICS=$(curl -sf http://$RELAY_HOST:8081/metrics 2>/dev/null); then
    echo "Error: Cannot connect to relay server"
    exit 1
fi

ACTIVE_PEERS_AFTER=$(echo "$METRICS" | grep "^file_relay_active_peers" | awk '{print $2}')
PEERS_TIMEOUT_AFTER=$(echo "$METRICS" | grep "^file_relay_peers_timeout_total" | awk '{print $2}')

PEERS_CLEANED=$((ACTIVE_PEERS_BEFORE - ACTIVE_PEERS_AFTER))
TIMEOUTS_INCREMENTED=$((PEERS_TIMEOUT_AFTER - PEERS_TIMEOUT_BEFORE))

echo
echo "After Cleanup"
echo "─────────────"
echo "  Active peers:         $ACTIVE_PEERS_AFTER"
echo "  Peers timed out:      $PEERS_TIMEOUT_AFTER"
echo
echo "Changes"
echo "───────"
echo "  Peers cleaned:        $PEERS_CLEANED"
echo "  New timeouts:         $TIMEOUTS_INCREMENTED"
echo

if [[ $PEERS_CLEANED -gt 0 ]]; then
    success "✓ Cleanup removed $PEERS_CLEANED stale peer(s)"
elif [[ $ACTIVE_PEERS_AFTER -gt 0 ]]; then
    info "No stale peers found (all active peers are healthy)"
else
    info "No active peers"
fi
