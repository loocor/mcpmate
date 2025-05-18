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

## Configuration File

The project uses the `mcp.json` configuration file to define server settings. The format is as follows:

```json
{
  "mcpServers": {
    "server_name": {
      "kind": "stdio",
      "command": "npx",
      "commandPath": "./runtime/node-darwin-arm64/bin",  // Optional, specify the path of the command
      "args": [
        "--loglevel", "verbose",  // Note: parameters and values must be separated
        "-y", "package-name"
      ],
      "env": {
        "ENV_VAR": "value"
      }
    }
  }
}
```

Configuration options explained:

- `kind`: Server type, supports "stdio", "sse", and "streamable_http"
- `command`: The command to execute (usually `npx`)
- `commandPath`: (Optional) Path to the command, if specified, will be joined with `command` to form the full path
- `args`: Array of command-line arguments. **Important**: Parameters and values must be separate array elements, e.g., `["--loglevel", "verbose"]` instead of `["--loglevel verbose"]`
- `env`: Environment variable object

### MCPMate Desktop

The planned MCPMate Desktop is a cross-platform desktop application based on the Tauri2 framework, which will provide:

- Graphical interface for managing MCP servers
- Scenario presets and one-click switching
- Intelligent recommendations and guidance
- Configuration templates and version control
- Cross-device synchronization
- Real-time monitoring and auditing
- Security risk detection

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

- **Backend**: Rust, based on [Model Context Protocol Rust SDK](https://github.com/modelcontextprotocol/rust-sdk)
- **Frontend**: Planned to use Tauri2 framework + React
- **Data Storage**: Local configuration files + optional cloud sync
- **Communication**: RESTful API + WebSocket

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
