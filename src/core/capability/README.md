# MCPMate 统一能力查询模块

基于现有基础设施的统一能力查询入口，支持tools、resources、prompts等能力类型的标准化查询。

## 快速开始

```rust
use crate::core::capability::*;
use crate::api::handlers::common::InspectParams;

// 构建统一查询服务
let service = UnifiedQueryServiceBuilder::new()
    .with_cache(cache_manager)
    .with_pool(connection_pool)
    .with_database(database)
    .build()?;

// 查询工具能力
let params = InspectParams {
    refresh: None,
    format: None,
    include_meta: Some(true),
    timeout: None,
};

let result = service.query_capabilities(
    "test-server",
    CapabilityType::Tools,
    &params,
    QueryContext::ApiCall,
).await?;

println!("Found {} tools from {}", result.items.len(), result.metadata.source);
```

## 核心概念

### 查询上下文
- `ApiCall` - API调用场景，临时实例用完即关
- `McpClient` - MCP客户端场景，创建持久实例

### 数据源优先级
1. ReDB缓存（最快，<5ms）
2. 运行时实例（10-50ms）
3. 临时实例（100-500ms）

### 能力类型
- `Tools` - 工具能力
- `Resources` - 资源能力  
- `Prompts` - 提示能力
- `ResourceTemplates` - 资源模板能力

## 高级配置

```rust
let service = UnifiedQueryServiceBuilder::new()
    .with_cache(cache_manager)
    .with_pool(connection_pool)
    .with_database(database)
    .with_timeout(Duration::from_secs(60))
    .build()?;
```

## 错误处理

```rust
match service.query_capabilities(...).await {
    Ok(result) => {
        // 处理成功结果
    }
    Err(CapabilityError::ServerDisabled { server_id }) => {
        println!("Server {} is disabled", server_id);
    }
    Err(CapabilityError::CacheError(msg)) => {
        println!("Cache error: {}", msg);
    }
    Err(e) => {
        println!("Unexpected error: {}", e);
    }
}
```

## 性能基准

基于现有Redb缓存性能：
- 缓存命中：< 5ms
- 运行时查询：10-50ms  
- 临时实例：100-500ms
- 整体吞吐量：1000+ QPS

## 相关模块

- `crate::core::cache` - 缓存系统
- `crate::core::pool` - 连接池管理
- `crate::api::handlers::common` - API共用逻辑
- `crate::common::capability` - 能力类型定义

## 架构决策

采用"直接调用为主，按需解耦"策略：
- 复用现有基础设施，避免重复造轮子
- 保持零额外开销，性能最优
- 代码简洁，易于理解和调试
- 为后续演进预留空间

## 端口接口使用示例

### 缓存和运行时端口使用

```rust
use crate::core::capability::ports::*;
use crate::core::capability::domain::*;

async fn example_usage(
    cache: Arc<dyn CachePort>,
    runtime: Arc<dyn RuntimePort>,
) -> Result<CapabilityResult, CapabilityError> {
    
    // Create cache key
    let cache_key = CacheKey {
        server_id: "test-server".to_string(),
        capability_type: CapabilityType::Tools,
        freshness_requirement: "cache_preferred".to_string(),
    };
    
    // Try cache
    if let Some(entry) = cache.get(&cache_key).await? {
        if !entry.is_expired() {
            return Ok(entry.into_result());
        }
    }
    
    // Fallback to runtime
    let instances = runtime.get_connected("test-server").await?;
    // ... 处理运行时结果
    
    Ok(result)
}
```

### 健康检查使用示例

```rust
use crate::core::capability::ports::*;

async fn health_check_example(ports: Vec<Arc<dyn HealthCheck>>) {
    for port in ports {
        match port.health_check().await {
            PortHealth::Healthy => println!("Port is healthy"),
            PortHealth::Degraded => println!("Port is degraded"),
            PortHealth::Unhealthy => println!("Port is unhealthy"),
        }
        
        let details = port.health_details().await;
        println!("Health details: {}", details);
    }
}
```

## 变更日志

### v1.0.0
- 初始统一查询实现
- 支持tools/resources/prompts查询
- 集成ReDB缓存和运行时实例
- 完整的错误处理和超时保护