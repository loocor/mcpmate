# 连接池动态扩容与服务管理

## 概述

MCPMate 的 SSE 模式设计为支持多个下游客户端并发连接，这需要一个灵活的连接池管理机制，能够根据负载动态扩展实例数量，并提供有效的服务生命周期管理。本文档描述了连接池的动态扩容机制和服务管理策略。

## 背景与动机

在多客户端环境中，不同的 IDE 或应用可能同时发送请求到 MCPMate。为了提供高效的服务，我们需要：

1. 避免单个实例成为瓶颈
2. 确保请求能够及时得到处理
3. 优化资源使用，避免创建过多不必要的实例
4. 提供灵活的服务管理机制，允许启动和停止特定服务

## 连接池动态扩容机制

### 1. 实例状态管理

连接池中的每个实例可以处于以下状态之一：

- **Ready**：实例已连接且空闲，可以处理新请求
- **Busy**：实例正在处理请求，暂时不可用
- **Initializing**：实例正在初始化或连接中
- **Error**：实例遇到错误，暂时不可用
- **Shutdown**：实例已关闭，不可用

### 2. 按需创建实例

当新的请求到来时，连接池按照以下逻辑处理：

1. 遍历服务的所有现有实例，寻找状态为 `Ready` 的实例
2. 如果找到 `Ready` 实例，将其状态设置为 `Busy` 并用于处理请求
3. 如果所有实例都是 `Busy` 状态，则创建一个新的实例
4. 新实例初始化完成后，用于处理当前请求

```rust
async fn get_or_create_instance(&mut self, server_name: &str) -> Result<InstanceRef> {
    // 尝试获取现有的 Ready 实例
    if let Some(instance) = self.find_ready_instance(server_name) {
        instance.update_busy();
        return Ok(instance);
    }

    // 如果没有 Ready 实例，创建新实例
    let connection = UpstreamConnection::new(server_name.to_string());
    let instance_id = connection.id.clone();

    // 添加到连接池
    self.connections
        .entry(server_name.to_string())
        .or_insert_with(HashMap::new)
        .insert(instance_id.clone(), connection);

    // 初始化连接
    self.trigger_connect(server_name, &instance_id).await?;

    // 获取并返回新创建的实例
    let instance = self.get_instance(server_name, &instance_id)?;
    instance.update_busy();

    Ok(instance)
}
```

### 3. 实例选择策略

当有多个 `Ready` 状态的实例可用时，连接池使用以下策略选择实例：

1. **最近最少使用 (LRU)**：优先选择最长时间未使用的实例
2. **负载均衡**：尝试均匀分配请求到不同实例
3. **健康状态**：优先选择健康状态良好的实例（如内存使用较低的）

```rust
fn find_ready_instance(&self, server_name: &str) -> Option<InstanceRef> {
    let instances = self.connections.get(server_name)?;

    // 筛选出 Ready 状态的实例
    let ready_instances: Vec<_> = instances
        .values()
        .filter(|conn| conn.is_connected())
        .collect();

    if ready_instances.is_empty() {
        return None;
    }

    // 按最后活动时间排序（LRU 策略）
    let instance = ready_instances
        .into_iter()
        .min_by_key(|conn| conn.last_activity_time)?;

    Some(instance)
}
```

### 4. 实例生命周期管理

连接池还负责管理实例的生命周期：

1. **空闲超时**：长时间未使用的实例可能会被自动关闭
2. **错误恢复**：处于错误状态的实例可能会尝试自动重连
3. **健康检查**：定期检查实例的健康状态，关闭不健康的实例

```rust
async fn cleanup_idle_instances(&mut self) {
    let now = Instant::now();
    let idle_timeout = Duration::from_secs(self.config.idle_timeout_seconds);

    for (server_name, instances) in &mut self.connections {
        let idle_instances: Vec<_> = instances
            .iter()
            .filter(|(_, conn)| {
                conn.is_connected() && now.duration_since(conn.last_activity_time) > idle_timeout
            })
            .map(|(id, _)| id.clone())
            .collect();

        for instance_id in idle_instances {
            tracing::info!(
                "Closing idle instance '{}' of server '{}'",
                instance_id,
                server_name
            );

            if let Err(e) = self.disconnect(server_name, &instance_id).await {
                tracing::error!(
                    "Error disconnecting idle instance '{}' of server '{}': {}",
                    instance_id,
                    server_name,
                    e
                );
            }
        }
    }
}
```

## 服务管理策略

### 1. 服务启动

当调用 `start_server` 操作时，连接池执行以下操作：

1. 检查服务是否已在配置中定义
2. 检查服务是否已启用（在规则配置中）
3. 创建一个新的实例并初始化连接
4. 更新规则配置，启用该服务
5. 通知下游客户端工具列表已更新

```rust
async fn start_server(&mut self, server_name: &str, force: bool) -> Result<InstanceRef> {
    // 检查服务是否存在
    if !self.config.mcp_servers.contains_key(server_name) {
        return Err(anyhow!("Server '{}' not found in configuration", server_name));
    }

    // 检查是否已有实例
    let has_instances = self.connections.get(server_name).map(|m| !m.is_empty()).unwrap_or(false);

    // 如果已有实例且不强制重启，返回错误
    if has_instances && !force {
        return Err(anyhow!("Server '{}' already has instances", server_name));
    }

    // 如果强制重启，先停止现有实例
    if has_instances && force {
        self.stop_server(server_name).await?;
    }

    // 创建新实例
    let connection = UpstreamConnection::new(server_name.to_string());
    let instance_id = connection.id.clone();

    // 添加到连接池
    self.connections
        .entry(server_name.to_string())
        .or_insert_with(HashMap::new)
        .insert(instance_id.clone(), connection);

    // 启用服务
    self.rule_config.insert(server_name.to_string(), true);

    // 初始化连接
    self.trigger_connect(server_name, &instance_id).await?;

    // 通知工具列表变更
    self.notify_tool_list_changed();

    // 返回新创建的实例
    self.get_instance(server_name, &instance_id)
}
```

### 2. 服务停止

当调用 `stop_server` 操作时，连接池执行以下操作：

1. 遍历服务的所有实例，调用 `disconnect()` 方法终止它们
2. 更新规则配置，禁用该服务
3. 通知下游客户端工具列表已更新

```rust
async fn stop_server(&mut self, server_name: &str) -> Result<Vec<String>> {
    // 获取所有实例 ID
    let instance_ids = if let Some(instances) = self.connections.get(server_name) {
        instances.keys().cloned().collect::<Vec<_>>()
    } else {
        return Ok(vec![]);
    };

    // 断开所有实例
    let mut disconnected_ids = Vec::new();
    for instance_id in &instance_ids {
        match self.disconnect(server_name, instance_id).await {
            Ok(_) => {
                disconnected_ids.push(instance_id.clone());
            }
            Err(e) => {
                tracing::error!(
                    "Failed to disconnect instance '{}' of server '{}': {}",
                    instance_id,
                    server_name,
                    e
                );
            }
        }
    }

    // 禁用服务
    self.rule_config.insert(server_name.to_string(), false);

    // 通知工具列表变更
    self.notify_tool_list_changed();

    Ok(disconnected_ids)
}
```

### 3. 实例重连

当调用 `reconnect` 操作时，连接池执行以下操作：

1. 断开现有连接（终止上游服务进程）
2. 保留相同的实例 ID
3. 创建新的上游服务进程
4. 重新建立连接

需要注意的是，虽然我们称之为"重连"，但实际上是创建了一个新的上游服务进程，只是在我们的代码中使用相同的实例 ID 来引用它。

```rust
async fn reconnect(&mut self, server_name: &str, instance_id: &str) -> Result<()> {
    // 断开现有连接
    self.disconnect(server_name, instance_id).await?;

    // 重新连接
    self.trigger_connect(server_name, instance_id).await
}
```

## 实现考虑

### 1. 配置选项

为了更好地控制连接池的行为，我们可以添加以下配置选项：

```json
{
  "connection_pool": {
    "max_instances_per_server": 5,
    "idle_timeout_seconds": 300,
    "health_check_interval_seconds": 60,
    "reconnect_backoff_seconds": [1, 5, 15, 30, 60]
  }
}
```

### 2. 监控与指标

为了帮助诊断和优化连接池，我们应该收集以下指标：

- 每个服务的实例数量
- 实例状态分布（Ready、Busy、Error 等）
- 实例创建和销毁率
- 请求处理时间
- 资源使用情况（CPU、内存）

### 3. 错误处理

连接池应该实现健壮的错误处理机制：

- 区分临时错误和永久错误
- 实现指数退避重试策略
- 提供详细的错误日志和诊断信息
- 在错误状态下保护系统资源

## 使用场景

### 1. 高并发 IDE 环境

在多个开发者同时使用 IDE 的环境中：

1. 每个 IDE 实例可能同时发送多个请求
2. 连接池会根据需要创建新实例
3. 当负载减轻时，空闲实例会被自动关闭

### 2. 服务管理

管理员可以通过 API 控制服务的生命周期：

1. 启动特定服务，使其工具可用
2. 停止不需要的服务，释放资源
3. 重启服务以应用配置更改

### 3. 资源优化

在资源受限的环境中：

1. 限制每个服务的最大实例数
2. 更积极地关闭空闲实例
3. 优先选择资源使用较低的实例

## 结论

连接池的动态扩容机制和服务管理策略是 MCPMate 支持多客户端并发连接的关键。通过智能地管理实例生命周期和服务状态，我们可以提供高效、可靠的 MCP 代理服务，同时优化资源使用。

这些机制使 MCPMate 能够适应不同的负载模式和使用场景，从单用户环境到大型团队协作环境都能提供一致的性能和用户体验。
