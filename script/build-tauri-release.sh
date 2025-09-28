#!/usr/bin/env bash
set -euo pipefail

# build-tauri-release.sh
#
# Helper wrapper around the release process discussed in docs/desktop-release-guide.md.
# Usage examples:
#   script/build-tauri-release.sh                  # Release profile, macOS universal dmg
#   script/build-tauri-release.sh --profile debug  # Debug profile
#   script/build-tauri-release.sh --targets universal-apple-darwin,x86_64-pc-windows-msvc
#   script/build-tauri-release.sh --skip-board     # Reuse existing board/dist

PROFILE="release"
BUILD_BOARD=1
TARGETS="universal-apple-darwin"
BUNDLES="dmg"
EXTRA_ARGS=()

usage() {
  cat <<'EOF'
build-tauri-release.sh [options]

Options:
  --profile <release|debug>   Cargo profile (default: release)
  --targets <list>            Comma-separated tauri targets (default: universal-apple-darwin)
  --bundles <list>            Bundles passed to cargo tauri (default: dmg)
  --skip-board                Skip rebuilding board/dist
  --extra "..."               Extra arguments passed verbatim to cargo tauri build
  -h, --help                  Show this message

Examples:
  # macOS Universal release DMG
  script/build-tauri-release.sh

  # Release macOS DMG + Windows x64 MSI
  script/build-tauri-release.sh \
    --targets universal-apple-darwin,x86_64-pc-windows-msvc \
    --bundles dmg,msi

  # Debug build for quick smoke test
  script/build-tauri-release.sh --profile debug --skip-board

Notes:
  * Windows targets require running on Windows with the MSVC toolchain.
  * Set CI=true in the environment to skip Finder AppleScript on macOS 26.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --profile)
      PROFILE="$2"
      shift 2
      ;;
    --targets)
      TARGETS="$2"
      shift 2
      ;;
    --bundles)
      BUNDLES="$2"
      shift 2
      ;;
    --skip-board)
      BUILD_BOARD=0
      shift
      ;;
    --extra)
      EXTRA_ARGS+=("$2")
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage
      exit 1
      ;;
  esac
done

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"
BOARD_DIR="$ROOT_DIR/../board"

if [[ $BUILD_BOARD -eq 1 ]]; then
  echo "[build-tauri-release] Building board/dist..."
  npm --prefix "$BOARD_DIR" install >/dev/null
  npm --prefix "$BOARD_DIR" run build
else
  echo "[build-tauri-release] Skipping board build (using existing dist)."
fi

IFS=',' read -r -a TARGET_LIST <<< "$TARGETS"

for TARGET in "${TARGET_LIST[@]}"; do
  echo "[build-tauri-release] Building Tauri target: $TARGET (profile=$PROFILE bundles=$BUNDLES)"
  cargo tauri build \
    --$PROFILE \
    --target "$TARGET" \
    --bundles "$BUNDLES" \
    "${EXTRA_ARGS[@]}"
done

echo "[build-tauri-release] Done. Artifacts are under backend/tauri/src-tauri/target/<target>/<profile>/bundle/"

