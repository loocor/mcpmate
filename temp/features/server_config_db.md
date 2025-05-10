# MCP 服务器配置数据库管理设计

## 背景

目前，MCPMate 使用 `config/mcp.json` 文件管理 MCP 服务器配置。这种基于文件的配置管理方式存在一些限制：

1. 不支持运行时动态更新
2. 缺乏版本控制和历史记录
3. 敏感信息（如 API 密钥）没有特殊保护
4. 配置分散在多个文件中，不便于统一管理

为了解决这些问题，我们计划将 MCP 服务器配置纳入数据库管理，提供更灵活、更安全的配置管理方式。

## 数据模型设计

### 服务器配置表

```sql
-- 服务器基本信息表
CREATE TABLE IF NOT EXISTS server_config (
    id TEXT PRIMARY KEY,               -- UUID 字符串
    name TEXT NOT NULL UNIQUE,         -- 服务器名称，如 "firecrawl"
    server_type TEXT NOT NULL,         -- 服务器类型，如 "stdio", "sse"
    url TEXT,                          -- 对于 sse 类型服务器的 URL
    command TEXT,                      -- 对于 stdio 类型服务器的命令
    transport_type TEXT,               -- 传输类型，如 "Stdio", "Sse", "StreamableHttp"
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 服务器参数表（用于存储 args 数组）
CREATE TABLE IF NOT EXISTS server_args (
    id TEXT PRIMARY KEY,               -- UUID 字符串
    server_id TEXT NOT NULL,           -- 关联到 server_config 的 id
    arg_index INTEGER NOT NULL,        -- 参数在数组中的位置
    arg_value TEXT NOT NULL,           -- 参数值
    FOREIGN KEY (server_id) REFERENCES server_config(id) ON DELETE CASCADE,
    UNIQUE(server_id, arg_index)
);

-- 服务器环境变量表（用于存储 env 对象）
CREATE TABLE IF NOT EXISTS server_env (
    id TEXT PRIMARY KEY,               -- UUID 字符串
    server_id TEXT NOT NULL,           -- 关联到 server_config 的 id
    env_key TEXT NOT NULL,             -- 环境变量名
    env_value TEXT NOT NULL,           -- 环境变量值
    FOREIGN KEY (server_id) REFERENCES server_config(id) ON DELETE CASCADE,
    UNIQUE(server_id, env_key)
);

-- 服务器元数据表（记录服务器的来源信息）
CREATE TABLE IF NOT EXISTS server_meta (
    id TEXT PRIMARY KEY,               -- UUID 字符串
    server_id TEXT NOT NULL,           -- 关联到 server_config 的 id
    description TEXT,                  -- 服务器描述
    author TEXT,                       -- 作者或组织
    website TEXT,                      -- 网站 URL
    repository TEXT,                   -- 代码仓库 URL
    category TEXT,                     -- 分类
    recommended_scenario TEXT,         -- 推荐场景
    rating INTEGER,                    -- 评分
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (server_id) REFERENCES server_config(id) ON DELETE CASCADE,
    UNIQUE(server_id)
);

-- 配置套装表
CREATE TABLE IF NOT EXISTS config_suit (
    id TEXT PRIMARY KEY,               -- UUID 字符串
    name TEXT NOT NULL UNIQUE,         -- 套装名称，如 "default", "cursor", "claude"
    description TEXT,                  -- 套装描述
    type TEXT NOT NULL,                -- 套装类型：'host_app', 'scenario', 'shared'
    multi_select BOOLEAN NOT NULL DEFAULT 0, -- 是否支持多选
    priority INTEGER NOT NULL DEFAULT 0, -- 优先级
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 配置套装-服务器关联表
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
);

-- 配置套装-工具关联表
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
);
```

## 特殊字段处理

### 1. `args` 数组处理

`args` 是一个字符串数组，在 JSON 中表示为：

```json
"args": ["blender-mcp", "--verbose"]
```

在数据库中，我们使用 `server_args` 表存储这个数组：

- 每个数组元素作为一条记录
- `position` 字段保持数组顺序
- 查询时按 `position` 排序重建数组

示例：

| id                                   | server_id                            | arg_index | arg_value   |
| ------------------------------------ | ------------------------------------ | --------- | ----------- |
| 550e8400-e29b-41d4-a716-446655440000 | 123e4567-e89b-12d3-a456-426614174000 | 0         | blender-mcp |
| 550e8400-e29b-41d4-a716-446655440001 | 123e4567-e89b-12d3-a456-426614174000 | 1         | --verbose   |

### 2. `env` 对象处理

`env` 是一个键值对对象，在 JSON 中表示为：

```json
"env": {
  "FIRECRAWL_API_KEY": "fc-200cbeb18dd647818bbbc10c30e7b9c0"
}
```

在数据库中，我们使用 `server_env` 表存储这个对象：

- 每个键值对作为一条记录
- 添加 `is_secret` 标志，用于标识敏感信息
- 敏感信息可以加密存储

示例：

| id                                   | server_id                            | env_key           | env_value                           |
| ------------------------------------ | ------------------------------------ | ----------------- | ----------------------------------- |
| 550e8400-e29b-41d4-a716-446655440002 | 123e4567-e89b-12d3-a456-426614174001 | FIRECRAWL_API_KEY | fc-200cbeb18dd647818bbbc10c30e7b9c0 |

## 版本控制设计

使用 `server_config_history` 表记录所有配置变更：

1. 每次修改配置时，将变更前的完整配置（包括 args 和 env）保存为 JSON
2. 记录变更类型、变更时间和变更者
3. 支持查看历史记录和回滚到特定版本

## 服务器元数据

`server_metadata` 表用于记录服务器的元数据：

1. 来源平台（如 Glama, Anthropic 等）
2. 介绍页面 URL（如 https://glama.ai/mcp/servers/@PaddleHQ/paddle-mcp-server）
3. 作者、许可证、标签等信息

这些元数据可以用于：

1. 提高服务器的可发现性和可理解性
2. 为未来的服务器目录、搜索、分类等功能做准备
3. 支持社区功能，如评分、评论等

## API 设计

### 1. 服务器配置 API

```
GET /api/mcp/servers                    # 获取所有服务器配置
GET /api/mcp/servers/{name}             # 获取特定服务器配置
POST /api/mcp/servers                   # 创建新服务器配置
PUT /api/mcp/servers/{name}             # 更新服务器配置
DELETE /api/mcp/servers/{name}          # 删除服务器配置
```

### 2. 服务器实例 API

```
GET /api/mcp/servers/{name}/instances   # 获取服务器实例列表
GET /api/mcp/servers/{name}/instances/{id} # 获取特定实例详情
POST /api/mcp/servers/{name}/instances  # 创建新实例
DELETE /api/mcp/servers/{name}/instances/{id} # 删除实例
POST /api/mcp/servers/{name}/instances/{id}/reconnect # 重新连接实例
POST /api/mcp/servers/{name}/instances/{id}/disconnect # 断开实例连接
```

### 3. 服务器操作 API

```
POST /api/mcp/servers/{name}/enable     # 启用服务器
POST /api/mcp/servers/{name}/disable    # 禁用服务器
```

### 4. 工具管理 API

```
GET /api/mcp/tools                      # 获取所有工具
GET /api/mcp/tools/{name}               # 获取特定工具详情
POST /api/mcp/tools/{name}/enable       # 启用工具
POST /api/mcp/tools/{name}/disable      # 禁用工具
```

### 5. 配置套装 API

```
GET /api/mcp/suits                      # 获取所有配置套装
GET /api/mcp/suits/{name}               # 获取特定配置套装详情
POST /api/mcp/suits                     # 创建新配置套装
PUT /api/mcp/suits/{name}               # 更新配置套装
DELETE /api/mcp/suits/{name}            # 删除配置套装
```

### 6. 配置套装服务器关联 API

```
GET /api/mcp/suits/{name}/servers       # 获取配置套装中的服务器
POST /api/mcp/suits/{name}/servers      # 添加服务器到配置套装
PUT /api/mcp/suits/{name}/servers/{server_name} # 更新服务器在配置套装中的状态
DELETE /api/mcp/suits/{name}/servers/{server_name} # 从配置套装中移除服务器
```

### 7. 配置套装工具关联 API

```
GET /api/mcp/suits/{name}/tools         # 获取配置套装中的工具
POST /api/mcp/suits/{name}/tools        # 添加工具到配置套装
PUT /api/mcp/suits/{name}/tools/{tool_name} # 更新工具在配置套装中的状态
DELETE /api/mcp/suits/{name}/tools/{tool_name} # 从配置套装中移除工具
```

## 安全考虑

### 1. 敏感信息保护

- 环境变量中的敏感信息（如 API 密钥）应加密存储
- 提供 API 时可以选择隐藏敏感信息
- 日志中不应记录敏感信息

### 2. 访问控制

- API 应实现适当的访问控制
- 敏感操作（如删除配置、修改敏感信息）应要求额外的权限

### 3. 审计日志

- 记录所有配置变更的详细信息
- 包括变更者、变更时间、变更内容等

## 初始化与迁移流程

### 1. 首次启动

- 检查数据库是否存在，如果不存在则创建
- 检查 `config/mcp.json` 是否存在
- 如果存在，将其导入到数据库中
- 如果不存在，使用默认配置初始化数据库

### 2. 配置迁移

- 提供导入/导出功能，支持 JSON 格式
- 支持批量导入/导出
- 支持选择性导入/导出（如仅导出非敏感信息）

## 实现策略

### 1. 渐进式迁移

- ✅ 第一阶段：实现基本的数据库存储和 API
- ✅ 第二阶段：实现从旧结构到新结构的迁移
- ✅ 第三阶段：更新应用代码以使用新结构
- ✅ 第四阶段：将整数 ID 迁移到 UUID
- 🔄 第五阶段：实现配置套装功能
- 🔄 第六阶段：清理旧代码

### 2. 兼容性保证

- 保持与现有配置文件格式的兼容性
- 提供配置文件与数据库之间的双向同步
- 在迁移期间，同时支持配置文件和数据库

## UI 界面设计

### 1. 服务器配置管理

- 列表视图：显示所有服务器及其状态
- 详情视图：显示服务器详细配置
- 编辑表单：支持编辑服务器配置

### 2. 环境变量管理

- 表格视图：显示所有环境变量
- 编辑表单：支持添加/编辑环境变量
- 敏感信息处理：提供隐藏/显示敏感信息的选项

### 3. 版本控制界面

- 历史记录：显示配置变更历史
- 比较视图：比较不同版本的配置
- 回滚功能：支持回滚到特定版本

## 总结

将 MCP 服务器配置纳入数据库管理可以提供更灵活、更安全的配置管理方式。通过精心设计的数据模型和 API，我们可以实现：

1. 更灵活的配置管理，支持动态更新
2. 更好的安全性，特别是对敏感信息的保护
3. 版本控制和历史记录，支持回滚
4. 完整的元数据管理，提高可发现性和可理解性
5. 完整的 API 和 UI 界面，提供良好的用户体验

这个方案可以分阶段实施，确保平稳过渡，同时保持与现有系统的兼容性。
