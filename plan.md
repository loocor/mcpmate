# 设计方案

确保我们在实现时不会遗漏任何重要内容：

## 1. 状态模型
我们将保持与 Rust SDK 一致的状态枚举：

```rust
pub enum ConnectionStatus {
    Initializing,  // 初始化中
    Ready,         // 就绪
    Busy,          // 忙碌
    Error(String), // 错误
    Shutdown,      // 已关闭
}
```

## 2. 操作集
每个状态允许的操作：

- Initializing: disconnect, reconnect, cancel
- Ready: disconnect, reconnect
- Busy: disconnect, reconnect
- Error: disconnect, reconnect
- Shutdown: 只有 reconnect

特殊说明：

- disconnect 有普通和强制两种模式
- reconnect 有普通和重置两种模式

## 3. 实例标识
为每个服务实例分配唯一 ID，确保操作针对正确的实例：

```rust
pub struct UpstreamConnection {
    pub id: String,  // 实例唯一ID
    pub name: String,
    pub status: ConnectionStatus,
    // 其他字段...
}
```

## 4. API 端点设计
采用嵌套路径模式，使用专用端点表示不同操作模式：

```http
  # 服务管理
  GET    /api/mcp/servers                       # 列出所有服务
  GET    /api/mcp/servers/:name                 # 获取服务信息

  # 实例管理
  GET    /api/mcp/servers/:name/instances       # 列出服务的所有实例
  GET    /api/mcp/servers/:name/instances/:id   # 获取特定实例信息
  GET    /api/mcp/servers/:name/instances/:id/health # 获取实例健康状态

  # 实例操作 - 基本操作
  POST   /api/mcp/servers/:name/instances/:id/disconnect       # 正常断开连接
  POST   /api/mcp/servers/:name/instances/:id/disconnect/force # 强制断开连接
  POST   /api/mcp/servers/:name/instances/:id/reconnect        # 正常重新连接
  POST   /api/mcp/servers/:name/instances/:id/reconnect/reset  # 重置并重新连接
  POST   /api/mcp/servers/:name/instances/:id/cancel           # 取消初始化（仅 Initializing 状态）

  # 系统管理
  GET    /api/system/status                     # 获取系统状态
  GET    /api/system/metrics                    # 获取系统指标
}
```

## 5. 响应模型

```rust
// 服务响应
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerResponse {
    pub name: String,
    pub instances_count: usize,
    pub enabled: bool,
}

// 实例列表响应
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerInstancesResponse {
    pub name: String,
    pub instances: Vec<ServerInstanceSummary>,
}

// 实例摘要
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerInstanceSummary {
    pub id: String,
    pub status: String,
    pub connected_at: Option<String>,
}

// 实例详情响应
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerInstanceResponse {
    pub id: String,
    pub name: String,
    pub status: String,
    pub allowed_operations: Vec<String>,
    pub details: ServerInstanceDetails,
}

// 实例详情
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerInstanceDetails {
    pub connection_attempts: u32,
    pub last_connected_seconds: Option<u64>,
    pub tools_count: usize,
    pub error_message: Option<String>,
    pub server_type: String,
}
```

## 6. 实现注意事项

- 多实例支持：
  - 修改连接池结构，支持一个服务有多个实例
  - 为每个新连接生成唯一 ID
- 状态转换：
  - 确保状态转换逻辑清晰
  - 在每个状态下只允许特定的操作
- 错误处理：
  - 提供清晰的错误消息
  - 对于不允许的操作，返回适当的错误代码
- 文档：
  - 为每个端点提供详细文档
  - 说明每个状态允许的操作

# 实施步骤

每进行一个阶段，需要交给老陈进行测试确认。

## 第一阶段：基础结构调整
- 更新 ConnectionStatus 枚举，与 Rust SDK 保持一致
- 为连接添加实例 ID 支持
- 修改连接池结构，支持多实例

## 第二阶段：API 模型更新
- 定义新的响应模型，支持实例信息
- 实现状态转换逻辑和操作限制

## 第三阶段：端点实现
- 实现服务和实例管理端点
- 实现各种操作端点，包括专用变体

## 第四阶段：测试和优化 (暂时忽略)
- 编写单元测试和集成测试
- 优化性能和错误处理
