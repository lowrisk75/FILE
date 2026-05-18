#!/bin/bash
#
# FILE Relay Server - Proxmox LXC Deployment
#
# Deploys FILE Relay Server to a Proxmox LXC container for testing
#
# Usage:
#   ./deploy-lxc.sh <lxc-id> [lxc-name]
#
# Example:
#   ./deploy-lxc.sh 199 file-relay-test
#

set -euo pipefail

# Configuration
LXC_ID="${1:-}"
LXC_NAME="${2:-file-relay-test}"
PVE_HOST="100.123.83.107"
PVE_USER="root"

RELAY_USER="file-relay"
INSTALL_DIR="/opt/file-relay"
BINARY_NAME="file-relay"

# GitHub release settings (optional - will download pre-built binaries)
GITHUB_REPO="kevinnadjarian/FILE"
GITHUB_RELEASE="latest"  # or specific tag like "v0.1.0"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

error() { echo -e "${RED}ERROR: $1${NC}" >&2; exit 1; }
info() { echo -e "${GREEN}INFO: $1${NC}"; }
warn() { echo -e "${YELLOW}WARN: $1${NC}"; }

# Validate arguments
if [ -z "$LXC_ID" ]; then
    error "Usage: $0 <lxc-id> [lxc-name]"
fi

# Function to get binary for target architecture
get_binary() {
    local target_arch="$1"
    local binary_file="file-relay-linux-${target_arch}"
    local local_binary="../../target/${target_arch}-unknown-linux-gnu/release/file-relay"
    local download_url=""

    # Try GitHub releases first
    if [ "$GITHUB_RELEASE" = "latest" ]; then
        download_url="https://github.com/${GITHUB_REPO}/releases/latest/download/${binary_file}"
    else
        download_url="https://github.com/${GITHUB_REPO}/releases/download/${GITHUB_RELEASE}/${binary_file}"
    fi

    info "Target architecture: ${target_arch}"

    # Try to download from GitHub
    if command -v curl >/dev/null 2>&1; then
        info "Attempting to download pre-built binary from GitHub..."
        if curl -fsSL "$download_url" -o "/tmp/${binary_file}" 2>/dev/null; then
            info "✅ Downloaded pre-built binary from GitHub"
            echo "/tmp/${binary_file}"
            return 0
        else
            warn "GitHub download failed, trying local build..."
        fi
    fi

    # Fall back to local binary
    if [ -f "$local_binary" ]; then
        info "✅ Using local build: $local_binary"
        echo "$local_binary"
        return 0
    fi

    # Fall back to native target if cross-compiled not available
    local native_binary="../../target/release/file-relay"
    if [ -f "$native_binary" ]; then
        warn "⚠️ Using native build (may not match target architecture!)"
        warn "   Build for ${target_arch} with: cargo build --release --target ${target_arch}-unknown-linux-gnu"
        echo "$native_binary"
        return 0
    fi

    error "No binary found! Options:
  1. Push code and let GitHub Actions build it
  2. Build locally: cargo build --release --target ${target_arch}-unknown-linux-gnu
  3. Build native: cargo build --release"
}

info "Deploying FILE Relay Server to Proxmox LXC"
info "  LXC ID: $LXC_ID"
info "  LXC Name: $LXC_NAME"
info "  PVE Host: $PVE_HOST"

# Test SSH to Proxmox
info "Testing SSH connection to Proxmox..."
if ! ssh -o ConnectTimeout=5 "$PVE_USER@$PVE_HOST" "echo 'SSH OK'" > /dev/null 2>&1; then
    error "Cannot connect to $PVE_USER@$PVE_HOST via SSH"
fi

# Check if LXC exists
info "Checking LXC status..."
LXC_STATUS=$(ssh "$PVE_USER@$PVE_HOST" "pct status $LXC_ID 2>/dev/null" || echo "NOT_FOUND")

if echo "$LXC_STATUS" | grep -q "NOT_FOUND\|does not exist"; then
    warn "LXC $LXC_ID does not exist. Creating..."

    # Create LXC container
    ssh "$PVE_USER@$PVE_HOST" bash <<CREATE_LXC
pct create $LXC_ID local:vztmpl/ubuntu-22.04-standard_22.04-1_amd64.tar.zst \
    --hostname $LXC_NAME \
    --cores 2 \
    --memory 2048 \
    --swap 512 \
    --net0 name=eth0,bridge=vmbr2,ip=dhcp \
    --storage rpool-ct \
    --rootfs rpool-ct:8 \
    --unprivileged 1 \
    --features nesting=1 \
    --onboot 1 \
    --start 1

# Wait for container to start
sleep 5

# Wait for network
for i in {1..30}; do
    if pct exec $LXC_ID -- ping -c 1 1.1.1.1 > /dev/null 2>&1; then
        echo "Network ready"
        break
    fi
    sleep 1
done

# Update system
pct exec $LXC_ID -- apt-get update
pct exec $LXC_ID -- apt-get upgrade -y
pct exec $LXC_ID -- apt-get install -y curl wget net-tools

echo "LXC created successfully"
CREATE_LXC

elif echo "$LXC_STATUS" | grep -q "stopped"; then
    info "Starting LXC $LXC_ID..."
    ssh "$PVE_USER@$PVE_HOST" "pct start $LXC_ID"
    sleep 5
else
    info "LXC $LXC_ID is already running"
fi

# Get LXC IP address and architecture
info "Getting LXC IP address..."
LXC_IP=$(ssh "$PVE_USER@$PVE_HOST" "pct exec $LXC_ID -- hostname -I | awk '{print \$1}'")
info "  LXC IP: $LXC_IP"

# Detect target architecture
info "Detecting target architecture..."
LXC_ARCH=$(ssh "$PVE_USER@$PVE_HOST" "pct exec $LXC_ID -- uname -m")
case "$LXC_ARCH" in
    x86_64)
        TARGET_ARCH="x86_64"
        ;;
    aarch64|arm64)
        TARGET_ARCH="aarch64"
        ;;
    *)
        error "Unsupported architecture: $LXC_ARCH"
        ;;
esac
info "  Target architecture: $TARGET_ARCH"

# Get correct binary for target architecture
BINARY_PATH=$(get_binary "$TARGET_ARCH")
info "  Binary: $BINARY_PATH"

# Create user and directories
info "Creating user and directories in LXC..."
ssh "$PVE_USER@$PVE_HOST" pct exec $LXC_ID -- bash <<'LXC_SETUP'
# Create user if not exists
if ! id file-relay > /dev/null 2>&1; then
    useradd --system --shell /bin/bash --home-dir /opt/file-relay file-relay
    echo "Created user: file-relay"
fi

# Create directories
mkdir -p /opt/file-relay
mkdir -p /var/log/file-relay
chown file-relay:file-relay /opt/file-relay
chown file-relay:file-relay /var/log/file-relay

echo "Directories created"
LXC_SETUP

# Upload binary via Proxmox host
info "Uploading binary..."
scp "$BINARY_PATH" "$PVE_USER@$PVE_HOST:/tmp/$BINARY_NAME"
ssh "$PVE_USER@$PVE_HOST" "pct push $LXC_ID /tmp/$BINARY_NAME $INSTALL_DIR/$BINARY_NAME"
ssh "$PVE_USER@$PVE_HOST" "rm /tmp/$BINARY_NAME"

# Set permissions
info "Setting permissions..."
ssh "$PVE_USER@$PVE_HOST" pct exec $LXC_ID -- bash <<PERMS
chown file-relay:file-relay $INSTALL_DIR/$BINARY_NAME
chmod 755 $INSTALL_DIR/$BINARY_NAME
PERMS

# Upload and install systemd service
info "Installing systemd service..."
scp "../systemd/file-relay.service" "$PVE_USER@$PVE_HOST:/tmp/file-relay.service"
ssh "$PVE_USER@$PVE_HOST" "pct push $LXC_ID /tmp/file-relay.service /etc/systemd/system/file-relay.service"
ssh "$PVE_USER@$PVE_HOST" "rm /tmp/file-relay.service"

# Enable and start service
info "Starting service..."
ssh "$PVE_USER@$PVE_HOST" pct exec $LXC_ID -- bash <<'START_SERVICE'
systemctl daemon-reload
systemctl enable file-relay.service
systemctl restart file-relay.service

# Wait for service
sleep 2

# Check status
if systemctl is-active --quiet file-relay.service; then
    echo "✅ Service is running"
else
    echo "❌ Service failed to start"
    systemctl status file-relay.service --no-pager
    exit 1
fi
START_SERVICE

# Verify endpoints
info "Verifying endpoints..."
sleep 3

if curl -sf "http://$LXC_IP:8081/health" > /dev/null 2>&1; then
    info "✅ Health endpoint OK"
else
    warn "⚠️ Health endpoint not responding yet"
fi

if curl -sf "http://$LXC_IP:8081/challenge" > /dev/null 2>&1; then
    info "✅ Challenge endpoint OK"
else
    warn "⚠️ Challenge endpoint not responding yet"
fi

# Show service status
info "Service status:"
ssh "$PVE_USER@$PVE_HOST" pct exec $LXC_ID -- systemctl status file-relay --no-pager | head -20

info ""
info "🚀 Deployment complete!"
info ""
info "LXC Details:"
info "  ID: $LXC_ID"
info "  Name: $LXC_NAME"
info "  IP: $LXC_IP"
info ""
info "Access:"
info "  SSH: ssh root@$LXC_IP (or pct enter $LXC_ID on PVE)"
info "  Health: curl http://$LXC_IP:8081/health"
info "  Challenge: curl http://$LXC_IP:8081/challenge"
info "  Metrics: curl http://$LXC_IP:8081/metrics"
info ""
info "Logs:"
info "  pct exec $LXC_ID -- journalctl -u file-relay -f"
info ""
info "Next steps:"
info "  1. Test with a client: Connect to $LXC_IP:8080 (UDP)"
info "  2. Monitor metrics: watch curl http://$LXC_IP:8081/metrics"
info "  3. If OK, deploy to Hetzner Strasbourg (FSN1)"
