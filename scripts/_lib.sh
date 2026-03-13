#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

project_dir() {
  local name="$1"
  printf '%s/%s\n' "$ROOT_DIR" "$name"
}

require_command() {
  local cmd="$1"
  if ! command -v "$cmd" >/dev/null 2>&1; then
    printf 'Missing required command: %s\n' "$cmd" >&2
    exit 1
  fi
}

run_in() {
  local dir="$1"
  shift
  (
    cd "$dir"
    "$@"
  )
}

print_section() {
  local title="$1"
  printf '\n== %s ==\n' "$title"
}
