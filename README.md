# MCPMate

MCPMate is organized as a lightweight monorepo for a solo maintainer. The
repository keeps product surfaces side by side so backend, board, website, and
desktop changes can be developed and validated together without forcing a heavy
workspace toolchain.

## Repository Layout

- `backend/` - Rust MCP gateway, management API, bridge binary, and runtime core.
- `board/` - React + Vite management dashboard for MCPMate.
- `website/` - Marketing site and product-facing documentation pages.
- `desktop/` - Desktop application work, packaging, and platform-specific shells.
- `sdk/` - Rust MCP SDK used by MCPMate.
- `cherry/` - Cherry Studio configuration integration library.
- `docs/` - Product documentation workspace when used.
- `workspace-progress.md` - Cross-project progress index.
- `AGENTS.md` - Top-level collaboration rules for the full repository.

## Working Style

This repo intentionally stays light:

- No Turbo/Nx/workspace-wide dependency manager.
- Each subproject keeps its own build system.
- Root scripts only delegate to existing subproject commands.
- Cross-project tasks are coordinated from the repository root.

## Common Workflows

Run these commands from the repository root:

```bash
# Check local toolchain expectations and entrypoints
./scripts/doctor

# Start the backend
./scripts/dev-backend

# Start the board
./scripts/dev-board

# Start backend + board together
./scripts/dev-all

# Run fast validation across the main maintained projects
./scripts/check
```

## Subproject Entry Points

- Backend details: `backend/README.md`
- Board details: `board/README.md`
- Website details: `website/README.md`
- Desktop details: `desktop/README.md`
- SDK details: `sdk/README.md`
- Cherry details: `cherry/README.md`

## Current Monorepo Boundary

The repository root is the coordination layer. Subprojects remain top-level
siblings and are not nested under `backend/`. This keeps the product easier to
maintain while preserving clear boundaries between runtime, UI, site, and SDK
code.
