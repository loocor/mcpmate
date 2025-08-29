# MCPMate API Routes

This directory contains route definitions for the MCPMate Proxy API server.

## Purpose

Routes define the URL structure and HTTP methods for the API endpoints. They map incoming requests to the appropriate handler functions.

## Files

- `mod.rs` - Routes module entry point that combines all route modules
- `mcp.rs` - Routes for MCP server management (`/api/mcp/servers/*`)
- `tool.rs` - Routes for tool management (`/api/mcp/tools/*`)
- `profile.rs` - Routes for profile management (`/api/mcp/profile/*`)
- `specs.rs` - Routes for MCP specification-compliant endpoints (`/api/mcp/specs/*`)
- `system.rs` - Routes for system operations (`/api/system/*`)
- `notifs.rs` - Routes for notification management (`/api/notifications/*`)

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

## API Design Improvement Suggestions

Through a systematic review of API endpoints, we've identified several potential issues or inconsistencies that should be addressed:

### 1. Inconsistent Path Naming

- Some endpoints use plural forms (e.g., `/api/mcp/servers`), while others use singular forms (e.g., `/api/system/status`). We recommend standardizing on plural forms for resource collections.

### 2. Inconsistent Operation Verbs

- Some operations use HTTP methods (e.g., PUT for updates), while others use verbs in the path (e.g., `/enable`, `/disable`). This mixed approach can cause confusion.

### 3. Missing Delete Operations

- Server management lacks a delete endpoint (DELETE method), while profile management includes delete operations.

### 4. Inconsistent Parameter Naming

- In different paths, parameters of the same type sometimes use `{name}` and sometimes use `{id}`, which can be confusing.

### 5. Inconsistent Batch Operations

- Profile management includes batch operation endpoints, but server and tool management lack corresponding batch operations.

### 6. Documentation Mismatch with Actual Code

- Some endpoints mentioned in documentation (e.g., `/api/system/info` and `/api/system/health`) are actually implemented as `/api/system/status` and `/api/system/metrics` in the code.

### 7. Undefined Response Formats

- Response formats are not clearly defined for all endpoints, which can cause issues when client process responses.

## Recommended Improvements

1. **Standardize Naming Conventions**:
   - Use plural forms for resource collections
   - Keep parameter naming consistent (e.g., standardize on either `{id}` or `{name}`)

2. **Complete CRUD Operations**:
   - Add complete CRUD operations for all resources
   - Add delete operations for server management

3. **Standardize Operation Methods**:
   - Use HTTP methods for operations where possible (GET, POST, PUT, DELETE)
   - For operations that don't fit the CRUD model, use consistent verb naming (e.g., `/activate`, `/deactivate`)

4. **Add Batch Operations**:
   - Add batch operation endpoints for server and tool management, consistent with profile management

5. **Update Documentation**:
   - Ensure documentation matches the actual code
   - Add detailed request and response format documentation for all endpoints

6. **Add Version Control**:
   - Consider adding version numbers to API paths (e.g., `/api/v1/mcp/servers`)

7. **Standardize Error Handling**:
   - Define a unified error response format
   - Ensure all endpoints use the same error handling mechanism
