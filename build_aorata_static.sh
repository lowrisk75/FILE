#!/bin/bash
#
# Build AORATA Rust crates as static library for iOS/macOS
#
# Usage:
#   ./build_aorata_static.sh [ios|macos|ios-sim]
#

set -e

PLATFORM="${1:-macos}"
BUILD_DIR="$(pwd)/build"
CRATES_DIR="$(pwd)/crates"

echo "🔨 Building AORATA static library for platform: $PLATFORM"

# Platform-specific targets
case "$PLATFORM" in
  ios)
    TARGETS=(
      "aarch64-apple-ios"
    )
    ;;
  ios-sim)
    TARGETS=(
      "aarch64-apple-ios-sim"
      "x86_64-apple-ios"
    )
    ;;
  macos)
    TARGETS=(
      "aarch64-apple-darwin"
      "x86_64-apple-darwin"
    )
    ;;
  *)
    echo "❌ Unknown platform: $PLATFORM"
    echo "Usage: $0 [ios|macos|ios-sim]"
    exit 1
    ;;
esac

# Install targets if needed
for TARGET in "${TARGETS[@]}"; do
  if ! rustup target list | grep -q "$TARGET (installed)"; then
    echo "📦 Installing target: $TARGET"
    rustup target add "$TARGET"
  fi
done

# Build each target
mkdir -p "$BUILD_DIR"

for TARGET in "${TARGETS[@]}"; do
  echo ""
  echo "🔧 Building for $TARGET..."

  cd "$CRATES_DIR"
  cargo build --release --target "$TARGET" --workspace

  # Copy static libraries (workspace target is at parent level)
  LIB_DIR="$(dirname "$CRATES_DIR")/target/$TARGET/release"
  TARGET_BUILD_DIR="$BUILD_DIR/$TARGET"
  mkdir -p "$TARGET_BUILD_DIR"

  for CRATE in core_transport crypto_fabric vfs_guard edge_ai relay_daemon; do
    LIB_NAME="lib${CRATE}.a"
    if [ -f "$LIB_DIR/$LIB_NAME" ]; then
      cp "$LIB_DIR/$LIB_NAME" "$TARGET_BUILD_DIR/"
      echo "  ✅ $LIB_NAME"
    else
      echo "  ⚠️  $LIB_NAME not found (may need [lib] in Cargo.toml)"
    fi
  done
done

# Create universal binary (macOS/iOS sim only)
if [ "${#TARGETS[@]}" -gt 1 ]; then
  echo ""
  echo "🔗 Creating universal binary..."
  UNIVERSAL_DIR="$BUILD_DIR/universal"
  mkdir -p "$UNIVERSAL_DIR"

  for CRATE in core_transport crypto_fabric vfs_guard edge_ai relay_daemon; do
    LIB_NAME="lib${CRATE}.a"

    # Collect all arch-specific libraries
    LIBS=()
    for TARGET in "${TARGETS[@]}"; do
      LIB_PATH="$BUILD_DIR/$TARGET/$LIB_NAME"
      if [ -f "$LIB_PATH" ]; then
        LIBS+=("$LIB_PATH")
      fi
    done

    # Create universal binary with lipo
    if [ ${#LIBS[@]} -gt 0 ]; then
      lipo -create "${LIBS[@]}" -output "$UNIVERSAL_DIR/$LIB_NAME"
      echo "  ✅ Universal $LIB_NAME"
    fi
  done
fi

echo ""
echo "✅ Build complete!"
echo ""
echo "📦 Static libraries:"
if [ "${#TARGETS[@]}" -gt 1 ]; then
  echo "  $UNIVERSAL_DIR/"
else
  echo "  $BUILD_DIR/${TARGETS[0]}/"
fi
echo ""
echo "Next steps:"
echo "  1. Add libraries to Xcode: Build Phases → Link Binary With Libraries"
echo "  2. Add header search path: Build Settings → Header Search Paths → \$(SRCROOT)/../build/include"
echo "  3. Import in Swift: import AORATACore"
