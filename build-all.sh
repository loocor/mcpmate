#!/bin/bash
set -e

# Build for all targets in release mode
TARGETS=(
  "x86_64-unknown-linux-gnu"
  "aarch64-unknown-linux-gnu"
  "x86_64-pc-windows-gnullvm"
  "aarch64-pc-windows-gnullvm"
  "x86_64-apple-darwin"
  "aarch64-apple-darwin"
)

for target in "${TARGETS[@]}"; do
  echo "Building for $target..."
  cargo zigbuild --release --target "$target"
done

# Optionally build macOS universal binary
echo "Building macOS universal binary..."
cargo zigbuild --release --target universal2-apple-darwin
