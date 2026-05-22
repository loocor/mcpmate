# MCPMate Docker Image

The Docker image packages MCPMate as a Remote Core distribution. It runs the
Backend Core service, serves the Board web UI through nginx, and exposes the MCP
endpoint from the same container.

## Image Contents

- MCPMate Backend Core binary.
- Board static web assets.
- nginx reverse proxy for the Board, API, WebSocket, and MCP endpoints.
- Persistent data directory rooted at `/data/.mcpmate`.

## Build

```bash
bash packaging/docker/docker-build.sh
```

The script tags the image with the backend package version and `latest`:

```text
ghcr.io/loocor/mcpmate:<version>
ghcr.io/loocor/mcpmate:latest
```

Use `MCPMATE_IMAGE` to override the image name:

```bash
MCPMATE_IMAGE=example/mcpmate bash packaging/docker/docker-build.sh
```

## Run

```bash
docker run --rm \
  --name mcpmate \
  -p 3000:3000 \
  -p 8080:8080 \
  -p 8000:8000 \
  -v mcpmate-data:/data/.mcpmate \
  ghcr.io/loocor/mcpmate:latest
```

Default endpoints:

- Board: `http://127.0.0.1:3000`
- Backend API: `http://127.0.0.1:8080`
- MCP endpoint: `http://127.0.0.1:8000/mcp`

## Runtime Configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `MCPMATE_DASHBOARD_PORT` | `3000` | Public Board port inside the container. |
| `MCPMATE_API_PORT` | `8080` | Public Backend API port inside the container. |
| `MCPMATE_MCP_PORT` | `8000` | Public MCP endpoint port inside the container. |
| `MCPMATE_DATA_DIR` | `/data/.mcpmate` | Backend state directory. |
| `MCPMATE_LOG` | `info` | MCPMate log level. |
| `MCPMATE_TRANSPORT` | `uni` | Backend MCP transport mode. |

## Distribution Scope

The image is intended for server-side Remote Core deployment. The Board build
keeps the regular management UI available, including client governance
workflows that do not require writing local client configuration files.

Keep Docker-specific packaging behavior under `packaging/docker/` where possible,
rather than adding runtime branches across the main Backend and Board surfaces.

If the Docker package needs presentation differences for Remote Core workflows,
prefer Docker-owned overlays or injected assets first. This keeps the main Board
and Backend code paths aligned while still allowing the container package to hide
or disable local-device affordances that are not meaningful in a server-side
deployment.
