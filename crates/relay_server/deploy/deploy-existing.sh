#!/bin/bash
#
# FILE Relay Server - Deploy to existing LXC
# Uses LXC 100 (stopped honeypot container)
#

set -euo pipefail

# Configuration
LXC_ID=100
PVE_HOST="root@10.9.8.8"

echo "🚀 FILE Relay Server Deployment"
echo "==============================="
echo "Target: LXC $LXC_ID (repurposing existing container)"
echo ""

# Step 1: Ensure LXC is running
echo "🔍 Checking LXC $LXC_ID..."
if ! ssh "$PVE_HOST" "pct status $LXC_ID" | grep -q "running"; then
    echo "🔄 Starting LXC..."
    ssh "$PVE_HOST" "pct start $LXC_ID"
    sleep 5
fi
echo "✅ LXC running"
echo ""

# Step 2: Get LXC IP
echo "📡 Getting LXC IP..."
LXC_IP=$(ssh "$PVE_HOST" "pct exec $LXC_ID -- ip -4 addr show eth0" | grep "inet " | awk '{print $2}' | cut -d'/' -f1 | head -1)
echo "✅ LXC IP: $LXC_IP"
echo ""

# Step 3: Install build dependencies
echo "📦 Installing dependencies..."
ssh "$PVE_HOST" "pct exec $LXC_ID -- bash" << 'EOFDEPS'
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq
apt-get install -y -qq \
    curl \
    build-essential \
    pkg-config \
    libssl-dev \
    git \
    ca-certificates
EOFDEPS
echo "✅ Dependencies installed"
echo ""

# Step 4: Install Rust
echo "🦀 Installing Rust..."
ssh "$PVE_HOST" "pct exec $LXC_ID -- bash" << 'EOFRUST'
if ! command -v cargo &>/dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
fi
EOFRUST
echo "✅ Rust installed"
echo ""

# Step 5: Copy project source
echo "📤 Copying project source..."
cd "$(dirname "$0")/../../.."  # FILE root
tar czf /tmp/file-project.tar.gz \
    --exclude=target \
    --exclude=.git \
    Cargo.toml \
    crates/

scp -q /tmp/file-project.tar.gz "$PVE_HOST:/tmp/"
ssh "$PVE_HOST" << EOFCOPY
pct push $LXC_ID /tmp/file-project.tar.gz /root/file-project.tar.gz
pct exec $LXC_ID -- tar xzf /root/file-project.tar.gz -C /root/
rm /tmp/file-project.tar.gz
EOFCOPY
rm /tmp/file-project.tar.gz
echo "✅ Source copied"
echo ""

# Step 6: Build relay
echo "🔨 Building relay_server (~5 min)..."
ssh "$PVE_HOST" "pct exec $LXC_ID -- bash" << 'EOFBUILD'
cd /root
source "$HOME/.cargo/env"
cargo build --release -p relay_server 2>&1 | grep -E "(Compiling|Finished|error)" || true
EOFBUILD
echo "✅ Build complete"
echo ""

# Step 7: Install binary and service
echo "⚙️  Installing service..."
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# Update service file with correct port
cat > /tmp/file-relay.service << 'EOFSVC'
[Unit]
Description=FILE Relay Server - Zero Trust P2P MPQUIC Relay
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=root
WorkingDirectory=/opt/file-relay
ExecStart=/opt/file-relay/file-relay --bind 0.0.0.0:8080 --log-level info
Restart=on-failure
RestartSec=10s
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
EOFSVC

scp -q /tmp/file-relay.service "$PVE_HOST:/tmp/"

ssh "$PVE_HOST" << EOFINSTALL
# Install binary
pct exec $LXC_ID -- mkdir -p /opt/file-relay
pct exec $LXC_ID -- cp /root/target/release/file-relay /opt/file-relay/
pct exec $LXC_ID -- chmod +x /opt/file-relay/file-relay

# Install service
pct push $LXC_ID /tmp/file-relay.service /etc/systemd/system/file-relay.service
pct exec $LXC_ID -- systemctl daemon-reload
pct exec $LXC_ID -- systemctl enable file-relay
rm /tmp/file-relay.service
EOFINSTALL
rm /tmp/file-relay.service
echo "✅ Service installed"
echo ""

# Step 8: Start service
echo "🚀 Starting FILE Relay Server..."
ssh "$PVE_HOST" "pct exec $LXC_ID -- systemctl restart file-relay"
sleep 3

# Check status
echo ""
echo "📊 Service status:"
ssh "$PVE_HOST" "pct exec $LXC_ID -- systemctl status file-relay --no-pager -l" || true
echo ""

# Show relay info
echo "✅ Deployment complete!"
echo ""
echo "🔗 FILE Relay Server:"
echo "   Internal: $LXC_IP:8080"
echo "   LXC ID: $LXC_ID"
echo ""
echo "📋 Next steps:"
echo "  1. Update AORATA: PacketTunnelProvider relay → '$LXC_IP:8080'"
echo "  2. OPNsense: Port Forward UDP 8080 → $LXC_IP:8080"
echo "  3. Monitor: ssh $PVE_HOST 'pct exec $LXC_ID -- journalctl -fu file-relay'"
