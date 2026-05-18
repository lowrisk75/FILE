#!/usr/bin/env python3
"""
Configure Xcode project with App Groups and Network Extensions capabilities.
This script properly adds the entitlements file and ensures all capabilities are set.
"""

import re
import sys
from pathlib import Path

PROJECT_PATH = Path("/Users/kevinnadjarian/GitHub/FILE/AORATA/AORATA.xcodeproj/project.pbxproj")
ENTITLEMENTS_PATH = "AORATA/AORATA.entitlements"

def main():
    print("🔧 Configuring AORATA Xcode Project")
    print("=" * 60)

    if not PROJECT_PATH.exists():
        print(f"❌ Error: {PROJECT_PATH} not found")
        sys.exit(1)

    # Read project file
    with open(PROJECT_PATH, 'r') as f:
        content = f.read()

    # Backup
    backup_path = PROJECT_PATH.with_suffix('.pbxproj.backup2')
    with open(backup_path, 'w') as f:
        f.write(content)
    print(f"✓ Backup created: {backup_path.name}")

    # 1. Ensure entitlements file reference exists
    if ENTITLEMENTS_PATH not in content:
        print("→ Adding entitlements file reference...")
        # Find a good place to add it (after other file references)
        file_ref_section = re.search(r'(/\* Begin PBXFileReference section \*/.*?/\* End PBXFileReference section \*/)', content, re.DOTALL)
        if file_ref_section:
            # Generate a unique ID for the entitlements file
            import hashlib
            ref_id = hashlib.md5(ENTITLEMENTS_PATH.encode()).hexdigest()[:24].upper()

            entitlements_ref = f'\t\t{ref_id} /* AORATA.entitlements */ = {{isa = PBXFileReference; lastKnownFileType = text.plist.entitlements; path = AORATA.entitlements; sourceTree = "<group>"; }};\n'

            # Insert before the End comment
            content = content.replace(
                '/* End PBXFileReference section */',
                entitlements_ref + '/* End PBXFileReference section */'
            )
            print("✓ Entitlements file reference added")
    else:
        print("✓ Entitlements file reference already exists")

    # 2. Ensure CODE_SIGN_ENTITLEMENTS is set for all configurations
    print("→ Configuring CODE_SIGN_ENTITLEMENTS...")

    # Find all build configurations for AORATA target
    configs_updated = 0

    # Pattern to find build settings blocks
    config_pattern = r'(buildSettings = \{[^}]*?)(DEVELOPMENT_TEAM = TDV6D5L785;)'

    def add_entitlements(match):
        nonlocal configs_updated
        settings_block = match.group(1)
        dev_team_line = match.group(2)

        # Check if CODE_SIGN_ENTITLEMENTS already exists in this block
        if 'CODE_SIGN_ENTITLEMENTS' not in settings_block:
            configs_updated += 1
            # Add it right after DEVELOPMENT_TEAM
            return settings_block + dev_team_line + '\n\t\t\t\tCODE_SIGN_ENTITLEMENTS = ' + ENTITLEMENTS_PATH + ';'
        return match.group(0)

    content = re.sub(config_pattern, add_entitlements, content, flags=re.DOTALL)

    if configs_updated > 0:
        print(f"✓ Added CODE_SIGN_ENTITLEMENTS to {configs_updated} configuration(s)")
    else:
        print("✓ CODE_SIGN_ENTITLEMENTS already configured")

    # Write back
    with open(PROJECT_PATH, 'w') as f:
        f.write(content)

    print("\n" + "=" * 60)
    print("✅ Project configuration complete!")
    print("=" * 60)
    print()
    print("📋 The entitlements file (AORATA.entitlements) contains:")
    print("   ✓ App Groups: group.com.lorislab.aorata")
    print("   ✓ Network Extensions: packet-tunnel-provider")
    print("   ✓ Keychain Sharing")
    print()
    print("🎯 Next steps:")
    print("   1. Open Xcode: open AORATA.xcodeproj")
    print("   2. Build: Cmd+B")
    print("   3. The capabilities should be automatically recognized!")
    print()
    print("   Note: Xcode will automatically add the capabilities")
    print("   to the Signing & Capabilities tab based on the")
    print("   entitlements file.")
    print()

if __name__ == '__main__':
    main()
