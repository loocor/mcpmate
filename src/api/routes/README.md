# API Routes

This directory contains route definitions for the MCP Proxy API server.

## Purpose

Routes define the URL structure and HTTP methods for the API endpoints. They map incoming requests to the appropriate handler functions.

## Files

- `mod.rs` - Routes module entry point that combines all route modules
- `mcp.rs` - Routes for MCP server management (`/api/mcp/servers/*`)
- `system.rs` - Routes for system operations (`/api/system/*`)

## Route Structure

Routes follow a RESTful design pattern:

- Collection endpoints: `/api/{domain}/{collection}` (e.g., `/api/mcp/servers`)
- Resource endpoints: `/api/{domain}/{collection}/{id}` (e.g., `/api/mcp/servers/my-server`)
- Action endpoints: `/api/{domain}/{collection}/{id}/{action}` (e.g., `/api/mcp/servers/my-server/enable`)

## Adding New Routes

To add new routes:

1. Create a new route module or extend an existing one
2. Define the routes using Axum's routing system
3. Import and include the routes in `mod.rs`
