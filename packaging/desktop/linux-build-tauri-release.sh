#!/usr/bin/env bash
set -euo pipefail

PROFILE="release"
BUILD_BOARD=1
TARGET="x86_64-unknown-linux-gnu"
BUNDLES="appimage,deb"
declare -a EXTRA_ARGS=()
OUTPUT_DIR="${HOME}/Downloads"

usage() {
  cat <<'USAGE'
Usage: linux-build-tauri-release.sh [options]

Options:
  --profile <release|debug>   Cargo profile (default: release)
  --target <triple>           Rust target triple (default: x86_64-unknown-linux-gnu)
  --bundles <list>            Comma-separated bundle types (default: appimage,deb)
  --skip-board                Reuse existing board/dist instead of rebuilding
  --extra "..."               Extra argument forwarded to cargo tauri build (repeatable)
  --output-dir <path>         Directory to collect generated artifacts (default: ~/Downloads)
  -h, --help                  Show this help message

Examples:
  packaging/desktop/linux-build-tauri-release.sh
  packaging/desktop/linux-build-tauri-release.sh --target x86_64-unknown-linux-gnu --bundles appimage,deb
  packaging/desktop/linux-build-tauri-release.sh --profile debug --skip-board

Notes:
  * Requires libssl-dev, libgtk-3-dev, libwebkit2gtk-4.1-dev, and other dependencies.
  * See .github/workflows/desktop-linux.yml for GitHub Actions setup.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --profile)
      PROFILE="$2"
      shift 2
      ;;
    --target)
      TARGET="$2"
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
    --output-dir)
      OUTPUT_DIR="$2"
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

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ROOT_DIR="$(cd "$WORKSPACE_ROOT/desktop" && pwd)"
BACKEND_DIR="$(cd "$WORKSPACE_ROOT/backend" && pwd)"
TAURI_SRC_DIR="$(cd "$ROOT_DIR/src-tauri" && pwd)"
LICENSE_SCRIPT="$WORKSPACE_ROOT/packaging/desktop/generate-open-source-notices.sh"
SIDECAR_OUTPUT_DIR="$BACKEND_DIR/target/sidecars"

log() { echo "[linux-build-tauri-release] $*"; }

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "[linux-build-tauri-release] missing required command: $1" >&2
    exit 1
  fi
}

mkdir -p "$OUTPUT_DIR"
OUTPUT_DIR="$(cd "$OUTPUT_DIR" && pwd)"

: "${BOARD_DIR_OVERRIDE:=}"
if [[ -n "$BOARD_DIR_OVERRIDE" ]]; then
  BOARD_DIR="$BOARD_DIR_OVERRIDE"
else
  BOARD_DIR="$WORKSPACE_ROOT/board"
fi
BOARD_DIR="$(cd "$BOARD_DIR" && pwd)"

if [[ ! -f "$BOARD_DIR/package.json" ]]; then
  echo "[linux-build-tauri-release] unable to locate board/package.json at $BOARD_DIR" >&2
  exit 1
fi

require_cmd cargo
require_cmd bun

if [[ $BUILD_BOARD -eq 1 ]]; then
  log "building board/dist"
  (
    cd "$BOARD_DIR"
    bun install --frozen-lockfile
    bun run build
  )
  
  if [[ -x "$LICENSE_SCRIPT" ]]; then
    log "generating open source notices"
    "$LICENSE_SCRIPT" \
      --board-dir "$BOARD_DIR" \
      --backend-dir "$BACKEND_DIR" \
      --tauri-dir "$TAURI_SRC_DIR" \
      --output "$BOARD_DIR/public/open-source-notices.json"
  else
    echo "[linux-build-tauri-release] missing license generator at $LICENSE_SCRIPT" >&2
    exit 1
  fi
else
  log "skipping board build"
fi

mkdir -p "$SIDECAR_OUTPUT_DIR"

if [[ -d "$SIDECAR_OUTPUT_DIR" ]]; then
  find "$SIDECAR_OUTPUT_DIR" -maxdepth 1 -type f \( -name 'bridge*' -o -name 'mcpmate-core*' \) -delete
fi

build_bridge_sidecar() {
  local target="$1"

  local cargo_profile_flags=""
  if [[ "$PROFILE" == "release" ]]; then
    cargo_profile_flags="--release"
  fi

  log "building bridge sidecar for $target"
  cargo build \
    --manifest-path "$BACKEND_DIR/Cargo.toml" \
    -p mcpmate \
    ${cargo_profile_flags} \
    --bin bridge \
    --target "$target"
  
  local build_subdir="debug"
  if [[ "$PROFILE" == "release" ]]; then
    build_subdir="release"
  fi
  local built_path="$BACKEND_DIR/target/$target/$build_subdir/bridge"
  if [[ ! -f "$built_path" ]]; then
    echo "[linux-build-tauri-release] missing bridge binary at $built_path" >&2
    exit 1
  fi
  cp "$built_path" "$SIDECAR_OUTPUT_DIR/bridge-$target"
  chmod +x "$SIDECAR_OUTPUT_DIR/bridge-$target"
  cp "$built_path" "$SIDECAR_OUTPUT_DIR/bridge"
  chmod +x "$SIDECAR_OUTPUT_DIR/bridge"
}

build_core_sidecar() {
  local target="$1"

  local cargo_profile_flags=""
  if [[ "$PROFILE" == "release" ]]; then
    cargo_profile_flags="--release"
  fi

  log "building core sidecar for $target"
  cargo build \
    --manifest-path "$BACKEND_DIR/Cargo.toml" \
    -p mcpmate \
    ${cargo_profile_flags} \
    --bin mcpmate \
    --target "$target"
  
  local build_subdir="debug"
  if [[ "$PROFILE" == "release" ]]; then
    build_subdir="release"
  fi
  local built_path="$BACKEND_DIR/target/$target/$build_subdir/mcpmate"
  if [[ ! -f "$built_path" ]]; then
    echo "[linux-build-tauri-release] missing core binary at $built_path" >&2
    exit 1
  fi
  cp "$built_path" "$SIDECAR_OUTPUT_DIR/mcpmate-core-$target"
  chmod +x "$SIDECAR_OUTPUT_DIR/mcpmate-core-$target"
  cp "$built_path" "$SIDECAR_OUTPUT_DIR/mcpmate-core"
  chmod +x "$SIDECAR_OUTPUT_DIR/mcpmate-core"
}

log "building target=$TARGET profile=$PROFILE bundles=$BUNDLES"

build_bridge_sidecar "$TARGET"
build_core_sidecar "$TARGET"

cmd=(
  cargo tauri build
  --target "$TARGET"
  --bundles "$BUNDLES"
)

if [[ "$PROFILE" == "debug" ]]; then
  cmd+=(--debug)
elif [[ "$PROFILE" != "release" ]]; then
  echo "[linux-build-tauri-release] unsupported profile: $PROFILE" >&2
  exit 1
fi

if ((${#EXTRA_ARGS[@]})); then
  cmd+=("${EXTRA_ARGS[@]}")
fi

# Allow extra flags via env var (space-separated string)
if [[ -n "${TAURI_BUILD_EXTRA:-}" ]]; then
  # shellcheck disable=SC2206
  extraFromEnv=( ${TAURI_BUILD_EXTRA} )
  cmd+=("${extraFromEnv[@]}")
fi

(
  cd "$ROOT_DIR"
  MCPMATE_SKIP_SIDECAR_BUILD=1 "${cmd[@]}"
)

log "artifact: src-tauri/target/$TARGET/$PROFILE/bundle"

bundle_dir="$ROOT_DIR/src-tauri/target/$TARGET/$PROFILE/bundle"

# Collect artifacts from bundle directory
declare -a artifacts=()
if [[ -d "$bundle_dir/appimage" ]]; then
  while IFS= read -r -d '' file; do
    artifacts+=("$file")
  done < <(find "$bundle_dir/appimage" -maxdepth 1 -type f -name "*.AppImage" -print0 2>/dev/null) || true
fi
if [[ -d "$bundle_dir/deb" ]]; then
  while IFS= read -r -d '' file; do
    artifacts+=("$file")
  done < <(find "$bundle_dir/deb" -maxdepth 1 -type f -name "*.deb" -print0 2>/dev/null) || true
fi

if [[ ${#artifacts[@]} -eq 0 ]]; then
  echo "[linux-build-tauri-release] warning: no artifacts found in $bundle_dir" >&2
  exit 1
fi

# Normalize arch label for release artifact naming
case "$TARGET" in
  aarch64-unknown-linux-gnu) ARCH_LABEL="aarch64" ;;
  x86_64-unknown-linux-gnu)  ARCH_LABEL="x64" ;;
  *)                         ARCH_LABEL="$TARGET" ;;
esac

log "copying ${#artifacts[@]} artifact(s) to $OUTPUT_DIR"
for artifact in "${artifacts[@]}"; do
  base="$(basename "$artifact")"
  lower="$(echo "$base" | tr '[:upper:]' '[:lower:]')"
  case "$lower" in
    *.appimage) out_name="mcpmate_desktop_linux_${ARCH_LABEL}.AppImage" ;;
    *.deb)      out_name="mcpmate_desktop_linux_${ARCH_LABEL}.deb" ;;
    *)          out_name="$base" ;;
  esac
  cp -f "$artifact" "$OUTPUT_DIR/$out_name"
  log "copied $base -> $out_name"
done

log "build completed successfully"
