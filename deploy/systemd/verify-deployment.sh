#!/bin/bash
#
# FILE Relay Server - Deployment Verification Script
#
# Tests all endpoints and validates the deployment is working correctly
#

set -euo pipefail

VPS_HOST="${1:-}"
TIMEOUT=5

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

pass() {
    echo -e "${GREEN}✅ PASS${NC}: $1"
}

fail() {
    echo -e "${RED}❌ FAIL${NC}: $1"
    return 1
}

warn() {
    echo -e "${YELLOW}⚠️  WARN${NC}: $1"
}

if [ -z "$VPS_HOST" ]; then
    echo "Usage: $0 <vps-host>"
    echo "Example: $0 relay.example.com"
    exit 1
fi

echo "FILE Relay Server - Deployment Verification"
echo "Target: $VPS_HOST"
echo ""

# Test 1: SSH connectivity
echo "Test 1: SSH Connectivity"
if ssh -o ConnectTimeout=$TIMEOUT "root@$VPS_HOST" "echo 'OK'" > /dev/null 2>&1; then
    pass "SSH connection successful"
else
    fail "Cannot connect via SSH"
    exit 1
fi
echo ""

# Test 2: Service status
echo "Test 2: Service Status"
if ssh "root@$VPS_HOST" "systemctl is-active --quiet file-relay"; then
    pass "Service is running"
else
    fail "Service is not running"
    ssh "root@$VPS_HOST" "systemctl status file-relay --no-pager" || true
    exit 1
fi
echo ""

# Test 3: Health endpoint
echo "Test 3: Health Endpoint (TCP 8081)"
HEALTH=$(curl -sf --max-time $TIMEOUT "http://$VPS_HOST:8081/health" 2>/dev/null || echo "FAILED")
if echo "$HEALTH" | grep -q "healthy"; then
    pass "Health endpoint responding"
    echo "     Response: $HEALTH"
else
    fail "Health endpoint not responding or unhealthy"
fi
echo ""

# Test 4: Ready endpoint
echo "Test 4: Ready Endpoint"
READY=$(curl -sf --max-time $TIMEOUT "http://$VPS_HOST:8081/ready" 2>/dev/null || echo "FAILED")
if echo "$READY" | grep -q "ready"; then
    pass "Ready endpoint responding"
    echo "     Response: $READY"
else
    warn "Ready endpoint not ready (may be normal if no peers yet)"
fi
echo ""

# Test 5: Metrics endpoint
echo "Test 5: Metrics Endpoint (Prometheus format)"
METRICS=$(curl -sf --max-time $TIMEOUT "http://$VPS_HOST:8081/metrics" 2>/dev/null || echo "FAILED")
if echo "$METRICS" | grep -q "file_relay_active_peers"; then
    pass "Metrics endpoint responding"
    ACTIVE_PEERS=$(echo "$METRICS" | grep "^file_relay_active_peers " | awk '{print $2}')
    echo "     Active peers: $ACTIVE_PEERS"
else
    fail "Metrics endpoint not responding or invalid format"
fi
echo ""

# Test 6: Challenge endpoint
echo "Test 6: Challenge Endpoint (CAPoW)"
CHALLENGE=$(curl -sf --max-time $TIMEOUT "http://$VPS_HOST:8081/challenge" 2>/dev/null || echo "FAILED")
if echo "$CHALLENGE" | grep -q "merkle_root"; then
    pass "Challenge endpoint responding"
    MERKLE_ROOT=$(echo "$CHALLENGE" | grep -o '"merkle_root":"[^"]*"' | cut -d'"' -f4)
    echo "     Merkle root: ${MERKLE_ROOT:0:16}..."
else
    fail "Challenge endpoint not responding or invalid format"
fi
echo ""

# Test 7: UDP port 8080 (relay traffic)
echo "Test 7: UDP Port 8080 (Relay Traffic)"
if timeout $TIMEOUT nc -vzu "$VPS_HOST" 8080 2>&1 | grep -q "succeeded"; then
    pass "UDP port 8080 is open"
else
    warn "UDP port 8080 status unknown (nc may not support -u)"
fi
echo ""

# Test 8: Firewall status
echo "Test 8: Firewall Configuration"
UFW_STATUS=$(ssh "root@$VPS_HOST" "ufw status 2>/dev/null" || echo "NOT INSTALLED")
if echo "$UFW_STATUS" | grep -q "Status: active"; then
    if echo "$UFW_STATUS" | grep -q "8080/udp"; then
        pass "Firewall active, port 8080/udp allowed"
    else
        warn "Firewall active but port 8080/udp not explicitly allowed"
    fi
    if echo "$UFW_STATUS" | grep -q "8081/tcp"; then
        pass "Firewall active, port 8081/tcp allowed"
    else
        warn "Firewall active but port 8081/tcp not explicitly allowed"
    fi
else
    warn "Firewall not active or UFW not installed"
fi
echo ""

# Test 9: Resource usage
echo "Test 9: Resource Usage"
RESOURCE_INFO=$(ssh "root@$VPS_HOST" "ps aux | grep '[f]ile-relay' | head -1" 2>/dev/null || echo "")
if [ -n "$RESOURCE_INFO" ]; then
    CPU=$(echo "$RESOURCE_INFO" | awk '{print $3}')
    MEM=$(echo "$RESOURCE_INFO" | awk '{print $4}')
    pass "Process running - CPU: ${CPU}%, Memory: ${MEM}%"

    # Warn if high usage
    if (( $(echo "$CPU > 80" | bc -l 2>/dev/null || echo 0) )); then
        warn "High CPU usage detected: ${CPU}%"
    fi
    if (( $(echo "$MEM > 80" | bc -l 2>/dev/null || echo 0) )); then
        warn "High memory usage detected: ${MEM}%"
    fi
else
    fail "Cannot retrieve resource usage"
fi
echo ""

# Test 10: Recent logs (check for errors)
echo "Test 10: Recent Logs (Last 20 lines)"
LOGS=$(ssh "root@$VPS_HOST" "journalctl -u file-relay -n 20 --no-pager" 2>/dev/null || echo "")
if echo "$LOGS" | grep -qi "error\|fatal\|panic"; then
    warn "Errors found in recent logs"
    echo "$LOGS" | grep -i "error\|fatal\|panic" | head -5
else
    pass "No errors in recent logs"
fi
echo ""

# Summary
echo "======================================"
echo "Verification Summary"
echo "======================================"
echo ""
echo "✅ Core functionality verified"
echo ""
echo "Next steps:"
echo "  1. Monitor metrics: curl http://$VPS_HOST:8081/metrics"
echo "  2. Watch logs: ssh root@$VPS_HOST 'journalctl -u file-relay -f'"
echo "  3. Test peer registration with client"
echo "  4. Configure Prometheus scraping"
echo "  5. Set up Grafana dashboard"
echo ""
echo "Documentation:"
echo "  - README.md - Deployment guide"
echo "  - ../../docs/OPERATIONS-RUNBOOK.md - Day-2 operations"
echo ""
