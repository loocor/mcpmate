# MCPMate API Routes

This directory contains route definitions for the MCPMate Proxy API server.

## Purpose

Routes define the URL structure and HTTP methods for the API endpoints. They map incoming requests to the appropriate handler functions.

## Files

- `mod.rs` - Routes module entry point that combines all route modules
- `mcp.rs` - Routes for MCP server management (`/api/mcp/servers/*`)
- `system.rs` - Routes for system operations (`/api/system/*`)

## MCP Routes

The `mcp.rs` file defines routes for MCP server and instance management:

- Server routes:
  - `GET /api/mcp/servers` - List all servers
  - `GET /api/mcp/servers/{name}` - Get a specific server

- Instance routes:
  - `GET /api/mcp/servers/{name}/instances` - List all instances for a server
  - `GET /api/mcp/servers/{name}/instances/{id}` - Get a specific instance
  - `GET /api/mcp/servers/{name}/instances/{id}/health` - Check instance health
  - `POST /api/mcp/servers/{name}/instances/{id}/disconnect` - Disconnect an instance
  - `POST /api/mcp/servers/{name}/instances/{id}/disconnect/force` - Force disconnect an instance
  - `POST /api/mcp/servers/{name}/instances/{id}/reconnect` - Reconnect an instance
  - `POST /api/mcp/servers/{name}/instances/{id}/reconnect/reset` - Reset and reconnect an instance
  - `POST /api/mcp/servers/{name}/instances/{id}/cancel` - Cancel an initializing instance

## System Routes

The `system.rs` file defines routes for system operations:

- `GET /api/system/info` - Get system information
- `GET /api/system/health` - Check system health

## Route Structure

Routes follow a RESTful design pattern:

- Collection endpoints: `/api/{domain}/{collection}` (e.g., `/api/mcp/servers`)
- Resource endpoints: `/api/{domain}/{collection}/{id}` (e.g., `/api/mcp/servers/my-server`)
- Nested resource endpoints: `/api/{domain}/{collection}/{id}/{subcollection}` (e.g., `/api/mcp/servers/my-server/instances`)
- Nested resource action endpoints: `/api/{domain}/{collection}/{id}/{subcollection}/{subid}/{action}` (e.g., `/api/mcp/servers/my-server/instances/123/disconnect`)

## Adding New Routes

To add new routes:

1. Create a new route module or extend an existing one
2. Define the routes using Axum's routing system
3. Import and include the routes in `mod.rs`
