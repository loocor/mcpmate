# Packaging

This directory is the home for non-code distribution assets and release orchestration.

## Structure

- `desktop/` — Tauri desktop packaging, platform release helper scripts, and update/licensing packaging utilities.
- `docker/` — container image build assets for bundled MCPMate distribution.
- `registry/` — MCP registry metadata and registry publishing/validation helpers.
- `standalone/` — standalone backend packaging and cross-platform build helpers.

## Scope

Keep runtime and product logic in `backend/` and `desktop/` source code.
Use `packaging/` only for distribution assets, release scripts, and related automation entrypoints.
