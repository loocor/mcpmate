# 服务管理功能

## 功能概述

服务管理功能允许用户通过 API 启用或禁用特定的上游服务。这个功能对于资源管理和工具可用性控制非常重要，使用户能够：

1. 启用需要的服务，使其工具可用
2. 禁用不需要的服务，释放系统资源
3. 通过通知系统通知客户端工具列表的变化

## 实现状态

✅ **已完成**

服务管理功能已经完全实现，具有以下特性：

- 提供 API 端点启用和禁用服务
- 支持服务级别的操作（启用/禁用整个服务）
- 支持通过 `config/rule.json5` 配置服务的默认启用状态

✅ **已完成**

- 与通知系统集成，发送工具列表变更通知
- 服务配置持久化（已迁移到 SQLite 数据库）
- 使用 UUID 作为数据库主键，提高可扩展性

## 技术细节

### API 端点

服务管理功能提供了以下 API 端点：

1. **启用服务**：`POST /api/mcp/servers/{name}/enable`
   - 启用指定的服务，使其工具可用
   - 通过重新连接服务的默认实例实现

2. **禁用服务**：`POST /api/mcp/servers/{name}/disable`
   - 禁用指定的服务，使其工具不可用
   - 通过断开服务的所有实例连接实现

3. **通知工具列表变更**：`POST /api/notifications/tools/changed`
   - 接收工具列表变更请求
   - 根据请求启用或禁用服务
   - 向下游客户端发送工具列表变更通知

### 实现方法

服务管理功能的实现采用了模块化和代码复用的方法：

1. **服务启用**：
   - 检查服务是否存在
   - 获取服务的默认实例
   - 调用 `reset_reconnect` 方法重新连接实例
   - 返回操作结果

2. **服务禁用**：
   - 检查服务是否存在
   - 获取服务的所有实例
   - 对每个实例调用 `force_disconnect` 方法断开连接
   - 返回操作结果

3. **通知集成**：
   - 接收工具列表变更请求
   - 根据请求的操作类型和作用域启用或禁用服务
   - 向下游客户端发送工具列表变更通知

### 配置持久化

服务的启用/禁用状态现在通过 SQLite 数据库进行管理，使用配置套装（Config Suit）系统。主要表结构如下：

**服务器配置表 (server_config)**
```sql
CREATE TABLE IF NOT EXISTS server_config (
    id TEXT PRIMARY KEY,               -- UUID 字符串
    name TEXT NOT NULL UNIQUE,         -- 服务器名称，如 "firecrawl"
    server_type TEXT NOT NULL,         -- 服务器类型，如 "stdio", "sse"
    command TEXT,                      -- 对于 stdio 类型服务器的命令
    url TEXT,                          -- 对于 sse 类型服务器的 URL
    transport_type TEXT,               -- 传输类型，如 "Stdio", "Sse", "StreamableHttp"
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
)
```

**配置套装表 (config_suit)**
```sql
CREATE TABLE IF NOT EXISTS config_suit (
    id TEXT PRIMARY KEY,               -- UUID 字符串
    name TEXT NOT NULL UNIQUE,         -- 套装名称，如 "default", "cursor", "claude"
    description TEXT,                  -- 套装描述
    type TEXT NOT NULL,                -- 套装类型：'host_app', 'scenario', 'shared'
    multi_select BOOLEAN NOT NULL DEFAULT 0, -- 是否支持多选
    priority INTEGER NOT NULL DEFAULT 0, -- 优先级
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
)
```

**配置套装-服务器关联表 (config_suit_server)**
```sql
CREATE TABLE IF NOT EXISTS config_suit_server (
    id TEXT PRIMARY KEY,               -- UUID 字符串
    config_suit_id TEXT NOT NULL,      -- 关联到 config_suit 的 id
    server_id TEXT NOT NULL,           -- 关联到 server_config 的 id
    enabled BOOLEAN NOT NULL DEFAULT 1, -- 是否启用
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (config_suit_id) REFERENCES config_suit(id) ON DELETE CASCADE,
    FOREIGN KEY (server_id) REFERENCES server_config(id) ON DELETE CASCADE,
    UNIQUE(config_suit_id, server_id)
)
```

服务的运行时启用/禁用状态现在会持久化到数据库中，重启应用后会保持之前的状态。系统默认使用名为 "default" 的配置套装，但未来将支持多个配置套装，以适应不同的使用场景。

## 使用方法

### 启用服务

```bash
curl -X POST http://localhost:8080/api/mcp/servers/firecrawl/enable
```

响应示例：
```json
{
  "id": "instance-1",
  "name": "firecrawl",
  "result": "Successfully enabled server",
  "status": "Ready",
  "allowed_operations": ["disable"]
}
```

### 禁用服务

```bash
curl -X POST http://localhost:8080/api/mcp/servers/firecrawl/disable
```

响应示例：
```json
{
  "id": "instance-1",
  "name": "firecrawl",
  "result": "Successfully disabled server",
  "status": "Shutdown",
  "allowed_operations": ["enable"]
}
```

### 发送工具列表变更通知

```bash
curl -X POST http://localhost:8080/api/notifications/tools/changed \
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

- 服务级别的操作会影响该服务下的所有工具
- 某些客户端可能不会自动刷新工具列表，需要手动刷新
- 目前只支持默认配置套装，未来将支持多个配置套装
- 工具前缀名功能尚未完全实现，可能导致同名工具冲突

## 未来计划

- ✅ 将服务配置和工具配置统一到数据库中管理
- ✅ 实现服务运行时状态的持久化
- ✅ 使用 UUID 作为数据库主键，提高可扩展性
- 🔄 初始化默认配置套装（default suit）
- 🔄 支持多配置套装选择，实现服务器和工具的去重和合并
- 🔄 实现工具前缀名功能，解决同名工具冲突问题
- 🔄 提供 UI 界面进行服务管理
- 🔄 提供更多的服务管理功能，如重启服务、查看服务日志等
- 🔄 完善通知系统集成，实时通知客户端服务状态变更
