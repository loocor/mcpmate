# MCPMate Desktop

Cross-platform desktop application built with Tauri 2, wrapping the MCPMate backend and dashboard for macOS, Windows, and Linux.

## Quick Start

```bash
# From desktop/tauri directory
cd desktop/tauri
cargo tauri dev
```

## Documentation

- **[Tauri README](./tauri/README.md)** - Build commands, configuration, and release process
- **[Tauri AGENTS.md](./tauri/AGENTS.md)** - Development guide and architecture details
- **[Release Guide](./tauri/docs/desktop-release-guide.md)** - Desktop release workflow

## Project Structure

```
desktop/
├── README.md           # This file
└── tauri/              # Tauri desktop application
    ├── src-tauri/      # Rust backend integration
    ├── docs/           # Release documentation
    └── script/         # Build automation scripts
```

## Requirements

- Rust toolchain
- Node.js (for dashboard build)
- Tauri CLI 2.x: `cargo install tauri-cli --locked`

## Build

```bash
# Build dashboard assets
npm --prefix ../../board run build

# Build desktop app
cargo tauri build
```

See [Tauri README](./tauri/README.md) for detailed build options and platform-specific instructions.
