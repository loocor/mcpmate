#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: download-stats.sh [--json] [tag]

Query GitHub Release asset download counts for this repository.

Arguments:
  tag       Optional release tag to filter to a single release

Options:
  --json    Print raw JSON summary instead of a table
  -h, --help

Examples:
  scripts/download-stats.sh
  scripts/download-stats.sh v0.0.0-test-9
  scripts/download-stats.sh --json v0.0.0-test-9
USAGE
}

OUTPUT_MODE="table"
FILTER_TAG=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --json)
      OUTPUT_MODE="json"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      if [[ -n "$FILTER_TAG" ]]; then
        echo "error: only one tag filter is supported" >&2
        usage >&2
        exit 1
      fi
      FILTER_TAG="$1"
      shift
      ;;
  esac
done

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "error: missing required command: $1" >&2
    exit 1
  fi
}

require_cmd gh
require_cmd jq

if ! gh auth status >/dev/null 2>&1; then
  echo "error: gh CLI not authenticated. Run: gh auth login" >&2
  exit 1
fi

REPO="$(gh repo view --json nameWithOwner --jq '.nameWithOwner')"
CAPTURED_AT="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

releases_json="$(gh api "repos/${REPO}/releases?per_page=100")"

summary_json="$(jq \
  --arg captured_at "$CAPTURED_AT" \
  --arg repo "$REPO" \
  --arg tag "$FILTER_TAG" '
  {
    capturedAt: $captured_at,
    repository: $repo,
    releases: [
      .[]
      | select($tag == "" or .tag_name == $tag)
      | {
          tagName: .tag_name,
          name: .name,
          draft: .draft,
          prerelease: .prerelease,
          publishedAt: .published_at,
          totalDownloads: ([.assets[].download_count] | add // 0),
          assets: [
            .assets[] | {
              name,
              size,
              contentType: .content_type,
              downloadCount: .download_count,
              url: .browser_download_url
            }
          ]
        }
    ]
  }
' <<<"$releases_json")"

if [[ "$OUTPUT_MODE" == "json" ]]; then
  printf '%s\n' "$summary_json"
  exit 0
fi

release_count="$(jq '.releases | length' <<<"$summary_json")"
if [[ "$release_count" -eq 0 ]]; then
  if [[ -n "$FILTER_TAG" ]]; then
    echo "No release found for tag: $FILTER_TAG"
  else
    echo "No releases found."
  fi
  exit 0
fi

echo "Repository: $REPO"
echo "Captured at: $CAPTURED_AT"
echo

jq -r '
  .releases[] |
  "Release: " + .tagName,
  (if .prerelease then "Type: prerelease" else "Type: stable" end),
  "Published: " + (.publishedAt // "n/a"),
  "Total downloads: " + (.totalDownloads | tostring),
  "| Asset | Downloads | Size (bytes) |",
  "| --- | ---: | ---: |",
  (.assets[] | "| \(.name) | \(.downloadCount) | \(.size) |"),
  ""
' <<<"$summary_json"
