# MCPMate

MCPMate 是一个综合性的 Model Context Protocol (MCP) 管理中心，旨在解决 MCP 生态系统中的配置复杂性、资源消耗、安全风险等问题，为用户提供统一的管理平台。

## 项目背景与愿景

随着 MCP 生态系统的快速发展，越来越多的开发者和创作者开始在多个工具（如 Claude Desktop、Cursor、Zed、Cherry Studio 等）中使用 MCP 服务来增强 AI 助手的能力。然而，这种分散的使用方式带来了显著的挑战：

- **配置复杂且重复**：需要在多个客户端中重复配置相同的 MCP 服务器
- **高昂的上下文切换成本**：不同的工作场景需要频繁切换 MCP 服务器配置
- **资源消耗与管理困难**：同时运行多个 MCP 服务器会消耗大量系统资源
- **安全风险与监控缺失**：配置错误或安全风险难以被及时发现
- **缺乏统一管理**：需要在多个工具间切换来管理不同的 MCP 服务

MCPMate 旨在通过集中化的配置管理、智能化的服务调度和增强的安全防护，解决这些问题，大幅提高易用性，减少用户的手动配置负担，并为团队协作提供支持。

## 核心组件

### Proxy

项目的核心组件之一是 `proxy`，这是一个高性能的 MCP 代理服务器，能够：

- 连接到多个 MCP 服务器并聚合它们的工具
- 提供统一的接口给 AI 客户端
- 支持多种传输协议（SSE、Streamable HTTP 或统一模式）
- 实时监控和审计 MCP 通信
- 检测潜在的安全风险（如工具投毒）
- 智能管理服务器资源
- 支持多实例管理
- 提供 RESTful API 用于管理和监控

### Bridge

`bridge` 是一个轻量级的桥接组件，用于将stdio模式的MCP客户端（如Claude Desktop）连接到HTTP模式的MCPMate代理服务器：

- 将stdio协议转换为HTTP协议（支持SSE和Streamable HTTP），无需修改客户端
- 自动继承HTTP服务的所有功能和工具
- 极简设计，只需配置服务地址
- 适用于仅支持stdio模式的客户端（如Claude Desktop）

## 配置管理

MCPMate 现已采用以数据库为核心的配置管理系统，围绕“配置套装（Config Suit）”概念展开。所有服务器、工具和配置套装信息均存储在本地 SQLite 数据库（`config/mcpmate.db`）中。这种方式支持灵活、动态和持久化的服务与工具管理，具备多套装激活、场景切换、团队协作等高级能力。

### 关键概念

- **配置套装（Config Suit）**：配置套装是一组为特定场景或应用定制的 MCP 服务器与工具集合。用户可创建、激活和切换多个配置套装，无需重启 MCPMate 即可动态变更可用服务和工具。
- **数据库存储**：所有配置信息均以结构化表（如 `server_config`、`server_args`、`config_suit` 等）形式存储于 SQLite 数据库中。不建议直接编辑数据库，请通过 API 管理。
- **API 驱动管理**：所有配置操作（如创建、更新、启用/禁用服务器和工具、管理配置套装等）均通过 RESTful API 完成。详见 [API 文档](./src/api/README.md)。
- **mcp.json 兼容说明**：`mcp.json` 文件现仅用于首次迁移或兼容旧版本。首次启动时，如数据库为空且存在 mcp.json，MCPMate 会自动迁移其内容到数据库。后续配置请通过数据库和 API 管理。

#### 示例：通过 API 创建新 MCP 服务器

添加新 MCP 服务器可使用如下 API 接口：

```http
POST /api/mcp/servers
Content-Type: application/json

{
  "name": "python-server",
  "kind": "stdio", // 或 "sse", "streamable_http"
  "command": "python", // stdio 服务器用
  "url": "http://example.com/sse", // sse/streamable_http 服务器用
  "args": ["-m", "mcp_server"],
  "env": { "DEBUG": "true" },
  "enabled": true
}
```

更多关于配置套装和 API 用法，详见 [配置管理](./docs/features/configuration_management.md) 和 [API 文档](./src/api/README.md)。

### MCPMate Desktop

计划中的 MCPMate Desktop 是一个基于 Tauri2 框架的跨平台桌面应用，将提供：

- 图形化界面，用于管理 MCP 服务器
- 场景预设与一键切换功能
- 智能推荐与引导
- 配置模板与版本控制
- 跨设备同步
- 实时监控与审计
- 安全风险检测

### MCPMate Inspector

计划中的 MCPMate Inspector 是一个安全审计组件，将提供：

- 实时监控 MCP 通信
- 检测工具投毒等安全风险
- 敏感数据检测
- 完整日志记录
- 安全警报

## API 接口

MCPMate Proxy 提供了 RESTful API 用于管理和监控 MCP 服务器，详见 [API 接口文档](./src/api/README.md)

## 技术架构

MCPMate 使用以下技术栈：

- **后端**：Rust 语言，基于 [Model Context Protocol Rust SDK](https://github.com/modelcontextprotocol/rust-sdk)
- **前端**：计划使用 Tauri2 框架 + React
- **数据存储**：本地配置文件 + 可选的云同步
- **通信**：RESTful API + WebSocket

## 未来计划

我们的开发路线图包括：

1. **核心代理功能完善**：增强 MCPMate Proxy 的稳定性、性能和功能
2. **桌面应用开发**：构建 MCPMate Desktop 应用，提供图形化界面
3. **安全审计增强**：开发 MCPMate Inspector，提供更强大的安全审计功能
4. **场景预设与智能切换**：实现基于上下文的自动配置切换
5. **团队协作功能**：添加配置共享、角色访问控制等团队功能
6. **云同步与多设备支持**：实现配置的云端同步和多设备支持

## 贡献

欢迎贡献代码、报告问题或提出建议。请通过 GitHub Issues 或 Pull Requests 提交你的贡献。

## 许可证

本项目采用 [MIT 许可证](LICENSE)。
