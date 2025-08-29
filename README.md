# MCPMate

MCPMate is a comprehensive Model Context Protocol (MCP) management center designed to address configuration complexity, resource consumption, security risks, and other issues in the MCP ecosystem, providing users with a unified management platform.

## Project Background & Vision

With the rapid development of the MCP ecosystem, more and more developers and creators are using MCP services in various tools (such as Claude Desktop, Cursor, Zed, Cherry Studio, etc.) to enhance the capabilities of AI assistants. However, this decentralized usage brings significant challenges:

- **Complex and repetitive configuration**: The same MCP server needs to be configured repeatedly in multiple clients
- **High context-switching cost**: Different work scenarios require frequent switching of MCP server configurations
- **Resource consumption and management difficulties**: Running multiple MCP servers simultaneously consumes a lot of system resources
- **Security risks and lack of monitoring**: Configuration errors or security risks are difficult to detect in time
- **Lack of unified management**: Managing different MCP services requires switching between multiple tools

MCPMate aims to solve these problems through centralized configuration management, intelligent service scheduling, and enhanced security protection, greatly improving usability, reducing manual configuration burden, and providing support for team collaboration.

## Core Components

### Proxy

One of the core components of the project is the `proxy`, a high-performance MCP proxy server that can:

- Connect to multiple MCP servers and aggregate their tools
- Provide a unified interface for AI clients
- Support multiple transport protocols (SSE, Streamable HTTP, or unified mode)
- Monitor and audit MCP communication in real time
- Detect potential security risks (such as tool poisoning)
- Intelligently manage server resources
- Support multi-instance management
- Provide RESTful API for management and monitoring

### Bridge

`bridge` is a lightweight bridging component used to connect stdio-mode MCP clients (such as Claude Desktop) to the HTTP-mode MCPMate proxy server:

- Converts stdio protocol to HTTP protocol (supports SSE and Streamable HTTP) without modifying the client
- Automatically inherits all functions and tools of the HTTP service
- Minimalist design, only requires service address configuration
- Suitable for clients that only support stdio mode (such as Claude Desktop)

### Runtime Manager

`runtime` is an intelligent runtime environment management tool that provides automated installation and management of various runtime environments:

- **Smart Download System**: 15-second intelligent timeout with automatic network diagnostics
- **Progress Tracking**: Real-time progress bars with download speed and stage information
- **Interactive Timeout Handling**: User-friendly timeout resolution with diagnostic reports
- **Multi-Runtime Support**: Node.js, uv (Python), and Bun.js runtime management
- **Environment Integration**: Automatic environment variable configuration for seamless MCP server usage
- **Network Diagnostics**: DNS resolution and connection testing with troubleshooting suggestions

#### Quick Start with Runtime Manager

```bash
# Install Node.js for JavaScript MCP servers
runtime install node

# Install uv for Python MCP servers
runtime install uv

# Install with interactive timeout handling
runtime install node --interactive --verbose

# List installed runtimes
runtime list

# Check runtime status
runtime check node
```

For detailed documentation, see [Runtime Manager Guide](./docs/runtime-manager.md).

## Configuration Management

MCPMate now uses a database-driven configuration management system, centered around the concept of **Profile**. All server, tool, and profile information is stored in a local SQLite database (`config/mcpmate.db`). This enables flexible, dynamic, and persistent management of MCP servers and tools, supporting advanced features such as multi-profile activation, scenario-based switching, and team collaboration.

### Key Concepts

- **Profile**: A profile is a collection of MCP servers and tools tailored for specific scenarios or applications. Users can create, activate, and switch between multiple profile to dynamically change the available services and tools without restarting MCPMate.
- **Database Storage**: All configuration data is stored in structured tables (e.g., `server_config`, `server_args`, `profile`, etc.) within the SQLite database. Direct editing of the database is not recommended; use the provided APIs.
- **API-Driven Management**: All configuration operations (create, update, enable/disable servers and tools, manage profile, etc.) are performed via RESTful APIs. See [API Documentation](./src/api/README.md) for details and examples.
- **Legacy mcp.json**: The `mcp.json` file is now only used for initial migration or backward compatibility. On first startup, if the database is empty and `mcp.json` exists, MCPMate will automatically migrate its contents to the database. Ongoing configuration should be managed via the database and APIs.

#### Example: Creating a New MCP Server via API

To add a new MCP server, use the following API endpoint:

```http
POST /api/mcp/servers
Content-Type: application/json

{
  "name": "python-server",
  "kind": "stdio", // or "sse", "streamable_http"
  "command": "python", // for stdio servers
  "url": "http://example.com/sse", // for sse/streamable_http servers
  "args": ["-m", "mcp_server"],
  "env": { "DEBUG": "true" },
  "enabled": true
}
```

For more details on profile and API usage, see [Configuration Management](./docs/features/configuration_management.md) and [API Documentation](./src/api/README.md).

### MCPMate Desktop

MCPMate Desktop is a native desktop application that provides a comprehensive graphical interface for managing MCP servers:

**Current Status**: ✅ **macOS version available** - Native SwiftUI application with full functionality

**Key Features**:
- Complete graphical interface for managing MCP servers, profile, and tools
- Real-time monitoring and status display with live updates
- Intelligent client detection and configuration generation
- Native system integration with MenuBar support and notifications
- Automated build and packaging system with DMG installer
- FFI integration with Rust backend for optimal performance

**Planned Platforms**:
- **Windows**: Native WinUI 3 application (planned)
- **Linux**: Native GTK 4 application (planned)

### MCPMate Inspector

The planned MCPMate Inspector is a security auditing component that will provide:

- Real-time monitoring of MCP communication
- Detection of security risks such as tool poisoning
- Sensitive data detection
- Complete log recording
- Security alerts

## API

MCPMate Proxy provides a RESTful API for managing and monitoring MCP servers. See [API Documentation](./src/api/README.md) for details.

## Technical Architecture

MCPMate uses the following technology stack:

- **Backend**: Rust with FFI support, based on [Model Context Protocol Rust SDK](https://github.com/modelcontextprotocol/rust-sdk)
- **Desktop**: Native applications per platform (SwiftUI for macOS, WinUI 3 for Windows, GTK 4 for Linux)
- **Data Storage**: SQLite database with structured configuration management
- **Communication**: RESTful API + FFI integration for desktop applications
- **Build System**: Automated build scripts with cross-platform packaging support

## Future Plans

Our development roadmap includes:

1. **Core proxy feature improvement**: Enhance the stability, performance, and functionality of MCPMate Proxy
2. **Desktop application development**: Build the MCPMate Desktop app with a graphical interface
3. **Security audit enhancement**: Develop MCPMate Inspector for more powerful security auditing
4. **Scenario presets and intelligent switching**: Implement context-based automatic configuration switching
5. **Team collaboration features**: Add configuration sharing, role-based access control, and other team features
6. **Cloud sync and multi-device support**: Implement cloud sync and multi-device support for configurations

## Contribution

Contributions, issue reports, and suggestions are welcome. Please submit your contributions via GitHub Issues or Pull Requests.

## License

This project is licensed under the [MIT License](LICENSE).
