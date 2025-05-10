# Stdio Bridge 功能

## 功能概述

Stdio Bridge 是 MCPMate 的一个关键组件，它允许仅支持 stdio 模式的客户端（如 Claude Desktop）连接到 HTTP 模式的上游服务。Bridge 组件充当双重角色：

1. 作为 HTTP 客户端连接到上游服务（支持 SSE 和 Streamable HTTP 协议）
2. 作为 stdio 服务器向下游客户端提供服务

## 实现状态

✅ **已完成**

Bridge 组件已经完全实现，具有以下功能：

- 通过 stdio 模式接收来自客户端（如 Claude Desktop）的请求
- 将请求转发到上游 HTTP 服务（支持 SSE 和 Streamable HTTP 协议）
- 将上游服务的响应返回给客户端
- 支持工具列表变更通知的转发

## 技术细节

### 架构

Bridge 组件的架构如下：

```
Claude Desktop (stdio) <--> Bridge (stdio/HTTP) <--> MCPMate Proxy (HTTP) <--> 上游服务
```

其中 HTTP 可以是 SSE 或 Streamable HTTP 协议。

### 关键组件

1. **BridgeClient**：连接到上游 HTTP 服务的客户端
   - 实现了 `ClientHandler` trait
   - 处理来自上游服务的通知，特别是工具列表变更通知
   - 支持 SSE 和 Streamable HTTP 协议

2. **BridgeServer**：向下游客户端提供 stdio 服务的服务器
   - 实现了 `ServerHandler` trait
   - 将请求转发到上游服务
   - 将上游服务的响应返回给客户端

### 通知处理

Bridge 组件实现了工具列表变更通知的处理：

1. 当上游服务发送工具列表变更通知时，`BridgeClient` 的 `on_tool_list_changed` 方法会被调用
2. 该方法设置一个标志，表示工具列表已更改
3. 当客户端调用 `list_tools` 或 `call_tool` 方法时，Bridge 会检查该标志
4. 如果标志为 true，Bridge 会向客户端发送工具列表变更通知

## 使用方法

Bridge 组件可以通过以下命令启动：

```bash
cargo run --bin bridge -- --url http://127.0.0.1:8000/mcp
```

参数说明：
- `--url`：上游服务的 URL，默认为 `http://127.0.0.1:8000/mcp`（Streamable HTTP）
- `--sse-url`：上游 SSE 服务的 URL，如果使用 SSE 协议
- `--log-level`：日志级别，默认为 `info`

## 限制和注意事项

- Bridge 组件目前只支持一个上游服务（SSE 或 Streamable HTTP）
- 如果上游服务不可用，Bridge 会报告错误，但不会自动重试
- Bridge 组件不会缓存工具列表，每次请求都会从上游服务获取
- 某些客户端（如 Cursor）可能不会自动刷新工具列表，需要手动刷新

## 未来计划

- 支持多个上游服务
- 实现工具级别的过滤和修改
- 添加自动重连机制
- 实现工具列表缓存
