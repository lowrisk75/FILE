#!/usr/bin/env bash
# health-check.sh - Comprehensive health check for FILE Relay Server
# Usage: ./scripts/ops/health-check.sh [relay-host] [--verbose]

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

error() { echo -e "${RED}✗ $1${NC}"; }
success() { echo -e "${GREEN}✓ $1${NC}"; }
warn() { echo -e "${YELLOW}⚠ $1${NC}"; }
info() { echo -e "${BLUE}ℹ $1${NC}"; }

RELAY_HOST="${1:-localhost}"
VERBOSE=false

if [[ $# -ge 2 && "$2" == "--verbose" ]]; then
    VERBOSE=true
fi

CHECKS_PASSED=0
CHECKS_FAILED=0
CHECKS_WARNED=0

check() {
    local name="$1"
    local command="$2"
    local success_msg="$3"
    local error_msg="$4"

    if [[ "$VERBOSE" == true ]]; then
        info "Running: $command"
    fi

    if eval "$command" >/dev/null 2>&1; then
        success "$success_msg"
        ((CHECKS_PASSED++))
        return 0
    else
        error "$error_msg"
        ((CHECKS_FAILED++))
        return 1
    fi
}

warn_check() {
    local name="$1"
    local command="$2"
    local success_msg="$3"
    local warn_msg="$4"

    if [[ "$VERBOSE" == true ]]; then
        info "Running: $command"
    fi

    if eval "$command" >/dev/null 2>&1; then
        success "$success_msg"
        ((CHECKS_PASSED++))
    else
        warn "$warn_msg"
        ((CHECKS_WARNED++))
    fi
}

echo
echo "═══════════════════════════════════════════════════════"
echo "  FILE Relay Server - Health Check"
echo "  Target: $RELAY_HOST"
echo "═══════════════════════════════════════════════════════"
echo

# Endpoint availability checks
echo "🌐 Endpoint Availability"
echo "─────────────────────────"

check "health_endpoint" \
    "curl -sf http://$RELAY_HOST:8081/health" \
    "Health endpoint responding" \
    "Health endpoint unreachable (http://$RELAY_HOST:8081/health)"

check "ready_endpoint" \
    "curl -sf http://$RELAY_HOST:8081/ready" \
    "Ready endpoint responding" \
    "Ready endpoint failing (capacity or internal error)"

check "metrics_endpoint" \
    "curl -sf http://$RELAY_HOST:8081/metrics" \
    "Metrics endpoint responding" \
    "Metrics endpoint unreachable"

echo

# Metrics validation
echo "📊 Metrics Validation"
echo "─────────────────────"

METRICS=$(curl -sf http://$RELAY_HOST:8081/metrics 2>/dev/null || echo "")

if [[ -z "$METRICS" ]]; then
    error "Cannot fetch metrics (server down or unreachable)"
    ((CHECKS_FAILED++))
else
    # Check for required metrics
    REQUIRED_METRICS=(
        "file_relay_active_peers"
        "file_relay_peers_registered_total"
        "file_relay_capow_verified_total"
        "file_relay_capow_rejected_total"
        "file_relay_rate_limited_total"
        "file_relay_packets_forwarded_total"
        "file_relay_peers_timeout_total"
    )

    for metric in "${REQUIRED_METRICS[@]}"; do
        if echo "$METRICS" | grep -q "^$metric"; then
            success "Metric present: $metric"
            ((CHECKS_PASSED++))
        else
            error "Metric missing: $metric"
            ((CHECKS_FAILED++))
        fi
    done

    # Extract metric values
    ACTIVE_PEERS=$(echo "$METRICS" | grep "^file_relay_active_peers" | awk '{print $2}')
    PEERS_REGISTERED=$(echo "$METRICS" | grep "^file_relay_peers_registered_total" | awk '{print $2}')
    CAPOW_VERIFIED=$(echo "$METRICS" | grep "^file_relay_capow_verified_total" | awk '{print $2}')
    CAPOW_REJECTED=$(echo "$METRICS" | grep "^file_relay_capow_rejected_total" | awk '{print $2}')
    RATE_LIMITED=$(echo "$METRICS" | grep "^file_relay_rate_limited_total" | awk '{print $2}')
    PACKETS_FORWARDED=$(echo "$METRICS" | grep "^file_relay_packets_forwarded_total" | awk '{print $2}')
    PEERS_TIMEOUT=$(echo "$METRICS" | grep "^file_relay_peers_timeout_total" | awk '{print $2}')

    echo
    echo "📈 Current Metrics"
    echo "──────────────────"
    echo "  Active peers:         $ACTIVE_PEERS"
    echo "  Peers registered:     $PEERS_REGISTERED"
    echo "  CAPoW verified:       $CAPOW_VERIFIED"
    echo "  CAPoW rejected:       $CAPOW_REJECTED"
    echo "  Rate limited:         $RATE_LIMITED"
    echo "  Packets forwarded:    $PACKETS_FORWARDED"
    echo "  Peer timeouts:        $PEERS_TIMEOUT"
fi

echo

# Capacity checks
echo "⚡ Capacity Status"
echo "──────────────────"

if [[ -n "$ACTIVE_PEERS" ]]; then
    if [[ $ACTIVE_PEERS -lt 900 ]]; then
        success "Capacity healthy ($ACTIVE_PEERS/1000 peers, ${ACTIVE_PEERS%.*}% utilized)"
        ((CHECKS_PASSED++))
    elif [[ $ACTIVE_PEERS -lt 1000 ]]; then
        warn "Capacity high ($ACTIVE_PEERS/1000 peers, ${ACTIVE_PEERS%.*}% utilized)"
        ((CHECKS_WARNED++))
    else
        error "Capacity reached ($ACTIVE_PEERS/1000 peers, at max capacity)"
        ((CHECKS_FAILED++))
    fi
fi

echo

# Security checks
echo "🔒 Security Health"
echo "──────────────────"

if [[ -n "$CAPOW_REJECTED" && -n "$CAPOW_VERIFIED" ]]; then
    TOTAL_CAPOW=$((CAPOW_VERIFIED + CAPOW_REJECTED))

    if [[ $TOTAL_CAPOW -gt 0 ]]; then
        REJECTION_RATE=$((CAPOW_REJECTED * 100 / TOTAL_CAPOW))

        if [[ $REJECTION_RATE -lt 10 ]]; then
            success "CAPoW rejection rate healthy ($REJECTION_RATE%)"
            ((CHECKS_PASSED++))
        elif [[ $REJECTION_RATE -lt 50 ]]; then
            warn "CAPoW rejection rate elevated ($REJECTION_RATE%)"
            ((CHECKS_WARNED++))
        else
            error "CAPoW rejection rate critical ($REJECTION_RATE%, possible DoS attack)"
            ((CHECKS_FAILED++))
        fi
    else
        info "No CAPoW verifications yet"
    fi
fi

if [[ -n "$RATE_LIMITED" ]]; then
    if [[ $RATE_LIMITED -lt 100 ]]; then
        success "Rate limiting healthy ($RATE_LIMITED total)"
        ((CHECKS_PASSED++))
    elif [[ $RATE_LIMITED -lt 1000 ]]; then
        warn "Rate limiting elevated ($RATE_LIMITED total)"
        ((CHECKS_WARNED++))
    else
        error "Rate limiting critical ($RATE_LIMITED total, possible attack)"
        ((CHECKS_FAILED++))
    fi
fi

echo

# Network checks
echo "🌍 Network Connectivity"
echo "───────────────────────"

warn_check "udp_port" \
    "nc -vzu $RELAY_HOST 8080 2>&1 | grep -q succeeded" \
    "UDP port 8080 reachable" \
    "UDP port 8080 unreachable (firewall or server down)"

check "tcp_metrics_port" \
    "nc -vz $RELAY_HOST 8081 2>&1 | grep -q succeeded" \
    "TCP port 8081 reachable" \
    "TCP port 8081 unreachable"

echo

# Summary
echo "═══════════════════════════════════════════════════════"
echo "  Summary"
echo "═══════════════════════════════════════════════════════"
echo
echo "  ✓ Checks passed:  $CHECKS_PASSED"
echo "  ⚠ Checks warned:  $CHECKS_WARNED"
echo "  ✗ Checks failed:  $CHECKS_FAILED"
echo

if [[ $CHECKS_FAILED -eq 0 ]]; then
    if [[ $CHECKS_WARNED -eq 0 ]]; then
        success "All checks passed! Relay is healthy."
        exit 0
    else
        warn "Some checks raised warnings. Review above."
        exit 0
    fi
else
    error "Health check failed. Review errors above."
    exit 1
fi
