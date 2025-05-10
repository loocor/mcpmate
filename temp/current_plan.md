# MCPMate 开发计划

## 当前状态

MCPMate 项目已经完成了以下关键里程碑：

1. **基础代理功能**：实现了基本的 MCP 代理功能，能够连接上游 MCP 服务器并转发请求。
2. **连接池重构**：完成了连接池的重构，提高了连接稳定性和工具映射缓存效率。
3. **资源监控**：实现了进程资源监控功能，能够跟踪和限制上游服务器进程的资源使用。
4. **多协议支持**：实现了对 SSE 和 Streamable HTTP 协议的支持，以及统一模式下的同时支持。
5. **Stdio 模式支持**：实现了 Bridge 组件，支持 stdio 模式的客户端（如 Claude Desktop）连接到 HTTP 模式的上游服务。
6. **服务管理功能**：实现了服务启用/禁用功能，允许用户通过 API 控制服务的可用性。
7. **通知系统**：实现了工具列表变更通知功能，在服务启用/禁用时通知下游客户端。

## 当前开发任务

### 配置管理重构

我们正在进行配置管理系统的重构，将原有的基于文件的配置（`config/mcp.json`和`config/rule.json5`）迁移到 SQLite 数据库中。这项重构包括以下内容：

#### 1. 数据库表结构设计

**服务器配置表 (server_config)**
```sql
CREATE TABLE IF NOT EXISTS server_config (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    server_type TEXT NOT NULL,
    command TEXT,
    url TEXT,
    transport_type TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
)
```

**服务器参数表 (server_args)**
```sql
CREATE TABLE IF NOT EXISTS server_args (
    id TEXT PRIMARY KEY,
    server_id TEXT NOT NULL,
    arg_index INTEGER NOT NULL,
    arg_value TEXT NOT NULL,
    FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE,
    UNIQUE(server_id, arg_index)
)
```

**服务器环境变量表 (server_env)**
```sql
CREATE TABLE IF NOT EXISTS server_env (
    id TEXT PRIMARY KEY,
    server_id TEXT NOT NULL,
    env_key TEXT NOT NULL,
    env_value TEXT NOT NULL,
    FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE,
    UNIQUE(server_id, env_key)
)
```

**服务器元数据表 (server_meta)**
```sql
CREATE TABLE IF NOT EXISTS server_meta (
    id TEXT PRIMARY KEY,
    server_id TEXT NOT NULL,
    description TEXT,
    author TEXT,
    website TEXT,
    repository TEXT,
    category TEXT,
    recommended_scenario TEXT,
    rating INTEGER,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE,
    UNIQUE(server_id)
)
```

**配置套装表 (config_suit)**
```sql
CREATE TABLE IF NOT EXISTS config_suit (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    type TEXT NOT NULL, -- 'host_app', 'scenario', 'shared'
    multi_select BOOLEAN NOT NULL DEFAULT 0,
    priority INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
)
```

**配置套装-服务器关联表 (config_suit_server)**
```sql
CREATE TABLE IF NOT EXISTS config_suit_server (
    id TEXT PRIMARY KEY,
    config_suit_id TEXT NOT NULL,
    server_id TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT 1,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (config_suit_id) REFERENCES config_suit (id) ON DELETE CASCADE,
    FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE,
    UNIQUE(config_suit_id, server_id)
)
```

**配置套装-工具关联表 (config_suit_tool)**
```sql
CREATE TABLE IF NOT EXISTS config_suit_tool (
    id TEXT PRIMARY KEY,
    config_suit_id TEXT NOT NULL,
    server_id TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    prefixed_name TEXT,
    enabled BOOLEAN NOT NULL DEFAULT 1,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (config_suit_id) REFERENCES config_suit (id) ON DELETE CASCADE,
    FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE,
    UNIQUE(config_suit_id, server_id, tool_name)
)
```

#### 2. 代码结构重构

**新的文件结构**
```
src/conf/
├── mod.rs                 # 主模块，包含Database结构体和基本功能
├── initialization.rs      # 数据库初始化逻辑
├── migration.rs           # 从JSON文件迁移到数据库的临时代码
├── models/
│   ├── mod.rs             # 模型模块入口
│   ├── server.rs          # 服务器相关模型
│   ├── tool.rs            # 工具相关模型
│   └── config_suit.rs     # 配置套装相关模型
└── operations/
    ├── mod.rs             # 操作模块入口
    ├── server.rs          # 服务器相关操作
    ├── tool.rs            # 工具相关操作
    └── config_suit.rs     # 配置套装相关操作
```

**命名优化**
- 模型命名：
  - `ServerConfig` → `Server`（更简洁，因为在`models::server`模块中，已经明确是配置相关）
  - `ToolConfig` → `Tool`（同理）
  - `ConfigSuit` → 保持这个名称，因为它很清晰地表达了其功能

- 操作函数命名：
  - `get_server_config` → `get_server`（更简洁）
  - `upsert_server_config` → `upsert_server`（更简洁）
  - 对于配置套装相关的函数，使用`get_config_suit`、`apply_config_suit`等名称

#### 3. 新功能支持

**服务器元数据**
- 支持记录服务器的详细信息，如描述、作者、官网、仓库地址等
- 支持按类别、推荐场景、评级等进行筛选和搜索

**配置套装 (Config Suit)**
- 支持创建和管理不同场景的配置组合
- 支持三种应用方式：
  1. 作为下游MCP Client的来源配置（如Cursor、Windsurf等）
  2. 作为应用场景主题（如编码、文案、运营、媒体等）
  3. 可分享的配置（导出为不同客户端可识别的格式）

**多选模式**
- 支持同时使用多个配置套装
- 自动去重和合并服务/工具
- 通过优先级解决冲突

#### 4. 实施策略

**分阶段实施**
1. ✅ 第一阶段：创建新的数据库表结构和基本模型
2. ✅ 第二阶段：实现从旧结构到新结构的迁移
3. ✅ 第三阶段：更新应用代码以使用新结构
4. ✅ 第四阶段：将整数 ID 迁移到 UUID
5. 🔄 第五阶段：实现配置套装功能
6. 🔄 第六阶段：清理旧代码

**保持向后兼容**
- 在过渡期间，保留旧的接口但内部实现使用新结构
- 添加废弃警告，指导用户迁移到新接口

**充分测试**
- 为新结构编写全面的单元测试
- 确保所有现有功能在新结构下仍然正常工作
- 进行集成测试，验证整个系统的行为

**增量提交**
- 将重构分解为多个小的、独立的提交
- 每个提交后确保代码可以正常构建和运行
- 便于问题定位和回滚

#### 5. 下一步计划

**配置套装功能完善**
1. 初始化默认配置套装（default suit）
   - 在数据库初始化时自动创建默认配置套装
   - 确保所有服务器和工具都有对应的配置

2. 多套装支持
   - 修改 `get_enabled_servers` 函数，支持传入多个配置套装 ID
   - 实现服务器和工具的去重和合并
   - 当没有指定配置套装时，使用默认配置套装

3. 工具前缀名功能
   - 实现工具前缀名的自动生成和管理
   - 确保不同服务器的同名工具可以被正确区分

4. API 接口优化
   - 简化 API 模型，移除冗余字段
   - 统一 API 响应格式
   - 提供更灵活的查询和过滤选项

## 参考资料

1. MCP 规范：
   - 官方规范：https://modelcontextprotocol.io/specification/2025-03-26/
   - Schema 定义：https://raw.githubusercontent.com/modelcontextprotocol/modelcontextprotocol/refs/heads/main/schema/2025-03-26/schema.ts
   - 服务器工具规范：https://modelcontextprotocol.io/specification/2025-03-26/server/tools

2. rust-sdk 文档：
   - 主页：https://docs.rs/rmcp/latest/rmcp/
   - 完整索引：https://docs.rs/rmcp/latest/rmcp/all.html
   - 服务模块：https://docs.rs/rmcp/latest/rmcp/service/index.html
   - 传输模块：https://docs.rs/rmcp/latest/rmcp/transport/index.html

3. 客户端文档：
   - Claude Desktop：https://docs.anthropic.com/claude/docs/claude-desktop-app
   - Cursor：https://cursor.sh/docs

4. rust-sdk 示例：
   - stdio 服务器：`@rust-sdk/examples/servers/src/std_io.rs`
   - SSE 服务器：`@rust-sdk/examples/servers/src/axum.rs`
   - Streamable HTTP 服务器：`@rust-sdk/examples/servers/src/axum_streamable_http.rs`
   - WebSocket 服务器：`@rust-sdk/examples/servers/src/websocket.rs`
   - 通知示例：`@rust-sdk/crates/rmcp/tests/test_notification.rs`

## 功能文档

详细的功能文档请参见 `docs/features/` 目录：

1. [连接池扩展](./features/connection_pool_scaling.md)
2. [Stdio Bridge](./features/stdio_bridge.md)
3. [服务管理](./features/service_management.md)
4. [通知系统](./features/notification_system.md)
5. [Tauri 界面](./features/tauri_interface.md)
6. [Host App 检测](./features/host_app_detection.md)
7. [运行时路径管理](./features/runtime_path.md)
8. [资源代理](./features/resource_proxy.md)
9. [跨平台支持](./features/cross_platform.md)
