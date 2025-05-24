#!/bin/bash
# Universal build script - detects platform and calls appropriate build script

set -e

BUILD_MODE="${1:-debug}"
TARGET="$2"
PLATFORM="$3"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Platform detection and target mapping
detect_platform_and_target() {
    if [ -n "$PLATFORM" ]; then
        case "$PLATFORM" in
            linux|linux-x64)
                TARGET="x86_64-unknown-linux-gnu"
                PLATFORM_SCRIPT="$SCRIPT_DIR/build-linux.sh"
                ;;
            linux-arm64)
                TARGET="aarch64-unknown-linux-gnu"
                PLATFORM_SCRIPT="$SCRIPT_DIR/build-linux.sh"
                ;;
            windows|win64)
                TARGET="x86_64-pc-windows-gnullvm"
                PLATFORM_SCRIPT="$SCRIPT_DIR/build-windows.sh"
                ;;
            windows-arm64)
                TARGET="aarch64-pc-windows-gnullvm"
                PLATFORM_SCRIPT="$SCRIPT_DIR/build-windows.sh"
                ;;
            macos|macos-x64)
                TARGET="x86_64-apple-darwin"
                PLATFORM_SCRIPT="$SCRIPT_DIR/build-macos.sh"
                ;;
            macos-arm64)
                TARGET="aarch64-apple-darwin"
                PLATFORM_SCRIPT="$SCRIPT_DIR/build-macos.sh"
                ;;
            *)
                echo "❌ Unknown platform: $PLATFORM"
                exit 1
                ;;
        esac
    elif [ -n "$TARGET" ]; then
        case "$TARGET" in
            *linux*)
                PLATFORM_SCRIPT="$SCRIPT_DIR/build-linux.sh"
                ;;
            *windows*)
                PLATFORM_SCRIPT="$SCRIPT_DIR/build-windows.sh"
                ;;
            *apple*)
                PLATFORM_SCRIPT="$SCRIPT_DIR/build-macos.sh"
                ;;
            *)
                echo "❌ Cannot determine platform for target: $TARGET"
                exit 1
                ;;
        esac
    else
        # No target specified, build for current platform
        case "$(uname -s)" in
            Linux)
                TARGET="x86_64-unknown-linux-gnu"
                PLATFORM_SCRIPT="$SCRIPT_DIR/build-linux.sh"
                ;;
            Darwin)
                if [[ "$(uname -m)" == "arm64" ]]; then
                    TARGET="aarch64-apple-darwin"
                else
                    TARGET="x86_64-apple-darwin"
                fi
                PLATFORM_SCRIPT="$SCRIPT_DIR/build-macos.sh"
                ;;
            CYGWIN*|MINGW*|MSYS*)
                TARGET="x86_64-pc-windows-gnullvm"
                PLATFORM_SCRIPT="$SCRIPT_DIR/build-windows.sh"
                ;;
            *)
                echo "❌ Unsupported platform: $(uname -s)"
                exit 1
                ;;
        esac
    fi
}

echo "🔧 Universal Build Script"
echo "Build Mode: $BUILD_MODE"
echo "Target: ${TARGET:-auto-detect}"
echo "Platform: ${PLATFORM:-auto-detect}"
echo ""

detect_platform_and_target

echo "Detected target: $TARGET"
echo "Using build script: $PLATFORM_SCRIPT"
echo ""

# Make sure the platform script is executable
chmod +x "$PLATFORM_SCRIPT"

# Call the appropriate platform build script
"$PLATFORM_SCRIPT" "$BUILD_MODE" "$TARGET"