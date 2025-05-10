# 自定义运行时路径管理

## 概述

提供自定义运行时路径管理功能，允许用户为不同的上游服务器指定不同的运行时环境，以避免系统依赖冲突并提高稳定性。

## 背景与动机

不同的 MCP 服务器可能依赖不同版本的运行时环境（如 Node.js、Python 等）。当这些服务器在同一系统上运行时，可能会出现依赖冲突。通过提供自定义运行时路径管理功能，我们可以为每个服务器创建隔离的运行时环境，避免这些冲突并提高系统稳定性。

## 功能目标

1. 允许用户为每类上游服务器指定自定义运行时路径
2. 自动检测系统中已安装的运行时环境
3. 提供运行时环境的版本管理功能
4. 支持多种运行时环境（Node.js、Python 等）
5. 实现运行时环境的自动下载和安装
6. 提供运行时环境的健康检查和诊断功能

## 技术设计

### 1. 运行时环境配置

#### 1.1 配置结构

```json
{
  "runtimes": {
    "node": {
      "paths": {
        "default": "/usr/local/bin/node",
        "v16": "/path/to/node/v16/bin/node",
        "v18": "/path/to/node/v18/bin/node"
      },
      "package_managers": {
        "npm": {
          "paths": {
            "default": "/usr/local/bin/npm",
            "v16": "/path/to/node/v16/bin/npm",
            "v18": "/path/to/node/v18/bin/npm"
          },
          "cache_dirs": {
            "default": "~/.npm",
            "v16": "/path/to/node/v16/cache",
            "v18": "/path/to/node/v18/cache"
          }
        },
        "npx": {
          "paths": {
            "default": "/usr/local/bin/npx",
            "v16": "/path/to/node/v16/bin/npx",
            "v18": "/path/to/node/v18/bin/npx"
          }
        },
        "uv": {
          "paths": {
            "default": "/usr/local/bin/uv",
            "latest": "/path/to/uv/latest/bin/uv"
          },
          "cache_dirs": {
            "default": "~/.uv",
            "latest": "/path/to/uv/latest/cache"
          }
        }
      }
    },
    "python": {
      "paths": {
        "default": "/usr/bin/python3",
        "3.9": "/path/to/python/3.9/bin/python3",
        "3.10": "/path/to/python/3.10/bin/python3"
      },
      "package_managers": {
        "pip": {
          "paths": {
            "default": "/usr/bin/pip3",
            "3.9": "/path/to/python/3.9/bin/pip3",
            "3.10": "/path/to/python/3.10/bin/pip3"
          },
          "cache_dirs": {
            "default": "~/.cache/pip",
            "3.9": "/path/to/python/3.9/cache",
            "3.10": "/path/to/python/3.10/cache"
          }
        }
      }
    }
  }
}
```

#### 1.2 服务器配置集成

在服务器配置中，用户可以指定要使用的运行时环境：

```json
{
  "mcpServers": {
    "server1": {
      "type": "child_process",
      "command": "npx",
      "args": ["@anthropic/claude-api", "serve"],
      "runtime": {
        "node": "v18",
        "package_manager": "npx"
      }
    },
    "server2": {
      "type": "child_process",
      "command": "python",
      "args": ["-m", "mcp_server"],
      "runtime": {
        "python": "3.10",
        "package_manager": "pip"
      }
    }
  }
}
```

### 2. 运行时环境检测

MCPMate 将自动检测系统中已安装的运行时环境：

```rust
async fn detect_runtimes() -> Result<RuntimesConfig> {
    let mut config = RuntimesConfig::default();

    // 检测 Node.js
    if let Some(node_path) = detect_node_path() {
        config.runtimes.insert("node".to_string(), RuntimeConfig {
            paths: HashMap::from([("default".to_string(), node_path)]),
            // 其他配置...
        });
    }

    // 检测 Python
    if let Some(python_path) = detect_python_path() {
        config.runtimes.insert("python".to_string(), RuntimeConfig {
            paths: HashMap::from([("default".to_string(), python_path)]),
            // 其他配置...
        });
    }

    // 检测其他运行时环境...

    Ok(config)
}
```

### 3. 环境变量注入

当启动上游服务器时，MCPMate 将根据配置注入适当的环境变量：

```rust
fn prepare_environment(server_config: &ServerConfig) -> HashMap<String, String> {
    let mut env = HashMap::new();

    // 添加基本环境变量
    env.insert("PATH".to_string(), get_path_for_runtime(server_config));

    // 添加运行时特定的环境变量
    if let Some(runtime) = &server_config.runtime {
        if let Some(node_version) = &runtime.node {
            env.insert("NODE_PATH".to_string(), get_node_path_for_version(node_version));

            // 添加包管理器特定的环境变量
            if let Some(package_manager) = &runtime.package_manager {
                match package_manager.as_str() {
                    "npm" => {
                        env.insert("NPM_CONFIG_CACHE".to_string(), get_npm_cache_for_version(node_version));
                    }
                    "uv" => {
                        env.insert("UV_CACHE_DIR".to_string(), get_uv_cache_for_version(node_version));
                    }
                    // 其他包管理器...
                    _ => {}
                }
            }
        }

        // 处理其他运行时环境...
    }

    env
}
```

### 4. 运行时环境管理

MCPMate 将提供运行时环境的管理功能：

1. **版本管理**：安装、更新和删除不同版本的运行时环境
2. **健康检查**：检查运行时环境的健康状态
3. **诊断**：诊断运行时环境的问题
4. **自动修复**：自动修复常见问题

## 使用场景

### 1. 多版本 Node.js

用户系统中安装了多个版本的 Node.js，不同的 MCP 服务器需要使用不同的版本：

1. 服务器 A 需要 Node.js v16
2. 服务器 B 需要 Node.js v18

通过自定义运行时路径管理功能，用户可以为每个服务器指定正确的 Node.js 版本，避免版本冲突。

### 2. Python 虚拟环境

用户需要为不同的 Python MCP 服务器使用不同的虚拟环境：

1. 服务器 A 需要使用虚拟环境 A
2. 服务器 B 需要使用虚拟环境 B

通过自定义运行时路径管理功能，用户可以为每个服务器指定正确的 Python 虚拟环境，避免依赖冲突。

### 3. 包管理器缓存隔离

用户需要为不同的服务器使用不同的包管理器缓存目录，以避免缓存冲突：

1. 服务器 A 使用 npm 缓存 A
2. 服务器 B 使用 npm 缓存 B

通过自定义运行时路径管理功能，用户可以为每个服务器指定不同的缓存目录，避免缓存冲突。

## 参考资料

1. Node.js 版本管理工具：
   - nvm: https://github.com/nvm-sh/nvm
   - n: https://github.com/tj/n
   - volta: https://volta.sh/

2. Python 版本管理工具：
   - pyenv: https://github.com/pyenv/pyenv
   - conda: https://docs.conda.io/

3. 包管理器文档：
   - npm: https://docs.npmjs.com/
   - pip: https://pip.pypa.io/
   - uv: https://github.com/astral-sh/uv
