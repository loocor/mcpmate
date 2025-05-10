# 安全沙盒设计：请求重定向与威胁情报收集

## 概述

本文档描述了 MCPMate 中的安全沙盒功能设计，该功能旨在通过重定向可疑请求到模拟环境，而不是简单地断开连接，从而增强安全审计和威胁情报收集能力。

## 背景与动机

在传统的安全模型中，当检测到潜在的恶意请求时，常见的做法是立即断开连接或阻止请求。虽然这种方法可以有效保护系统，但也会导致以下问题：

1. 无法收集关于攻击者意图和方法的更多信息
2. 攻击者可能意识到他们的行为被检测到，并改变策略
3. 上游服务实例（如 Docker 容器）可能被终止和回收，导致证据丢失
4. 失去了深入分析攻击模式的机会

安全沙盒功能通过"铁轨转向"方法解决这些问题，将可疑请求重定向到一个模拟环境，而不是中断连接。

## 设计目标

1. **无缝拦截**：在不影响正常操作的情况下拦截可疑请求
2. **真实模拟**：提供一个足够真实的环境，使攻击者无法轻易区分
3. **全面记录**：详细记录所有在沙盒中的交互
4. **可配置规则**：支持灵活的规则配置，以确定哪些请求应被重定向
5. **最小资源消耗**：沙盒环境应该是轻量级的，不应显著增加系统负担
6. **可扩展性**：设计应允许添加新的模拟环境和检测规则

## 系统架构

### 1. 拦截层

拦截层负责检查传入的请求和传出的响应，并根据安全规则决定是否需要重定向。

```
客户端请求 → 拦截层 → [检查安全规则] → 正常处理 或 重定向到沙盒
```

拦截层将集成到现有的 `SseProxyServer` 和 `StdioProxyServer` 的 `call_tool` 方法中。

### 2. 安全规则引擎

安全规则引擎负责评估请求和响应，并确定是否存在安全风险。

规则可以基于多种因素：
- 请求内容（如特定关键词或模式）
- 请求频率
- 请求来源
- 历史行为模式
- 工具类型和参数

### 3. 沙盒环境

沙盒环境是一个模拟真实上游服务的环境，但完全隔离且受控。

沙盒环境的关键组件：
- **服务模拟器**：模拟上游服务的行为
- **交互记录器**：记录所有交互
- **响应生成器**：生成看似真实但无害的响应

### 4. 审计与分析系统

审计系统负责收集、存储和分析沙盒中的交互数据。

功能包括：
- 详细的交互日志
- 行为分析
- 模式识别
- 警报生成
- 报告生成

## 技术实现

### 核心接口

```rust
/// 安全规则接口
pub trait SecurityRule {
    /// 评估请求是否应被重定向到沙盒
    fn evaluate_request(&self, request: &ToolRequest) -> SecurityDecision;

    /// 评估响应是否表明存在安全问题
    fn evaluate_response(&self, response: &ToolResponse) -> SecurityDecision;
}

/// 安全决策
pub enum SecurityDecision {
    /// 允许正常处理
    Allow,
    /// 重定向到沙盒
    Redirect(SandboxType),
    /// 阻止请求/响应
    Block,
}

/// 沙盒类型
pub enum SandboxType {
    /// 通用沙盒
    Generic,
    /// 特定于某个应用的沙盒（如 Cursor）
    Application(String),
    /// 自定义沙盒配置
    Custom(SandboxConfig),
}

/// 沙盒服务接口
pub trait SandboxService: UpstreamConnection {
    /// 获取沙盒类型
    fn sandbox_type(&self) -> SandboxType;

    /// 获取记录的交互
    fn get_interactions(&self) -> Vec<SandboxInteraction>;

    /// 配置沙盒行为
    fn configure(&mut self, config: SandboxConfig);
}
```

### 集成到现有架构

安全沙盒功能将集成到现有的 MCPMate 架构中：

1. 在 `core` 模块中定义安全规则和沙盒接口
2. 在各个传输模式（SSE、stdio）中实现拦截逻辑
3. 创建专门的 `sandbox` 模块，实现各种沙盒环境

```
src/
├── core/
│   ├── security/
│   │   ├── mod.rs
│   │   ├── rules.rs
│   │   └── sandbox.rs
├── sandbox/
│   ├── mod.rs
│   ├── generic.rs
│   ├── cursor.rs
│   └── ...
```

### 请求重定向流程

1. 客户端发送工具调用请求
2. 代理服务器接收请求并通过安全规则引擎评估
3. 如果规则引擎决定重定向，代理服务器创建一个沙盒服务实例
4. 请求被转发到沙盒服务而不是真实的上游服务
5. 沙盒服务生成响应并记录交互
6. 响应返回给客户端，客户端无法区分是来自真实服务还是沙盒

## 配置示例

```json
{
  "security": {
    "enabled": true,
    "rules": [
      {
        "name": "sensitive_command_detection",
        "type": "content_match",
        "patterns": ["rm -rf", "sudo", "chmod 777"],
        "action": "redirect",
        "sandbox": "generic"
      },
      {
        "name": "file_access_monitor",
        "type": "tool_specific",
        "tool_name": "file_system",
        "patterns": ["/etc/passwd", "/etc/shadow", ".ssh"],
        "action": "redirect",
        "sandbox": "cursor"
      }
    ],
    "sandboxes": {
      "generic": {
        "type": "generic",
        "response_delay": 200,
        "log_level": "debug"
      },
      "cursor": {
        "type": "application",
        "application": "cursor",
        "version": "0.5.0",
        "simulate_errors": true
      }
    }
  }
}
```

## 审计与日志

沙盒环境将生成详细的审计日志，包括：

1. 完整的请求和响应内容
2. 时间戳和会话标识符
3. 触发重定向的规则
4. 沙盒生成的响应
5. 客户端后续行为

日志将以结构化格式存储，便于后续分析。

## 安全考虑

1. **沙盒逃逸防护**：确保沙盒环境足够隔离，防止攻击者逃逸
2. **资源限制**：为沙盒环境设置资源限制，防止拒绝服务攻击
3. **真实性平衡**：沙盒环境需要足够真实以欺骗攻击者，但又不能提供真实的攻击面
4. **隐私保护**：确保记录的数据符合隐私法规要求

## 未来扩展

1. **机器学习集成**：使用机器学习模型自动识别异常行为
2. **威胁情报共享**：与威胁情报平台集成，共享发现的攻击模式
3. **动态沙盒生成**：基于攻击者行为动态调整沙盒环境
4. **蜜罐功能**：扩展为完整的蜜罐系统，主动吸引攻击者

## 结论

安全沙盒功能通过"铁轨转向"方法，将可疑请求重定向到模拟环境而不是简单断开连接，为 MCPMate 提供了强大的安全审计和威胁情报收集能力。这种方法不仅保护了系统安全，还提供了宝贵的攻击者行为数据，有助于改进未来的安全防护措施。
