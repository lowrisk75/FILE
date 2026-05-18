#!/bin/bash
#
# FILE Relay Server - Multi-VPS Deployment
# Deploys to 3 geographically distributed VPS for optimal latency
#
# Requirements:
# - SSH access to all 3 VPS as root
# - VPS IPs configured below
# - Debian 12 installed on all VPS
#

set -euo pipefail

# Configuration - UPDATE THESE AFTER VPS CREATION
RELAY_EU_IP=""       # Hetzner Germany (Falkenstein)
RELAY_US_IP=""       # Vultr New York
RELAY_ASIA_IP=""     # Vultr Tokyo

RELAY_PORT=8080

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check configuration
if [[ -z "$RELAY_EU_IP" || -z "$RELAY_US_IP" || -z "$RELAY_ASIA_IP" ]]; then
    echo -e "${RED}❌ ERROR: VPS IPs not configured${NC}"
    echo "Edit this script and set RELAY_EU_IP, RELAY_US_IP, RELAY_ASIA_IP"
    exit 1
fi

echo "🚀 FILE Multi-Relay Deployment"
echo "=============================="
echo "Target VPS:"
echo "  🇪🇺 Europe: $RELAY_EU_IP (Hetzner Germany)"
echo "  🇺🇸 USA:    $RELAY_US_IP (Vultr New York)"
echo "  🇯🇵 Asia:   $RELAY_ASIA_IP (Vultr Tokyo)"
echo ""

# Function to deploy to a single VPS
deploy_single() {
    local VPS_IP=$1
    local VPS_NAME=$2
    local VPS_FLAG=$3

    echo -e "${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${GREEN}$VPS_FLAG Deploying to $VPS_NAME ($VPS_IP)${NC}"
    echo -e "${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""

    # Step 1: Test SSH connectivity
    echo "🔍 Testing SSH connectivity..."
    if ! ssh -o ConnectTimeout=10 -o BatchMode=yes root@"$VPS_IP" exit 2>/dev/null; then
        echo -e "${RED}❌ Cannot connect to $VPS_IP${NC}"
        echo "Make sure:"
        echo "  - VPS is running"
        echo "  - SSH key is added to VPS"
        echo "  - Firewall allows SSH (port 22)"
        return 1
    fi
    echo -e "${GREEN}✅ SSH connectivity OK${NC}"
    echo ""

    # Step 2: Install dependencies
    echo "📦 Installing dependencies..."
    ssh root@"$VPS_IP" << 'EOFDEPS'
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq
apt-get install -y -qq \
    curl \
    build-essential \
    pkg-config \
    libssl-dev \
    git \
    ca-certificates \
    htop \
    fail2ban
EOFDEPS
    echo -e "${GREEN}✅ Dependencies installed${NC}"
    echo ""

    # Step 3: Install Rust
    echo "🦀 Installing Rust..."
    ssh root@"$VPS_IP" << 'EOFRUST'
if ! command -v cargo &>/dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
    source "$HOME/.cargo/env"
    echo 'source "$HOME/.cargo/env"' >> ~/.bashrc
fi
EOFRUST
    echo -e "${GREEN}✅ Rust installed${NC}"
    echo ""

    # Step 4: Copy project source
    echo "📤 Copying project source..."
    cd "$(dirname "$0")/../../.."  # FILE root

    # Create tarball excluding build artifacts
    tar czf /tmp/file-project-$VPS_IP.tar.gz \
        --exclude=target \
        --exclude=.git \
        --exclude='*.tar.gz' \
        Cargo.toml \
        Cargo.lock \
        crates/

    # Upload to VPS
    scp -q /tmp/file-project-$VPS_IP.tar.gz root@"$VPS_IP":/root/file-project.tar.gz
    ssh root@"$VPS_IP" << 'EOFEXTRACT'
cd /root
rm -rf file-project 2>/dev/null || true
mkdir -p file-project
tar xzf file-project.tar.gz -C file-project/
rm file-project.tar.gz
EOFEXTRACT
    rm /tmp/file-project-$VPS_IP.tar.gz
    echo -e "${GREEN}✅ Source copied${NC}"
    echo ""

    # Step 5: Build relay
    echo "🔨 Building relay_server (~5-10 minutes)..."
    ssh root@"$VPS_IP" << 'EOFBUILD'
cd /root/file-project
source "$HOME/.cargo/env"

# Show build progress (filter noise)
cargo build --release -p relay_server 2>&1 | grep -E "(Compiling|Finished|error|warning:)" || true

# Verify binary was built
if [[ ! -f target/release/file-relay ]]; then
    echo "❌ Build failed - binary not found"
    exit 1
fi
EOFBUILD

    if [[ $? -ne 0 ]]; then
        echo -e "${RED}❌ Build failed on $VPS_NAME${NC}"
        return 1
    fi
    echo -e "${GREEN}✅ Build complete${NC}"
    echo ""

    # Step 6: Install binary and systemd service
    echo "⚙️  Installing service..."

    # Upload systemd service file
    cat > /tmp/file-relay-$VPS_IP.service << EOFSVC
[Unit]
Description=FILE Relay Server - Zero Trust P2P MPQUIC Relay ($VPS_NAME)
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=root
WorkingDirectory=/opt/file-relay
ExecStart=/opt/file-relay/file-relay --bind 0.0.0.0:$RELAY_PORT --log-level info
Restart=on-failure
RestartSec=10s
LimitNOFILE=65536

# Security hardening
PrivateTmp=yes
NoNewPrivileges=yes
ProtectSystem=strict
ReadWritePaths=/opt/file-relay

[Install]
WantedBy=multi-user.target
EOFSVC

    scp -q /tmp/file-relay-$VPS_IP.service root@"$VPS_IP":/tmp/file-relay.service

    ssh root@"$VPS_IP" << EOFINSTALL
# Install binary
mkdir -p /opt/file-relay
cp /root/file-project/target/release/file-relay /opt/file-relay/
chmod +x /opt/file-relay/file-relay

# Install systemd service
mv /tmp/file-relay.service /etc/systemd/system/file-relay.service
systemctl daemon-reload
systemctl enable file-relay
EOFINSTALL

    rm /tmp/file-relay-$VPS_IP.service
    echo -e "${GREEN}✅ Service installed${NC}"
    echo ""

    # Step 7: Configure firewall
    echo "🔒 Configuring firewall..."
    ssh root@"$VPS_IP" << EOFFIREWALL
# UFW firewall
if ! command -v ufw &>/dev/null; then
    apt-get install -y -qq ufw
fi

# Default deny incoming
ufw --force reset
ufw default deny incoming
ufw default allow outgoing

# Allow SSH (rate limited)
ufw limit 22/tcp comment 'SSH rate limited'

# Allow FILE relay (UDP + TCP for QUIC)
ufw allow $RELAY_PORT/udp comment 'FILE Relay QUIC'
ufw allow $RELAY_PORT/tcp comment 'FILE Relay QUIC/TCP fallback'

# Enable
ufw --force enable
EOFFIREWALL
    echo -e "${GREEN}✅ Firewall configured${NC}"
    echo ""

    # Step 8: Start service
    echo "🚀 Starting FILE Relay Server..."
    ssh root@"$VPS_IP" "systemctl restart file-relay"
    sleep 3

    # Step 9: Check status
    echo ""
    echo "📊 Service status:"
    ssh root@"$VPS_IP" "systemctl status file-relay --no-pager -l" || true
    echo ""

    # Step 10: Verify port listening
    echo "🔍 Verifying UDP port $RELAY_PORT..."
    if ssh root@"$VPS_IP" "ss -ulnp | grep -q :$RELAY_PORT"; then
        echo -e "${GREEN}✅ Relay listening on UDP $RELAY_PORT${NC}"
    else
        echo -e "${RED}❌ WARNING: UDP port $RELAY_PORT not listening${NC}"
    fi
    echo ""

    echo -e "${GREEN}✅ $VPS_FLAG $VPS_NAME deployment complete!${NC}"
    echo ""
}

# Deploy to all 3 VPS in parallel
echo "Starting parallel deployment to 3 VPS..."
echo ""

deploy_single "$RELAY_EU_IP" "Europe" "🇪🇺" &
PID_EU=$!

deploy_single "$RELAY_US_IP" "USA" "🇺🇸" &
PID_US=$!

deploy_single "$RELAY_ASIA_IP" "Asia" "🇯🇵" &
PID_ASIA=$!

# Wait for all deployments
wait $PID_EU
STATUS_EU=$?

wait $PID_US
STATUS_US=$?

wait $PID_ASIA
STATUS_ASIA=$?

# Summary
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📊 DEPLOYMENT SUMMARY"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

if [[ $STATUS_EU -eq 0 ]]; then
    echo -e "🇪🇺 Europe:  ${GREEN}✅ SUCCESS${NC} - $RELAY_EU_IP:$RELAY_PORT"
else
    echo -e "🇪🇺 Europe:  ${RED}❌ FAILED${NC} - $RELAY_EU_IP"
fi

if [[ $STATUS_US -eq 0 ]]; then
    echo -e "🇺🇸 USA:     ${GREEN}✅ SUCCESS${NC} - $RELAY_US_IP:$RELAY_PORT"
else
    echo -e "🇺🇸 USA:     ${RED}❌ FAILED${NC} - $RELAY_US_IP"
fi

if [[ $STATUS_ASIA -eq 0 ]]; then
    echo -e "🇯🇵 Asia:    ${GREEN}✅ SUCCESS${NC} - $RELAY_ASIA_IP:$RELAY_PORT"
else
    echo -e "🇯🇵 Asia:    ${RED}❌ FAILED${NC} - $RELAY_ASIA_IP"
fi

echo ""

if [[ $STATUS_EU -eq 0 && $STATUS_US -eq 0 && $STATUS_ASIA -eq 0 ]]; then
    echo -e "${GREEN}🎉 All 3 relays deployed successfully!${NC}"
    echo ""
    echo "📋 Next steps:"
    echo "  1. Update AORATA app relay list:"
    echo "     - $RELAY_EU_IP:$RELAY_PORT"
    echo "     - $RELAY_US_IP:$RELAY_PORT"
    echo "     - $RELAY_ASIA_IP:$RELAY_PORT"
    echo ""
    echo "  2. Test P2P connection:"
    echo "     - AORATA on iPhone (5G) ↔ Mac (WiFi)"
    echo ""
    echo "  3. Monitor relay logs:"
    echo "     ssh root@$RELAY_EU_IP 'journalctl -fu file-relay'"
    echo ""
    echo "🔗 FILE relay network operational!"
    exit 0
else
    echo -e "${RED}⚠️  Some deployments failed${NC}"
    echo "Check logs above for errors"
    exit 1
fi
