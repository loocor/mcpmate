# MCPMan API Handlers

This directory contains handler functions for the MCPMan Proxy API server.

## Purpose

Handlers implement the business logic for API endpoints. They process incoming requests, interact with the MCPMan Proxy server, and return appropriate responses.

## Files

- `mod.rs` - Handlers module entry point
- `mcp.rs` - Handlers for MCP server and instance management
- `system.rs` - Handlers for system operations

## MCP Handlers

The `mcp.rs` file contains handlers for managing MCPMan servers and instances:

- Server management:
  - `list_servers` - List all MCP servers
  - `get_server` - Get a specific MCP server

- Instance management:
  - `list_instances` - List all instances for a specific server
  - `get_instance` - Get a specific instance
  - `check_health` - Check the health of a specific instance
  - `disconnect` - Disconnect an instance
  - `force_disconnect` - Force disconnect an instance
  - `reconnect` - Reconnect an instance
  - `reset_reconnect` - Reset and reconnect an instance
  - `cancel` - Cancel an initializing instance

## System Handlers

The `system.rs` file contains handlers for system operations:

- `get_system_info` - Get system information
- `check_health` - Check system health

## Handler Structure

Each handler typically:

1. Extracts and validates request parameters
2. Interacts with the MCPMan Proxy server (via the connection pool)
3. Transforms the result into an appropriate response
4. Handles errors and returns appropriate status codes

## Error Handling

Handlers use a consistent error handling approach:

- Domain-specific errors are mapped to appropriate HTTP status codes
- Error responses include a descriptive message and, when appropriate, additional details
- Unexpected errors are logged and return a generic 500 Internal Server Error

## Adding New Handlers

To add new handlers:

1. Create a new handler module or extend an existing one
2. Implement the handler functions
3. Import and expose the handlers in `mod.rs`
