# MCPMate API Module

This module contains the RESTful API implementation for the MCPMate Proxy server.

## Purpose

The API module provides HTTP endpoints for controlling and monitoring the MCPMate Proxy server. It allows external systems (like the MCPMate Desktop application) to interact with the proxy server without using the MCP protocol directly, avoiding circular dependencies.

## Structure

- `mod.rs` - API module entry point and configuration
- `server.rs` - API server implementation using Axum
- `routes/` - Route definitions for different API domains
- `handlers/` - Request handler implementations
- `models/` - Request and response data models

## API Endpoints Reference

This project adopts a consistent “list/details/create/update/delete/manage” style with query parameters for reads and JSON payloads for writes. All endpoints are served under `/api`.

### 1. Server Management

- List and details
  - `GET /mcp/servers/list`
  - `GET /mcp/servers/details?id=SERVxxxx`
- Mutations
  - `POST /mcp/servers/create`
  - `POST /mcp/servers/update`
  - `DELETE /mcp/servers/delete`
  - `POST /mcp/servers/import`
  - `POST /mcp/servers/preview` (side‑effect free)
  - `POST /mcp/servers/manage` with body `{ id, action: "enable"|"disable", sync? }`
- Instances
  - `GET /mcp/servers/instances/list?id=SERVxxxx` (omit `id` to list all)
  - `GET /mcp/servers/instances/details?server=SERVxxxx&instance=INSTyyyy`
  - `GET /mcp/servers/instances/health?server=...&instance=...`
  - `POST /mcp/servers/instances/manage`
- Capabilities and cache
  - `GET /mcp/servers/tools|resources|resources/templates|prompts`
  - `GET /mcp/servers/cache/detail`, `POST /mcp/servers/cache/reset`

### 2. Profile Management

- List and details
  - `GET /mcp/profile/list`, `GET /mcp/profile/details?id=...`
- Mutations
  - `POST /mcp/profile/create`, `POST /mcp/profile/update`, `DELETE /mcp/profile/delete`
  - `POST /mcp/profile/manage` (activate/deactivate)
- Components
  - `GET /mcp/profile/servers|tools|resources|prompts/list`
  - `POST /mcp/profile/servers|tools|resources|prompts/manage`

### 3. Client Configuration Management

- `GET /client/list`
- `GET /client/config/details`
- `POST /client/config/apply|restore|import`
- `POST /client/manage`
- `GET /client/backups/list`, `POST /client/backups/delete`
- `GET /client/backups/policy`, `POST /client/backups/policy`

### 4. Inspector (Proxy/Native Diagnostics)

- `GET /mcp/inspector/tool|prompt|resource/list`
- `POST /mcp/inspector/tool/call`, `POST /mcp/inspector/tool/call/start`, `POST /mcp/inspector/tool/call/cancel`
- `GET /mcp/inspector/resource/read`, `POST /mcp/inspector/prompt/get`
- `POST /mcp/inspector/session/open|close`
- SSE events (stream): `GET /mcp/inspector/tool/call/events`

### 5. System & Runtime

- System: `GET /system/status`, `GET /system/metrics`
- Runtime: `POST /runtime/install`, `GET /runtime/status`, `GET /runtime/cache`, `POST /runtime/cache/reset`
  - **Response**: The created Profile object

- **GET /api/mcp/profile/{id}**
  - **Function**: Get detailed information about a specific profile
  - **Parameters**: Profile ID
  - **Response**: Detailed profile information

- **PUT /api/mcp/profile/{id}**
  - **Function**: Update a profile
  - **Parameters**: Profile ID
  - **Request**: Updated profile information
  - **Response**: Updated profile
  - **Request Body**:
    ```json
    {
      "name": "string (optional)",
      "description": "string (optional)",
      "profile_type": "string (optional)",
      "multi_select": boolean (optional),
      "priority": number (optional),
      "is_active": boolean (optional),
      "is_default": boolean (optional)
    }
    ```
  - **Response**: The updated Profile object

- **DELETE /api/mcp/profile/{id}**
  - **Function**: Delete a profile
  - **Parameters**: Profile ID
  - **Response**: Operation result
  - **Response Body**:
    ```json
    {
      "id": "string",
      "name": "string",
      "result": "string",
      "status": "string",
      "allowed_operations": []
    }
    ```

- **POST /api/mcp/profile/{id}/activate**
  - **Function**: Activate a profile
  - **Parameters**: Profile ID
  - **Response**: Operation result
  - **Response Body**:
    ```json
    {
      "id": "string",
      "name": "string",
      "result": "string",
      "status": "string",
      "allowed_operations": ["string", "..."]
    }
    ```

- **POST /api/mcp/profile/{id}/deactivate**
  - **Function**: Deactivate a profile
  - **Parameters**: Profile ID
  - **Response**: Operation result
  - **Response Body**:
    ```json
    {
      "id": "string",
      "name": "string",
      "result": "string",
      "status": "string",
      "allowed_operations": ["string", "..."]
    }
    ```

- **POST /api/mcp/profile/batch/activate**
  - **Function**: Batch activate profile
  - **Request**: List of profile IDs
  - **Response**: Operation result
  - **Request Body**:
    ```json
    {
      "ids": ["string", "..."]
    }
    ```
  - **Response Body**:
    ```json
    {
      "success_count": number,
      "successful_ids": ["string", "..."],
      "failed_ids": {
        "id1": "error message",
        "id2": "error message"
      }
    }
    ```

- **POST /api/mcp/profile/batch/deactivate**
  - **Function**: Batch deactivate profile
  - **Request**: List of profile IDs
  - **Response**: Operation result
  - **Request Body**:
    ```json
    {
      "ids": ["string", "..."]
    }
    ```
  - **Response Body**:
    ```json
    {
      "success_count": number,
      "successful_ids": ["string", "..."],
      "failed_ids": {
        "id1": "error message",
        "id2": "error message"
      }
    }
    ```

#### Profile Server Management
- **GET /api/mcp/profile/{id}/servers/**
  - **Function**: List servers in a profile
  - **Parameters**: Profile ID
  - **Response**: List of servers
  - **Response Body**:
    ```json
    {
      "profile_id": "string",
      "profile_name": "string",
      "servers": [
        {
          "id": "string",
          "name": "string",
          "enabled": boolean,
          "allowed_operations": ["string", "..."]
        },
        // ... more servers
      ]
    }
    ```

- **POST /api/mcp/profile/{id}/servers/{server_id}/enable**
  - **Function**: Enable a server in a profile
  - **Parameters**: Profile ID, server ID
  - **Response**: Operation result
  - **Response Body**:
    ```json
    {
      "id": "string",
      "name": "string",
      "result": "string",
      "status": "string",
      "allowed_operations": ["string", "..."]
    }
    ```

- **POST /api/mcp/profile/{id}/servers/{server_id}/disable**
  - **Function**: Disable a server in a profile
  - **Parameters**: Profile ID, server ID
  - **Response**: Operation result
  - **Response Body**:
    ```json
    {
      "id": "string",
      "name": "string",
      "result": "string",
      "status": "string",
      "allowed_operations": ["string", "..."]
    }
    ```

- **POST /api/mcp/profile/{id}/servers/batch/enable**
  - **Function**: Batch enable servers in a profile
  - **Parameters**: Profile ID
  - **Request**: List of server IDs
  - **Response**: Operation result
  - **Request Body**:
    ```json
    {
      "ids": ["string", "..."]
    }
    ```
  - **Response Body**:
    ```json
    {
      "success_count": number,
      "successful_ids": ["string", "..."],
      "failed_ids": {
        "id1": "error message",
        "id2": "error message"
      }
    }
    ```

- **POST /api/mcp/profile/{id}/servers/batch/disable**
  - **Function**: Batch disable servers in a profile
  - **Parameters**: Profile ID
  - **Request**: List of server IDs
  - **Response**: Operation result
  - **Request Body**:
    ```json
    {
      "ids": ["string", "..."]
    }
    ```
  - **Response Body**:
    ```json
    {
      "success_count": number,
      "successful_ids": ["string", "..."],
      "failed_ids": {
        "id1": "error message",
        "id2": "error message"
      }
    }
    ```

#### Profile Tool Management
- **GET /api/mcp/profile/{id}/tools/**
  - **Function**: List tools in a profile
  - **Parameters**: Profile ID
  - **Response**: List of tools
  - **Response Body**:
    ```json
    {
      "profile_id": "string",
      "profile_name": "string",
      "tools": [
        {
          "id": "string",
          "server_name": "string",
          "tool_name": "string",
          "prefixed_name": "string (optional)",
          "enabled": boolean,
          "allowed_operations": ["string", "..."]
        },
        // ... more tools
      ]
    }
    ```

- **POST /api/mcp/profile/{id}/tools/{tool_id}/enable**
  - **Function**: Enable a tool in a profile
  - **Parameters**: Profile ID, tool ID
  - **Response**: Operation result
  - **Response Body**:
    ```json
    {
      "id": "string",
      "name": "string",
      "result": "string",
      "status": "string",
      "allowed_operations": ["string", "..."]
    }
    ```

- **POST /api/mcp/profile/{id}/tools/{tool_id}/disable**
  - **Function**: Disable a tool in a profile
  - **Parameters**: Profile ID, tool ID
  - **Response**: Operation result
  - **Response Body**:
    ```json
    {
      "id": "string",
      "name": "string",
      "result": "string",
      "status": "string",
      "allowed_operations": ["string", "..."]
    }
    ```

- **POST /api/mcp/profile/{id}/tools/batch/enable**
  - **Function**: Batch enable tools in a profile
  - **Parameters**: Profile ID
  - **Request**: List of tool IDs
  - **Response**: Operation result
  - **Request Body**:
    ```json
    {
      "ids": ["string", "..."]
    }
    ```
  - **Response Body**:
    ```json
    {
      "success_count": number,
      "successful_ids": ["string", "..."],
      "failed_ids": {
        "id1": "error message",
        "id2": "error message"
      }
    }
    ```

- **POST /api/mcp/profile/{id}/tools/batch/disable**
  - **Function**: Batch disable tools in a profile
  - **Parameters**: Profile ID
  - **Request**: List of tool IDs
  - **Response**: Operation result
  - **Request Body**:
    ```json
    {
      "ids": ["string", "..."]
    }
    ```
  - **Response Body**:
    ```json
    {
      "success_count": number,
      "successful_ids": ["string", "..."],
      "failed_ids": {
        "id1": "error message",
        "id2": "error message"
      }
    }
    ```

### 3. Specification-Compliant API

> **Note**: These endpoints are the primary means for tool discovery and information retrieval. They provide tool information in the standard MCP specification format.

- **GET /api/mcp/specs/tools/**
  - **Function**: List all tools across all connected servers
  - **Response**: List of tools in MCP specification format

- **GET /api/mcp/specs/tools/{server_name}**
  - **Function**: List tools for a specific server
  - **Parameters**: Server name
  - **Response**: List of tools in MCP specification format

- **GET /api/mcp/specs/tools/{server_name}/{tool_name}**
  - **Function**: Get detailed information about a specific tool
  - **Parameters**: Server name, tool name
  - **Response**: Detailed tool information in MCP specification format

### 4. System Management

- **GET /api/system/status**
  - **Function**: Get system status
  - **Response**: System status information, including uptime, version, etc.

- **GET /api/system/metrics**
  - **Function**: Get system metrics
  - **Response**: System performance metrics, such as CPU usage, memory usage, etc.

### 5. MCP Notification Delivery

MCPMate relies on the MCP protocol's native notification messages (e.g., `tools/listChanged`, `prompts/listChanged`, `resources/listChanged`) to inform clients about configuration updates. No separate REST endpoints are provided.

## Usage

The API server is started alongside the MCPMate Proxy server and provides a RESTful interface for controlling and monitoring the proxy server. This API is designed to be used by the MCPMate Desktop application and other clientlications.

## Example Usage

### Creating a new stdio server
```bash
curl -X POST http://localhost:8000/api/mcp/servers \
  -H "Content-Type: application/json" \
  -d '{
    "name": "python-server",
    "kind": "stdio",
    "command": "python",
    "args": ["-m", "mcp_server"],
    "env": {
      "DEBUG": "true"
    },
    "enabled": true
  }'
```

### Updating a server
```bash
curl -X PUT http://localhost:8000/api/mcp/servers/python-server \
  -H "Content-Type: application/json" \
  -d '{
    "args": ["-m", "mcp_server", "--verbose"],
    "enabled": false
  }'
```

### Importing servers from JSON
```bash
curl -X POST http://localhost:8000/api/mcp/servers/import \
  -H "Content-Type: application/json" \
  -d '{
    "mcpServers": {
      "node-server": {
        "type": "stdio",
        "command": "node",
        "args": ["server.js"]
      },
      "openai-server": {
        "type": "streamable_http",
        "url": "https://api.openai.com/v1/mcp"
      }
    }
  }'
```

## Directory Structure
```
src/
├── api/                  # API related code
│   ├── mod.rs            # API module entry point
│   ├── server.rs         # API server implementation
│   ├── routes/           # Route definitions
│   │   ├── mod.rs        # Routes module entry point
│   │   ├── mcp.rs        # MCP server related routes
│   │   ├── tool.rs       # Tool management related routes
│   │   ├── profile.rs       # Profile management related routes
│   │   ├── specs.rs      # MCP specification-compliant routes
│   │   └── system.rs     # System related routes
│   ├── handlers/         # Request handlers
│   │   ├── mod.rs        # Handlers module entry point
│   │   ├── mcp.rs        # MCP server related handlers
│   │   ├── tool.rs       # Tool management related handlers
│   │   ├── profile.rs       # Profile management related handlers
│   │   ├── specs.rs      # MCP specification-compliant handlers
│   │   └── system.rs     # System related handlers
│   └── models/           # Request/response models
│       ├── mod.rs        # Models module entry point
│       ├── mcp.rs        # MCP server related models
│       ├── tool.rs       # Tool management related models
│       ├── profile.rs       # Profile management related models
│       ├── specs.rs      # MCP specification-compliant models
│       └── system.rs     # System related models
└── proxy/                # Existing proxy service code
    └── ...
```
