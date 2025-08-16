# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

MCPMate is a comprehensive Model Context Protocol (MCP) management center written in Rust. It consists of three main components:

1. **Proxy Server** (`mcpmate`): Core MCP proxy that aggregates multiple MCP servers into a unified interface
2. **Bridge** (`bridge`): Lightweight bridge converting stdio MCP clients to HTTP-based MCPMate proxy
3. **Runtime Manager**: Intelligent runtime environment management for JavaScript/Python/Bun MCP servers

## Development Commands

### Build & Compilation
```bash
# Build for development (debug mode)
cargo build

# Build for release
cargo build --release

# Cross-platform builds
./script/build-all.sh [debug|release]   # All platforms
./script/build-macos.sh [debug|release] [target]
./script/build-linux.sh [debug|release] [target]
./script/build-windows.sh [debug|release] [target]

# Universal macOS binary
./script/build-universal.sh [debug|release]
```

### Testing
```bash
# Run all tests
cargo test

# Run specific test with output
cargo test test_name -- --nocapture

# Run tests with logging
RUST_LOG=debug cargo test

# Integration tests
cargo test --test integration

# Unit tests in specific module
cargo test --lib config::client
```

### Code Quality
```bash
# Format code
cargo fmt

# Check formatting
cargo fmt -- --check

# Lint with Clippy
cargo clippy

# Clippy for all targets
cargo clippy --all-targets --all-features

# Check without building
cargo check
```

### Database & Migration
```bash
# Database is SQLite-based, migrations run automatically on startup
# Database file: config/mcpmate.db
# Schema files: src/config/database.rs
```

### Running the Application
```bash
# Start MCPMate proxy (default ports: API 8000, MCP 8080)
cargo run

# With custom ports
cargo run -- --api-port 9000 --mcp-port 9090

# Enable debug logging
RUST_LOG=debug cargo run

# Start bridge component
cargo run --bin bridge

# Runtime manager examples
cargo run -- runtime install node
cargo run -- runtime list
```

## Architecture Overview

### Module Structure
- **`src/main.rs`**: Main entry point for MCPMate proxy server
- **`src/lib.rs`**: Library exports for FFI integration with desktop applications
- **`src/core/`**: Core business logic with layered architecture:
  - `foundation/`: Infrastructure layer (errors, utils, types)
  - `events/`: Event system for inter-component communication
  - `connection/`: Individual connection management
  - `transport/`: Protocol transport layer (stdio, SSE, HTTP)
  - `pool/`: Connection pool management
  - `protocol/`: MCP protocol handling (tools, resources, prompts)
  - `proxy/`: Main proxy server implementation
  - `cache/`: High-performance Redb-based caching system
- **`src/config/`**: Database-driven configuration management
- **`src/api/`**: RESTful API server for external control
- **`src/runtime/`**: Runtime environment management
- **`src/system/`**: System detection and platform utilities
- **`src/interop/`**: FFI bridge for desktop app integration

### Key Design Patterns

#### Database-Driven Configuration
All configuration is stored in SQLite (`config/mcpmate.db`) with structured tables:
- `server_config`: MCP server definitions
- `config_suit`: Configuration suits for scenario-based management
- `config_suit_server`/`config_suit_tool`: Many-to-many relationships

#### Config Suits System
Central concept for managing MCP servers and tools:
- **Host App Suits**: Per-application configurations (Claude Desktop, Cursor, etc.)
- **Scenario Suits**: Task-specific tool collections (coding, research, etc.)
- **Shared Suits**: Common tool sets across applications
- Multi-activation support for complex workflows

#### Connection Pool Architecture
- Lazy connection initialization
- Health monitoring and auto-reconnection
- Parallel connection management
- Resource cleanup and lifecycle management

#### Transport Layer Abstraction
Supports multiple MCP transport protocols:
- **stdio**: Process-based communication
- **SSE**: Server-Sent Events over HTTP
- **streamable_http**: Bidirectional HTTP streaming

### API Integration
RESTful API provides external control interface:
- Server management: `/api/mcp/servers/*`
- Config suit management: `/api/mcp/suits/*`
- Tool discovery: `/api/mcp/specs/tools/*`
- System monitoring: `/api/system/*`

## Testing Strategy

### Unit Tests
Located in `tests/unit/` with module-specific structure:
- Use `#[tokio::test]` for async tests
- Mock external dependencies with `mockall`
- Use `serial_test` for tests requiring exclusive access
- Test utilities in `tests/common/`

### Integration Tests
Located in `tests/integration/`:
- End-to-end workflow testing
- Database integration testing
- API endpoint testing with `axum-test`

### Test Configuration
- `clippy.toml`: Relaxed complexity thresholds for business logic
- `allow-unwrap-in-tests = true`: Permits unwrap in test code
- Use `test-case` for parameterized tests

## Development Notes

### Code Style
- Rust 2024 edition with 120-character line width
- Vertical function parameter layout for readability
- Group imports by std/external/crate
- Comprehensive error handling with `anyhow`/`thiserror`

### Feature Flags
- `interop`: Enables FFI bridge for desktop applications
- `standalone`: Standalone deployment mode

### Platform Support
Cross-compilation targets:
- Linux: x86_64, aarch64
- Windows: x86_64, aarch64 (gnullvm)
- macOS: x86_64, aarch64 (Intel/Apple Silicon)

### Performance Considerations
- Uses `lru` cache for frequently accessed data
- `dashmap` for concurrent hashmap operations
- Connection pooling with configurable limits
- Async/await throughout for non-blocking I/O

### Security
- Audit trail logging in `src/audit/`
- Policy-based access control
- Input validation for all API endpoints
- Secure handling of environment variables and credentials