# 通知系统

## 功能概述

通知系统是 MCPMate 的一个关键组件，它允许系统在工具列表变更时通知下游客户端。这个功能对于保持客户端工具列表的最新状态非常重要，特别是在启用或禁用服务时。

## 实现状态

✅ **基础功能已完成**
⚠️ **客户端自动刷新待测试**

通知系统的基础功能已经实现，包括：

- 提供 API 端点接收工具列表变更请求
- 与服务管理功能集成，根据请求启用或禁用服务
- 向下游客户端发送工具列表变更通知

但是，客户端自动刷新功能还需要进一步测试，因为不同的客户端可能有不同的行为。

## 技术细节

### 通知流程

通知系统的工作流程如下：

1. 系统接收工具列表变更请求（通过 API 端点）
2. 根据请求启用或禁用相应的服务
3. 向下游客户端发送工具列表变更通知
4. 客户端收到通知后，重新获取工具列表

### API 端点

通知系统提供了以下 API 端点：

- **工具列表变更通知**：`POST /api/notifications/tools/changed`
  - 接收工具列表变更请求
  - 根据请求启用或禁用服务
  - 向下游客户端发送工具列表变更通知

### 请求格式

工具列表变更请求的格式如下：

```json
{
  "operation": "Enable|Disable|Update",
  "scope": "All|Services|Tools",
  "service_ids": ["service1", "service2"],
  "tools": [
    {
      "name": "tool1",
      "service_id": "service1"
    }
  ],
  "reason": "Optional reason for the change"
}
```

参数说明：
- `operation`：操作类型，可以是 `Enable`、`Disable` 或 `Update`
- `scope`：操作范围，可以是 `All`（所有服务）、`Services`（指定服务）或 `Tools`（指定工具）
- `service_ids`：当 `scope` 为 `Services` 时，指定要操作的服务 ID 列表
- `tools`：当 `scope` 为 `Tools` 时，指定要操作的工具列表
- `reason`：操作原因，可选

### 实现细节

通知系统的实现涉及以下几个关键组件：

1. **通知处理器**：`src/api/handlers/notification.rs`
   - 处理工具列表变更请求
   - 调用服务管理功能启用或禁用服务
   - 向下游客户端发送通知

2. **服务集成**：`src/sse/server.rs`
   - 实现 `notify_tool_list_changed` 方法
   - 向下游客户端发送工具列表变更通知

3. **Bridge 集成**：`src/bin/bridge.rs`
   - 实现 `on_tool_list_changed` 方法
   - 处理来自上游服务的工具列表变更通知
   - 向下游客户端转发通知

## 使用方法

### 发送工具列表变更通知

```bash
curl -X POST http://localhost:8000/api/notifications/tools/changed \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "Disable",
    "scope": "Services",
    "service_ids": ["firecrawl"],
    "reason": "Testing service disable"
  }'
```

响应示例：
```json
{
  "notified_clients": 2,
  "message": "Notified 2 clients about tools list change",
  "details": {
    "operation": "Disable",
    "scope": "Services",
    "services_affected": 1,
    "tools_affected": 6
  }
}
```

## 限制和注意事项

- 目前只支持服务级别的操作，工具级别的操作尚未实现
- 不同的客户端可能有不同的行为，有些可能不会自动刷新工具列表
- 通知系统目前不支持持久化，重启应用后会丢失通知状态

## 未来计划

- 实现工具级别的通知
- 改进客户端自动刷新机制
- 实现通知状态的持久化
- 提供更多的通知类型，如服务状态变更、工具配置变更等
