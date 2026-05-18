#!/bin/bash
#
# FILE Relay Server - Deployment Script
# Deploys to Proxmox LXC 180 (10.9.8.180)
#

set -euo pipefail

# Configuration
LXC_ID=180
LXC_IP="10.9.8.180"
PVE_HOST="root@10.9.8.8"
SERVICE_NAME="file-relay"
INSTALL_DIR="/opt/file-relay"

echo "🚀 FILE Relay Server Deployment"
echo "================================"
echo "Target: LXC $LXC_ID ($LXC_IP)"
echo ""

# Step 1: Build for Linux x86_64
echo "📦 Building relay_server for Linux..."
cd "$(dirname "$0")/../.."  # Go to FILE root
cargo build --release --target x86_64-unknown-linux-gnu -p relay_server

BINARY_PATH="target/x86_64-unknown-linux-gnu/release/file-relay"

if [ ! -f "$BINARY_PATH" ]; then
    echo "❌ Binary not found: $BINARY_PATH"
    exit 1
fi

echo "✅ Binary built: $(du -h "$BINARY_PATH" | cut -f1)"
echo ""

# Step 2: Check if LXC exists
echo "🔍 Checking LXC $LXC_ID..."
if ! ssh "$PVE_HOST" "pct status $LXC_ID" &>/dev/null; then
    echo "⚠️  LXC $LXC_ID does not exist. Creating..."

    # Create Debian 12 LXC
    ssh "$PVE_HOST" << 'EOF'
pct create 180 \
  local:vztmpl/debian-12-standard_12.7-1_amd64.tar.zst \
  --hostname file-relay \
  --memory 1024 \
  --cores 2 \
  --net0 name=eth0,bridge=vmbr0,tag=10,ip=10.9.8.180/24,gw=10.9.8.2 \
  --storage local-lvm \
  --rootfs local-lvm:8 \
  --features nesting=1 \
  --unprivileged 1 \
  --start 1
EOF

    echo "✅ LXC created"
    echo "⏳ Waiting for LXC to start..."
    sleep 10
else
    echo "✅ LXC exists"

    # Ensure it's running
    if ! ssh "$PVE_HOST" "pct status $LXC_ID" | grep -q "running"; then
        echo "🔄 Starting LXC..."
        ssh "$PVE_HOST" "pct start $LXC_ID"
        sleep 5
    fi
fi
echo ""

# Step 3: Install dependencies in LXC
echo "📦 Installing dependencies..."
ssh "$PVE_HOST" "pct exec $LXC_ID -- bash" << 'EOF'
apt-get update -qq
apt-get install -y -qq ca-certificates
EOF
echo "✅ Dependencies installed"
echo ""

# Step 4: Create user and directory
echo "👤 Setting up file user..."
ssh "$PVE_HOST" "pct exec $LXC_ID -- bash" << EOF
if ! id -u file &>/dev/null; then
    useradd -r -s /bin/false -d $INSTALL_DIR file
fi
mkdir -p $INSTALL_DIR
chown file:file $INSTALL_DIR
EOF
echo "✅ User created"
echo ""

# Step 5: Copy binary
echo "📤 Copying binary to LXC..."
cat "$BINARY_PATH" | ssh "$PVE_HOST" "cat > /tmp/file-relay.tmp"
ssh "$PVE_HOST" << EOF
pct push $LXC_ID /tmp/file-relay.tmp $INSTALL_DIR/file-relay
pct exec $LXC_ID -- chmod +x $INSTALL_DIR/file-relay
pct exec $LXC_ID -- chown file:file $INSTALL_DIR/file-relay
rm /tmp/file-relay.tmp
EOF
echo "✅ Binary deployed"
echo ""

# Step 6: Install systemd service
echo "⚙️  Installing systemd service..."
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cat "$SCRIPT_DIR/file-relay.service" | ssh "$PVE_HOST" "cat > /tmp/file-relay.service"
ssh "$PVE_HOST" << EOF
pct push $LXC_ID /tmp/file-relay.service /etc/systemd/system/file-relay.service
pct exec $LXC_ID -- systemctl daemon-reload
pct exec $LXC_ID -- systemctl enable file-relay
rm /tmp/file-relay.service
EOF
echo "✅ Service installed"
echo ""

# Step 7: Start service
echo "🚀 Starting FILE Relay Server..."
ssh "$PVE_HOST" "pct exec $LXC_ID -- systemctl restart file-relay"
sleep 2

# Check status
echo "📊 Service status:"
ssh "$PVE_HOST" "pct exec $LXC_ID -- systemctl status file-relay --no-pager" || true
echo ""

echo "✅ Deployment complete!"
echo ""
echo "📋 Next steps:"
echo "  1. Configure OPNsense firewall: allow UDP 8080 to 10.9.8.180"
echo "  2. Update AORATA app to connect to 10.9.8.180:8080"
echo "  3. Monitor logs: ssh $PVE_HOST 'pct exec $LXC_ID -- journalctl -fu file-relay'"
echo ""
echo "🔗 Relay endpoint: $LXC_IP:8080"
