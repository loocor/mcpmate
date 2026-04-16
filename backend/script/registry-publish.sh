#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BACKEND_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
ROOT_DIR="$(cd "${BACKEND_DIR}/.." && pwd)"

"${SCRIPT_DIR}/registry-validate.sh"

if ! command -v mcp-publisher >/dev/null 2>&1; then
  echo "mcp-publisher is required. Install it from the official MCP Registry releases before publishing." >&2
  exit 1
fi

cd "${ROOT_DIR}"
mcp-publisher publish
