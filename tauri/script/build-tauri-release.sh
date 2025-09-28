#!/usr/bin/env bash
set -euo pipefail

PROFILE="release"
BUILD_BOARD=1
TARGETS="universal-apple-darwin"
BUNDLES="dmg"
EXTRA_ARGS=()

usage() {
  cat <<'USAGE'
Usage: build-tauri-release.sh [options]

Options:
  --profile <release|debug>   Cargo profile (default: release)
  --targets <list>            Comma-separated Tauri targets (default: universal-apple-darwin)
  --bundles <list>            Bundles passed to cargo tauri build (default: dmg)
  --skip-board                Reuse existing board/dist instead of rebuilding
  --extra "..."               Extra argument forwarded to cargo tauri build (repeatable)
  -h, --help                  Show this help message

Examples:
  script/build-tauri-release.sh
  script/build-tauri-release.sh --targets universal-apple-darwin,x86_64-pc-windows-msvc --bundles dmg,msi
  script/build-tauri-release.sh --profile debug --skip-board

Notes:
  * Windows targets must be built on Windows with the MSVC toolchain available.
  * Set CI=true in the environment on macOS 26+ to skip Finder AppleScript during DMG creation.
USAGE
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
      usage >&2
      exit 1
      ;;
  esac
  shift
done

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"
BOARD_DIR="$ROOT_DIR/../board"

if [[ $BUILD_BOARD -eq 1 ]]; then
  echo "[build-tauri-release] building board/dist"
  npm --prefix "$BOARD_DIR" install >/dev/null
  npm --prefix "$BOARD_DIR" run build
else
  echo "[build-tauri-release] skipping board build"
fi

IFS=',' read -r -a TARGET_LIST <<< "$TARGETS"

for TARGET in "${TARGET_LIST[@]}"; do
  echo "[build-tauri-release] building target=$TARGET profile=$PROFILE bundles=$BUNDLES"
  cargo tauri build \
    --$PROFILE \
    --target "$TARGET" \
    --bundles "$BUNDLES" \
    "${EXTRA_ARGS[@]}"
  echo "[build-tauri-release] artifact: src-tauri/target/$TARGET/$PROFILE/bundle"

done

