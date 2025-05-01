# MCP-Proxy

MCP-Proxy 是一个基于 Rust 的 Model Context Protocol (MCP) 代理服务器实现，旨在简化与 MCP 服务器的交互，并提供一种统一的方式来访问多个 MCP 服务器的工具。

## 项目概述

MCP（Model Context Protocol）是一个开放标准，用于定义 AI 模型与外部工具和资源的交互方式。本项目提供了以下功能：

1. **MCP 代理服务器**：将多个 MCP 服务器的工具整合到一起，提供统一的接口
2. **MCP 客户端**：用于连接和管理 MCP 服务器
3. **工具调用**：支持调用 MCP 服务器提供的各种工具
4. **配置管理**：通过 JSON 配置文件管理服务器设置
5. **API 接口**：提供 RESTful API 用于管理和监控 MCP 服务器

## 技术基础

本项目基于 [Model Context Protocol Rust SDK](https://github.com/modelcontextprotocol/rust-sdk) 进行开发，该 SDK 提供了完整的 MCP 客户端和服务器实现框架。

## 核心功能

### MCP 代理服务器

项目的核心是 `mcp-proxy`，这是一个功能完整的 MCP 代理服务器，能够：

- 连接到多个 MCP 服务器
- 收集所有服务器的工具信息
- 将这些工具作为自己的工具提供给调用者
- 将工具调用请求转发给适当的服务器
- 处理错误和重连
- 支持多实例管理
- 提供 RESTful API 用于管理和监控

### MCP 客户端

除了代理服务器外，项目还提供了 `mcp-client` 工具，用于与 MCP 服务器交互：

## 配置文件

项目使用 `mcp.json` 配置文件来定义服务器设置。以下是配置文件的格式：

```json
{
  "mcpServers": {
    "server_name": {
      "kind": "stdio",
      "command": "npx",
      "commandPath": "./runtime/node-darwin-arm64/bin",  // 可选，指定命令的路径
      "args": [
        "--loglevel", "verbose",  // 注意：参数和值必须分开
        "-y", "package-name"
      ],
      "env": {
        "ENV_VAR": "value"
      }
    }
  }
}
```

配置选项说明：

- `kind`: 服务器类型，支持 "stdio" 和 "sse"
- `command`: 要执行的命令（通常是 `npx`）
- `commandPath`: （可选）命令的路径，如果指定，将与 `command` 拼接形成完整路径
- `args`: 命令行参数数组。**重要**：参数和值必须作为单独的数组元素，例如 `["--loglevel", "verbose"]` 而不是 `["--loglevel verbose"]`
- `env`: 环境变量对象

## 示例

`sample` 目录包含了各种 MCP 工具调用的示例配置。

## API 接口

MCP-Proxy 提供了 RESTful API 用于管理和监控 MCP 服务器，详见 [API 接口文档](./src/api/README.md)

## 未来计划

未来，我们计划添加以下功能：

1. **更多传输类型**：支持 TCP 和 WebSocket 等传输类型，提供更好的稳定性和可扩展性
2. **更多服务器支持**：添加对更多 MCP 服务器的支持
3. **更好的错误处理**：提供更详细的错误信息和恢复机制
4. **更好的日志记录**：提供更详细的日志记录，以便于调试
5. **更好的配置管理**：提供更灵活的配置管理，包括环境变量和占位符支持
6. **Web 界面**：提供 Web 界面用于管理和监控 MCP 服务器

## 贡献

欢迎贡献代码、报告问题或提出建议。请通过 GitHub Issues 或 Pull Requests 提交你的贡献。
