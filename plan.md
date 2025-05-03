# MCPMan 开发计划

## 当前状态

MCPMan 项目已经完成了以下关键里程碑：

1. **基础代理功能**：实现了基本的 MCP 代理功能，能够连接上游 MCP 服务器并转发请求。
2. **连接池重构**：完成了连接池的重构，提高了连接稳定性和工具映射缓存效率。
3. **资源监控**：实现了进程资源监控功能，能够跟踪和限制上游服务器进程的资源使用。

## 下一阶段：增强兼容性和用户体验

在下一阶段，我们将专注于提升 MCPMan 的兼容性和用户体验，使其能够支持更多的应用场景并简化用户配置流程。

### 计划中可能的下一阶段方向有：

#### 1. 增加 stdio 服务模式

- 优势：扩大兼容性，支持 Claude desktop 等不支持 SSE 的应用
- 复杂度：中等，可以复用现有的 stdio 连接逻辑
- 影响：立即提升产品兼容性范围

#### 2. 增加 Prompts、Resources 资源的转发代理支持

- 势：完整支持 MCP 协议的所有功能
- 复杂度：中等到高，需要实现新的资源管理和转发逻辑
- 影响：提升功能完整性，支持更复杂的应用场景

#### 3. 开发 Tauri 界面

- 优势：大幅提升用户友好性，使非技术用户也能使用
- 复杂度：高，需要设计和实现完整的 UI/UX
- 影响：从开发者工具转变为面向一般用户的产品

#### 4. 检测已安装 host app 并管理配置

- 优势：简化配置流程，提供一站式管理体验
- 复杂度：中等，需要实现检测逻辑和配置文件读写
- 影响：提升用户体验，减少手动配置需求

#### 5. 提供自定义运行时路径管理

- 优势：增强环境隔离，避免系统依赖冲突
- 复杂度：中等，需要实现路径管理和环境变量控制
- 影响：提升稳定性和可靠性

#### 6. 支持 Windows 等其他平台

- 优势：扩大用户群体，提升市场覆盖
- 复杂度：高，需要处理跨平台兼容性问题
- 影响：显著扩大潜在用户群体

### 优先级建议
考虑到用户价值、实现复杂度和基础设施需求，我建议以下优先顺序：

#### 第一优先级：增加 stdio 服务模式
这是一个相对容易实现但能带来立即价值的功能。通过支持 stdio 服务模式，我们可以让 MCPMan 与更多应用兼容，特别是 Claude desktop 等流行应用。这也为后续的 Tauri 界面开发奠定基础。

#### 第二优先级：检测已安装 host app 并管理配置
这个功能可以大幅简化用户配置流程，提供更好的用户体验。它也是构建完整生态系统的重要一步，让 MCPMan 成为真正的"管理器"而不仅仅是代理。

#### 第三优先级：提供自定义运行时路径管理
这个功能可以解决环境依赖问题，提升系统稳定性。它也是支持多平台的基础工作之一。

#### 后续优先级：

- 增加 Prompts、Resources 资源的转发代理支持
- 开发 Tauri 界面
- 支持 Windows 等其他平台

### 阶段 6：增加 stdio 服务模式

#### 背景和动机

目前，MCPMan 作为代理服务器，通过 SSE 协议暴露 MCP 服务。然而，许多应用（如 Claude desktop）不支持 SSE 协议，而是使用 stdio 协议与 MCP 服务器通信。为了支持这些应用，我们需要实现 stdio 服务模式，使 MCPMan 能够作为 stdio 服务暴露给这些应用。

#### 目标

1. 实现 stdio 服务模式，使 MCPMan 能够作为 stdio 服务暴露给客户端应用
2. 确保 stdio 服务模式与现有的 SSE 服务模式共存，不影响现有功能
3. 实现消息通知机制，当服务器和工具列表发生变化时通知客户端应用
4. 提供配置选项，允许用户启用和配置 stdio 服务模式
5. 提供文档和示例，说明如何将 MCPMan 与各种客户端应用集成

#### 技术方案

1. **设计 stdio 服务接口**
   - 实现 `StdioServer` 结构体，负责处理 stdio 通信
   - 使用 rust-sdk 提供的 `serve_server` 函数创建 stdio 服务器
   - 实现请求处理逻辑，将客户端请求转发到上游 MCP 服务

2. **实现请求转发逻辑**
   - 设计请求路由机制，将客户端请求路由到适当的上游服务
   - 实现工具调用转发，将工具调用请求转发到拥有该工具的上游服务
   - 处理特殊请求类型，如 `ListToolsRequest`、`InitializeRequest` 等

3. **集成到现有架构**
   - 在 `main.rs` 中添加 stdio 服务启动选项
   - 添加配置选项，允许用户启用和配置 stdio 服务模式
   - 确保 stdio 服务与现有的 SSE 服务共享连接池和工具映射缓存

4. **提供文档和示例**
   - 更新 README.md，说明如何启用和使用 stdio 服务模式
   - 提供示例配置，说明如何将 MCPMan 与各种客户端应用集成
   - 添加故障排除指南，帮助用户解决常见问题

#### 具体实施步骤

1. **创建 stdio 服务器模块**
   ```rust
   // src/stdio_server.rs
   use rmcp::{model::*, service::*, transport::*};
   use tokio::sync::Mutex;
   use std::sync::Arc;

   pub struct StdioServer {
       connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
       // 其他必要字段
   }

   impl StdioServer {
       pub fn new(connection_pool: Arc<Mutex<UpstreamConnectionPool>>) -> Self {
           Self {
               connection_pool,
               // 初始化其他字段
           }
       }

       pub async fn start() -> Result<()> {
           // 实现 stdio 服务器启动逻辑
           let stdio = StdioTransport::new(std::io::stdin(), std::io::stdout());
           serve_server(self, stdio).await?;
           Ok(())
       }

       // 实现请求处理逻辑
       async fn handle_request(&self, request: Request) -> Result<Response> {
           // 根据请求类型分发到不同的处理函数
           match request {
               Request::Initialize(req) => self.handle_initialize(req).await,
               Request::ListTools(req) => self.handle_list_tools(req).await,
               Request::CallTool(req) => self.handle_call_tool(req).await,
               // 处理其他请求类型
               _ => Err(anyhow::anyhow!("Unsupported request type")),
           }
       }

       // 实现各种请求处理函数
       async fn handle_initialize(&self, req: InitializeRequest) -> Result<Response> {
           // 处理初始化请求
           // ...
       }

       async fn handle_list_tools(&self, req: ListToolsRequest) -> Result<Response> {
           // 处理工具列表请求
           // ...
       }

       async fn handle_call_tool(&self, req: CallToolRequest) -> Result<Response> {
           // 处理工具调用请求
           // ...
       }
   }

   // 实现 Service trait
   #[async_trait]
   impl Service<RoleServer> for StdioServer {
       async fn handle(&self, request: Request) -> Result<Response, ServiceError> {
           self.handle_request(request).await.map_err(|e| {
               ServiceError::McpError(McpError::internal_error(e.to_string()))
           })
       }
   }
   ```

2. **更新配置结构体**
   ```rust
   // src/config.rs
   #[derive(Debug, Deserialize)]
   pub struct Config {
       // 现有字段
       pub mcp_servers: HashMap<String, ServerConfig>,
       pub proxy: ProxyConfig,

       // 新增字段
       #[serde(default)]
       pub stdio_server: StdioServerConfig,
   }

   #[derive(Debug, Deserialize, Default)]
   pub struct StdioServerConfig {
       #[serde(default)]
       pub enabled: bool,

       #[serde(default = "default_stdio_server_name")]
       pub server_name: String,
   }

   fn default_stdio_server_name() -> String {
       "mcpman".to_string()
   }
   ```

3. **更新 main.rs**
   ```rust
   // src/main.rs
   #[tokio::main]
   async fn main() -> Result<()> {
       // 初始化日志和配置
       // ...

       // 创建连接池
       let connection_pool = Arc::new(Mutex::new(UpstreamConnectionPool::new(
           Arc::new(config.clone()),
           Arc::new(rule_config.clone()),
       )));

       // 初始化连接池
       {
           let mut pool = connection_pool.lock().await;
           pool.initialize();
       }

       // 启动健康检查
       UpstreamConnectionPool::start_health_check(connection_pool.clone());

       // 创建代理服务器
       let proxy_server = Arc::new(ProxyServer::new(connection_pool.clone()));

       // 启动 SSE 服务器
       let sse_server_handle = tokio::spawn(async move {
           // 启动 SSE 服务器
           // ...
       });

       // 如果启用了 stdio 服务器，则启动它
       if config.stdio_server.enabled {
           let stdio_server = StdioServer::new(connection_pool.clone());
           tokio::spawn(async move {
               if let Err(e) = stdio_server.start().await {
                   tracing::error!("Error starting stdio server: {}", e);
               }
           });
       }

       // 启动 API 服务器
       // ...

       // 等待所有服务器完成
       // ...

       Ok(())
   }
   ```

4. **添加命令行选项**
   ```rust
   // src/main.rs
   use clap::{App, Arg};

   fn main() -> Result<()> {
       let matches = App::new("MCPMan")
           .version("0.1.0")
           .author("Your Name")
           .about("MCP Manager")
           .arg(
               Arg::with_name("stdio")
                   .long("stdio")
                   .help("Run in stdio mode (for integration with Claude desktop, etc.)")
                   .takes_value(false),
           )
           .get_matches();

       // 如果指定了 stdio 参数，则以 stdio 模式运行
       if matches.is_present("stdio") {
           // 创建连接池
           // ...

           // 创建 stdio 服务器
           let stdio_server = StdioServer::new(connection_pool);

           // 启动 stdio 服务器
           return stdio_server.start().await.map_err(|e| anyhow::anyhow!("Error starting stdio server: {}", e));
       }

       // 否则，以正常模式运行
       // ...
   }
   ```

#### 工作量评估

| 任务 | 工作量（人天） | 复杂度 | 关键依赖 |
|------|--------------|--------|---------|
| 设计 stdio 服务接口 | 0.5-1 | 中 | rust-sdk 的 stdio 服务器实现 |
| 实现请求转发逻辑 | 1-1.5 | 高 | 对 MCP 协议的理解 |
| 实现消息通知机制 | 0.5-1 | 中 | 对 MCP 通知协议的理解 |
| 集成到现有架构 | 0.5-1 | 中 | 现有代码的理解 |
| 提供文档和示例 | 0.5 | 低 | 前面任务的完成 |

总体工作量估计：
- **纯人工实现**：3-5 天
- **协作实现**：2-4 天

### 阶段 7：检测已安装 host app 并管理配置

在完成 stdio 服务模式后，我们将实现检测已安装 host app 并管理配置的功能，这将在下一个计划文档中详细说明。

## 参考资料

1. MCP 规范：https://raw.githubusercontent.com/modelcontextprotocol/modelcontextprotocol/refs/heads/main/schema/2025-03-26/schema.ts
2. rust-sdk 文档：
   - 主页：https://docs.rs/rmcp/latest/rmcp/
   - 完整索引：https://docs.rs/rmcp/latest/rmcp/all.html
   - 服务模块：https://docs.rs/rmcp/latest/rmcp/service/index.html
   - 传输模块：https://docs.rs/rmcp/latest/rmcp/transport/index.html
3. Claude desktop 文档：https://docs.anthropic.com/claude/docs/claude-desktop-app
4. rust-sdk 示例：
   - stdio 服务器：`@rust-sdk/examples/servers/src/std_io.rs`
