#!/usr/bin/env bash
set -euo pipefail

PROFILE="release"
BUILD_BOARD=1
TARGETS="universal-apple-darwin,aarch64-apple-darwin,x86_64-apple-darwin"
BUNDLES="dmg"
declare -a EXTRA_ARGS=()

usage() {
  cat <<'USAGE'
Usage: macos-build-tauri-release.sh [options]

Options:
  --profile <release|debug>   Cargo profile (default: release)
  --targets <list>            Comma-separated Tauri targets
                               (default: universal-apple-darwin,aarch64-apple-darwin,x86_64-apple-darwin)
  --bundles <list>            Bundles passed to cargo tauri build (default: dmg)
  --skip-board                Reuse existing board/dist instead of rebuilding
  --extra "..."               Extra argument forwarded to cargo tauri build (repeatable)
  -h, --help                  Show this help message

Examples:
  script/macos-build-tauri-release.sh
  script/macos-build-tauri-release.sh --targets universal-apple-darwin --bundles dmg
  script/macos-build-tauri-release.sh --profile debug --skip-board

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
      shift 1
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
done

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"
WORKSPACE_DIR="$(cd "$ROOT_DIR/.." && pwd)"
SIDECAR_OUTPUT_DIR="$WORKSPACE_DIR/target/sidecars"
# Skip Finder automation by default to avoid DMG AppleScript prompts on recent macOS.
: "${CI:=true}"
export CI
# Allow overriding the dashboard location while defaulting to the workspace `board/` package.
: "${BOARD_DIR_OVERRIDE:=}"
if [[ -n "$BOARD_DIR_OVERRIDE" ]]; then
  BOARD_DIR="$BOARD_DIR_OVERRIDE"
else
  BOARD_DIR="$(cd "$ROOT_DIR/../.." && pwd)/board"
fi

if [[ ! -f "$BOARD_DIR/package.json" ]]; then
  echo "[macos-build-tauri-release] unable to locate board/package.json at $BOARD_DIR" >&2
  exit 1
fi

if [[ $BUILD_BOARD -eq 1 ]]; then
  echo "[macos-build-tauri-release] building board/dist"
  npm --prefix "$BOARD_DIR" install >/dev/null
  npm --prefix "$BOARD_DIR" run build
else
  echo "[macos-build-tauri-release] skipping board build"
fi

IFS=',' read -r -a TARGET_LIST <<< "$TARGETS"

mkdir -p "$SIDECAR_OUTPUT_DIR"

if [[ -d "$SIDECAR_OUTPUT_DIR" ]]; then
  find "$SIDECAR_OUTPUT_DIR" -maxdepth 1 -type f -name 'bridge*' -delete
fi

build_bridge_sidecar() {
  local target="$1"

  local -a cargo_profile_args=()
  if [[ "$PROFILE" == "release" ]]; then
    cargo_profile_args+=(--release)
  fi

  case "$target" in
    universal-apple-darwin)
      local -a archs=(aarch64-apple-darwin x86_64-apple-darwin)
      local -a arch_paths=()
      for arch in "${archs[@]}"; do
        echo "[macos-build-tauri-release] building bridge sidecar for $arch"
        cargo build \
          --manifest-path "$WORKSPACE_DIR/Cargo.toml" \
          "${cargo_profile_args[@]}" \
          --bin bridge \
          --target "$arch"
        local build_subdir="debug"
        if [[ "$PROFILE" == "release" ]]; then
          build_subdir="release"
        fi
        local built_path="$WORKSPACE_DIR/target/$arch/$build_subdir/bridge"
        if [[ ! -f "$built_path" ]]; then
          echo "[macos-build-tauri-release] missing bridge binary at $built_path" >&2
          exit 1
        fi
        cp "$built_path" "$SIDECAR_OUTPUT_DIR/bridge-$arch"
        chmod +x "$SIDECAR_OUTPUT_DIR/bridge-$arch"
        arch_paths+=("$built_path")
      done
      if ! command -v xcrun >/dev/null 2>&1; then
        echo "[macos-build-tauri-release] xcrun is required to create universal binaries" >&2
        exit 1
      fi
      echo "[macos-build-tauri-release] creating universal bridge sidecar"
      xcrun lipo -create "${arch_paths[@]}" -output "$SIDECAR_OUTPUT_DIR/bridge-universal-apple-darwin"
      chmod +x "$SIDECAR_OUTPUT_DIR/bridge-universal-apple-darwin"
      cp "$SIDECAR_OUTPUT_DIR/bridge-universal-apple-darwin" "$SIDECAR_OUTPUT_DIR/bridge"
      ;;
    *)
      echo "[macos-build-tauri-release] building bridge sidecar for $target"
      cargo build \
        --manifest-path "$WORKSPACE_DIR/Cargo.toml" \
        "${cargo_profile_args[@]}" \
        --bin bridge \
        --target "$target"
      local build_subdir="debug"
      if [[ "$PROFILE" == "release" ]]; then
        build_subdir="release"
      fi
      local built_path="$WORKSPACE_DIR/target/$target/$build_subdir/bridge"
      if [[ ! -f "$built_path" ]]; then
        echo "[macos-build-tauri-release] missing bridge binary at $built_path" >&2
        exit 1
      fi
      cp "$built_path" "$SIDECAR_OUTPUT_DIR/bridge-$target"
      chmod +x "$SIDECAR_OUTPUT_DIR/bridge-$target"
      cp "$built_path" "$SIDECAR_OUTPUT_DIR/bridge"
      chmod +x "$SIDECAR_OUTPUT_DIR/bridge"
      ;;
  esac
}

for TARGET in "${TARGET_LIST[@]}"; do
  echo "[macos-build-tauri-release] building target=$TARGET profile=$PROFILE bundles=$BUNDLES"

  build_bridge_sidecar "$TARGET"

  cmd=(
    cargo tauri build
    --target "$TARGET"
    --bundles "$BUNDLES"
  )

  if [[ "$PROFILE" == "debug" ]]; then
    cmd+=(--debug)
  elif [[ "$PROFILE" != "release" ]]; then
    echo "[macos-build-tauri-release] unsupported profile: $PROFILE" >&2
    exit 1
  fi

  if ((${#EXTRA_ARGS[@]})); then
    cmd+=("${EXTRA_ARGS[@]}")
  fi

  "${cmd[@]}"

  echo "[macos-build-tauri-release] artifact: src-tauri/target/$TARGET/$PROFILE/bundle"

done
