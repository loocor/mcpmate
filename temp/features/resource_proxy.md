# Prompts 和 Resources 资源转发代理支持

## 概述

MCP 协议除了工具（Tools）外，还定义了 Prompts 和 Resources 两种重要资源类型。为了提供完整的 MCP 代理功能，MCPMate 需要支持这些资源的转发和管理。

## 背景与动机

目前，MCPMate 主要关注于工具（Tools）的转发和管理，但 MCP 协议还定义了 Prompts 和 Resources 两种重要资源类型。为了提供完整的 MCP 代理功能，我们需要扩展 MCPMate，使其能够支持这些资源的转发和管理。

## 功能目标

1. 实现 Prompts 资源的转发和管理
2. 实现 Resources 资源的转发和管理
3. 提供资源缓存机制，提高性能
4. 实现资源变更通知机制
5. 提供资源过滤和转换功能
6. 确保与现有工具转发功能的兼容性

## 技术设计

### 1. Prompts 资源转发

#### 1.1 接口定义

```rust
// Prompts 相关接口
async fn list_prompts(&self, pagination: Option<PaginatedRequestParam>) -> Result<ListPromptsResult>;
async fn get_prompt(&self, name: String, arguments: Option<Value>) -> Result<GetPromptResult>;
```

#### 1.2 转发逻辑

1. 接收客户端的 Prompts 请求
2. 根据规则配置决定是否转发
3. 选择合适的上游服务器
4. 转发请求并处理响应
5. 返回结果给客户端

### 2. Resources 资源转发

#### 2.1 接口定义

```rust
// Resources 相关接口
async fn list_resources(&self, pagination: Option<PaginatedRequestParam>) -> Result<ListResourcesResult>;
async fn read_resource(&self, uri: String) -> Result<ReadResourceResult>;
async fn list_resource_templates(&self, pagination: Option<PaginatedRequestParam>) -> Result<ListResourceTemplatesResult>;
```

#### 2.2 转发逻辑

1. 接收客户端的 Resources 请求
2. 根据规则配置决定是否转发
3. 选择合适的上游服务器
4. 转发请求并处理响应
5. 返回结果给客户端

### 3. 资源缓存机制

为了提高性能，MCPMate 将实现资源缓存机制：

1. 缓存 Prompts 和 Resources 的列表和内容
2. 实现缓存过期策略
3. 提供缓存刷新机制
4. 支持缓存统计和监控

### 4. 资源变更通知

当上游服务器的资源发生变化时，MCPMate 需要通知客户端：

1. 监听上游服务器的资源变更通知
2. 更新本地缓存
3. 向客户端发送变更通知

### 5. 资源过滤和转换

MCPMate 将提供资源过滤和转换功能：

1. 根据规则配置过滤资源
2. 支持资源内容的转换和修改
3. 提供资源合并功能，将多个上游服务器的资源合并为一个统一视图

## 配置选项

```json
{
  "resource_proxy": {
    "enabled": true,
    "cache_enabled": true,
    "cache_ttl": 300,
    "filters": {
      "prompts": {
        "include": ["*"],
        "exclude": []
      },
      "resources": {
        "include": ["*"],
        "exclude": []
      }
    }
  }
}
```

## 使用场景

### 1. 提示词管理

用户可以通过 MCPMate 管理和使用多个上游服务器提供的提示词模板，例如：

1. 从公司内部服务器获取标准提示词模板
2. 从开源社区获取通用提示词模板
3. 使用本地自定义提示词模板

### 2. 资源访问

用户可以通过 MCPMate 访问多个上游服务器提供的资源，例如：

1. 访问公司内部文档和知识库
2. 访问开源项目的文档和示例
3. 访问本地文件系统中的资源

## 参考资料

1. MCP 规范中的 Prompts 部分：https://modelcontextprotocol.io/docs/concepts/prompts
2. MCP 规范中的 Resources 部分：https://modelcontextprotocol.io/docs/concepts/resources
3. rust-sdk 中的相关实现：
   - Prompts 相关接口：https://docs.rs/rmcp/latest/rmcp/model/struct.Prompt.html
   - Resources 相关接口：https://docs.rs/rmcp/latest/rmcp/model/struct.Resource.html
