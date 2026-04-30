#!/bin/bash
# Build script for Linux platforms

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
BACKEND_DIR="${WORKSPACE_ROOT}/backend"

BUILD_MODE="${1:-debug}"
TARGET="${2:-x86_64-unknown-linux-gnu}"

echo "🐧 Building for Linux: $TARGET ($BUILD_MODE)"

CURRENT_ARCH=$(uname -m)
CURRENT_OS=$(uname -s)

IS_NATIVE=false
if [ "$CURRENT_OS" = "Linux" ]; then
    if [ "$CURRENT_ARCH" = "x86_64" ] && [ "$TARGET" = "x86_64-unknown-linux-gnu" ]; then
        IS_NATIVE=true
    elif [ "$CURRENT_ARCH" = "aarch64" ] && [ "$TARGET" = "aarch64-unknown-linux-gnu" ]; then
        IS_NATIVE=true
    fi
fi

if [ "$IS_NATIVE" = "true" ]; then
    echo "Building natively for current platform"
    BUILD_TOOL=(cargo build)
else
    if command -v cargo-zigbuild >/dev/null 2>&1; then
        BUILD_TOOL=(cargo zigbuild)
        echo "Using cargo-zigbuild for cross-compilation"
    else
        echo "⚠️  cargo-zigbuild not found, falling back to regular cargo"
        BUILD_TOOL=(cargo build)
        rustup target add "$TARGET" || echo "Failed to install target, continuing..."
    fi
fi

BUILD_ARGS=()
if [ "$IS_NATIVE" = "false" ]; then
    BUILD_ARGS+=(--target "$TARGET")
fi
if [ "$BUILD_MODE" = "release" ]; then
    BUILD_ARGS+=(--release)
fi

echo "Running: ${BUILD_TOOL[*]} ${BUILD_ARGS[*]}"
(
    cd "$BACKEND_DIR"
    "${BUILD_TOOL[@]}" "${BUILD_ARGS[@]}"
)

echo "✅ Linux build completed"
