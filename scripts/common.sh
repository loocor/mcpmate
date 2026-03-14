#!/usr/bin/env bash
#
# common.sh - Shared library for MCPMate development scripts
#
# Usage: source "$(cd "$(dirname "$0")" && pwd)/common.sh"
#
# Provides:
#   - ROOT_DIR: Absolute path to monorepo root
#   - project_dir <name>: Returns absolute path to a subproject
#   - require_command <cmd>: Exits if command not found
#   - run_in <dir> <cmd...>: Runs command in specified directory
#   - print_section <title>: Prints a section header

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
