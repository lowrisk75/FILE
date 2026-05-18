#!/usr/bin/env bash
# capacity-calculator.sh - Calculate resource requirements for FILE Relay Server
# Usage: ./scripts/ops/capacity-calculator.sh <target-peers> [--json]

set -euo pipefail

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

info() { echo -e "${BLUE}$1${NC}"; }
warn() { echo -e "${YELLOW}$1${NC}"; }
success() { echo -e "${GREEN}$1${NC}"; }

if [[ $# -lt 1 ]]; then
    echo "Usage: $0 <target-peers> [--json]"
    echo
    echo "Examples:"
    echo "  $0 1000              # Calculate for 1,000 peers"
    echo "  $0 5000 --json       # Output as JSON"
    exit 1
fi

TARGET_PEERS="$1"
JSON_OUTPUT=false

if [[ $# -ge 2 && "$2" == "--json" ]]; then
    JSON_OUTPUT=true
fi

# Validate input
if ! [[ "$TARGET_PEERS" =~ ^[0-9]+$ ]]; then
    echo "Error: target-peers must be a number"
    exit 1
fi

# Constants (based on baseline measurements)
BASE_MEMORY_MB=512
MEMORY_PER_PEER_MB=0.5
BASE_CPU_MILLICORES=100
CPU_PER_100_PEERS_MILLICORES=50
NETWORK_PER_PEER_MBPS=0.5

# Calculations
TOTAL_MEMORY_MB=$(echo "$BASE_MEMORY_MB + ($TARGET_PEERS * $MEMORY_PER_PEER_MB)" | bc)
TOTAL_CPU_MILLICORES=$(echo "$BASE_CPU_MILLICORES + (($TARGET_PEERS / 100) * $CPU_PER_100_PEERS_MILLICORES)" | bc)
TOTAL_NETWORK_MBPS=$(echo "$TARGET_PEERS * $NETWORK_PER_PEER_MBPS" | bc)

# Convert to human-readable units
TOTAL_MEMORY_GB=$(echo "scale=2; $TOTAL_MEMORY_MB / 1024" | bc)
TOTAL_CPU_CORES=$(echo "scale=2; $TOTAL_CPU_MILLICORES / 1000" | bc)
TOTAL_NETWORK_GBPS=$(echo "scale=2; $TOTAL_NETWORK_MBPS / 1000" | bc)

# Add 20% headroom
MEMORY_WITH_HEADROOM=$(echo "$TOTAL_MEMORY_GB * 1.2" | bc)
CPU_WITH_HEADROOM=$(echo "$TOTAL_CPU_CORES * 1.2" | bc)
NETWORK_WITH_HEADROOM=$(echo "$TOTAL_NETWORK_GBPS * 1.2" | bc)

# Round up for resource requests
MEMORY_REQUEST_GB=$(printf "%.0f" "$MEMORY_WITH_HEADROOM")
CPU_REQUEST_CORES=$(printf "%.1f" "$CPU_WITH_HEADROOM")

# Deployment strategy (horizontal scaling at 1000 peers per instance)
PEERS_PER_INSTANCE=1000
INSTANCE_COUNT=$(echo "($TARGET_PEERS + $PEERS_PER_INSTANCE - 1) / $PEERS_PER_INSTANCE" | bc)

# Per-instance resources
INSTANCE_MEMORY_GB=$(echo "scale=2; $MEMORY_REQUEST_GB / $INSTANCE_COUNT" | bc)
INSTANCE_CPU_CORES=$(echo "scale=2; $CPU_REQUEST_CORES / $INSTANCE_COUNT" | bc)

# Ensure minimum per instance
if (( $(echo "$INSTANCE_MEMORY_GB < 1" | bc -l) )); then
    INSTANCE_MEMORY_GB=1
fi
if (( $(echo "$INSTANCE_CPU_CORES < 0.5" | bc -l) )); then
    INSTANCE_CPU_CORES=0.5
fi

# Cloud instance recommendations
# AWS
if (( $(echo "$INSTANCE_CPU_CORES <= 1" | bc -l) )); then
    AWS_INSTANCE="t3.small"
elif (( $(echo "$INSTANCE_CPU_CORES <= 2" | bc -l) )); then
    AWS_INSTANCE="t3.medium"
else
    AWS_INSTANCE="t3.large"
fi

# GCP
if (( $(echo "$INSTANCE_CPU_CORES <= 1" | bc -l) )); then
    GCP_INSTANCE="e2-small"
elif (( $(echo "$INSTANCE_CPU_CORES <= 2" | bc -l) )); then
    GCP_INSTANCE="e2-medium"
else
    GCP_INSTANCE="e2-standard-2"
fi

# Digital Ocean
if (( $(echo "$INSTANCE_CPU_CORES <= 1" | bc -l) )); then
    DO_INSTANCE="s-1vcpu-2gb"
elif (( $(echo "$INSTANCE_CPU_CORES <= 2" | bc -l) )); then
    DO_INSTANCE="s-2vcpu-4gb"
else
    DO_INSTANCE="s-4vcpu-8gb"
fi

# JSON output
if [[ "$JSON_OUTPUT" == true ]]; then
    cat <<EOF
{
  "input": {
    "target_peers": $TARGET_PEERS
  },
  "single_instance": {
    "cpu_cores": $TOTAL_CPU_CORES,
    "memory_gb": $TOTAL_MEMORY_GB,
    "network_gbps": $TOTAL_NETWORK_GBPS
  },
  "with_headroom": {
    "cpu_cores": $CPU_REQUEST_CORES,
    "memory_gb": $MEMORY_REQUEST_GB,
    "network_gbps": $NETWORK_WITH_HEADROOM
  },
  "horizontal_scaling": {
    "instance_count": $INSTANCE_COUNT,
    "peers_per_instance": $PEERS_PER_INSTANCE,
    "per_instance": {
      "cpu_cores": $INSTANCE_CPU_CORES,
      "memory_gb": $INSTANCE_MEMORY_GB
    }
  },
  "cloud_recommendations": {
    "aws": "$AWS_INSTANCE",
    "gcp": "$GCP_INSTANCE",
    "digitalocean": "$DO_INSTANCE"
  }
}
EOF
    exit 0
fi

# Human-readable output
echo
echo "═══════════════════════════════════════════════════════"
echo "  FILE Relay Server - Capacity Calculator"
echo "═══════════════════════════════════════════════════════"
echo
echo "📊 Target Configuration"
echo "───────────────────────"
echo "  Target peers:         $TARGET_PEERS"
echo

echo "💻 Single Instance Requirements (Not Recommended)"
echo "──────────────────────────────────────────────────"
echo "  CPU:                  $TOTAL_CPU_CORES cores"
echo "  Memory:               $TOTAL_MEMORY_GB GB"
echo "  Network:              $TOTAL_NETWORK_GBPS Gbps"
echo
warn "⚠ Running $TARGET_PEERS peers on a single instance is not recommended."
warn "  Single point of failure, no fault tolerance."
echo

echo "✅ Recommended: Horizontal Scaling"
echo "───────────────────────────────────"
echo "  Instance count:       $INSTANCE_COUNT"
echo "  Peers per instance:   $PEERS_PER_INSTANCE"
echo
echo "  Per-instance resources (with 20% headroom):"
echo "    CPU:                $INSTANCE_CPU_CORES cores"
echo "    Memory:             $INSTANCE_MEMORY_GB GB"
echo

echo "☁️  Cloud Instance Recommendations"
echo "────────────────────────────────────"
echo "  AWS:                  $AWS_INSTANCE"
echo "  GCP:                  $GCP_INSTANCE"
echo "  DigitalOcean:         $DO_INSTANCE"
echo

echo "🐳 Docker Compose Configuration"
echo "────────────────────────────────"
cat <<EOF
services:
  relay-server:
    deploy:
      resources:
        limits:
          cpus: '$INSTANCE_CPU_CORES'
          memory: ${INSTANCE_MEMORY_GB}G
        reservations:
          cpus: '$(echo "$INSTANCE_CPU_CORES * 0.5" | bc)'
          memory: $(echo "$INSTANCE_MEMORY_GB * 0.5" | bc)G
EOF
echo

echo "☸️  Kubernetes Configuration"
echo "────────────────────────────"
cat <<EOF
spec:
  replicas: $INSTANCE_COUNT
  template:
    spec:
      containers:
      - name: relay-server
        resources:
          requests:
            cpu: $(echo "$INSTANCE_CPU_CORES * 1000" | bc)m
            memory: ${INSTANCE_MEMORY_GB}Gi
          limits:
            cpu: $(echo "$INSTANCE_CPU_CORES * 1000" | bc)m
            memory: ${INSTANCE_MEMORY_GB}Gi
        env:
        - name: MAX_PEERS
          value: "$PEERS_PER_INSTANCE"
EOF
echo

echo "📝 Notes"
echo "────────"
echo "  • Memory calculation: ${BASE_MEMORY_MB}MB base + ${MEMORY_PER_PEER_MB}MB per peer"
echo "  • CPU calculation: ${BASE_CPU_MILLICORES}m base + ${CPU_PER_100_PEERS_MILLICORES}m per 100 peers"
echo "  • Network calculation: ${NETWORK_PER_PEER_MBPS} Mbps per peer (forwarding only)"
echo "  • 20% headroom added for bursts and overhead"
echo "  • Horizontal scaling at $PEERS_PER_INSTANCE peers per instance (recommended)"
echo
success "✓ Capacity planning complete!"
echo
