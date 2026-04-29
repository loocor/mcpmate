# Packaging

This directory is the home for non-code distribution assets and release orchestration.

## Structure

- `desktop/` — Tauri desktop packaging and release helper scripts.
- `standalone/` — standalone backend packaging and cross-platform build helpers.

## Scope

Keep runtime and product logic in `backend/` and `desktop/` source code.
Use `packaging/` only for distribution assets, release scripts, and related automation entrypoints.
