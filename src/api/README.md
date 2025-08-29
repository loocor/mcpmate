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

### 1. Server Management

#### Basic Server Operations
- **GET /api/mcp/servers/**
  - **Function**: List all servers
  - **Response**: List of servers with their names, types, and status information

- **POST /api/mcp/servers/**
  - **Function**: Create a new server
  - **Request**: Server configuration (name, type, URL/command, etc.)
  - **Response**: Information about the created server
  - **Request Body**:
    ```json
    {
      "name": "string (required)",
      "kind": "string (required, one of: stdio, sse, streamable_http)",
      "command": "string (required for stdio servers)",
      "url": "string (required for sse and streamable_http servers)",
      "args": ["string", "..."] (optional, for stdio servers),
      "env": { "key": "value", ... } (optional, for stdio servers),
      "enabled": boolean (optional, default: true)
    }
    ```
  - **Response Body**:
    ```json
    {
      "name": "string",
      "enabled": boolean,
      "server_type": "string",
      "command": "string (optional, for stdio servers)",
      "url": "string (optional, for sse and streamable_http servers)",
      "args": ["string", "..."] (optional, for stdio servers),
      "env": { "key": "value", ... } (optional, for stdio servers),
      "meta": {
        "description": "string (optional)",
        "author": "string (optional)",
        "website": "string (optional)",
        "repository": "string (optional)",
        "category": "string (optional)",
        "recommended_scenario": "string (optional)",
        "rating": number (optional)
      } (optional),
      "created_at": "string (ISO 8601 format, optional)",
      "updated_at": "string (ISO 8601 format, optional)",
      "instances": [] // Empty array for new servers
    }
    ```
  - **Error Responses**:
    - `400 Bad Request`: Invalid server type or missing required fields
    - `409 Conflict`: Server with the same name already exists
    - `500 Internal Server Error`: Database or other server error

- **POST /api/mcp/servers/import**
  - **Function**: Bulk import server configurations
  - **Request**: Data containing multiple server configurations
  - **Response**: Import results
  - **Request Body**:
    ```json
    {
      "mcpServers": {
        "server1": {
          "type": "stdio",
          "command": "string (for stdio servers)",
          "args": ["string", "..."] (optional, for stdio servers),
          "env": { "key": "value", ... } (optional, for stdio servers)
        },
        "server2": {
          "type": "sse",
          "url": "string (for sse servers)"
        },
        // ... more servers
      }
    }
    ```
  - **Response Body**:
    ```json
    {
      "imported_count": number,
      "imported_servers": ["string", "..."],
      "failed_servers": ["string", "..."]
    }
    ```
  - **Error Responses**:
    - `400 Bad Request`: Invalid JSON format or server configurations
    - `500 Internal Server Error`: Database or other server error

- **GET /api/mcp/servers/{name}**
  - **Function**: Get detailed information about a specific server
  - **Parameters**: Server name
  - **Response**: Detailed server configuration and status

- **PUT /api/mcp/servers/{name}**
  - **Function**: Update server configuration
  - **Parameters**: Server name
  - **Request**: Updated server configuration
  - **Response**: Updated server information
  - **Request Body**:
    ```json
    {
      "kind": "string (optional, one of: stdio, sse, streamable_http)",
      "command": "string (optional, for stdio servers)",
      "url": "string (optional, for sse and streamable_http servers)",
      "args": ["string", "..."] (optional, for stdio servers),
      "env": { "key": "value", ... } (optional, for stdio servers),
      "enabled": boolean (optional)
    }
    ```
  - **Response Body**:
    ```json
    {
      "name": "string",
      "enabled": boolean,
      "server_type": "string",
      "command": "string (optional, for stdio servers)",
      "url": "string (optional, for sse and streamable_http servers)",
      "args": ["string", "..."] (optional, for stdio servers),
      "env": { "key": "value", ... } (optional, for stdio servers),
      "meta": {
        "description": "string (optional)",
        "author": "string (optional)",
        "website": "string (optional)",
        "repository": "string (optional)",
        "category": "string (optional)",
        "recommended_scenario": "string (optional)",
        "rating": number (optional)
      } (optional),
      "created_at": "string (ISO 8601 format, optional)",
      "updated_at": "string (ISO 8601 format, optional)",
      "instances": [
        {
          "id": "string",
          "status": "string",
          "started_at": "string (ISO 8601 format, optional)",
          "connected_at": "string (ISO 8601 format, optional)"
        },
        // ... more instances if any
      ]
    }
    ```
  - **Error Responses**:
    - `400 Bad Request`: Invalid server type or incompatible configuration
    - `404 Not Found`: Server with the specified name does not exist
    - `500 Internal Server Error`: Database or other server error

- **POST /api/mcp/servers/{name}/enable**
  - **Function**: Enable a server
  - **Parameters**: Server name
  - **Response**: Operation result

- **POST /api/mcp/servers/{name}/disable**
  - **Function**: Disable a server
  - **Parameters**: Server name
  - **Response**: Operation result

- **GET /api/mcp/servers/{name}/instances**
  - **Function**: List all instances for a specific server
  - **Parameters**: Server name
  - **Response**: List of instances

#### Instance Management
- **GET /api/mcp/servers/{name}/instances/{id}**
  - **Function**: Get detailed information about a specific instance
  - **Parameters**: Server name, instance ID
  - **Response**: Detailed instance information

- **GET /api/mcp/servers/{name}/instances/{id}/health**
  - **Function**: Check instance health status
  - **Parameters**: Server name, instance ID
  - **Response**: Health status information

- **POST /api/mcp/servers/{name}/instances/{id}/disconnect**
  - **Function**: Disconnect an instance
  - **Parameters**: Server name, instance ID
  - **Response**: Operation result

- **POST /api/mcp/servers/{name}/instances/{id}/disconnect/force**
  - **Function**: Force disconnect an instance
  - **Parameters**: Server name, instance ID
  - **Response**: Operation result

- **POST /api/mcp/servers/{name}/instances/{id}/reconnect**
  - **Function**: Reconnect an instance
  - **Parameters**: Server name, instance ID
  - **Response**: Operation result

- **POST /api/mcp/servers/{name}/instances/{id}/reconnect/reset**
  - **Function**: Reset and reconnect an instance
  - **Parameters**: Server name, instance ID
  - **Response**: Operation result

- **POST /api/mcp/servers/{name}/instances/{id}/cancel**
  - **Function**: Cancel an initializing instance
  - **Parameters**: Server name, instance ID
  - **Response**: Operation result



### 2. Profile Management

> **Note**: Tool enabling/disabling should be managed through Profile. This is the primary interface for managing tool availability.

#### Basic Profile Operations
- **GET /api/mcp/profile/**
  - **Function**: List all profile
  - **Response**: List of profile
  - **Response Body**:
    ```json
    {
      "profile": [
        {
          "id": "string",
          "name": "string",
          "description": "string (optional)",
          "profile_type": "string (host_app, scenario, shared)",
          "multi_select": boolean,
          "priority": number,
          "is_active": boolean,
          "is_default": boolean,
          "allowed_operations": ["string", "..."]
        },
        // ... more profile
      ]
    }
    ```

- **POST /api/mcp/profile/**
  - **Function**: Create a new profile
  - **Request**: Profile information
  - **Response**: Created profile
  - **Request Body**:
    ```json
    {
      "name": "string (required)",
      "description": "string (optional)",
      "profile_type": "string (required, one of: host_app, scenario, shared)",
      "multi_select": boolean (optional),
      "priority": number (optional),
      "is_active": boolean (optional),
      "is_default": boolean (optional),
      "clone_from_id": "string (optional)"
    }
    ```
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

### 5. Notification Management

- **POST /api/notifications/tools/changed**
  - **Function**: Notify that the tool list has changed
  - **Request**: Change information
  - **Response**: Operation result

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
        "type": "sse",
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
│   │   ├── notifs.rs     # Notification related routes
│   │   └── system.rs     # System related routes
│   ├── handlers/         # Request handlers
│   │   ├── mod.rs        # Handlers module entry point
│   │   ├── mcp.rs        # MCP server related handlers
│   │   ├── tool.rs       # Tool management related handlers
│   │   ├── profile.rs       # Profile management related handlers
│   │   ├── specs.rs      # MCP specification-compliant handlers
│   │   ├── notification.rs # Notification related handlers
│   │   └── system.rs     # System related handlers
│   └── models/           # Request/response models
│       ├── mod.rs        # Models module entry point
│       ├── mcp.rs        # MCP server related models
│       ├── tool.rs       # Tool management related models
│       ├── profile.rs       # Profile management related models
│       ├── specs.rs      # MCP specification-compliant models
│       ├── notifications.rs # Notification related models
│       └── system.rs     # System related models
└── proxy/                # Existing proxy service code
    └── ...
```