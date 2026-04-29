#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BACKEND_DIR="$(cd "${SCRIPT_DIR}/../../backend" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
SERVER_JSON="${SCRIPT_DIR}/server.json"
DOCKERFILE="${ROOT_DIR}/packaging/docker/Dockerfile"
ENTRYPOINT_SCRIPT="${ROOT_DIR}/packaging/docker/docker-entrypoint.sh"

python3 - "${SERVER_JSON}" "${BACKEND_DIR}/Cargo.toml" "${DOCKERFILE}" "${ENTRYPOINT_SCRIPT}" <<'PY'
import json
import re
import sys
from pathlib import Path

server_path = Path(sys.argv[1])
cargo_path = Path(sys.argv[2])
dockerfile_path = Path(sys.argv[3])
entrypoint_path = Path(sys.argv[4])

server = json.loads(server_path.read_text(encoding="utf-8"))
cargo = cargo_path.read_text(encoding="utf-8")
dockerfile = dockerfile_path.read_text(encoding="utf-8")
entrypoint = entrypoint_path.read_text(encoding="utf-8")

required_fields = ["$schema", "name", "description", "version", "packages"]
missing = [field for field in required_fields if not server.get(field)]
if missing:
    raise SystemExit(f"server.json missing required field(s): {', '.join(missing)}")

if not server["name"].startswith("io.github.loocor/"):
    raise SystemExit("server.json name must use the io.github.loocor namespace")

packages = server.get("packages")
if not isinstance(packages, list) or not packages:
    raise SystemExit("server.json packages must be a non-empty array")

package = packages[0]
for field in ["registryType", "identifier", "version", "transport"]:
    if not package.get(field):
        raise SystemExit(f"server.json packages[0] missing required field: {field}")

version_match = re.search(r'^version\s*=\s*"([^"]+)"', cargo, re.MULTILINE)
if not version_match:
    raise SystemExit("backend Cargo.toml version not found")

cargo_version = version_match.group(1)
if server["version"] != cargo_version:
    raise SystemExit(f"version mismatch: server.json={server['version']} backend/Cargo.toml={cargo_version}")

if package["version"] != cargo_version:
    raise SystemExit(f"package version mismatch: package={package['version']} backend/Cargo.toml={cargo_version}")

label = f'LABEL io.modelcontextprotocol.server.name="{server["name"]}"'
if label not in dockerfile:
    raise SystemExit("backend/Dockerfile MCP Registry label does not match server.json name")

for env_name in ("MCPMATE_API_PORT", "MCPMATE_MCP_PORT", "MCPMATE_DASHBOARD_PORT", "MCPMATE_LOG", "MCPMATE_TRANSPORT"):
    if env_name not in entrypoint:
        raise SystemExit(f"docker-entrypoint.sh does not consume declared environment variable: {env_name}")

print("Registry metadata validation passed")
PY

IMAGE_NAME="${MCPMATE_IMAGE:-ghcr.io/loocor/mcpmate}"
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

if command -v docker >/dev/null 2>&1 && docker image inspect "${IMAGE_NAME}:${VERSION}" >/dev/null 2>&1; then
  ACTUAL_LABEL="$(docker inspect "${IMAGE_NAME}:${VERSION}" --format='{{ index .Config.Labels "io.modelcontextprotocol.server.name" }}')"
  EXPECTED_LABEL="$(python3 - "${SERVER_JSON}" <<'PY'
import json
import sys
from pathlib import Path

print(json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))["name"])
PY
)"
  if [[ "${ACTUAL_LABEL}" != "${EXPECTED_LABEL}" ]]; then
    echo "Docker label mismatch: expected ${EXPECTED_LABEL}, got ${ACTUAL_LABEL}" >&2
    exit 1
  fi
  echo "Docker image label validation passed"
else
  echo "Docker image ${IMAGE_NAME}:${VERSION} not found; skipped image label validation"
fi
