#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: collect-updater-artifacts.sh --bundle-dir <dir> --output-dir <dir>

Scans a Tauri build bundle directory for updater .sig files,
then copies them (and macOS .app.tar.gz bundles) to the output directory.

For Linux and Windows, the update bundles (.AppImage, .msi) are also
the installers and are uploaded separately by the build scripts — only
the .sig files are collected here. For macOS, the .app.tar.gz update
bundle is unique to the updater and is collected alongside its .sig.

Options:
  --bundle-dir <dir>   Tauri bundle output directory (e.g. .../release/bundle)
  --output-dir <dir>   Destination directory for collected updater files
  -h, --help           Show this help

Example:
  packaging/desktop/collect-updater-artifacts.sh \
    --bundle-dir desktop/src-tauri/target/aarch64-apple-darwin/release/bundle \
    --output-dir updater-artifacts
USAGE
}

BUNDLE_DIR=""
OUTPUT_DIR=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --bundle-dir) BUNDLE_DIR="$2"; shift 2 ;;
    --output-dir) OUTPUT_DIR="$2"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; usage >&2; exit 1 ;;
  esac
done

if [[ -z "$BUNDLE_DIR" || -z "$OUTPUT_DIR" ]]; then
  echo "--bundle-dir and --output-dir are required" >&2
  usage >&2
  exit 1
fi

mkdir -p "$OUTPUT_DIR"

log() { echo "[collect-updater-artifacts] $*"; }

collected=0

# macOS updater bundles: .app.tar.gz + .app.tar.gz.sig
# The .app.tar.gz is a dedicated updater format (not the same as .dmg),
# so we collect both the bundle and its signature.
while IFS= read -r -d '' sig; do
  bundle="${sig%.sig}"
  if [[ -f "$bundle" ]]; then
    cp -f "$bundle" "$OUTPUT_DIR/"
    cp -f "$sig" "$OUTPUT_DIR/"
    log "collected $(basename "$bundle") + sig"
    ((collected++)) || true
  fi
done < <(find "$BUNDLE_DIR" -type f -name "*.app.tar.gz.sig" -print0 2>/dev/null) || true

# Linux updater: .AppImage.sig only (the .AppImage itself is uploaded as an installer)
while IFS= read -r -d '' sig; do
  cp -f "$sig" "$OUTPUT_DIR/"
  log "collected $(basename "$sig")"
  ((collected++)) || true
done < <(find "$BUNDLE_DIR" -type f -name "*.AppImage.sig" -print0 2>/dev/null) || true

# Windows updater: .msi.sig / .nsis.zip.sig / .msi.zip.sig only
while IFS= read -r -d '' sig; do
  cp -f "$sig" "$OUTPUT_DIR/"
  log "collected $(basename "$sig")"
  ((collected++)) || true
done < <(find "$BUNDLE_DIR" -type f \( -name "*.nsis.zip.sig" -o -name "*.msi.zip.sig" -o -name "*.msi.sig" \) -print0 2>/dev/null) || true

if [[ $collected -eq 0 ]]; then
  log "warning: no updater artifacts found in $BUNDLE_DIR"
  log "bundle layout:"
  find "$BUNDLE_DIR" -type f -name "*.sig" -o -name "*.tar.gz" -o -name "*.AppImage" -o -name "*.nsis.zip" -o -name "*.msi.zip" -o -name "*.msi" | head -20 || true
fi

log "collected $collected updater artifact pair(s) to $OUTPUT_DIR"
