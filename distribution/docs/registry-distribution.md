# MCPMate Registry Distribution

This directory centralizes MCPMate's distribution assets so release tooling no longer lives inside the `backend/` subproject.

## Layout

- `distribution/docker/` — OCI image definition and runtime entrypoint.
- `distribution/scripts/` — local build, multi-platform build, registry validation, and publish scripts.
- `distribution/docs/` — distribution-specific documentation.
- `server.json` — repository-level MCP Registry manifest that remains at the repo root.

## Directory Boundaries

- `distribution/docker/` only contains files that Docker directly consumes during image build or container startup.
- `distribution/scripts/` contains repository-level workflow entrypoints invoked by developers or CI.
- `docker-build.sh` stays in `distribution/scripts/` because it is a wrapper around the whole repository build context, not a Docker runtime asset.
- `registry-validate.sh` stays in `distribution/scripts/` because it validates cross-cutting release consistency across `server.json`, `backend/Cargo.toml`, `distribution/docker/Dockerfile`, and `distribution/docker/docker-entrypoint.sh`.
- `registry-publish.sh` stays in `distribution/scripts/` because publishing with `mcp-publisher` is a release workflow concern, not a Docker-local concern.

## Common Commands

```bash
# Build the local OCI image
bash distribution/scripts/docker-build.sh

# Validate MCP Registry metadata and Docker label alignment
bash distribution/scripts/registry-validate.sh

# Publish with mcp-publisher after validation succeeds
bash distribution/scripts/registry-publish.sh
```

## Notes

- The Docker build context remains the repository root because the image bundles `backend/` and `board/` together.
- GitHub Actions uses the same `distribution/docker/Dockerfile` and `distribution/scripts/registry-validate.sh` entrypoints.
- `server.json` stays at the root because it is the repository-wide MCP Registry manifest rather than a backend-only artifact.
- This split is intentional: `distribution/docker/` holds build/runtime assets, while `distribution/scripts/` holds orchestration commands.
