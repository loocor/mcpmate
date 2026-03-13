#!/bin/bash
# Build script for macOS platforms

set -e

BUILD_MODE="${1:-debug}"
TARGET="${2:-aarch64-apple-darwin}"

echo "🍎 Building for macOS: $TARGET ($BUILD_MODE)"

# Detect current platform
CURRENT_ARCH=$(uname -m)
CURRENT_OS=$(uname -s)

# Check if we're building for the current platform
IS_NATIVE=false
if [ "$CURRENT_OS" = "Darwin" ]; then
    if [ "$CURRENT_ARCH" = "arm64" ] && [ "$TARGET" = "aarch64-apple-darwin" ]; then
        IS_NATIVE=true
    elif [ "$CURRENT_ARCH" = "x86_64" ] && [ "$TARGET" = "x86_64-apple-darwin" ]; then
        IS_NATIVE=true
    fi
fi

# Choose build command based on whether it's native or cross-compilation
if [ "$IS_NATIVE" = "true" ]; then
    echo "Building natively for current platform"
    BUILD_CMD="cargo build"
else
    # Check if zigbuild is available for cross-compilation
    if command -v cargo-zigbuild &> /dev/null; then
        BUILD_CMD="cargo zigbuild"
        echo "Using cargo-zigbuild for cross-compilation"
    else
        echo "⚠️  cargo-zigbuild not found, falling back to regular cargo"
        BUILD_CMD="cargo build"

        # Install target for regular cargo
        rustup target add "$TARGET" || echo "Failed to install target, continuing..."
    fi
fi

if [ "$BUILD_MODE" = "release" ]; then
    BUILD_CMD="$BUILD_CMD --release"
fi

# Only add target flag if not building natively
if [ "$IS_NATIVE" = "false" ]; then
    BUILD_CMD="$BUILD_CMD --target $TARGET"
fi

echo "Running: $BUILD_CMD"
$BUILD_CMD

echo "✅ macOS build completed"