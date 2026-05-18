#!/bin/bash
#
# FILE Relay Server - GitHub Repository Setup
#
# Creates GitHub repo and pushes code to trigger first multi-arch build
#

set -euo pipefail

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info() { echo -e "${GREEN}INFO: $1${NC}"; }
warn() { echo -e "${YELLOW}WARN: $1${NC}"; }

info "Setting up GitHub repository for FILE Relay Server"
echo ""

# Check if gh CLI is available
if ! command -v gh >/dev/null 2>&1; then
    warn "GitHub CLI (gh) not found. Install with: brew install gh"
    echo ""
    echo "Manual setup:"
    echo "  1. Go to https://github.com/new"
    echo "  2. Create repository: kevinnadjarian/FILE"
    echo "  3. Run: git remote add origin git@github.com:kevinnadjarian/FILE.git"
    echo "  4. Run: git push -u origin main"
    exit 1
fi

# Check if already has remote
if git remote | grep -q origin; then
    info "Remote 'origin' already exists:"
    git remote -v | grep origin
    echo ""
    read -p "Push to existing remote? (y/n) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        info "Pushing to existing remote..."
        git push -u origin main
        info "✅ Pushed to GitHub! Check Actions at:"
        echo "   https://github.com/kevinnadjarian/FILE/actions"
    fi
    exit 0
fi

# Create GitHub repo
info "Creating GitHub repository kevinnadjarian/FILE..."
gh repo create kevinnadjarian/FILE \
    --public \
    --description "FILE - Zero Trust P2P Network with MPQUIC relay and CAPoW anti-DoS" \
    --source . \
    --remote origin \
    --push

info "✅ Repository created and code pushed!"
echo ""
info "Next steps:"
echo "  1. Check GitHub Actions: https://github.com/kevinnadjarian/FILE/actions"
echo "  2. Wait ~5-10 min for x86_64 + aarch64 builds"
echo "  3. Download artifacts or deploy directly"
echo ""
info "Deploy to Proxmox (auto-downloads from GitHub):"
echo "  cd deploy/proxmox"
echo "  ./deploy-lxc.sh 199"
echo ""
info "Deploy to VPS (auto-downloads from GitHub):"
echo "  cd deploy/systemd"
echo "  ./deploy.sh relay-fsn1.hetzner.net"
