#!/bin/bash
# Build script for Linux platforms

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DIST_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
ROOT_DIR="$(cd "${DIST_DIR}/.." && pwd)"
BACKEND_DIR="${ROOT_DIR}/backend"

BUILD_MODE="${1:-debug}"
TARGET="${2:-x86_64-unknown-linux-gnu}"

echo "🐧 Building for Linux: $TARGET ($BUILD_MODE)"

# Detect current platform
CURRENT_ARCH=$(uname -m)
CURRENT_OS=$(uname -s)

# Check if we're building for the current platform
IS_NATIVE=false
if [ "$CURRENT_OS" = "Linux" ]; then
    if [ "$CURRENT_ARCH" = "x86_64" ] && [ "$TARGET" = "x86_64-unknown-linux-gnu" ]; then
        IS_NATIVE=true
    elif [ "$CURRENT_ARCH" = "aarch64" ] && [ "$TARGET" = "aarch64-unknown-linux-gnu" ]; then
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

echo "Running in ${BACKEND_DIR}: $BUILD_CMD"
( cd "${BACKEND_DIR}" && $BUILD_CMD )

echo "✅ Linux build completed"
