# 工具管理功能

## 功能概述

工具管理功能允许用户通过 API 启用或禁用特定的 MCP 工具。这个功能对于资源管理和工具可用性控制非常重要，使用户能够：

1. 启用需要的工具，使其可用
2. 禁用不需要的工具，释放系统资源
3. 通过通知系统通知客户端工具列表的变化

## 实现状态

✅ **已完成**

工具管理功能已经完全实现，具有以下特性：

- 提供 API 端点启用和禁用工具
- 支持工具级别的操作（启用/禁用特定工具）
- 使用 SQLite 数据库持久化工具配置
- 支持工具别名设置，允许用户自定义工具名称
- 提供详细的日志记录，便于调试和故障排除

✅ **已完成**

- 与通知系统集成，发送工具列表变更通知
- 使用 UUID 作为数据库主键，提高可扩展性
- 添加工具前缀名功能，解决同名工具冲突问题

## 技术细节

### API 端点

工具管理功能提供了以下 API 端点：

1. **列出所有工具**：`GET /api/mcp/tools`
   - 返回所有工具的配置信息
   - 包括数据库中已有记录的工具和尚未在数据库中有记录的工具
   - 未记录的工具默认为启用状态

2. **获取工具配置**：`GET /api/mcp/tools/{server_name}/{tool_name}`
   - 获取特定工具的配置信息
   - 返回工具的详细配置，包括 ID、名称、别名、启用状态和时间戳

3. **启用工具**：`POST /api/mcp/tools/{server_name}/{tool_name}/enable`
   - 启用指定的工具，使其可用
   - 保留现有的别名设置（如果有）

4. **禁用工具**：`POST /api/mcp/tools/{server_name}/{tool_name}/disable`
   - 禁用指定的工具，使其不可用
   - 保留现有的别名设置（如果有）

5. **更新工具配置**：`POST /api/mcp/tools/{server_name}/{tool_name}`
   - 更新工具的配置信息
   - 支持更新工具的启用状态和别名

### 实现方法

工具管理功能的实现采用了模块化和代码复用的方法：

1. **数据库持久化**：
   - 使用 SQLite 数据库存储工具配置
   - 提供 CRUD 操作接口
   - 自动创建数据库和表结构

2. **工具启用/禁用**：
   - 在数据库中记录工具的启用/禁用状态
   - 在工具列表和工具调用时检查工具状态
   - 过滤禁用的工具，不返回给客户端

3. **通知集成**：
   - 工具状态变更时发送通知
   - 客户端可以刷新工具列表

### 数据库结构

工具配置现在使用配置套装（Config Suit）系统进行管理，主要表结构如下：

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

**配置套装-工具关联表 (config_suit_tool)**
```sql
CREATE TABLE IF NOT EXISTS config_suit_tool (
    id TEXT PRIMARY KEY,               -- UUID 字符串
    config_suit_id TEXT NOT NULL,      -- 关联到 config_suit 的 id
    server_id TEXT NOT NULL,           -- 关联到 server_config 的 id
    tool_name TEXT NOT NULL,           -- 工具名称
    prefixed_name TEXT,                -- 前缀名称，用于区分不同服务器的同名工具
    enabled BOOLEAN NOT NULL DEFAULT 1, -- 是否启用
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (config_suit_id) REFERENCES config_suit(id) ON DELETE CASCADE,
    FOREIGN KEY (server_id) REFERENCES server_config(id) ON DELETE CASCADE,
    UNIQUE(config_suit_id, server_id, tool_name)
)
```

表字段说明：
- `id`：UUID 字符串，唯一标识每条记录
- `config_suit_id`：配置套装 ID，关联到 config_suit 表
- `server_id`：服务器 ID，关联到 server_config 表
- `tool_name`：工具名称，如 "firecrawl_scrape"、"playwright_screenshot" 等
- `prefixed_name`：工具前缀名，用于区分不同服务器的同名工具，如 "firecrawl:scrape"
- `enabled`：工具是否启用，1 表示启用，0 表示禁用
- `created_at`：记录创建时间
- `updated_at`：记录更新时间

配置套装 ID、服务器 ID 和工具名称的组合必须唯一，这是通过 `UNIQUE(config_suit_id, server_id, tool_name)` 约束实现的。

## 使用方法

### 列出所有工具

```bash
curl -X GET http://localhost:8080/api/mcp/tools
```

响应示例：
```json
{
  "tools": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "server_name": "firecrawl",
      "tool_name": "firecrawl_scrape",
      "prefixed_name": "firecrawl:scrape",
      "enabled": true,
      "created_at": "2025-05-08T17:44:20+00:00",
      "updated_at": "2025-05-08T18:20:10+00:00"
    },
    {
      "id": "550e8400-e29b-41d4-a716-446655440001",
      "server_name": "playwright",
      "tool_name": "playwright_screenshot",
      "prefixed_name": null,
      "enabled": false,
      "created_at": "2025-05-08T17:54:26+00:00",
      "updated_at": "2025-05-08T17:54:26+00:00"
    },
    {
      "id": null,
      "server_name": "fetch",
      "tool_name": "fetch",
      "prefixed_name": null,
      "enabled": true,
      "created_at": null,
      "updated_at": null
    }
  ]
}
```

### 获取工具配置

```bash
curl -X GET http://localhost:8080/api/mcp/tools/firecrawl/firecrawl_scrape
```

响应示例：
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "server_name": "firecrawl",
  "tool_name": "firecrawl_scrape",
  "prefixed_name": "firecrawl:scrape",
  "enabled": true,
  "created_at": "2025-05-08T17:44:20+00:00",
  "updated_at": "2025-05-08T18:20:10+00:00",
  "allowed_operations": ["disable"]
}
```

### 启用工具

```bash
curl -X POST http://localhost:8080/api/mcp/tools/firecrawl/firecrawl_scrape/enable
```

响应示例：
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "server_name": "firecrawl",
  "tool_name": "firecrawl_scrape",
  "result": "Successfully enabled tool",
  "status": "Enabled",
  "allowed_operations": ["disable"]
}
```

### 禁用工具

```bash
curl -X POST http://localhost:8080/api/mcp/tools/firecrawl/firecrawl_scrape/disable
```

响应示例：
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "server_name": "firecrawl",
  "tool_name": "firecrawl_scrape",
  "result": "Successfully disabled tool",
  "status": "Disabled",
  "allowed_operations": ["enable"]
}
```

### 更新工具配置

```bash
curl -X POST http://localhost:8080/api/mcp/tools/firecrawl/firecrawl_scrape \
  -H "Content-Type: application/json" \
  -d '{"enabled": true, "alias_name": "Web Scraper"}'
```

响应示例：
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "server_name": "firecrawl",
  "tool_name": "firecrawl_scrape",
  "prefixed_name": "firecrawl:scrape",
  "enabled": true,
  "created_at": "2025-05-08T17:44:20+00:00",
  "updated_at": "2025-05-08T18:20:10+00:00",
  "allowed_operations": ["disable"]
}
```

## 限制和注意事项

- 工具配置在应用启动时自动创建数据库和表结构
- 数据库文件位于 `config/mcpmate.db`
- 默认情况下，所有工具都是启用的
- 数据库中没有记录的工具会显示为启用状态，ID 为 null
- 工具状态变更后，客户端可能需要重新连接才能看到变化
- 数据库支持自动迁移，会自动添加新的列
- 目前只支持默认配置套装，未来将支持多个配置套装
- 工具前缀名功能尚未完全实现，可能导致同名工具冲突

## 未来计划

- ✅ 将工具配置统一到数据库中管理
- ✅ 实现工具运行时状态的持久化
- ✅ 使用 UUID 作为数据库主键，提高可扩展性
- ✅ 添加工具前缀名功能，解决同名工具冲突问题
- 🔄 初始化默认配置套装（default suit）
- 🔄 支持多配置套装选择，实现服务器和工具的去重和合并
- 🔄 完善工具前缀名功能，自动生成和管理前缀名
- 🔄 实现批量工具启用/禁用
- 🔄 添加更多工具配置选项（如超时、重试等）
- 🔄 提供更详细的工具使用统计
- 🔄 完善通知系统集成，实时通知客户端工具列表变更
- 🔄 提供 UI 界面进行工具管理
