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
  - `/api/mcp/servers` - List all servers
  - `/api/mcp/servers/:name` - Get a specific server
  - `/api/mcp/servers/:name/instances` - List all instances for a server
  - `/api/mcp/servers/:name/instances/:id` - Get a specific instance
  - `/api/mcp/servers/:name/instances/:id/health` - Check instance health
  - `/api/mcp/servers/:name/instances/:id/disconnect` - Disconnect an instance
  - `/api/mcp/servers/:name/instances/:id/disconnect/force` - Force disconnect an instance
  - `/api/mcp/servers/:name/instances/:id/reconnect` - Reconnect an instance
  - `/api/mcp/servers/:name/instances/:id/reconnect/reset` - Reset and reconnect an instance
  - `/api/mcp/servers/:name/instances/:id/cancel` - Cancel an initializing instance

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