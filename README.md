# MCPMate

[中文](./README_CN.md) | **English**

<p align="center">
  <img src="./docs/assets/servers.png" alt="MCPMate Dashboard" width="100%">
</p>

> **One management center for all your MCP servers and AI clients.**

MCPMate is a comprehensive Model Context Protocol (MCP) management center designed to simplify configuration, reduce resource consumption, and enhance security across the MCP ecosystem.

## Table of Contents

- [Why MCPMate?](#why-mcpmate)
- [Core Components](#core-components)
- [Screenshots](#screenshots)
- [Quick Start](#quick-start)
- [Architecture](#architecture)
- [Key Features](#key-features)
- [Development](#development)
- [Roadmap](#roadmap)
- [Contributing](#contributing)
- [License](#license)

## Why MCPMate?

Managing MCP servers across multiple AI tools (Claude Desktop, Cursor, Zed, Cherry Studio, etc.) brings significant challenges:

- **Complex, repetitive configuration** — The same MCP server needs to be configured repeatedly in each client
- **High context-switching cost** — Different work scenarios require frequent MCP configuration changes
- **Resource overhead** — Running multiple MCP servers simultaneously consumes system resources
- **Security blind spots** — Configuration errors or security risks are hard to detect
- **Fragmented management** — No single place to manage all MCP services

MCPMate solves these problems through centralized configuration management, intelligent service scheduling, and enhanced security protection.

## Core Components

### Proxy

A high-performance MCP proxy server that:

- Connects to multiple MCP servers and aggregates their tools
- Provides a unified interface for AI clients
- Supports multiple transport protocols (SSE, Streamable HTTP, stdio)
- Monitors and audits MCP communication in real time
- Detects potential security risks (e.g., tool poisoning)
- Intelligently manages server resources
- Provides RESTful API for management and monitoring

### Bridge

A lightweight bridging component that connects stdio-mode MCP clients (like Claude Desktop) to the HTTP-mode MCPMate proxy:

- Converts stdio protocol to HTTP (SSE or Streamable HTTP) without modifying the client
- Automatically inherits all functions and tools from the HTTP service
- Minimal configuration — only requires service address

### Runtime Manager

An intelligent runtime environment management tool:

- **Smart Download** — 15-second timeout with automatic network diagnostics
- **Progress Tracking** — Real-time progress bars with download speed
- **Multi-Runtime Support** — Node.js, uv (Python), and Bun.js
- **Environment Integration** — Automatic environment variable configuration

```bash
# Install Node.js for JavaScript MCP servers
runtime install node

# Install uv for Python MCP servers
runtime install uv

# List installed runtimes
runtime list
```

### Desktop App

Cross-platform desktop application built with Tauri 2:

- Complete graphical interface for managing MCP servers, profiles, and tools
- Real-time monitoring and status display
- Intelligent client detection and configuration generation
- System tray integration with native notifications
- Supports macOS, Windows, and Linux

## Screenshots

### Dashboard Overview

![Dashboard](./docs/assets/dashboard.png)

### Server Management

![Servers](./docs/assets/servers.png)

### Server Details — Tools

Browse all tools exposed by an MCP server with descriptions.

![Server Tools](./docs/assets/server-tools.png)

### Server Details — Resources

View resources provided by MCP servers.

![Server Resources](./docs/assets/server-resources.png)

### Profile Overview

Each profile aggregates servers, tools, resources, and prompts for a specific use case.

![Profile Overview](./docs/assets/profile-detail-overview.png)

### Profile — Tool Configuration

Enable or disable individual tools within a profile.

![Profile Tools](./docs/assets/profile-tools.png)

### Client Configuration

Configure management mode and capability source for each AI client.

![Client Configuration](./docs/assets/client-configuration.png)

## Quick Start

### Prerequisites

- Rust toolchain (1.75+)
- Node.js 18+ or Bun
- SQLite 3

### Installation

```bash
# Clone the repository
git clone https://github.com/loocor/MCPMate.git
cd MCPMate

# Build the backend
cd backend
cargo build --release

# Run the proxy
cargo run --release
```

The proxy starts with:
- REST API on `http://localhost:8080`
- MCP endpoint on `http://localhost:8000`

### Using the Dashboard

```bash
# From the repository root
cd board
bun install
bun run dev
```

The dashboard will be available at `http://localhost:5173`.

## Architecture

```
MCPMate/
├── backend/           # Rust MCP gateway, management API, bridge binary
├── board/             # React + Vite management dashboard
├── website/           # Marketing site and documentation
├── desktop/           # Tauri 2 desktop application
├── extension/cherry/  # Cherry Studio configuration integration
└── docs/              # Product documentation
```

Each subproject maintains its own build system and dependencies. See individual READMEs for details:

- [Backend](./backend/README.md) — Architecture, API, and development guide
- [Board](./board/README.md) — Dashboard features and UI development
- [Desktop](./desktop/README.md) — Desktop app build and configuration
- [Extension/Cherry](./extension/cherry/README.md) — Cherry Studio integration

## Key Features

### Profile-Based Configuration

Organize MCP servers into profiles for different scenarios:
- **Development** — Tools for coding, debugging, and testing
- **Writing** — Tools for content creation and research
- **Analysis** — Tools for data analysis and visualization

Switch between profiles instantly without restarting services.

### Multi-Client Support

MCPMate detects and configures multiple AI clients:
- Claude Desktop
- Cursor
- Zed
- Cherry Studio
- VS Code (with MCP extensions)

### Security

- Real-time MCP communication monitoring
- Tool poisoning detection
- Sensitive data detection
- Security alerts and audit logs

## Development

```bash
# Run all checks
./scripts/check

# Start backend + board together
./scripts/dev-all
```

See [AGENTS.md](./AGENTS.md) for development guidelines, coding standards, and contribution workflow.

## Roadmap

1. **Core proxy enhancement** — Improve stability, performance, and features
2. **Security audit** — Develop MCPMate Inspector for advanced security auditing
3. **Smart switching** — Context-based automatic profile switching
4. **Team collaboration** — Configuration sharing and role-based access control
5. **Cloud sync** — Multi-device configuration synchronization

## Contributing

Contributions are welcome! Please:

1. Read [AGENTS.md](./AGENTS.md) for development guidelines
2. Open an issue to discuss significant changes
3. Submit pull requests against the `main` branch

## License

MIT License — see [LICENSE](./LICENSE) for details.

---

Built with ❤️ by [Loocor](https://github.com/loocor)
