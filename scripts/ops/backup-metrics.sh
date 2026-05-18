#!/usr/bin/env bash
# backup-metrics.sh - Backup current metrics snapshot for analysis
# Usage: ./scripts/ops/backup-metrics.sh [relay-host] [output-dir]

set -euo pipefail

GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

info() { echo -e "${BLUE}$1${NC}"; }
success() { echo -e "${GREEN}$1${NC}"; }

RELAY_HOST="${1:-localhost}"
OUTPUT_DIR="${2:-./metrics-backups}"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
BACKUP_FILE="$OUTPUT_DIR/metrics-$TIMESTAMP.txt"
JSON_FILE="$OUTPUT_DIR/metrics-$TIMESTAMP.json"

mkdir -p "$OUTPUT_DIR"

info "Fetching metrics from $RELAY_HOST:8081..."

# Fetch raw metrics
if ! METRICS=$(curl -sf http://$RELAY_HOST:8081/metrics); then
    echo "Error: Failed to fetch metrics from $RELAY_HOST:8081"
    exit 1
fi

# Save raw metrics
echo "$METRICS" > "$BACKUP_FILE"
success "Raw metrics saved to $BACKUP_FILE"

# Extract and structure as JSON
ACTIVE_PEERS=$(echo "$METRICS" | grep "^file_relay_active_peers" | awk '{print $2}')
PEERS_REGISTERED=$(echo "$METRICS" | grep "^file_relay_peers_registered_total" | awk '{print $2}')
CAPOW_VERIFIED=$(echo "$METRICS" | grep "^file_relay_capow_verified_total" | awk '{print $2}')
CAPOW_REJECTED=$(echo "$METRICS" | grep "^file_relay_capow_rejected_total" | awk '{print $2}')
RATE_LIMITED=$(echo "$METRICS" | grep "^file_relay_rate_limited_total" | awk '{print $2}')
PACKETS_FORWARDED=$(echo "$METRICS" | grep "^file_relay_packets_forwarded_total" | awk '{print $2}')
PEERS_TIMEOUT=$(echo "$METRICS" | grep "^file_relay_peers_timeout_total" | awk '{print $2}')

# Calculate derived metrics
TOTAL_CAPOW=$((CAPOW_VERIFIED + CAPOW_REJECTED))
CAPOW_SUCCESS_RATE=0
if [[ $TOTAL_CAPOW -gt 0 ]]; then
    CAPOW_SUCCESS_RATE=$(echo "scale=2; $CAPOW_VERIFIED * 100 / $TOTAL_CAPOW" | bc)
fi

# Create JSON
cat > "$JSON_FILE" <<EOF
{
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "relay_host": "$RELAY_HOST",
  "metrics": {
    "active_peers": $ACTIVE_PEERS,
    "peers_registered_total": $PEERS_REGISTERED,
    "capow_verified_total": $CAPOW_VERIFIED,
    "capow_rejected_total": $CAPOW_REJECTED,
    "rate_limited_total": $RATE_LIMITED,
    "packets_forwarded_total": $PACKETS_FORWARDED,
    "peers_timeout_total": $PEERS_TIMEOUT
  },
  "derived": {
    "capow_success_rate_percent": $CAPOW_SUCCESS_RATE,
    "packets_per_peer": $(echo "scale=2; $PACKETS_FORWARDED / $ACTIVE_PEERS" | bc 2>/dev/null || echo 0)
  }
}
EOF

success "Structured metrics saved to $JSON_FILE"

# Summary
echo
echo "Snapshot Summary"
echo "────────────────"
echo "  Timestamp:            $(date)"
echo "  Active peers:         $ACTIVE_PEERS"
echo "  Packets forwarded:    $PACKETS_FORWARDED"
echo "  CAPoW success rate:   ${CAPOW_SUCCESS_RATE}%"
echo
success "✓ Metrics backup complete!"
