# API Module

This module contains the RESTful API implementation for the MCP Proxy server.

## Purpose

The API module provides HTTP endpoints for controlling and monitoring the MCP Proxy server. It allows external systems (like Tauri desktop applications) to interact with the proxy server without using the MCP protocol directly, avoiding circular dependencies.

## Structure

- `mod.rs` - API module entry point and configuration
- `server.rs` - API server implementation using Axum
- `routes/` - Route definitions for different API domains
- `handlers/` - Request handler implementations
- `models/` - Request and response data models

## API Domains

- `/api/mcp/servers/*` - Endpoints for managing MCP upstream servers
- `/api/system/*` - Endpoints for system-level operations and monitoring

## Usage

The API server is started alongside the MCP Proxy server and provides a RESTful interface for controlling and monitoring the proxy server.

## Directory structure
```
src/
├── api/                  # API 相关代码
│   ├── mod.rs            # API 模块入口
│   ├── server.rs         # API 服务器实现
│   ├── routes/           # 路由定义
│   │   ├── mod.rs        # 路由模块入口
│   │   ├── mcp.rs        # MCP 服务器相关路由
│   │   └── system.rs     # 系统相关路由
│   ├── handlers/         # 请求处理函数
│   │   ├── mod.rs        # 处理函数模块入口
│   │   ├── mcp.rs        # MCP 服务器相关处理函数
│   │   └── system.rs     # 系统相关处理函数
│   └── models/           # 请求/响应模型
│       ├── mod.rs        # 模型模块入口
│       ├── mcp.rs        # MCP 服务器相关模型
│       └── system.rs     # 系统相关模型
└── proxy/                # 现有的代理服务代码
    └── ...
```