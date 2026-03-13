# Cherry DB Manager

A Rust library for managing Cherry Studio LevelDB configurations, specifically designed for handling MCP (Model Context Protocol) server configurations.

## Overview

Cherry Studio stores its configuration data in LevelDB with a unique encoding format (UTF-16 LE with JSON strings). This library provides a clean, safe interface to read and modify MCP server configurations without dealing with the underlying complexity.

## Features

- 🔧 **Easy MCP Server Management** - Read, write, add, and remove MCP servers
- 🛡️ **Type-Safe** - Full Rust type safety with serde serialization
- 📦 **JSON-Friendly** - All request/response structures are JSON-serializable
- ⚡ **Performance-Focused** - Direct LevelDB access with minimal overhead
- 🔐 **Error Handling** - Comprehensive error types for better debugging

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
cherry-db-manager = "0.1.0"
```

## Quick Start

```rust
use cherry_db_manager::{CherryDbManager, DefaultCherryDbManager, ServerRequest};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = DefaultCherryDbManager::new();
    let db_path = "./path/to/cherry/studio/leveldb";

    // Read current server configuration
    let config = manager.read_mcp_config(db_path)?;
    println!("Found {} MCP servers", config.servers.len());

    // List all servers
    let servers = manager.list_servers(db_path)?;
    for server in servers.servers {
        println!("Server: {} ({})", server.id, server.name);
    }

    // Add a new server
    let new_server = ServerRequest {
        id: "my-server".to_string(),
        is_active: true,
        args: vec!["--config".to_string(), "config.json".to_string()],
        command: "node".to_string(),
        server_type: "stdio".to_string(),
        name: "My Custom Server".to_string(),
    };
    manager.add_server(db_path, &new_server)?;

    Ok(())
}
```

## API Reference

### Core Trait: `CherryDbManager`

```rust
pub trait CherryDbManager {
    fn read_mcp_config(&self, db_path: &str) -> Result<McpConfigResponse>;
    fn write_mcp_config(&self, db_path: &str, config: &McpConfigRequest) -> Result<()>;
    fn list_servers(&self, db_path: &str) -> Result<ServerListResponse>;
    fn add_server(&self, db_path: &str, server: &ServerRequest) -> Result<()>;
    fn remove_server(&self, db_path: &str, server_id: &str) -> Result<()>;
    fn server_exists(&self, db_path: &str, server_id: &str) -> Result<bool>;
}
```

### Key Types

- `ServerRequest` / `ServerResponse` - MCP server configuration
- `McpConfigRequest` / `McpConfigResponse` - MCP server list
- `ServerListResponse` - Server listing with metadata
- `CherryDbError` - Comprehensive error types

## Integration Guide

### For Existing Projects

To minimize impact on your main project, use feature flags:

**In your `Cargo.toml`:**
```toml
[dependencies]
cherry-db-manager = { version = "0.1.0", optional = true }

[features]
cherry-integration = ["cherry-db-manager"]
```

**In your code:**
```rust
#[cfg(feature = "cherry-integration")]
mod cherry_integration {
    use cherry_db_manager::*;

    pub fn sync_mcp_servers(db_path: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let manager = DefaultCherryDbManager::new();
        let servers = manager.list_servers(db_path)?;
        Ok(servers.servers.into_iter().map(|s| s.id).collect())
    }
}
```

### Error Handling

```rust
use cherry_db_manager::{CherryDbError, Result};

match manager.read_mcp_config(db_path) {
    Ok(config) => println!("Success: {} servers", config.servers.len()),
    Err(CherryDbError::ConfigNotFound) => println!("No MCP config found"),
    Err(CherryDbError::InvalidPath(path)) => println!("Invalid path: {}", path),
    Err(CherryDbError::DatabaseError(msg)) => println!("DB error: {}", msg),
    Err(e) => println!("Other error: {}", e),
}
```

## Data Format Details

Cherry Studio uses a unique storage format:
- **LevelDB** for fast key-value storage
- **UTF-16 LE encoding** with a header byte (0x00)
- **Nested JSON strings** for modular configuration

The MCP configuration is stored as a JSON string under the `mcp` key in the main configuration object.

## Examples

Run the included example:
```bash
cargo run --example basic_usage
```

## Development

This library is designed to be:
- **Minimal** - No unnecessary dependencies
- **Safe** - No `unwrap()` or `expect()` in public APIs
- **Testable** - Clear separation between I/O and logic
- **Maintainable** - Well-documented and structured

## License

MIT License - see [LICENSE](LICENSE) for details.

## Contributing

Issues and pull requests welcome! Please ensure all tests pass and follow the existing code style.