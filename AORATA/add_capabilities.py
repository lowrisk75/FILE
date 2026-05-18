#!/usr/bin/env python3
"""
Add App Groups and Network Extensions capabilities to Xcode project.
This modifies the TargetAttributes section to include SystemCapabilities.
"""

import re
import sys
from pathlib import Path

PROJECT_PATH = Path("/Users/kevinnadjarian/GitHub/FILE/AORATA/AORATA.xcodeproj/project.pbxproj")

def main():
    print("🔧 Adding Capabilities to AORATA Target")
    print("=" * 60)

    if not PROJECT_PATH.exists():
        print(f"❌ Error: {PROJECT_PATH} not found")
        sys.exit(1)

    # Read project file
    with open(PROJECT_PATH, 'r') as f:
        content = f.read()

    # Backup
    backup_path = PROJECT_PATH.with_suffix('.pbxproj.backup3')
    with open(backup_path, 'w') as f:
        f.write(content)
    print(f"✓ Backup created: {backup_path.name}")

    # Find the target ID (5B38587F2FB736F100552C77)
    target_pattern = r'(5B38587F2FB736F100552C77 = \{[^}]*CreatedOnToolsVersion = 26\.4;)'

    # Check if SystemCapabilities already exists
    if 'SystemCapabilities' in content and 'com.apple.ApplicationGroups.iOS' in content:
        print("✓ Capabilities already configured")
        return

    # Add SystemCapabilities right after CreatedOnToolsVersion
    capabilities_block = '''
						SystemCapabilities = {
							com.apple.ApplicationGroups.iOS = {
								enabled = 1;
							};
							com.apple.NetworkExtensions.iOS = {
								enabled = 1;
							};
							com.apple.ApplicationGroups.Mac = {
								enabled = 1;
							};
							com.apple.NetworkExtensions.Mac = {
								enabled = 1;
							};
						};'''

    def add_capabilities(match):
        original = match.group(1)
        return original + capabilities_block

    content = re.sub(target_pattern, add_capabilities, content)

    # Write back
    with open(PROJECT_PATH, 'w') as f:
        f.write(content)

    print("✓ App Groups capability added (iOS + macOS)")
    print("✓ Network Extensions capability added (iOS + macOS)")
    print()
    print("=" * 60)
    print("✅ Capabilities configuration complete!")
    print("=" * 60)
    print()
    print("🎯 Next: Build the project")
    print("   xcodebuild -project AORATA.xcodeproj -scheme AORATA")
    print()

if __name__ == '__main__':
    main()
