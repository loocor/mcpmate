#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: release.sh [patch|minor|major]

Bumps the version, creates a git tag, and pushes to trigger the release workflow.

Version is derived from the latest semver tag (vX.Y.Z).
  patch  0.1.0 → 0.1.1  (default)
  minor  0.1.0 → 0.2.0
  major  0.1.0 → 1.0.0

Pre-flight checks:
  - Clean working tree
  - On main branch, up to date with remote
  - gh CLI authenticated
  - Target tag does not already exist
  - Key release files present
  - Latest CI on main is green

Example:
  scripts/release.sh           # patch bump
  scripts/release.sh minor     # minor bump
USAGE
}

BUMP="${1:-patch}"

if [[ "$BUMP" == "-h" || "$BUMP" == "--help" ]]; then
  usage
  exit 0
fi

if [[ "$BUMP" != "patch" && "$BUMP" != "minor" && "$BUMP" != "major" ]]; then
  echo "error: bump must be one of: patch, minor, major" >&2
  usage >&2
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

log() { echo "[release] $*"; }
err() { echo "[release] error: $*" >&2; }

# ── Pre-flight checks ────────────────────────────────────────────────────────

log "running pre-flight checks..."

# 1. Clean working tree
if [[ -n "$(git -C "$REPO_ROOT" status --porcelain)" ]]; then
  err "working tree is not clean. Commit or stash changes first."
  git -C "$REPO_ROOT" status --short
  exit 1
fi
log "  ✓ working tree clean"

# 2. On main branch
current_branch="$(git -C "$REPO_ROOT" branch --show-current)"
if [[ "$current_branch" != "main" ]]; then
  err "not on main branch (currently on: $current_branch)"
  exit 1
fi
log "  ✓ on main branch"

# 3. Up to date with remote
git -C "$REPO_ROOT" fetch origin main --quiet
ahead=$(git -C "$REPO_ROOT" rev-list --count "origin/main..HEAD")
behind=$(git -C "$REPO_ROOT" rev-list --count "HEAD..origin/main")
if [[ "$ahead" -gt 0 ]]; then
  err "local main is $ahead commit(s) ahead of origin/main. Push first."
  exit 1
fi
if [[ "$behind" -gt 0 ]]; then
  err "local main is $behind commit(s) behind origin/main. Pull first."
  exit 1
fi
log "  ✓ in sync with origin/main"

# 4. gh CLI authenticated
if ! command -v gh >/dev/null 2>&1; then
  err "gh CLI not found. Install: https://cli.github.com"
  exit 1
fi
if ! gh auth status >/dev/null 2>&1; then
  err "gh CLI not authenticated. Run: gh auth login"
  exit 1
fi
log "  ✓ gh CLI authenticated"

# 5. Key release files exist
required_files=(
  "desktop/src-tauri/tauri.conf.json"
  ".github/workflows/release.yml"
  "packaging/desktop/macos-build-tauri-release.sh"
  "packaging/desktop/linux-build-tauri-release.sh"
  "packaging/desktop/windows-build-tauri-release.sh"
  "packaging/desktop/collect-updater-artifacts.sh"
  "packaging/desktop/generate-update-manifest.sh"
  "desktop/src-tauri/tauri.release-overlay.json"
)
missing=0
for f in "${required_files[@]}"; do
  if [[ ! -f "$REPO_ROOT/$f" ]]; then
    err "  missing: $f"
    missing=1
  fi
done
if [[ $missing -eq 1 ]]; then
  err "required release files are missing"
  exit 1
fi
log "  ✓ key release files present"

# 6. Latest CI on main is green
latest_run=$(gh run list --branch main --workflow=desktop-macos.yml --limit=1 --json conclusion --jq '.[0].conclusion' 2>/dev/null || echo "")
if [[ "$latest_run" == "failure" ]]; then
  err "latest CI on main failed. Fix CI before releasing."
  exit 1
elif [[ -z "$latest_run" ]]; then
  log "  ⚠ could not verify CI status (no runs found or gh error) — continuing"
else
  log "  ✓ latest CI on main: $latest_run"
fi

# ── Compute next version ─────────────────────────────────────────────────────

latest_tag="$(git -C "$REPO_ROOT" tag --list 'v*' --sort=-version:refname | head -n1)"
if [[ -z "$latest_tag" ]]; then
  base_version="0.0.0"
  log "no existing semver tags found, starting from 0.0.0"
else
  base_version="${latest_tag#v}"
  log "latest tag: $latest_tag (version $base_version)"
fi

IFS='.' read -r major minor patch <<< "$base_version"
case "$BUMP" in
  major) major=$((major + 1)); minor=0; patch=0 ;;
  minor) minor=$((minor + 1)); patch=0 ;;
  patch) patch=$((patch + 1)) ;;
esac

next_version="${major}.${minor}.${patch}"
next_tag="v${next_version}"

# Check tag doesn't already exist
if git -C "$REPO_ROOT" tag --list "$next_tag" | grep -q .; then
  err "tag $next_tag already exists"
  exit 1
fi

log ""
log "  $latest_tag  →  $next_tag  ($BUMP)"
log ""

# ── Confirm ──────────────────────────────────────────────────────────────────

read -rp "Proceed with release $next_tag? [y/N] " answer
if [[ "$answer" != "y" && "$answer" != "Y" ]]; then
  log "aborted."
  exit 0
fi

# ── Tag and push ─────────────────────────────────────────────────────────────

git -C "$REPO_ROOT" tag "$next_tag"
git -C "$REPO_ROOT" push origin "$next_tag"

log "tag $next_tag pushed — release workflow triggered"
log "watch: gh run list --workflow=release.yml --limit=1"
