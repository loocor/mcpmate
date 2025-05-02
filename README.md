# MCPMan (MCP管理中心)

MCPMan 是一个综合性的 Model Context Protocol (MCP) 管理中心，旨在解决 MCP 生态系统中的配置复杂性、资源消耗、安全风险等问题，为用户提供统一的管理平台。

## 项目背景与愿景

随着 MCP 生态系统的快速发展，越来越多的开发者和创作者开始在多个工具（如 Claude Desktop、Cursor、Zed、Cherry Studio 等）中使用 MCP 服务来增强 AI 助手的能力。然而，这种分散的使用方式带来了显著的挑战：

- **配置复杂且重复**：需要在多个客户端中重复配置相同的 MCP 服务器
- **高昂的上下文切换成本**：不同的工作场景需要频繁切换 MCP 服务器配置
- **资源消耗与管理困难**：同时运行多个 MCP 服务器会消耗大量系统资源
- **安全风险与监控缺失**：配置错误或安全风险难以被及时发现
- **缺乏统一管理**：需要在多个工具间切换来管理不同的 MCP 服务

MCPMan 旨在通过集中化的配置管理、智能化的服务调度和增强的安全防护，解决这些问题，大幅提高易用性，减少用户的手动配置负担，并为团队协作提供支持。

## 核心组件

### MCPMan Proxy

项目的核心组件之一是 `mcpman-proxy`，这是一个高性能的 MCP 代理服务器，能够：

- 连接到多个 MCP 服务器并聚合它们的工具
- 提供统一的接口给 AI 客户端
- 实时监控和审计 MCP 通信
- 检测潜在的安全风险（如工具投毒）
- 智能管理服务器资源
- 支持多实例管理
- 提供 RESTful API 用于管理和监控

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

### MCPMan Desktop

计划中的 MCPMan Desktop 是一个基于 Tauri2 框架的跨平台桌面应用，将提供：

- 图形化界面，用于管理 MCP 服务器
- 场景预设与一键切换功能
- 智能推荐与引导
- 配置模板与版本控制
- 跨设备同步
- 实时监控与审计
- 安全风险检测

### MCPMan Inspector

计划中的 MCPMan Inspector 是一个安全审计组件，将提供：

- 实时监控 MCP 通信
- 检测工具投毒等安全风险
- 敏感数据检测
- 完整日志记录
- 安全警报

## API 接口

MCPMan Proxy 提供了 RESTful API 用于管理和监控 MCP 服务器，详见 [API 接口文档](./src/api/README.md)

## 技术架构

MCPMan 使用以下技术栈：

- **后端**：Rust 语言，基于 [Model Context Protocol Rust SDK](https://github.com/modelcontextprotocol/rust-sdk)
- **前端**：计划使用 Tauri2 框架 + React
- **数据存储**：本地配置文件 + 可选的云同步
- **通信**：RESTful API + WebSocket

## 未来计划

我们的开发路线图包括：

1. **核心代理功能完善**：增强 MCPMan Proxy 的稳定性、性能和功能
2. **桌面应用开发**：构建 MCPMan Desktop 应用，提供图形化界面
3. **安全审计增强**：开发 MCPMan Inspector，提供更强大的安全审计功能
4. **场景预设与智能切换**：实现基于上下文的自动配置切换
5. **团队协作功能**：添加配置共享、角色访问控制等团队功能
6. **云同步与多设备支持**：实现配置的云端同步和多设备支持

## 贡献

欢迎贡献代码、报告问题或提出建议。请通过 GitHub Issues 或 Pull Requests 提交你的贡献。

## 许可证

本项目采用 [MIT 许可证](LICENSE)。
