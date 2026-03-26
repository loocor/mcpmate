#!/usr/bin/env bash
set -euo pipefail

PROFILE="release"
BUILD_BOARD=1
TARGETS="aarch64-apple-darwin,x86_64-apple-darwin"
BUNDLES="dmg"
declare -a EXTRA_ARGS=()
OUTPUT_DIR="${HOME}/Downloads"
# Track copied DMG artifacts for post-build processing
declare -a COPIED_DMGS=()
# Signing / notarization controls (can also be provided via environment)
SKIP_NOTARIZE=0
SIGN_IDENTITY=""
APPLE_ID_OPT=""
APPLE_PASSWORD_OPT=""
APPLE_TEAM_ID_OPT=""
APPLE_API_KEY_OPT=""
APPLE_API_ISSUER_OPT=""
APPLE_API_KEY_PATH_OPT=""

# Diagnostics default (compile-time cfg). When enabled, desktop shell auto-enables
# market diagnostics and forwards front-end logs without user interaction.
DIAG_DEFAULT=0

# Load .env files from desktop/tauri so users don't need to pass flags each time.
load_env_files() {
  local script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  local tauri_dir="$(cd "${script_dir}/.." && pwd)"
  local env_main="${tauri_dir}/.env"
  local env_local="${tauri_dir}/.env.local"
  set +u
  if [[ -f "$env_main" ]]; then
    set -a; # export all sourced variables
    # shellcheck source=/dev/null
    source "$env_main"
    set +a
  fi
  if [[ -f "$env_local" ]]; then
    set -a; # export all sourced variables
    # shellcheck source=/dev/null
    source "$env_local"
    set +a
  fi
  set -u
}

usage() {
  cat <<'USAGE'
Usage: macos-build-tauri-release.sh [options]

Options:
  --profile <release|debug>   Cargo profile (default: release)
  --targets <list>            Comma-separated Tauri targets
                               (default: aarch64-apple-darwin,x86_64-apple-darwin)
  --bundles <list>            Bundles passed to cargo tauri build (default: dmg)
  --skip-board                Reuse existing board/dist instead of rebuilding
  --extra "..."               Extra argument forwarded to cargo tauri build (repeatable)
  --output-dir <path>         Directory to collect generated DMG files (default: ~/Downloads)
  --diag-default              Build with market diagnostics enabled by default
  --sign-identity <string>    macOS codesign identity (overrides APPLE_SIGNING_IDENTITY)
  --apple-id <email>          Apple ID used for notarization (sets APPLE_ID)
  --apple-password <pass>     App-specific password or keychain profile (sets APPLE_PASSWORD)
  --apple-team-id <TEAMID>    Apple Developer Team ID (sets APPLE_TEAM_ID)
  --apple-api-key <KEYID>     App Store Connect API key ID (sets APPLE_API_KEY)
  --apple-api-issuer <UUID>   App Store Connect API issuer ID (sets APPLE_API_ISSUER)
  --apple-api-key-path <p>    Path to .p8 API key file (sets APPLE_API_KEY_PATH)
  --skip-notarize             Do not attempt notarization even if credentials are present
                              (alias: --skip-notariz)
  -h, --help                  Show this help message

Examples:
  script/macos-build-tauri-release.sh
  script/macos-build-tauri-release.sh --targets aarch64-apple-darwin,x86_64-apple-darwin --bundles dmg
  script/macos-build-tauri-release.sh --profile debug --skip-board

Notes:
  * Windows targets must be built on Windows with the MSVC toolchain available.
  * Set CI=true in the environment on macOS 26+ to skip Finder AppleScript during DMG creation.
  * Signing/notarization logs hide team id, API issuer, and identity strings by default; set
    MCPMATE_BUILD_LOG_SIGNING_DETAILS=1 for the previous verbose output (avoid in public CI).
USAGE
}

# Preload environment defaults from .env files (if present)
load_env_files

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
    --output-dir)
      OUTPUT_DIR="$2"
      shift 2
      ;;
    --diag-default)
      DIAG_DEFAULT=1
      shift 1
      ;;
    --sign-identity)
      SIGN_IDENTITY="$2"
      shift 2
      ;;
    --apple-id)
      APPLE_ID_OPT="$2"
      shift 2
      ;;
    --apple-password)
      APPLE_PASSWORD_OPT="$2"
      shift 2
      ;;
    --apple-team-id)
      APPLE_TEAM_ID_OPT="$2"
      shift 2
      ;;
    --apple-api-key)
      APPLE_API_KEY_OPT="$2"
      shift 2
      ;;
    --apple-api-issuer)
      APPLE_API_ISSUER_OPT="$2"
      shift 2
      ;;
    --apple-api-key-path)
      APPLE_API_KEY_PATH_OPT="$2"
      shift 2
      ;;
    --skip-notarize|--skip-notariz)
      SKIP_NOTARIZE=1
      shift 1
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
WORKSPACE_ROOT="$(cd "$ROOT_DIR/../.." && pwd)"
BACKEND_DIR="$(cd "$WORKSPACE_ROOT/backend" && pwd)"
TAURI_SRC_DIR="$(cd "$ROOT_DIR/src-tauri" && pwd)"
LICENSE_SCRIPT="$ROOT_DIR/script/generate-open-source-notices.sh"
SIDECAR_OUTPUT_DIR="$BACKEND_DIR/target/sidecars"
# Skip Finder automation by default to avoid DMG AppleScript prompts on recent macOS.
: "${CI:=true}"
export CI

log() { echo "[macos-build-tauri-release] $*"; }

# When set to 1/true, print signing identity strings, Team ID, and API issuer (verbose; avoid in shared CI logs).
signing_details_enabled() {
  local v="${MCPMATE_BUILD_LOG_SIGNING_DETAILS:-}"
  [[ "$v" == "1" || "$v" == "true" || "$v" == "TRUE" ]]
}

# Prepare codesign identity and notarization environment if provided.
preflight_signing() {
  if [[ "$(uname -s)" != "Darwin" ]]; then
    return 0
  fi

  # If an identity is provided via flag, prefer it.
  if [[ -n "$SIGN_IDENTITY" ]]; then
    export APPLE_SIGNING_IDENTITY="$SIGN_IDENTITY"
  fi

  # Merge Apple ID credentials if provided via flags.
  if [[ -n "$APPLE_ID_OPT" ]]; then export APPLE_ID="$APPLE_ID_OPT"; fi
  if [[ -n "$APPLE_PASSWORD_OPT" ]]; then export APPLE_PASSWORD="$APPLE_PASSWORD_OPT"; fi
  if [[ -n "$APPLE_TEAM_ID_OPT" ]]; then export APPLE_TEAM_ID="$APPLE_TEAM_ID_OPT"; fi

  # Merge App Store Connect API key credentials if provided via flags.
  if [[ -n "$APPLE_API_KEY_OPT" ]]; then export APPLE_API_KEY="$APPLE_API_KEY_OPT"; fi
  if [[ -n "$APPLE_API_ISSUER_OPT" ]]; then export APPLE_API_ISSUER="$APPLE_API_ISSUER_OPT"; fi
  if [[ -n "$APPLE_API_KEY_PATH_OPT" ]]; then export APPLE_API_KEY_PATH="$APPLE_API_KEY_PATH_OPT"; fi

  # Optionally disable notarization even if credentials exist.
  if [[ $SKIP_NOTARIZE -eq 1 ]]; then
    # Ensure bundler doesn't pick up credentials from current environment.
    unset APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID APPLE_API_KEY APPLE_API_ISSUER APPLE_API_KEY_PATH
    log "notarization disabled by --skip-notarize"
  fi

  # Human-friendly summary (default: no org/team/identity strings in logs).
  if command -v security >/dev/null 2>&1; then
    if signing_details_enabled; then
      local id_summary
      id_summary=$(security find-identity -v -p codesigning 2>/dev/null | sed -n '1,5p' || true)
      log "available code signing identities (first few):\n${id_summary:-<none>}"
    else
      local id_count
      id_count=$(security find-identity -v -p codesigning 2>/dev/null | grep -E -c '^[[:space:]]+[0-9]+\)' || true)
      log "code signing: ${id_count:-0} matching keychain identity line(s) (set MCPMATE_BUILD_LOG_SIGNING_DETAILS=1 to list names)"
    fi
  fi

  if [[ -n "${APPLE_SIGNING_IDENTITY:-}" ]]; then
    if signing_details_enabled; then
      log "using codesign identity: ${APPLE_SIGNING_IDENTITY}"
    else
      log "code signing: APPLE_SIGNING_IDENTITY is set (value hidden; MCPMATE_BUILD_LOG_SIGNING_DETAILS=1 to print)"
    fi
  else
    log "no APPLE_SIGNING_IDENTITY provided; Tauri will choose a default or ad-hoc sign (development)."
  fi

  if [[ -n "${APPLE_API_KEY:-}" && -n "${APPLE_API_ISSUER:-}" && -n "${APPLE_API_KEY_PATH:-}" ]]; then
    # Prefer API key credentials; clear Apple ID creds to avoid ambiguity.
    if [[ -n "${APPLE_ID:-}" || -n "${APPLE_PASSWORD:-}" || -n "${APPLE_TEAM_ID:-}" ]]; then
      unset APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID
    fi
    if signing_details_enabled; then
      log "notarization via API key enabled (issuer=${APPLE_API_ISSUER})"
    else
      log "notarization via App Store Connect API key (issuer hidden)"
    fi
  elif [[ -n "${APPLE_ID:-}" && -n "${APPLE_PASSWORD:-}" && -n "${APPLE_TEAM_ID:-}" ]]; then
    if signing_details_enabled; then
      log "notarization via Apple ID enabled (team=${APPLE_TEAM_ID})"
    else
      log "notarization via Apple ID + app-specific password (team id hidden)"
    fi
  else
    log "no notarization credentials detected; artifact will NOT be notarized."
  fi
}

# Notarize a DMG using either Apple ID creds or App Store Connect API key, then staple.
notarize_and_staple_dmg() {
  local dmg_path="$1"
  if [[ ! -f "$dmg_path" ]]; then
    echo "[macos-build-tauri-release] notarize: missing dmg: $dmg_path" >&2
    return 2
  fi

  if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "[macos-build-tauri-release] notarize: skip (not macOS)"
    return 0
  fi

  if [[ $SKIP_NOTARIZE -eq 1 ]]; then
    echo "[macos-build-tauri-release] notarize: skip (--skip-notarize)"
    return 0
  fi

  if ! command -v xcrun >/dev/null 2>&1; then
    echo "[macos-build-tauri-release] notarize: xcrun not found; cannot notarize" >&2
    return 3
  fi

  echo "[macos-build-tauri-release] notarize: submitting DMG to Apple notary"

  # Prefer API key credentials if available; otherwise use Apple ID credentials.
  if [[ -n "${APPLE_API_KEY:-}" && -n "${APPLE_API_ISSUER:-}" && -n "${APPLE_API_KEY_PATH:-}" ]]; then
    if ! xcrun notarytool submit "$dmg_path" \
      --key "$APPLE_API_KEY_PATH" \
      --key-id "$APPLE_API_KEY" \
      --issuer "$APPLE_API_ISSUER" \
      --wait; then
      echo "[macos-build-tauri-release] error: DMG notarization failed (API key)" >&2
      return 65
    fi
  elif [[ -n "${APPLE_ID:-}" && -n "${APPLE_PASSWORD:-}" && -n "${APPLE_TEAM_ID:-}" ]]; then
    if ! xcrun notarytool submit "$dmg_path" \
      --apple-id "$APPLE_ID" \
      --team-id "$APPLE_TEAM_ID" \
      --password "$APPLE_PASSWORD" \
      --wait; then
      echo "[macos-build-tauri-release] error: DMG notarization failed (Apple ID)" >&2
      return 65
    fi
  else
    echo "[macos-build-tauri-release] notarize: no credentials; skip DMG notarization" >&2
    return 4
  fi

  echo "[macos-build-tauri-release] notarize: accepted; stapling ticket to DMG"
  if ! xcrun stapler staple "$dmg_path"; then
    echo "[macos-build-tauri-release] error: failed to staple DMG after notarization" >&2
    return 66
  fi
  xcrun stapler validate "$dmg_path" || echo "[macos-build-tauri-release] warning: stapler validate reported issues"
  return 0
}

# Prepare compile-time OpenAPI lock (hash password into Rust source embedded in the app).
prepare_openapi_lock() {
  local tauri_gen_dir="$TAURI_SRC_DIR/gen"
  mkdir -p "$tauri_gen_dir"

  local enabled_var="${MCPMATE_TAURI_OPENAPI_ENABLED:-}"
  local pw_var="${MCPMATE_TAURI_OPENAPI_PASSWORD:-}"

  # Consider enabled when either explicitly true/1/yes and password provided.
  local enabled=0
  if [[ -n "$pw_var" ]] && [[ "$enabled_var" =~ ^([Tt][Rr][Uu][Ee]|1|[Yy][Ee][Ss])$ ]]; then
    enabled=1
  fi

  local lock_file="$tauri_gen_dir/openapi_lock.rs"

  if [[ $enabled -eq 1 ]]; then
    if ! command -v shasum >/dev/null 2>&1; then
      echo "[macos-build-tauri-release] missing 'shasum' to compute password hash" >&2
      exit 1
    fi
    local hash
    hash=$(printf "%s" "$pw_var" | shasum -a 256 | awk '{print $1}')
    cat >"$lock_file" <<EOF
// Generated by macos-build-tauri-release.sh — DO NOT EDIT
pub const OPENAPI_LOCK_ENABLED: bool = true;
pub const OPENAPI_LOCK_HASH: &str = "sha256:${hash}";
EOF
    echo "[macos-build-tauri-release] embedded OpenAPI lock (sha256)"
  else
    rm -f "$lock_file"
    echo "[macos-build-tauri-release] OpenAPI lock disabled (no password or not enabled)"
  fi
}

mkdir -p "$OUTPUT_DIR"
OUTPUT_DIR="$(cd "$OUTPUT_DIR" && pwd)"
# Allow overriding the dashboard location while defaulting to the workspace `board/` package.
: "${BOARD_DIR_OVERRIDE:=}"
if [[ -n "$BOARD_DIR_OVERRIDE" ]]; then
  BOARD_DIR="$BOARD_DIR_OVERRIDE"
else
  BOARD_DIR="$WORKSPACE_ROOT/board"
fi
BOARD_DIR="$(cd "$BOARD_DIR" && pwd)"

if [[ ! -f "$BOARD_DIR/package.json" ]]; then
  echo "[macos-build-tauri-release] unable to locate board/package.json at $BOARD_DIR" >&2
  exit 1
fi

if [[ $BUILD_BOARD -eq 1 ]]; then
  echo "[macos-build-tauri-release] building board/dist"
  npm --prefix "$BOARD_DIR" install >/dev/null
	  if [[ -x "$LICENSE_SCRIPT" ]]; then
	    echo "[macos-build-tauri-release] generating open source notices"
	    "$LICENSE_SCRIPT" \
	      --board-dir "$BOARD_DIR" \
	      --backend-dir "$BACKEND_DIR" \
	      --tauri-dir "$TAURI_SRC_DIR" \
	      --output "$BOARD_DIR/public/open-source-notices.json"
  else
    echo "[macos-build-tauri-release] missing license generator at $LICENSE_SCRIPT" >&2
    exit 1
  fi
  npm --prefix "$BOARD_DIR" run build
else
  echo "[macos-build-tauri-release] skipping board build"
fi

IFS=',' read -r -a TARGET_LIST <<< "$TARGETS"

# Prepare signing / notarization environment on macOS.
preflight_signing

# Generate compile-time OpenAPI lock module (if enabled).
prepare_openapi_lock

# Enable market diagnostics-by-default when requested
if [[ $DIAG_DEFAULT -eq 1 ]]; then
  export MCPMATE_TAURI_MARKET_DIAG_DEFAULT=1
  log "market diagnostics default: ENABLED (MCPMATE_TAURI_MARKET_DIAG_DEFAULT=1)"
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

  echo "[macos-build-tauri-release] building bridge sidecar for $target"
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
    echo "[macos-build-tauri-release] missing bridge binary at $built_path" >&2
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

  echo "[macos-build-tauri-release] building core sidecar for $target"
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
    echo "[macos-build-tauri-release] missing core binary at $built_path" >&2
    exit 1
  fi
  cp "$built_path" "$SIDECAR_OUTPUT_DIR/mcpmate-core-$target"
  chmod +x "$SIDECAR_OUTPUT_DIR/mcpmate-core-$target"
  cp "$built_path" "$SIDECAR_OUTPUT_DIR/mcpmate-core"
  chmod +x "$SIDECAR_OUTPUT_DIR/mcpmate-core"
}

for TARGET in "${TARGET_LIST[@]}"; do
  echo "[macos-build-tauri-release] building target=$TARGET profile=$PROFILE bundles=$BUNDLES"

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
    echo "[macos-build-tauri-release] unsupported profile: $PROFILE" >&2
    exit 1
  fi

  if ((${#EXTRA_ARGS[@]})); then
    cmd+=("${EXTRA_ARGS[@]}")
  fi

  # Allow extra flags via env var (space-separated string), e.g. "--skip-stapling".
  if [[ -n "${TAURI_BUILD_EXTRA:-}" ]]; then
    # shellcheck disable=SC2206
    extraFromEnv=( ${TAURI_BUILD_EXTRA} )
    cmd+=("${extraFromEnv[@]}")
  fi

  "${cmd[@]}"

  echo "[macos-build-tauri-release] artifact: src-tauri/target/$TARGET/$PROFILE/bundle"

  bundle_dir="$ROOT_DIR/src-tauri/target/$TARGET/$PROFILE/bundle/dmg"
  if compgen -G "$bundle_dir"/*.dmg >/dev/null; then
    for dmg in "$bundle_dir"/*.dmg; do
      # Notarize the DMG (separately from the App) and then staple/validate.
      notarize_and_staple_dmg "$dmg" || echo "[macos-build-tauri-release] warning: DMG notarization/stapling step did not complete successfully"

      base="$(basename "$dmg")"
      lower="$(echo "$base" | tr '[:upper:]' '[:lower:]')"
      # Normalize output filename to GitHub Releases naming convention
      # - aarch64-apple-darwin  -> mcpmate_preview_aarch64.dmg
      # - x86_64-apple-darwin   -> mcpmate_preview_x64.dmg
      case "$TARGET" in
        aarch64-apple-darwin)
          out_name="mcpmate_preview_aarch64.dmg"
          ;;
        x86_64-apple-darwin)
          out_name="mcpmate_preview_x64.dmg"
          ;;
        *)
          # Fallback: keep tauri bundler's filename (lowercased)
          out_name="$lower"
          ;;
      esac
      dest="$OUTPUT_DIR/$out_name"
      cp -f "$dmg" "$dest"
      COPIED_DMGS+=("$dest")
      echo "[macos-build-tauri-release] copied $(basename "$dmg") -> $out_name"
    done
  else
    echo "[macos-build-tauri-release] warning: no DMG found under $bundle_dir" >&2
  fi

done

# --- Post-build: compute SHA256 of DMGs and update website/.env ---
log "post-build: computing SHA256 checksums for DMGs"

# Helper to compute sha256 as a lowercase hex string
sha256_of() {
  local file="$1"
  if ! command -v shasum >/dev/null 2>&1; then
    echo "missing shasum" >&2
    return 2
  fi
  shasum -a 256 "$file" | awk '{print $1}'
}

# Identify per-arch DMG from the ones produced in this run. Fallback to search OUTPUT_DIR.
ARM64_DMG=""
X64_DMG=""
for f in "${COPIED_DMGS[@]:-}"; do
  case "$(basename "$f")" in
    *aarch64*.dmg) ARM64_DMG="$f" ;;
    *arm64*.dmg)   ARM64_DMG="$f" ;;
    *x64*.dmg)     X64_DMG="$f" ;;
    *x86_64*.dmg)  X64_DMG="$f" ;;
  esac
done

if [[ -z "$ARM64_DMG" ]]; then
  # Find the most recent aarch64/arm64 dmg in OUTPUT_DIR
  ARM64_DMG=$(ls -1t "$OUTPUT_DIR"/*{aarch64,arm64}*.dmg 2>/dev/null | head -n1 || true)
fi
if [[ -z "$X64_DMG" ]]; then
  X64_DMG=$(ls -1t "$OUTPUT_DIR"/*{x64,x86_64}*.dmg 2>/dev/null | head -n1 || true)
fi

if [[ -z "$ARM64_DMG" && -z "$X64_DMG" ]]; then
  log "no DMG artifacts found for checksum generation; skipping env update"
  exit 0
fi

ARM64_SHA256=""
X64_SHA256=""
if [[ -n "$ARM64_DMG" && -f "$ARM64_DMG" ]]; then
  ARM64_SHA256=$(sha256_of "$ARM64_DMG")
  log "arm64 dmg: $(basename "$ARM64_DMG") sha256=$ARM64_SHA256"
fi
if [[ -n "$X64_DMG" && -f "$X64_DMG" ]]; then
  X64_SHA256=$(sha256_of "$X64_DMG")
  log "x64 dmg:   $(basename "$X64_DMG") sha256=$X64_SHA256"
fi

# Update website/.env with checksum values
WEBSITE_ENV="$WORKSPACE_ROOT/website/.env"
if [[ ! -f "$WEBSITE_ENV" ]]; then
  # If example exists, copy it; otherwise create an empty file.
  if [[ -f "$WORKSPACE_ROOT/website/.env.example" ]]; then
    cp "$WORKSPACE_ROOT/website/.env.example" "$WEBSITE_ENV"
  else
    touch "$WEBSITE_ENV"
  fi
fi

set_env_var() {
  local file="$1" key="$2" value="$3"
  # Ensure file ends with a newline to make sed/append safe
  if [[ -n "$value" ]]; then
    if grep -qE "^${key}=" "$file"; then
      # POSIX sed on macOS requires an arg to -i
      sed -i '' -E "s|^${key}=.*|${key}=${value}|" "$file"
    else
      printf "\n%s=%s\n" "$key" "$value" >> "$file"
    fi
  fi
}

if [[ -n "$ARM64_SHA256" ]]; then
  set_env_var "$WEBSITE_ENV" VITE_MAC_ARM64_SHA256 "$ARM64_SHA256"
fi
if [[ -n "$X64_SHA256" ]]; then
  set_env_var "$WEBSITE_ENV" VITE_MAC_X64_SHA256 "$X64_SHA256"
fi

log "updated checksums in website/.env"
