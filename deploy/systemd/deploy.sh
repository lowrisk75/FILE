#!/bin/bash
#
# FILE Relay Server - VPS Deployment Script
#
# Usage:
#   ./deploy.sh <vps-host> [--update-only]
#
# Example:
#   ./deploy.sh relay-de.example.com
#   ./deploy.sh relay-de.example.com --update-only
#

set -euo pipefail

# Configuration
VPS_HOST="${1:-}"
UPDATE_ONLY="${2:-false}"
RELAY_USER="file-relay"
INSTALL_DIR="/opt/file-relay"
LOG_DIR="/var/log/file-relay"
BINARY_NAME="file-relay"

# GitHub release settings (optional - will download pre-built binaries)
GITHUB_REPO="kevinnadjarian/FILE"
GITHUB_RELEASE="latest"  # or specific tag like "v0.1.0"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

error() {
    echo -e "${RED}ERROR: $1${NC}" >&2
    exit 1
}

info() {
    echo -e "${GREEN}INFO: $1${NC}"
}

warn() {
    echo -e "${YELLOW}WARN: $1${NC}"
}

# Validate arguments
if [ -z "$VPS_HOST" ]; then
    error "Usage: $0 <vps-host> [--update-only]"
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
            chmod +x "/tmp/${binary_file}"
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

info "Deploying FILE Relay Server to $VPS_HOST"

# Test SSH connection
info "Testing SSH connection..."
if ! ssh -o ConnectTimeout=5 "root@$VPS_HOST" "echo 'SSH OK'" > /dev/null 2>&1; then
    error "Cannot connect to root@$VPS_HOST via SSH"
fi

# Detect target architecture
info "Detecting target architecture..."
VPS_ARCH=$(ssh "root@$VPS_HOST" "uname -m")
case "$VPS_ARCH" in
    x86_64)
        TARGET_ARCH="x86_64"
        ;;
    aarch64|arm64)
        TARGET_ARCH="aarch64"
        ;;
    *)
        error "Unsupported architecture: $VPS_ARCH"
        ;;
esac
info "  Target architecture: $TARGET_ARCH"

# Get correct binary for target architecture
BINARY_PATH=$(get_binary "$TARGET_ARCH")
info "  Binary: $BINARY_PATH ($(du -h "$BINARY_PATH" | cut -f1))"

# Initial setup (skip if --update-only)
if [ "$UPDATE_ONLY" != "--update-only" ]; then
    info "Creating user and directories..."
    ssh "root@$VPS_HOST" bash <<'REMOTE_SETUP'
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
REMOTE_SETUP
fi

# Upload binary
info "Uploading binary..."
scp "$BINARY_PATH" "root@$VPS_HOST:$INSTALL_DIR/$BINARY_NAME"

# Set permissions
info "Setting permissions..."
ssh "root@$VPS_HOST" bash <<REMOTE_PERMS
chown file-relay:file-relay $INSTALL_DIR/$BINARY_NAME
chmod 755 $INSTALL_DIR/$BINARY_NAME
REMOTE_PERMS

# Upload systemd unit file
info "Installing systemd service..."
scp "file-relay.service" "root@$VPS_HOST:/etc/systemd/system/"

# Reload systemd and restart service
info "Reloading systemd and restarting service..."
ssh "root@$VPS_HOST" bash <<'REMOTE_SERVICE'
systemctl daemon-reload
systemctl enable file-relay.service
systemctl restart file-relay.service

# Wait for service to start
sleep 2

# Check service status
if systemctl is-active --quiet file-relay.service; then
    echo "✅ Service is running"
else
    echo "❌ Service failed to start"
    systemctl status file-relay.service --no-pager
    exit 1
fi
REMOTE_SERVICE

# Verify endpoints
info "Verifying endpoints..."
sleep 3

# Check health endpoint
if ssh "root@$VPS_HOST" "curl -sf http://localhost:8081/health" > /dev/null 2>&1; then
    info "✅ Health endpoint OK"
else
    warn "⚠️ Health endpoint not responding yet (may need more time)"
fi

# Check challenge endpoint
if ssh "root@$VPS_HOST" "curl -sf http://localhost:8081/challenge" > /dev/null 2>&1; then
    info "✅ Challenge endpoint OK"
else
    warn "⚠️ Challenge endpoint not responding yet"
fi

# Show service status
info "Service status:"
ssh "root@$VPS_HOST" "systemctl status file-relay.service --no-pager | head -20"

info ""
info "🚀 Deployment complete!"
info ""
info "Next steps:"
info "  1. Configure firewall: ufw allow 8080/udp && ufw allow 8081/tcp"
info "  2. Check logs: ssh root@$VPS_HOST 'journalctl -u file-relay -f'"
info "  3. Monitor metrics: curl http://$VPS_HOST:8081/metrics"
info "  4. Get challenge: curl http://$VPS_HOST:8081/challenge"
