# Tauri 界面开发

## 概述

为 MCPMate 开发一个基于 Tauri 的图形用户界面，使非技术用户也能轻松使用 MCPMate 的功能。

## 背景与动机

目前，MCPMate 主要面向开发者，通过命令行和配置文件进行操作。这对于非技术用户来说存在较高的使用门槛。通过开发一个直观的图形用户界面，我们可以大幅提升 MCPMate 的用户友好性，使其能够被更广泛的用户群体使用。

## 功能目标

1. 提供直观的服务器管理界面
2. 实现配置文件的可视化编辑
3. 提供实时状态监控和日志查看
4. 支持工具和资源的浏览和测试
5. 提供一键安装和配置功能
6. 实现系统托盘集成和后台运行
7. 支持多语言和主题定制

## 技术设计

### 1. 架构概览

```
MCPMate-Tauri/
├── src/                  # Rust 后端代码
│   ├── main.rs           # 主入口
│   ├── commands/         # Tauri 命令
│   └── ...
├── src-tauri/            # Tauri 配置
│   ├── tauri.conf.json   # Tauri 配置文件
│   └── ...
├── ui/                   # 前端代码
│   ├── src/              # React/Vue 源码
│   ├── public/           # 静态资源
│   └── ...
└── ...
```

### 2. 后端设计

后端将使用 Tauri 的命令系统与前端通信，并集成现有的 MCPMate 核心功能：

```rust
#[tauri::command]
async fn get_servers() -> Result<Vec<ServerInfo>, String> {
    // 获取服务器列表
}

#[tauri::command]
async fn start_server(name: String) -> Result<(), String> {
    // 启动服务器
}

#[tauri::command]
async fn stop_server(name: String) -> Result<(), String> {
    // 停止服务器
}

// 其他命令...
```

### 3. 前端设计

前端将使用 React 或 Vue 开发，提供以下主要页面和组件：

1. **仪表盘**：显示系统概览和状态
2. **服务器管理**：管理上游服务器
3. **工具浏览器**：浏览和测试可用工具
4. **资源浏览器**：浏览和管理资源
5. **配置编辑器**：编辑配置文件
6. **日志查看器**：查看系统日志
7. **设置**：系统设置和偏好

### 4. 系统托盘集成

MCPMate 将集成到系统托盘，允许用户在后台运行应用并快速访问常用功能：

```rust
fn main() {
    tauri::Builder::default()
        .system_tray(
            SystemTray::new()
                .with_menu(
                    SystemTrayMenu::new()
                        .add_item(CustomMenuItem::new("show", "显示主窗口"))
                        .add_item(CustomMenuItem::new("quit", "退出"))
                )
        )
        .on_system_tray_event(|app, event| {
            match event {
                SystemTrayEvent::MenuItemClick { id, .. } => {
                    match id.as_str() {
                        "show" => {
                            // 显示主窗口
                        }
                        "quit" => {
                            // 退出应用
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### 5. 一键安装和配置

MCPMate Tauri 应用将提供一键安装和配置功能，自动检测系统环境并配置 MCPMate：

1. 检测已安装的 LLM 应用
2. 自动配置 MCPMate 以支持这些应用
3. 提供向导式配置流程
4. 支持配置导入和导出

## 用户界面设计

### 1. 主界面

主界面将采用现代化的设计风格，包括：

1. 侧边导航栏
2. 顶部状态栏
3. 主内容区域
4. 底部状态栏

### 2. 服务器管理界面

服务器管理界面将提供以下功能：

1. 服务器列表，显示状态和基本信息
2. 添加、编辑和删除服务器的按钮
3. 启动、停止和重启服务器的按钮
4. 服务器详情面板，显示详细信息和统计数据

### 3. 工具浏览器

工具浏览器将提供以下功能：

1. 工具列表，按服务器分组
2. 工具详情面板，显示描述和参数
3. 工具测试面板，允许用户测试工具
4. 工具使用历史记录

### 4. 配置编辑器

配置编辑器将提供以下功能：

1. 可视化编辑界面，无需直接编辑 JSON
2. 配置验证和错误提示
3. 配置模板和预设
4. 配置导入和导出

## 发布和分发

MCPMate Tauri 应用将通过以下渠道发布和分发：

1. GitHub Releases
2. 应用内自动更新
3. 包管理器（如 Homebrew、Chocolatey 等）
4. 应用商店（如 Microsoft Store、Mac App Store 等）

## 参考资料

1. Tauri 官方文档：https://tauri.app/
2. React 官方文档：https://reactjs.org/
3. Vue 官方文档：https://vuejs.org/
4. Tauri 系统托盘示例：https://tauri.app/v1/guides/features/system-tray/
