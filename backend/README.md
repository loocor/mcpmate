# MCPMate Backend

The Rust backend for MCPMate, providing the MCP proxy server, management API, bridge binary, and runtime manager.

## Architecture

```
backend/
├── src/
│   ├── main.rs           # Proxy entrypoint
│   ├── bin/bridge.rs     # Bridge binary (stdio ↔ HTTP)
│   ├── core/             # MCP proxy logic
│   ├── api/              # HTTP handlers and routes
│   ├── common/           # Shared utilities
│   ├── clients/          # Client management
│   ├── runtime/          # Runtime installation
│   └── macros/           # Procedural macros
├── crates/               # Workspace crates
│   ├── mcpmate-api/      # API aggregation
│   ├── mcpmate-storage/  # Database layer
│   ├── mcpmate-system/   # System utilities
│   └── mcpmate-types/    # Shared types
├── config/               # Presets and defaults
├── docs/                 # Backend documentation
└── script/               # Build helpers
```

## Quick Start

```bash
# Development
cargo run

# With debug logging
RUST_LOG=debug cargo run

# Production build
cargo build --release
```

Default ports:
- REST API: `8080`
- MCP endpoint: `8000`

## API Reference

### Server Management

```http
GET    /api/mcp/servers           # List all servers
POST   /api/mcp/servers           # Create server
GET    /api/mcp/servers/{id}      # Get server details
PATCH  /api/mcp/servers/{id}      # Update server
DELETE /api/mcp/servers/{id}      # Delete server
POST   /api/mcp/servers/{id}/toggle  # Enable/disable
```

### Profile Management

```http
GET    /api/mcp/profile           # List profiles
POST   /api/mcp/profile           # Create profile
GET    /api/mcp/profile/{id}      # Get profile details
PATCH  /api/mcp/profile/{id}      # Update profile
DELETE /api/mcp/profile/{id}      # Delete profile
```

### Client Management

```http
GET    /api/mcp/clients           # List clients
GET    /api/mcp/clients/{id}      # Get client details
POST   /api/mcp/clients/{id}/apply  # Apply profile to client
```

### Runtime Management

```http
GET    /api/runtime/list          # List installed runtimes
POST   /api/runtime/install       # Install runtime
GET    /api/runtime/check/{name}  # Check runtime status
```

Full API documentation available at `http://localhost:8080/docs` when the server is running.

## Configuration

Configuration is stored in SQLite at `~/.mcpmate/mcpmate.db` by default.

Override data directory:
```bash
MCPMATE_DATA_DIR=/custom/path cargo run
```

### Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `MCPMATE_API_PORT` | REST API port | `8080` |
| `MCPMATE_MCP_PORT` | MCP endpoint port | `8000` |
| `MCPMATE_LOG` | Log level | `info` |
| `MCPMATE_DATA_DIR` | Data directory | `~/.mcpmate` |
| `MCPMATE_TRANSPORT` | Transport mode | `uni` |

## Bridge Binary

The `bridge` binary connects stdio-mode MCP clients to the HTTP proxy:

```bash
# Build
cargo build --release --bin bridge

# Run (configurable via environment)
MCPMATE_PROXY_URL=http://localhost:8000 ./bridge
```

Useful for clients like Claude Desktop that only support stdio transport.

## Development

### Commands

```bash
# Fast feedback
cargo check

# Lint
cargo clippy --all-targets --all-features -- -D warnings

# Format
cargo fmt --all

# Test
cargo test
cargo test --features interop
```

### Adding a New API Endpoint

1. Define the route in `src/api/routes/`
2. Create handler in `src/api/handlers/`
3. Add request/response models in `src/api/models/`
4. Update OpenAPI documentation
5. Add tests in `#[cfg(test)]` module

### Database Schema

Schema is managed via SQLx migrations. See `docs/schema/` for documentation.

Key tables:
- `server_config` — MCP server definitions
- `profile` — Profile definitions
- `client` — Detected AI clients

## Testing

### Unit Tests

```bash
cargo test
```

### Integration Tests

```bash
# Requires running server
cargo test --features interop
```

### MCP Inspector

```bash
# Terminal 1: Start backend
cargo run

# Terminal 2: Run inspector
npx @modelcontextprotocol/inspector --cli http://127.0.0.1:8000/mcp --transport http --method tools/list
```

## Documentation

- `docs/readme.md` — Documentation structure guide
- `docs/progress.md` — Development progress and MCP validation
- `docs/features/` — Feature specifications
- `docs/schema/` — Database schema documentation

## MCP Protocol

Follows the [MCP specification (2025-06-18)](https://modelcontextprotocol.io/specification/2025-06-18).

Uses the official `rmcp` crate from crates.io for protocol implementation.

## Related

- [MCPMate Dashboard](../board/) — React management UI
- [MCPMate Desktop](../desktop/) — Tauri desktop app
- [MCPMate Docs](../docs/) — Product documentation
