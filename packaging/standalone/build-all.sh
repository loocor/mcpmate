#!/bin/bash
# Build for all supported platforms using platform-specific build scripts

set -euo pipefail

echo "🚀 Building MCPMate for all platforms..."

# Get the script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Platform configurations: platform_name:target:build_script
PLATFORMS=(
  "Linux x64:x86_64-unknown-linux-gnu:build-linux.sh"
  "Linux ARM64:aarch64-unknown-linux-gnu:build-linux.sh"
  "Windows x64:x86_64-pc-windows-gnullvm:build-windows.sh"
  "Windows ARM64:aarch64-pc-windows-gnullvm:build-windows.sh"
  "macOS x64:x86_64-apple-darwin:build-macos.sh"
  "macOS ARM64:aarch64-apple-darwin:build-macos.sh"
)

BUILD_MODE="${1:-release}"
FAILED_BUILDS=()
SUCCESSFUL_BUILDS=()

echo "Build mode: $BUILD_MODE"
echo "Total platforms: ${#PLATFORMS[@]}"
echo ""

for platform_config in "${PLATFORMS[@]}"; do
  IFS=':' read -r platform_name target build_script <<< "$platform_config"

  echo "🔨 Building $platform_name ($target)..."

  BUILD_SCRIPT="$SCRIPT_DIR/$build_script"

  if [ ! -f "$BUILD_SCRIPT" ]; then
    echo "❌ Build script not found: $BUILD_SCRIPT"
    FAILED_BUILDS+=("$platform_name: script not found")
    continue
  fi

  # Make sure the build script is executable
  chmod +x "$BUILD_SCRIPT"

  # Run the build script
  if "$BUILD_SCRIPT" "$BUILD_MODE" "$target"; then
    echo "✅ $platform_name build completed"
    SUCCESSFUL_BUILDS+=("$platform_name")
  else
    echo "❌ $platform_name build failed"
    FAILED_BUILDS+=("$platform_name: build failed")
  fi

  echo ""
done

# Summary
echo "📊 Build Summary:"
echo "Successful builds: ${#SUCCESSFUL_BUILDS[@]}"
for build in "${SUCCESSFUL_BUILDS[@]}"; do
  echo "  ✅ $build"
done

if [ ${#FAILED_BUILDS[@]} -gt 0 ]; then
  echo ""
  echo "Failed builds: ${#FAILED_BUILDS[@]}"
  for build in "${FAILED_BUILDS[@]}"; do
    echo "  ❌ $build"
  done
  echo ""
  echo "⚠️  Some builds failed. Check the output above for details."
  exit 1
else
  echo ""
  echo "🎉 All builds completed successfully!"
fi
