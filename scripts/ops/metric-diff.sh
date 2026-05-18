#!/usr/bin/env bash
# metric-diff.sh - Compare two metric snapshots
# Usage: ./scripts/ops/metric-diff.sh <snapshot1.json> <snapshot2.json>

set -euo pipefail

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

info() { echo -e "${BLUE}$1${NC}"; }
success() { echo -e "${GREEN}$1${NC}"; }
warn() { echo -e "${YELLOW}$1${NC}"; }
error() { echo -e "${RED}$1${NC}"; }

if [[ $# -ne 2 ]]; then
    echo "Usage: $0 <snapshot1.json> <snapshot2.json>"
    echo
    echo "Compare two metric snapshots created by backup-metrics.sh"
    exit 1
fi

SNAP1="$1"
SNAP2="$2"

if [[ ! -f "$SNAP1" ]]; then
    error "Error: $SNAP1 not found"
    exit 1
fi

if [[ ! -f "$SNAP2" ]]; then
    error "Error: $SNAP2 not found"
    exit 1
fi

# Check for jq
if ! command -v jq &>/dev/null; then
    error "Error: jq is required but not installed"
    echo "Install with: brew install jq (macOS) or apt install jq (Linux)"
    exit 1
fi

# Extract timestamps
TS1=$(jq -r '.timestamp' "$SNAP1")
TS2=$(jq -r '.timestamp' "$SNAP2")

# Extract metrics
get_metric() {
    local file="$1"
    local metric="$2"
    jq -r ".metrics.$metric // 0" "$file"
}

# Calculate differences
calc_diff() {
    local val1="$1"
    local val2="$2"
    echo $(($val2 - $val1))
}

calc_percent() {
    local val1="$1"
    local val2="$2"
    if [[ $val1 -eq 0 ]]; then
        echo "N/A"
    else
        echo "scale=2; ($val2 - $val1) * 100 / $val1" | bc
    fi
}

# Metrics to compare
METRICS=(
    "active_peers"
    "peers_registered_total"
    "capow_verified_total"
    "capow_rejected_total"
    "rate_limited_total"
    "packets_forwarded_total"
    "peers_timeout_total"
)

echo
echo "═══════════════════════════════════════════════════════"
echo "  Metric Comparison"
echo "═══════════════════════════════════════════════════════"
echo
echo "Snapshot 1: $TS1"
echo "Snapshot 2: $TS2"
echo

printf "%-30s %12s %12s %12s %10s\n" "Metric" "Snapshot 1" "Snapshot 2" "Difference" "Change %"
echo "──────────────────────────────────────────────────────────────────────────────"

for metric in "${METRICS[@]}"; do
    VAL1=$(get_metric "$SNAP1" "$metric")
    VAL2=$(get_metric "$SNAP2" "$metric")
    DIFF=$(calc_diff "$VAL1" "$VAL2")
    PERCENT=$(calc_percent "$VAL1" "$VAL2")

    # Color code based on metric type and direction
    COLOR="$NC"
    if [[ "$metric" == "active_peers" ]]; then
        # Neutral for gauge
        COLOR="$BLUE"
    elif [[ "$metric" == *"rejected"* || "$metric" == *"limited"* || "$metric" == *"timeout"* ]]; then
        # Red for increasing errors
        if [[ $DIFF -gt 0 ]]; then
            COLOR="$RED"
        else
            COLOR="$GREEN"
        fi
    else
        # Green for increasing counters
        if [[ $DIFF -gt 0 ]]; then
            COLOR="$GREEN"
        fi
    fi

    printf "${COLOR}%-30s %12s %12s %12s %10s${NC}\n" \
        "$metric" "$VAL1" "$VAL2" "$DIFF" "$PERCENT"
done

echo

# Calculate rates (if time difference is available)
EPOCH1=$(date -j -f "%Y-%m-%dT%H:%M:%SZ" "$TS1" "+%s" 2>/dev/null || date -d "$TS1" "+%s" 2>/dev/null || echo 0)
EPOCH2=$(date -j -f "%Y-%m-%dT%H:%M:%SZ" "$TS2" "+%s" 2>/dev/null || date -d "$TS2" "+%s" 2>/dev/null || echo 0)
TIME_DIFF=$((EPOCH2 - EPOCH1))

if [[ $TIME_DIFF -gt 0 ]]; then
    echo "Rates (per second)"
    echo "──────────────────"

    PEERS_REGISTERED_DIFF=$(calc_diff "$(get_metric "$SNAP1" "peers_registered_total")" "$(get_metric "$SNAP2" "peers_registered_total")")
    PACKETS_FORWARDED_DIFF=$(calc_diff "$(get_metric "$SNAP1" "packets_forwarded_total")" "$(get_metric "$SNAP2" "packets_forwarded_total")")
    CAPOW_VERIFIED_DIFF=$(calc_diff "$(get_metric "$SNAP1" "capow_verified_total")" "$(get_metric "$SNAP2" "capow_verified_total")")

    REG_RATE=$(echo "scale=2; $PEERS_REGISTERED_DIFF / $TIME_DIFF" | bc)
    PACKET_RATE=$(echo "scale=2; $PACKETS_FORWARDED_DIFF / $TIME_DIFF" | bc)
    CAPOW_RATE=$(echo "scale=2; $CAPOW_VERIFIED_DIFF / $TIME_DIFF" | bc)

    echo "  Registration rate:    $REG_RATE req/s"
    echo "  Packet forward rate:  $PACKET_RATE pps"
    echo "  CAPoW verify rate:    $CAPOW_RATE proofs/s"
    echo
fi

# Health assessment
echo "Assessment"
echo "──────────"

CAPOW_REJECTED_DIFF=$(calc_diff "$(get_metric "$SNAP1" "capow_rejected_total")" "$(get_metric "$SNAP2" "capow_rejected_total")")
RATE_LIMITED_DIFF=$(calc_diff "$(get_metric "$SNAP1" "rate_limited_total")" "$(get_metric "$SNAP2" "rate_limited_total")")

if [[ $CAPOW_REJECTED_DIFF -gt 100 ]]; then
    error "⚠ High CAPoW rejection rate (+$CAPOW_REJECTED_DIFF): Possible DoS attack"
elif [[ $CAPOW_REJECTED_DIFF -gt 10 ]]; then
    warn "⚠ Elevated CAPoW rejection rate (+$CAPOW_REJECTED_DIFF): Monitor for attacks"
else
    success "✓ CAPoW rejection rate normal"
fi

if [[ $RATE_LIMITED_DIFF -gt 100 ]]; then
    error "⚠ High rate limiting (+$RATE_LIMITED_DIFF): Possible attack or misconfigured clients"
elif [[ $RATE_LIMITED_DIFF -gt 10 ]]; then
    warn "⚠ Elevated rate limiting (+$RATE_LIMITED_DIFF): Investigate client behavior"
else
    success "✓ Rate limiting normal"
fi

echo
