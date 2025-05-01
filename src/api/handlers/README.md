# API Handlers

This directory contains handler functions for the MCP Proxy API server.

## Purpose

Handlers implement the business logic for API endpoints. They process incoming requests, interact with the MCP Proxy server, and return appropriate responses.

## Files

- `mod.rs` - Handlers module entry point
- `mcp.rs` - Handlers for MCP server management
- `system.rs` - Handlers for system operations

## Handler Structure

Each handler typically:

1. Extracts and validates request parameters
2. Interacts with the MCP Proxy server (via the connection pool)
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
