#!/bin/bash
#
# FILE Relay Server - Simple Deployment
# Creates LXC, installs Rust, builds and runs relay on target
#

set -euo pipefail

# Configuration
LXC_ID=181
LXC_IP="10.9.8.181"
PVE_HOST="root@10.9.8.8"

echo "🚀 FILE Relay Server Deployment (build-on-target)"
echo "=================================================="
echo "Target: LXC $LXC_ID ($LXC_IP)"
echo ""

# Step 1: Create LXC if it doesn't exist
echo "🔍 Checking LXC $LXC_ID..."
if ! ssh "$PVE_HOST" "pct status $LXC_ID" &>/dev/null; then
    echo "⚠️  Creating LXC $LXC_ID..."

    ssh "$PVE_HOST" << 'EOFPVE'
pct create 181 \
  local:vztmpl/debian-12-standard_12.12-1_amd64.tar.zst \
  --hostname file-relay \
  --memory 2048 \
  --cores 2 \
  --net0 name=eth0,bridge=vmbr0,tag=10,ip=10.9.8.181/24,gw=10.9.8.2 \
  --rootfs rpool-ct:16 \
  --features nesting=1 \
  --onboot 1 \
  --start 1
EOFPVE

    echo "✅ LXC created"
    echo "⏳ Waiting 15s for network..."
    sleep 15
else
    echo "✅ LXC exists"
    if ! ssh "$PVE_HOST" "pct status $LXC_ID" | grep -q "running"; then
        ssh "$PVE_HOST" "pct start $LXC_ID"
        sleep 5
    fi
fi
echo ""

# Step 2: Install build dependencies
echo "📦 Installing build tools..."
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

# Step 3: Install Rust
echo "🦀 Installing Rust..."
ssh "$PVE_HOST" "pct exec $LXC_ID -- bash" << 'EOFRUST'
if ! command -v cargo &>/dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
    source "$HOME/.cargo/env"
fi
EOFRUST
echo "✅ Rust installed"
echo ""

# Step 4: Copy project source
echo "📤 Copying project source..."
cd "$(dirname "$0")/../../.."  # FILE root
tar czf /tmp/file-project.tar.gz \
    --exclude=target \
    --exclude=.git \
    Cargo.toml \
    crates/

scp /tmp/file-project.tar.gz "$PVE_HOST:/tmp/"
ssh "$PVE_HOST" << 'EOFCOPY'
pct push 180 /tmp/file-project.tar.gz /root/file-project.tar.gz
pct exec 180 -- tar xzf /root/file-project.tar.gz -C /root/
rm /tmp/file-project.tar.gz
EOFCOPY
rm /tmp/file-project.tar.gz
echo "✅ Source copied"
echo ""

# Step 5: Build relay
echo "🔨 Building relay_server (this takes ~5 minutes)..."
ssh "$PVE_HOST" "pct exec $LXC_ID -- bash" << 'EOFBUILD'
cd /root
source "$HOME/.cargo/env"
cargo build --release -p relay_server
EOFBUILD
echo "✅ Binary built"
echo ""

# Step 6: Install binary and systemd service
echo "⚙️  Installing service..."
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cat "$SCRIPT_DIR/file-relay.service" | ssh "$PVE_HOST" "cat > /tmp/file-relay.service"

ssh "$PVE_HOST" << 'EOFINSTALL'
# Create user
pct exec 180 -- bash -c "useradd -r -s /bin/false -d /opt/file-relay file 2>/dev/null || true"

# Install binary
pct exec 180 -- mkdir -p /opt/file-relay
pct exec 180 -- cp /root/target/release/file-relay /opt/file-relay/
pct exec 180 -- chown -R file:file /opt/file-relay
pct exec 180 -- chmod +x /opt/file-relay/file-relay

# Install service
pct push 180 /tmp/file-relay.service /etc/systemd/system/file-relay.service
pct exec 180 -- systemctl daemon-reload
pct exec 180 -- systemctl enable file-relay
rm /tmp/file-relay.service
EOFINSTALL
echo "✅ Service installed"
echo ""

# Step 7: Start service
echo "🚀 Starting FILE Relay Server..."
ssh "$PVE_HOST" "pct exec $LXC_ID -- systemctl restart file-relay"
sleep 3

# Check status
echo ""
echo "📊 Service status:"
ssh "$PVE_HOST" "pct exec $LXC_ID -- systemctl status file-relay --no-pager -l" || true
echo ""

echo "✅ Deployment complete!"
echo ""
echo "📋 Next steps:"
echo "  1. Configure OPNsense: Firewall → NAT → Port Forward → UDP 8080 → 10.9.8.180:8080"
echo "  2. Update AORATA app: PacketTunnelProvider relay address → '10.9.8.180:8080'"
echo "  3. Monitor: ssh $PVE_HOST 'pct exec $LXC_ID -- journalctl -fu file-relay'"
echo ""
echo "🔗 Relay endpoint: $LXC_IP:8080 (internal), <public-IP>:8080 (external)"
