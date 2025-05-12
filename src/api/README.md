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

## API Domains

- `/api/mcp/servers/*` - Endpoints for managing MCP upstream servers
  - `/api/mcp/servers` - List all servers (GET)
  - `/api/mcp/servers` - Create a new server (POST)
  - `/api/mcp/servers/import` - Import servers from JSON configuration (POST)
  - `/api/mcp/servers/:name` - Get a specific server (GET)
  - `/api/mcp/servers/:name` - Update a specific server (PUT)
  - `/api/mcp/servers/:name/enable` - Enable a server (POST)
  - `/api/mcp/servers/:name/disable` - Disable a server (POST)
  - `/api/mcp/servers/:name/instances` - List all instances for a server (GET)
  - `/api/mcp/servers/:name/instances/:id` - Get a specific instance (GET)
  - `/api/mcp/servers/:name/instances/:id/health` - Check instance health (GET)
  - `/api/mcp/servers/:name/instances/:id/disconnect` - Disconnect an instance (POST)
  - `/api/mcp/servers/:name/instances/:id/disconnect/force` - Force disconnect an instance (POST)
  - `/api/mcp/servers/:name/instances/:id/reconnect` - Reconnect an instance (POST)
  - `/api/mcp/servers/:name/instances/:id/reconnect/reset` - Reset and reconnect an instance (POST)
  - `/api/mcp/servers/:name/instances/:id/cancel` - Cancel an initializing instance (POST)

- `/api/mcp/tools/*` - Endpoints for managing MCP tools
  - `/api/mcp/tools` - List all tools
  - `/api/mcp/tools/:server_name/:tool_name` - Get a specific tool configuration
  - `/api/mcp/tools/:server_name/:tool_name/enable` - Enable a specific tool
  - `/api/mcp/tools/:server_name/:tool_name/disable` - Disable a specific tool
  - `/api/mcp/tools/:server_name/:tool_name` (POST) - Update a specific tool configuration

- `/api/notifications/*` - Endpoints for notification management
  - `/api/notifications/tools/changed` - Notify clients that the tools list has changed

- `/api/system/*` - Endpoints for system-level operations and monitoring
  - `/api/system/status` - Get system status
  - `/api/system/metrics` - Get system metrics

## Usage

The API server is started alongside the MCPMate Proxy server and provides a RESTful interface for controlling and monitoring the proxy server. This API is designed to be used by the MCPMate Desktop application and other client applications.

## Detailed API Documentation

### Server Management APIs

#### Create a new MCP server
- **Endpoint**: `/api/mcp/servers`
- **Method**: `POST`
- **Description**: Creates a new MCP server configuration
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
- **Response**:
  ```json
  {
    "name": "string",
    "enabled": boolean,
    "instances": [] // Empty array for new servers
  }
  ```
- **Error Responses**:
  - `400 Bad Request`: Invalid server type or missing required fields
  - `409 Conflict`: Server with the same name already exists
  - `500 Internal Server Error`: Database or other server error

#### Update an existing MCP server
- **Endpoint**: `/api/mcp/servers/{name}`
- **Method**: `PUT`
- **Description**: Updates an existing MCP server configuration
- **URL Parameters**:
  - `name`: Name of the server to update
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
- **Response**:
  ```json
  {
    "name": "string",
    "enabled": boolean,
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

#### Import servers from JSON configuration
- **Endpoint**: `/api/mcp/servers/import`
- **Method**: `POST`
- **Description**: Imports multiple MCP servers from a JSON configuration
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
- **Response**:
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

### Example Usage

#### Creating a new stdio server
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

#### Updating a server
```bash
curl -X PUT http://localhost:8000/api/mcp/servers/python-server \
  -H "Content-Type: application/json" \
  -d '{
    "args": ["-m", "mcp_server", "--verbose"],
    "enabled": false
  }'
```

#### Importing servers from JSON
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

## Directory structure
```
src/
├── api/                  # API 相关代码
│   ├── mod.rs            # API 模块入口
│   ├── server.rs         # API 服务器实现
│   ├── routes/           # 路由定义
│   │   ├── mod.rs        # 路由模块入口
│   │   ├── mcp.rs        # MCP 服务器相关路由
│   │   ├── tool.rs       # 工具管理相关路由
│   │   ├── notifications.rs # 通知相关路由
│   │   └── system.rs     # 系统相关路由
│   ├── handlers/         # 请求处理函数
│   │   ├── mod.rs        # 处理函数模块入口
│   │   ├── mcp.rs        # MCP 服务器相关处理函数
│   │   ├── tool.rs       # 工具管理相关处理函数
│   │   ├── notification.rs # 通知相关处理函数
│   │   └── system.rs     # 系统相关处理函数
│   └── models/           # 请求/响应模型
│       ├── mod.rs        # 模型模块入口
│       ├── mcp.rs        # MCP 服务器相关模型
│       ├── tool.rs       # 工具管理相关模型
│       ├── notifications.rs # 通知相关模型
│       └── system.rs     # 系统相关模型
└── proxy/                # 现有的代理服务代码
    └── ...
```