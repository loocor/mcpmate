#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: generate-update-manifest.sh --version <ver> --output <file> [options]

Required:
  --version <ver>              Version string written to the manifest
  --output <file>             Output JSON file path

Optional (repeatable):
  --platform <target> <url> <signature>
                              Add an entry under platforms[target] with the given download URL and
                              base64 Ed25519 signature. <target> may be either the Tauri updater
                              key (e.g. darwin-aarch64) or a Rust target triple.
  --notes <text>               Release notes (default: empty)
  --pub-date <ISO8601>         Publication timestamp (default: current UTC)

Example:
  packaging/desktop/generate-update-manifest.sh \
    --version 0.1.2 \
    --output dist/update.json \
    --notes "Fix desktop links" \
    --platform darwin-aarch64 https://cdn/app_0.1.2_arm64.app.tar.gz BASE64SIG_ARM64 \
    --platform darwin-x86_64 https://cdn/app_0.1.2_x64.app.tar.gz BASE64SIG_X64 \
    --platform windows-x86_64 https://cdn/app_0.1.2_x64.msi.zip BASE64SIG_WIN
USAGE
}

normalize_platform_key() {
  case "$1" in
    aarch64-apple-darwin) echo "darwin-aarch64" ;;
    x86_64-apple-darwin) echo "darwin-x86_64" ;;
    aarch64-pc-windows-msvc) echo "windows-aarch64" ;;
    x86_64-pc-windows-msvc) echo "windows-x86_64" ;;
    aarch64-unknown-linux-gnu) echo "linux-aarch64" ;;
    x86_64-unknown-linux-gnu) echo "linux-x86_64" ;;
    *) echo "$1" ;;
  esac
}

VERSION=""
OUTPUT=""
NOTES=""
PUB_DATE=""
declare -A PLATFORM_URLS
declare -A PLATFORM_SIGS

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version)
      VERSION="$2"
      shift 2
      ;;
    --output)
      OUTPUT="$2"
      shift 2
      ;;
    --notes)
      NOTES="$2"
      shift 2
      ;;
    --pub-date)
      PUB_DATE="$2"
      shift 2
      ;;
    --platform)
      TARGET=$(normalize_platform_key "$2")
      URL="$3"
      SIG="$4"
      PLATFORM_URLS["$TARGET"]="$URL"
      PLATFORM_SIGS["$TARGET"]="$SIG"
      shift 4
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

if [[ -z "$VERSION" || -z "$OUTPUT" ]]; then
  echo "--version and --output are required" >&2
  usage >&2
  exit 1
fi

if [[ -z "$PUB_DATE" ]]; then
  PUB_DATE="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
fi

tmpfile=$(mktemp)
{
  echo "{";
  echo "  \"version\": \"$VERSION\",";
  echo "  \"notes\": \"${NOTES//"/\\"}\",";
  echo "  \"pub_date\": \"$PUB_DATE\",";
  echo "  \"platforms\": {";
  first=1;
  for target in "${!PLATFORM_URLS[@]}"; do
    [[ $first -eq 0 ]] && echo "," || first=0
    url="${PLATFORM_URLS[$target]}"
    sig="${PLATFORM_SIGS[$target]}"
    printf "    \"%s\": {\n      \"signature\": \"%s\",\n      \"url\": \"%s\"\n    }" "$target" "$sig" "$url"
  done
  echo
  echo "  }";
  echo "}";
} > "$tmpfile"

mv "$tmpfile" "$OUTPUT"

echo "[generate-update-manifest] wrote $OUTPUT"
