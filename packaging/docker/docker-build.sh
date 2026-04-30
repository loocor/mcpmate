#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
BACKEND_DIR="$(cd "${ROOT_DIR}/backend" && pwd)"

VERSION="$(python3 - "${BACKEND_DIR}/Cargo.toml" <<'PY'
import re
import sys
from pathlib import Path

content = Path(sys.argv[1]).read_text(encoding="utf-8")
match = re.search(r'^version\s*=\s*"([^"]+)"', content, re.MULTILINE)
if not match:
    raise SystemExit("backend Cargo.toml version not found")
print(match.group(1))
PY
)"

IMAGE_NAME="${MCPMATE_IMAGE:-ghcr.io/loocor/mcpmate}"

docker build \
  --file "${ROOT_DIR}/packaging/docker/Dockerfile" \
  --tag "${IMAGE_NAME}:${VERSION}" \
  --tag "${IMAGE_NAME}:latest" \
  "${ROOT_DIR}"

echo "Built ${IMAGE_NAME}:${VERSION} and ${IMAGE_NAME}:latest"
