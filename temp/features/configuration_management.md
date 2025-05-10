# 配置管理功能

## 功能概述

配置管理功能允许用户通过统一的接口管理 MCPMate 的各种配置，包括服务配置、工具配置和系统配置。这个功能对于系统管理和维护非常重要，使用户能够：

1. 查看和修改服务配置
2. 查看和修改工具配置
3. 查看和修改系统配置
4. 导入和导出配置
5. 备份和恢复配置

## 实现状态

✅ **大部分完成**

配置管理功能目前大部分实现，具有以下特性：

- 服务配置已经使用 SQLite 数据库实现持久化
- 工具配置已经使用 SQLite 数据库实现持久化
- 配置套装系统已经实现，支持多套配置
- 使用 UUID 作为数据库主键，提高可扩展性
- 系统配置目前使用 `.env` 文件

🚧 **计划中**

- 初始化默认配置套装（default suit）
- 支持多配置套装选择，实现服务器和工具的去重和合并
- 完善工具前缀名功能，自动生成和管理前缀名
- 统一配置管理接口
- 将系统配置迁移到数据库
- 提供配置导入/导出功能
- 提供配置备份/恢复功能

## 当前配置文件

### 服务配置

服务配置已经使用 SQLite 数据库实现持久化，数据库文件位于 `config/mcpmate.db`，主要表结构如下：

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

**服务器参数表 (server_args)**
```sql
CREATE TABLE IF NOT EXISTS server_args (
    id TEXT PRIMARY KEY,               -- UUID 字符串
    server_id TEXT NOT NULL,           -- 关联到 server_config 的 id
    arg_index INTEGER NOT NULL,        -- 参数在数组中的位置
    arg_value TEXT NOT NULL,           -- 参数值
    FOREIGN KEY (server_id) REFERENCES server_config(id) ON DELETE CASCADE,
    UNIQUE(server_id, arg_index)
)
```

**服务器环境变量表 (server_env)**
```sql
CREATE TABLE IF NOT EXISTS server_env (
    id TEXT PRIMARY KEY,               -- UUID 字符串
    server_id TEXT NOT NULL,           -- 关联到 server_config 的 id
    env_key TEXT NOT NULL,             -- 环境变量名
    env_value TEXT NOT NULL,           -- 环境变量值
    FOREIGN KEY (server_id) REFERENCES server_config(id) ON DELETE CASCADE,
    UNIQUE(server_id, env_key)
)
```

**服务器元数据表 (server_meta)**
```sql
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
)
```

### 配置套装系统

配置套装系统已经使用 SQLite 数据库实现持久化，数据库文件位于 `config/mcpmate.db`，主要表结构如下：

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

### 系统配置

系统配置目前使用 `.env` 文件，包含一些环境变量：

```
PORT=8000
API_PORT=8080
LOG_LEVEL=info
```

## 未来计划

### 配置套装功能完善

计划完善配置套装功能，包括：

1. **初始化默认配置套装**：
   - 在数据库初始化时自动创建默认配置套装
   - 确保所有服务器和工具都有对应的配置

2. **多套装支持**：
   - 修改 `get_enabled_servers` 函数，支持传入多个配置套装 ID
   - 实现服务器和工具的去重和合并
   - 当没有指定配置套装时，使用默认配置套装

3. **工具前缀名功能**：
   - 实现工具前缀名的自动生成和管理
   - 确保不同服务器的同名工具可以被正确区分

4. **系统配置表**：
```sql
CREATE TABLE IF NOT EXISTS system_config (
    id TEXT PRIMARY KEY,               -- UUID 字符串
    key TEXT NOT NULL UNIQUE,          -- 配置键
    value TEXT NOT NULL,               -- 配置值
    description TEXT,                  -- 配置描述
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
)
```

### 初始化流程

已经实现以下初始化流程：

1. ✅ 检查数据库是否存在，如果不存在则创建
2. ✅ 检查表结构是否完整，如果不完整则创建或更新
3. ✅ 检查配置文件是否存在，如果存在则导入到数据库
4. ✅ 如果数据库和配置文件都不存在，则使用默认配置初始化数据库

计划完善以下初始化流程：

1. 🔄 初始化默认配置套装（default suit）
2. 🔄 自动为所有服务器和工具创建配置套装关联
3. 🔄 自动生成工具前缀名，解决同名工具冲突问题

### API 端点

已经实现以下 API 端点：

1. **服务器管理**：
   - ✅ `GET /api/mcp/servers` - 获取所有服务器
   - ✅ `GET /api/mcp/servers/{name}` - 获取特定服务器
   - ✅ `POST /api/mcp/servers` - 创建新服务器
   - ✅ `PUT /api/mcp/servers/{name}` - 更新服务器
   - ✅ `DELETE /api/mcp/servers/{name}` - 删除服务器
   - ✅ `POST /api/mcp/servers/{name}/enable` - 启用服务器
   - ✅ `POST /api/mcp/servers/{name}/disable` - 禁用服务器

2. **服务器实例管理**：
   - ✅ `GET /api/mcp/servers/{name}/instances` - 获取服务器实例列表
   - ✅ `GET /api/mcp/servers/{name}/instances/{id}` - 获取特定实例详情
   - ✅ `POST /api/mcp/servers/{name}/instances` - 创建新实例
   - ✅ `DELETE /api/mcp/servers/{name}/instances/{id}` - 删除实例
   - ✅ `POST /api/mcp/servers/{name}/instances/{id}/reconnect` - 重新连接实例
   - ✅ `POST /api/mcp/servers/{name}/instances/{id}/disconnect` - 断开实例连接

3. **工具管理**：
   - ✅ `GET /api/mcp/tools` - 获取所有工具
   - ✅ `GET /api/mcp/tools/{server_name}/{tool_name}` - 获取特定工具
   - ✅ `POST /api/mcp/tools/{server_name}/{tool_name}/enable` - 启用工具
   - ✅ `POST /api/mcp/tools/{server_name}/{tool_name}/disable` - 禁用工具

计划提供以下 API 端点：

1. **配置套装管理**：
   - 🔄 `GET /api/mcp/suits` - 获取所有配置套装
   - 🔄 `GET /api/mcp/suits/{name}` - 获取特定配置套装
   - 🔄 `POST /api/mcp/suits` - 创建新配置套装
   - 🔄 `PUT /api/mcp/suits/{name}` - 更新配置套装
   - 🔄 `DELETE /api/mcp/suits/{name}` - 删除配置套装

2. **配置套装服务器关联**：
   - 🔄 `GET /api/mcp/suits/{name}/servers` - 获取配置套装中的服务器
   - 🔄 `POST /api/mcp/suits/{name}/servers` - 添加服务器到配置套装
   - 🔄 `PUT /api/mcp/suits/{name}/servers/{server_name}` - 更新服务器在配置套装中的状态
   - 🔄 `DELETE /api/mcp/suits/{name}/servers/{server_name}` - 从配置套装中移除服务器

3. **配置套装工具关联**：
   - 🔄 `GET /api/mcp/suits/{name}/tools` - 获取配置套装中的工具
   - 🔄 `POST /api/mcp/suits/{name}/tools` - 添加工具到配置套装
   - 🔄 `PUT /api/mcp/suits/{name}/tools/{tool_name}` - 更新工具在配置套装中的状态
   - 🔄 `DELETE /api/mcp/suits/{name}/tools/{tool_name}` - 从配置套装中移除工具

4. **系统配置**：
   - 🔄 `GET /api/mcp/system/config` - 获取所有系统配置
   - 🔄 `GET /api/mcp/system/config/{key}` - 获取特定系统配置
   - 🔄 `POST /api/mcp/system/config/{key}` - 更新系统配置
   - 🔄 `DELETE /api/mcp/system/config/{key}` - 删除系统配置

5. **配置导入/导出**：
   - 🔄 `GET /api/mcp/system/export` - 导出所有配置
   - 🔄 `POST /api/mcp/system/import` - 导入配置

### UI 界面

计划提供 UI 界面进行配置管理，包括：

1. 服务器管理界面
   - 服务器列表
   - 服务器详情
   - 服务器创建/编辑/删除
   - 服务器启用/禁用
   - 服务器实例管理

2. 工具管理界面
   - 工具列表
   - 工具详情
   - 工具启用/禁用
   - 工具前缀名管理

3. 配置套装管理界面
   - 配置套装列表
   - 配置套装详情
   - 配置套装创建/编辑/删除
   - 配置套装服务器关联管理
   - 配置套装工具关联管理

4. 系统配置界面
   - 系统配置列表
   - 系统配置编辑
   - 配置导入/导出
   - 配置备份/恢复

## 限制和注意事项

- 配置变更可能需要重启应用才能生效
- 配置导入可能会覆盖现有配置，需要谨慎操作
- 配置备份应该定期进行，以防配置丢失
- 敏感信息（如 API 密钥）应该加密存储
- 目前只支持默认配置套装，未来将支持多个配置套装
- 工具前缀名功能尚未完全实现，可能导致同名工具冲突
- 数据库使用 UUID 作为主键，提高了可扩展性，但也增加了复杂性
