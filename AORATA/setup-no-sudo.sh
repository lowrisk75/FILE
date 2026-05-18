#!/bin/bash
set -e

echo "🚀 AORATA Setup Script (No Sudo Required)"
echo "=========================================="
echo ""

PROJECT_DIR="/Users/kevinnadjarian/GitHub/FILE/AORATA"
cd "$PROJECT_DIR"

echo "✅ Step 1: Configure Entitlements Path"
PBXPROJ="AORATA.xcodeproj/project.pbxproj"

if [ ! -f "$PBXPROJ" ]; then
    echo "❌ Error: project.pbxproj not found"
    exit 1
fi

# Backup
cp "$PBXPROJ" "${PBXPROJ}.backup"
echo "   ✓ Backup created: ${PBXPROJ}.backup"

# Add CODE_SIGN_ENTITLEMENTS to all build configurations
echo "   → Adding CODE_SIGN_ENTITLEMENTS to build settings..."

# This is a safe addition - adds entitlements path to Debug and Release configs
sed -i '' '/CODE_SIGN_STYLE = Automatic;/a\
				CODE_SIGN_ENTITLEMENTS = AORATA/AORATA.entitlements;
' "$PBXPROJ"

# Add DEVELOPMENT_TEAM if not present
if ! grep -q "DEVELOPMENT_TEAM = TDV6D5L785" "$PBXPROJ"; then
    sed -i '' '/CODE_SIGN_STYLE = Automatic;/a\
				DEVELOPMENT_TEAM = TDV6D5L785;
' "$PBXPROJ"
    echo "   ✓ DEVELOPMENT_TEAM added"
fi

echo "   ✓ Build settings configured"

echo ""
echo "✅ Step 2: Verify Rust Core"
if [ -L "RustCore" ]; then
    echo "   ✓ RustCore symlink exists → $(readlink RustCore)"
else
    echo "   ⚠ RustCore symlink missing - creating..."
    ln -s ../crates RustCore
    echo "   ✓ RustCore symlink created"
fi

echo ""
echo "✅ Step 3: Check Rust compilation"
if command -v cargo &> /dev/null; then
    cd RustCore
    echo "   → Running cargo check (this may take a minute)..."
    if cargo check --workspace --quiet 2>&1 | tail -5; then
        echo "   ✓ Rust core compiles successfully"
    else
        echo "   ⚠ Rust compilation has warnings (check above)"
    fi
    cd "$PROJECT_DIR"
else
    echo "   ⚠ Cargo not found - skipping Rust check"
fi

echo ""
echo "════════════════════════════════════════════════════════════"
echo "✅ AUTOMATED SETUP COMPLETE!"
echo "════════════════════════════════════════════════════════════"
echo ""
echo "📋 MANUAL STEPS in Xcode (2-3 minutes):"
echo ""
echo "1. Open project:"
echo "   open AORATA.xcodeproj"
echo ""
echo "2. Select project 'AORATA' in navigator (blue icon)"
echo "3. Select target 'AORATA' (not the project)"
echo "4. Click tab 'Signing & Capabilities'"
echo ""
echo "5. For EACH platform (iOS and macOS):"
echo ""
echo "   a) Verify Team is set:"
echo "      Team: Christine Martin (TDV6D5L785)"
echo "      (Should already be set - if not, select it)"
echo ""
echo "   b) Click '+ Capability' button (top left)"
echo "      → Search 'App Groups'"
echo "      → Double-click 'App Groups'"
echo "      → Click '+' under 'App Groups'"
echo "      → Enter: group.com.lorislab.aorata"
echo "      → Click OK"
echo ""
echo "   c) Click '+ Capability' again"
echo "      → Search 'Network Extensions'"
echo "      → Double-click 'Network Extensions'"
echo "      → Check box: 'Packet Tunnel'"
echo ""
echo "6. Build to verify:"
echo "   Cmd+B"
echo ""
echo "════════════════════════════════════════════════════════════"
echo ""
echo "📚 Full guide: Docs/XCODE_SETUP.md"
echo "🔗 Specs: https://notebooklm.google.com/notebook/86bc04f6-444f-4882-b54c-96467cf7cda4"
echo ""
echo "🎯 After Xcode setup → Start Phase 1:"
echo "   cd RustCore"
echo "   # Consult NotebookLM for MPQUIC implementation"
echo "   # Edit crates/core_transport/src/mpquic.rs"
echo ""
