# 检测已安装 host app 并管理配置

## 概述

自动检测用户系统中已安装的 LLM 应用（如 Claude Desktop、VSCode 等），并自动管理 MCPMate 的配置，以提供无缝的集成体验。

## 背景与动机

目前，用户需要手动配置 MCPMate 以与各种 LLM 应用集成，这增加了使用门槛。通过自动检测已安装的应用并管理配置，我们可以大幅简化用户的配置流程，提供一站式管理体验。

## 功能目标

1. 自动检测系统中已安装的 LLM 应用
2. 自动生成和管理适用于这些应用的 MCPMate 配置
3. 提供配置向导，引导用户完成必要的手动配置步骤
4. 监控应用安装状态的变化，并自动更新配置
5. 提供配置备份和恢复功能
6. 支持多用户环境下的配置隔离

## 技术设计

### 1. 应用检测机制

#### 1.1 检测策略

MCPMate 将使用以下策略检测已安装的应用：

1. **路径检测**：检查常见安装路径
2. **注册表检测**（Windows）：检查 Windows 注册表
3. **应用程序文件夹检测**（macOS）：检查 `/Applications` 文件夹
4. **包管理器查询**（Linux）：使用包管理器查询已安装的应用
5. **进程检测**：检查正在运行的进程

#### 1.2 支持的应用

初始版本将支持检测以下应用：

1. **Claude Desktop**
2. **Visual Studio Code**
3. **JetBrains IDEs**（IntelliJ IDEA、PyCharm 等）
4. **Cursor**
5. **其他常见 LLM 应用**

### 2. 配置管理

#### 2.1 配置生成

根据检测到的应用，MCPMate 将自动生成适当的配置：

```rust
fn generate_config_for_app(app_info: &AppInfo) -> Result<Config> {
    match app_info.app_type {
        AppType::ClaudeDesktop => generate_claude_desktop_config(app_info),
        AppType::VSCode => generate_vscode_config(app_info),
        AppType::JetBrainsIDE => generate_jetbrains_config(app_info),
        AppType::Cursor => generate_cursor_config(app_info),
        // 其他应用类型
        _ => Err(anyhow::anyhow!("Unsupported application type")),
    }
}
```

#### 2.2 配置存储

配置将存储在以下位置：

1. **用户配置目录**：
   - 在 macOS 上：`~/Library/Application Support/MCPMate/config/`
   - 在 Linux 上：`~/.config/mcpmate/`
   - 在 Windows 上：`%APPDATA%\MCPMate\config\`

2. **应用特定配置**：
   - 每个检测到的应用将有一个专用的配置文件
   - 配置文件将使用应用 ID 命名，例如 `claude-desktop.json`

#### 2.3 配置版本控制

MCPMate 将实现配置版本控制，以便在配置格式变更时进行平滑迁移：

1. 每个配置文件将包含版本信息
2. MCPMate 将在启动时检查配置版本并进行必要的迁移
3. 配置迁移将保留用户的自定义设置

### 3. 配置向导

对于需要用户手动配置的部分，MCPMate 将提供配置向导：

1. **检测结果展示**：显示检测到的应用列表
2. **配置选项**：提供配置选项和建议
3. **步骤引导**：引导用户完成必要的手动配置步骤
4. **配置验证**：验证配置是否正确

### 4. 监控和更新

MCPMate 将监控应用安装状态的变化，并自动更新配置：

1. **定期扫描**：定期扫描系统以检测新安装或卸载的应用
2. **事件监听**：监听系统事件，如应用安装和卸载事件
3. **配置更新**：根据检测结果更新配置

## 配置示例

### Claude Desktop 配置示例

```json
{
  "app_id": "claude-desktop",
  "app_name": "Claude Desktop",
  "app_path": "/Applications/Claude.app",
  "app_version": "1.0.0",
  "mcp_config": {
    "command": "/path/to/mcpmate/proxy-stdio",
    "args": ["--config", "/path/to/mcpmate/config/claude-desktop.json"]
  },
  "integration": {
    "type": "stdio",
    "config_path": "/Users/username/Library/Application Support/Claude/config.json",
    "config_key": "mcpCommand"
  }
}
```

### VSCode 配置示例

```json
{
  "app_id": "vscode",
  "app_name": "Visual Studio Code",
  "app_path": "/Applications/Visual Studio Code.app",
  "app_version": "1.60.0",
  "mcp_config": {
    "url": "http://localhost:8000/sse",
    "message_endpoint": "/message"
  },
  "integration": {
    "type": "sse",
    "config_path": "/Users/username/Library/Application Support/Code/User/settings.json",
    "config_key": "mcp.serverUrl"
  }
}
```

## 使用场景

### 1. 首次安装

用户首次安装 MCPMate 时，系统将：

1. 自动扫描系统中已安装的 LLM 应用
2. 生成适当的配置
3. 引导用户完成必要的手动配置步骤
4. 启动 MCPMate 服务

### 2. 新应用安装

用户安装新的 LLM 应用时，MCPMate 将：

1. 检测到新安装的应用
2. 生成适当的配置
3. 通知用户并提供配置选项
4. 根据用户选择更新配置

### 3. 应用更新

应用更新时，MCPMate 将：

1. 检测到应用版本变更
2. 检查配置兼容性
3. 必要时更新配置
4. 通知用户配置变更

## 参考资料

1. Claude Desktop 配置文档：https://docs.anthropic.com/claude/docs/claude-desktop-app
2. VSCode 扩展 API：https://code.visualstudio.com/api
3. JetBrains 插件开发文档：https://plugins.jetbrains.com/docs/intellij/welcome.html
4. Cursor 文档：https://cursor.sh/docs
