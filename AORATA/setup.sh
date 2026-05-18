#!/bin/bash
set -e

echo "🚀 AORATA Setup Script"
echo "======================"
echo ""

PROJECT_DIR="/Users/kevinnadjarian/GitHub/FILE/AORATA"
cd "$PROJECT_DIR"

echo "✅ Step 1: Verify project structure"
if [ ! -d "AORATA.xcodeproj" ]; then
    echo "❌ Error: AORATA.xcodeproj not found"
    exit 1
fi
echo "   ✓ Xcode project found"

echo ""
echo "✅ Step 2: Install xcodeproj gem (if needed)"
if ! command -v xcodeproj &> /dev/null; then
    echo "   Installing xcodeproj gem..."
    sudo gem install xcodeproj --quiet
else
    echo "   ✓ xcodeproj already installed"
fi

echo ""
echo "✅ Step 3: Configure project via Ruby script"

# Create Ruby script to modify Xcode project
cat > /tmp/aorata_setup.rb << 'RUBY_SCRIPT'
require 'xcodeproj'

project_path = '/Users/kevinnadjarian/GitHub/FILE/AORATA/AORATA.xcodeproj'
project = Xcodeproj::Project.open(project_path)

# Find main target
target = project.targets.find { |t| t.name == 'AORATA' }

unless target
  puts "❌ Error: AORATA target not found"
  exit 1
end

puts "   ✓ Found target: #{target.name}"

# 1. Add entitlements file reference if not already added
entitlements_path = 'AORATA/AORATA.entitlements'
entitlements_ref = project.files.find { |f| f.path == entitlements_path }

unless entitlements_ref
  puts "   → Adding entitlements file to project..."
  group = project.main_group.find_subpath('AORATA', true)
  file_ref = group.new_reference(entitlements_path)
  puts "   ✓ Entitlements file added"
else
  puts "   ✓ Entitlements file already in project"
end

# 2. Set Code Signing Entitlements in build settings
puts "   → Configuring entitlements in build settings..."
target.build_configurations.each do |config|
  config.build_settings['CODE_SIGN_ENTITLEMENTS'] = 'AORATA/AORATA.entitlements'
  config.build_settings['DEVELOPMENT_TEAM'] = 'TDV6D5L785'
  config.build_settings['CODE_SIGN_STYLE'] = 'Automatic'
  config.build_settings['PRODUCT_BUNDLE_IDENTIFIER'] = 'com.lorislab.aorata'
end
puts "   ✓ Build settings configured"

# 3. Save project
project.save
puts "   ✓ Project saved successfully"

puts ""
puts "✅ Xcode project configuration complete!"
RUBY_SCRIPT

# Run the Ruby script
ruby /tmp/aorata_setup.rb

echo ""
echo "✅ Step 4: Configure Info.plist files"

# iOS Info.plist
if [ -f "AORATA/Info.plist" ]; then
    echo "   → Configuring iOS Info.plist..."
    /usr/libexec/PlistBuddy -c "Set :CFBundleName Aorata" "AORATA/Info.plist" 2>/dev/null || \
    /usr/libexec/PlistBuddy -c "Add :CFBundleName string Aorata" "AORATA/Info.plist"

    /usr/libexec/PlistBuddy -c "Set :CFBundleDisplayName Aorata" "AORATA/Info.plist" 2>/dev/null || \
    /usr/libexec/PlistBuddy -c "Add :CFBundleDisplayName string Aorata" "AORATA/Info.plist"

    echo "   ✓ Info.plist configured"
else
    echo "   ⚠ Info.plist not found (may be generated at build time)"
fi

echo ""
echo "✅ Step 5: Build Rust core (basic check)"
if [ -d "RustCore" ]; then
    echo "   → Checking Rust toolchain..."
    if command -v cargo &> /dev/null; then
        cd RustCore
        echo "   → Running cargo check..."
        cargo check --workspace --quiet 2>&1 | grep -i "error" && echo "   ⚠ Rust errors detected" || echo "   ✓ Rust core compiles"
        cd ..
    else
        echo "   ⚠ Rust not installed - skipping Rust build"
    fi
else
    echo "   ⚠ RustCore symlink not found"
fi

echo ""
echo "✅ Step 6: Create Xcode schemes"
# Schemes are usually auto-created, just verify they exist
if [ -d "AORATA.xcodeproj/xcshareddata/xcschemes" ]; then
    echo "   ✓ Schemes directory exists"
else
    mkdir -p "AORATA.xcodeproj/xcshareddata/xcschemes"
    echo "   ✓ Schemes directory created"
fi

echo ""
echo "════════════════════════════════════════════════════════════"
echo "✅ AORATA SETUP COMPLETE!"
echo "════════════════════════════════════════════════════════════"
echo ""
echo "📋 Manual Steps Remaining in Xcode:"
echo ""
echo "1. Open Xcode:"
echo "   open AORATA.xcodeproj"
echo ""
echo "2. Select target AORATA → Signing & Capabilities tab"
echo ""
echo "3. Add capabilities (click '+ Capability'):"
echo "   ✓ App Groups → Add: group.com.lorislab.aorata"
echo "   ✓ Network Extensions → Check: Packet Tunnel"
echo ""
echo "4. Verify Signing:"
echo "   Team: Christine Martin (TDV6D5L785) ← Should be set"
echo "   Signing: Automatic ← Should be set"
echo ""
echo "5. Build & Run:"
echo "   Cmd+B (Build)"
echo "   Cmd+U (Test)"
echo "   Cmd+R (Run)"
echo ""
echo "📚 Documentation: Docs/XCODE_SETUP.md"
echo "🔗 NotebookLM: https://notebooklm.google.com/notebook/86bc04f6-444f-4882-b54c-96467cf7cda4"
echo ""
echo "🎯 Next: Implement Phase 1 (core_transport - MPQUIC)"
echo "   See: ../TODO.md"
echo ""
