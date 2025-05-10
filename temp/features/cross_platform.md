# 支持 Windows 等其他平台

## 概述

扩展 MCPMate 的平台支持，使其能够在 Windows 和其他操作系统上运行，从而扩大潜在用户群体。

## 背景与动机

目前，MCPMate 主要针对 macOS 和 Linux 平台开发和测试。然而，Windows 是一个广泛使用的操作系统，特别是在企业环境中。通过支持 Windows 和其他平台，我们可以显著扩大 MCPMate 的潜在用户群体，提升其市场覆盖率。

## 功能目标

1. 确保 MCPMate 在 Windows 上正常运行
2. 适配 Windows 特有的路径和环境变量处理
3. 支持 Windows 特有的进程管理和通信机制
4. 提供 Windows 安装包和自动更新功能
5. 确保跨平台一致的用户体验
6. 支持其他潜在平台（如 FreeBSD 等）

## 技术设计

### 1. 跨平台抽象层

为了确保代码在不同平台上的一致性，我们将实现一个跨平台抽象层：

```rust
trait PlatformOps {
    fn get_config_dir() -> PathBuf;
    fn get_cache_dir() -> PathBuf;
    fn get_log_dir() -> PathBuf;
    fn spawn_process(command: &str, args: &[&str], env: &HashMap<String, String>) -> Result<Child>;
    fn kill_process(pid: u32) -> Result<()>;
    fn is_process_running(pid: u32) -> bool;
    // 其他平台特定操作...
}

struct WindowsPlatform;
struct MacOSPlatform;
struct LinuxPlatform;

impl PlatformOps for WindowsPlatform {
    // Windows 特定实现...
}

impl PlatformOps for MacOSPlatform {
    // macOS 特定实现...
}

impl PlatformOps for LinuxPlatform {
    // Linux 特定实现...
}

fn get_platform_ops() -> Box<dyn PlatformOps> {
    #[cfg(target_os = "windows")]
    {
        Box::new(WindowsPlatform)
    }
    #[cfg(target_os = "macos")]
    {
        Box::new(MacOSPlatform)
    }
    #[cfg(target_os = "linux")]
    {
        Box::new(LinuxPlatform)
    }
    // 其他平台...
}
```

### 2. Windows 特有适配

#### 2.1 路径处理

Windows 使用反斜杠作为路径分隔符，而 Unix 系统使用正斜杠。我们需要确保路径处理在不同平台上的一致性：

```rust
fn normalize_path(path: &str) -> String {
    #[cfg(target_os = "windows")]
    {
        path.replace("/", "\\")
    }
    #[cfg(not(target_os = "windows"))]
    {
        path.to_string()
    }
}
```

#### 2.2 进程管理

Windows 和 Unix 系统在进程管理方面有显著差异。我们需要适配这些差异：

```rust
#[cfg(target_os = "windows")]
fn spawn_process(command: &str, args: &[&str], env: &HashMap<String, String>) -> Result<Child> {
    use std::os::windows::process::CommandExt;

    let mut cmd = Command::new(command);
    cmd.args(args)
       .envs(env)
       .creation_flags(0x08000000); // CREATE_NO_WINDOW

    cmd.spawn().map_err(|e| anyhow::anyhow!("Failed to spawn process: {}", e))
}

#[cfg(not(target_os = "windows"))]
fn spawn_process(command: &str, args: &[&str], env: &HashMap<String, String>) -> Result<Child> {
    let mut cmd = Command::new(command);
    cmd.args(args)
       .envs(env);

    cmd.spawn().map_err(|e| anyhow::anyhow!("Failed to spawn process: {}", e))
}
```

#### 2.3 服务安装

在 Windows 上，我们将支持将 MCPMate 安装为系统服务：

```rust
#[cfg(target_os = "windows")]
fn install_as_service() -> Result<()> {
    // 使用 Windows 服务 API 安装服务
    // ...
}

#[cfg(target_os = "windows")]
fn uninstall_service() -> Result<()> {
    // 使用 Windows 服务 API 卸载服务
    // ...
}
```

### 3. 安装包和分发

#### 3.1 Windows 安装包

我们将使用 NSIS 或 WiX 创建 Windows 安装包：

1. **安装程序功能**：
   - 安装 MCPMate 可执行文件和依赖
   - 创建开始菜单和桌面快捷方式
   - 注册文件关联
   - 配置防火墙规则
   - 安装系统服务（可选）

2. **自动更新**：
   - 检查新版本
   - 下载更新
   - 安装更新

#### 3.2 macOS 安装包

我们将创建 macOS 安装包（.pkg）和 DMG 文件：

1. **安装程序功能**：
   - 安装 MCPMate 可执行文件和依赖
   - 创建应用程序快捷方式
   - 配置 LaunchAgent（可选）

2. **自动更新**：
   - 使用 Sparkle 框架实现自动更新

#### 3.3 Linux 安装包

我们将创建 DEB 和 RPM 安装包：

1. **安装程序功能**：
   - 安装 MCPMate 可执行文件和依赖
   - 创建桌面快捷方式
   - 配置 systemd 服务（可选）

2. **自动更新**：
   - 通过包管理器实现更新

### 4. 跨平台测试

为了确保 MCPMate 在不同平台上的一致性，我们将实现跨平台测试：

1. **自动化测试**：
   - 使用 GitHub Actions 在不同平台上运行测试
   - 测试核心功能和平台特有功能

2. **手动测试清单**：
   - 为每个平台创建手动测试清单
   - 确保关键功能在所有平台上正常工作

## 使用场景

### 1. Windows 企业环境

在 Windows 企业环境中，用户可以：

1. 下载并安装 MCPMate Windows 安装包
2. 将 MCPMate 配置为系统服务，在后台运行
3. 使用 MCPMate 管理多个 MCP 服务器
4. 与企业内部的 LLM 应用集成

### 2. 跨平台开发团队

在跨平台开发团队中，不同成员可能使用不同的操作系统：

1. Windows 用户可以使用 Windows 版 MCPMate
2. macOS 用户可以使用 macOS 版 MCPMate
3. Linux 用户可以使用 Linux 版 MCPMate

所有用户都能获得一致的体验，共享相同的配置和功能。

### 3. 教育和研究环境

在教育和研究环境中，不同的实验室和教室可能使用不同的操作系统：

1. 学生可以在自己的计算机上安装适合其操作系统的 MCPMate 版本
2. 教师可以提供统一的指导，不受操作系统差异的影响
3. 研究人员可以在不同平台上复现实验结果

## 参考资料

1. Rust 跨平台开发指南：https://rust-lang.github.io/rust-forge/platform-support.html
2. Windows 服务 API 文档：https://docs.microsoft.com/en-us/windows/win32/services/service-control-manager
3. NSIS 文档：https://nsis.sourceforge.io/Docs/
4. WiX 文档：https://wixtoolset.org/documentation/
5. Sparkle 框架文档：https://sparkle-project.org/documentation/
6. systemd 服务文档：https://www.freedesktop.org/software/systemd/man/systemd.service.html
