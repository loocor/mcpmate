# MCPMate Gemini Agent Context

This document provides context for the Gemini agent to understand and interact with the MCPMate project.

## Project Overview

MCPMate is a comprehensive management center for Model Context Protocol (MCP) servers. It aims to simplify configuration, reduce resource consumption, and enhance security in the MCP ecosystem.

### Core Components

*   **Proxy:** A high-performance MCP proxy server that aggregates tools from multiple MCP servers, provides a unified interface for AI clients, and supports various transport protocols.
*   **Bridge:** A lightweight component that connects stdio-mode MCP clients to the HTTP-mode MCPMate proxy server.
*   **Runtime Manager:** An intelligent tool for managing runtime environments like Node.js, uv (Python), and Bun.js.
*   **MCPMate Desktop:** A native desktop application (currently for macOS) that provides a graphical interface for managing MCP servers.
*   **MCPMate Inspector:** A planned security auditing component for real-time monitoring and threat detection.

### Technology Stack

*   **Backend:** Rust
*   **Frontend (Desktop):** Native (SwiftUI for macOS)
*   **Data Storage:** SQLite for configuration management
*   **Communication:** RESTful API and FFI for desktop integration

## Building and Running

### Building

The project uses shell scripts for building. The main build script is `script/build-universal.sh`. To build for all supported platforms, use:

```bash
./script/build-all.sh
```

This will create builds for Linux (x64, ARM64), Windows (x64, ARM64), and macOS (x64, ARM64).

### Running

The application can be run from a deployment package created by the `deploy.sh` script. To create a deployment package and run the application:

1.  **Create the package:**
    ```bash
    ./deploy.sh
    ```
2.  **Run the application:**
    *   On macOS/Linux: `./dist/local/mcpmate` or `./dist/local/start.sh`
    *   On Windows: `dist\local\mcpmate.exe` or `dist\local\start.bat`

The application will start a web server on `http://localhost:8080`.

## Development Conventions

*   **Formatting:** The project uses `rustfmt` for code formatting. Configuration is in `rustfmt.toml`.
*   **Linting:** The project uses `clippy` for linting. Configuration is in `clippy.toml`.
*   **Configuration:** The project uses a database-driven configuration system with SQLite. The database file is `config/mcpmate.db`. The `mcp.json` file is used for initial migration.

## Key Files and Directories

*   `src/`: The main source code for the Rust backend.
*   `board/`: The source code for the web-based management interface (frontend).
*   `script/`: Build and deployment scripts.
*   `config/`: Configuration files, including the SQLite database.
*   `docs/`: Project documentation.
*   `Cargo.toml`: The Rust package manager configuration file.
*   `README.md`: The main project README file.
