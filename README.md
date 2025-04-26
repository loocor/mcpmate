# MCP-Proxy

这个项目是一个 MCP（Model Context Protocol）代理服务器和客户端工具集的实现，旨在简化与 MCP 服务器的交互，并提供一种统一的方式来访问多个 MCP 服务器的工具。

## 项目概述

MCP（Model Context Protocol）是一个开放标准，用于定义 AI 模型与外部工具和资源的交互方式。本项目提供了以下功能：

1. **MCP 代理服务器**：将多个 MCP 服务器的工具整合到一起，提供统一的接口
2. **MCP 客户端**：用于连接和管理 MCP 服务器
3. **工具调用**：支持调用 MCP 服务器提供的各种工具
4. **配置管理**：通过 JSON 配置文件管理服务器设置

本项目支持多种 MCP 服务器，包括：

- **Firecrawl**：提供网页抓取和搜索功能
- **Playwright**：提供浏览器自动化功能
- **Sequential Thinking**：提供顺序思考功能

## 核心功能

### MCP 代理服务器

项目的核心是 `mcp_proxy.py`，这是一个功能完整的 MCP 代理服务器，能够：

- 连接到多个 MCP 服务器
- 收集所有服务器的工具信息
- 将这些工具作为自己的工具提供给调用者
- 将工具调用请求转发给适当的服务器
- 处理错误和重连

### 客户端工具集

除了代理服务器外，项目还提供了一系列客户端工具，用于与 MCP 服务器交互：

## 服务器管理

### mcp_client.py

这是一个用于管理和查询 MCP 服务器的工具。它提供了以下命令：

#### 列出所有服务器

```bash
python mcp_client.py --list
```

#### 获取特定服务器的信息

```bash
python mcp_client.py --server <server_name>
```

例如：

```bash
python mcp_client.py --server firecrawl
```

#### 获取所有服务器的信息

```bash
python mcp_client.py --all
```

如果你已经安装了这个包（使用 `pip install -e .` 或 `uv pip install -e .`），你也可以使用 `mcp-client` 命令：

```bash
mcp-client --list
mcp-client --server firecrawl
mcp-client --all
```

## 通用工具

### mcp_tool.py

这是一个通用的工具，可以调用任何 MCP 服务器的任何工具。它支持单个工具调用和工具序列调用。

#### 单个工具调用

```bash
python mcp_tool.py --server <server_name> --tool <tool_name> --args '<json_args>'
```

例如：

```bash
python mcp_tool.py --server firecrawl --tool firecrawl_scrape --args '{"url": "https://modelcontextprotocol.io", "formats": ["markdown"], "onlyMainContent": true}'
```

#### 工具配置调用

```bash
python mcp_tool.py --conf <config_file>
```

例如：

```bash
python mcp_tool.py --conf sample/firecrawl_scrape.json
```

配置文件可以包含服务器信息，这样就不需要在命令行中指定 `--server` 参数：

```json
[
  {
    "server": "firecrawl",
    "tool": "firecrawl_scrape",
    "arguments": {
      "url": "https://modelcontextprotocol.io",
      "formats": ["markdown"],
      "onlyMainContent": true
    }
  }
]
```

#### 输出格式

默认情况下，`mcp_tool.py` 会输出详细的结果。如果你只想要简洁的输出，可以使用 `--output compact` 选项：

```bash
python mcp_tool.py --server firecrawl --tool firecrawl_scrape --args '{"url": "https://modelcontextprotocol.io"}' --output compact
```

## 特定工具

以下是一些针对特定 MCP 服务器的工具：

> **注意**：这些特定工具的功能现在也可以通过 `mcp_client.py` 和 `mcp_tool.py` 实现。

### mcp_scrape.py

这个工具使用 firecrawl 服务器的 firecrawl_scrape 工具抓取网页内容。

```bash
python mcp_scrape.py <url>
```

例如：

```bash
python mcp_scrape.py https://modelcontextprotocol.io
```

### mcp_playwright.py

这个工具使用 playwright 服务器的 playwright_navigate 和 playwright_screenshot 工具。

```bash
python mcp_playwright.py <url>
```

例如：

```bash
python mcp_playwright.py https://modelcontextprotocol.io
```

### mcp_thinking.py

这个工具使用 thinking 服务器的 sequentialthinking 工具。

```bash
python mcp_thinking.py "<problem>"
```

例如：

```bash
python mcp_thinking.py "如何设计一个高效的 MCP 客户端？"
```

### mcp_proxy.py

这个工具启动一个代理服务器，它能够将其他已启用的 MCP 服务器的工具作为自己的工具提供给调用者。

```bash
python mcp_proxy.py [options]
```

选项:
- `--config <path>`: 指定配置文件路径 (默认: mcp.json)
- `--stdio`: 以 stdio 模式运行 (用于 MCP 客户端)
- `--debug`: 启用调试日志
- `--log-file <path>`: 指定日志文件路径 (默认: logs/mcp_proxy.log)

这个代理服务器会连接到 `mcp.json` 中配置的所有已启用的服务器（`enabled: true`），并将它们的工具作为自己的工具提供给调用者。这对于需要访问多个 MCP 服务器工具的应用程序（如 Cursor）非常有用。

#### 在 Cursor 中使用代理服务器

要在 Cursor 中使用代理服务器，需要修改 `.cursor/mcp.json` 文件：

```json
{
  "servers": [
    {
      "name": "mcp-proxy",
      "transport": {
        "type": "stdio",
        "command": "/完整/路径/到/python",
        "args": [
          "/完整/路径/到/mcp_proxy.py",
          "--stdio",
          "--config",
          "/完整/路径/到/mcp.json",
          "--log-file",
          "/完整/路径/到/logs/cursor_proxy.log"
        ]
      }
    }
  ]
}
```

注意事项:
1. 使用 Python 解释器的**完整路径**，而不是简单的 `python` 命令
2. 使用 `mcp_proxy.py` 文件的**完整路径**
3. 使用 `mcp.json` 配置文件的**完整路径**
4. 指定日志文件的**完整路径**，以便于调试

#### 代理服务器的工作原理

代理服务器的工作流程如下:

1. 启动时，连接到 `mcp.json` 中配置的所有已启用的服务器
2. 收集所有服务器的工具信息
3. 将这些工具作为自己的工具提供给调用者
4. 当调用者请求调用工具时，代理服务器会找到拥有该工具的服务器，并将请求转发给它
5. 将服务器的响应返回给调用者

代理服务器具有以下特性:

- **自动重连**: 如果与服务器的连接断开，代理服务器会自动尝试重新连接
- **错误处理**: 代理服务器会捕获并记录错误，并尝试恢复
- **日志记录**: 代理服务器会记录详细的日志，以便于调试
- **超时处理**: 代理服务器会设置超时，避免无限等待

## 服务器配置

### mcp.json 配置文件

MCP 客户端使用 `mcp.json` 配置文件来定义服务器设置。以下是配置文件的格式：

```json
{
  "mcpServers": {
    "server_name": {
      "command": "npx",
      "commandPath": "./runtime/node-darwin-arm64/bin",  // 可选，指定命令的路径
      "args": [
        "--loglevel", "verbose",  // 注意：参数和值必须分开
        "-y",
        "package-name"
      ],
      "enabled": true,  // 可选，指定服务器是否启用
      "env": {
        "ENV_VAR": "value"
      }
    }
  }
}
```

配置选项说明：

- `command`: 要执行的命令（通常是 `npx`）
- `commandPath`: （可选）命令的路径，如果指定，将与 `command` 拼接形成完整路径
- `args`: 命令行参数数组。**重要**：参数和值必须作为单独的数组元素，例如 `["--loglevel", "verbose"]` 而不是 `["--loglevel verbose"]`
- `enabled`: （可选）指定服务器是否启用，默认为 `false`
- `env`: 环境变量对象

例如，要使用项目内的 Node.js 运行时，可以这样配置：

```json
{
  "mcpServers": {
    "firecrawl": {
      "command": "npx",
      "commandPath": "./runtime/node-darwin-arm64/bin",
      "args": [
        "--loglevel", "verbose",  // 参数和值必须分开
        "--cache", "/path/to/cache",
        "-y",
        "firecrawl-mcp"
      ],
      "enabled": true
    },
    "proxy": {
      "command": "python",
      "args": [
        "mcp_proxy.py",
        "--stdio"
      ],
      "enabled": true
    }
  }
}
```

## 工具配置

`sample` 目录包含了一些预定义的工具配置：

- `playwright_screenshot.json`: 使用 playwright 服务器导航到网页并截图
- `firecrawl_scrape.json`: 使用 firecrawl 服务器抓取网页内容
- `thinking_sequence.json`: 使用 thinking 服务器进行顺序思考

你可以创建自己的工具配置，有两种格式可供选择：

### 1. 单个工具调用

```json
[
  {
    "server": "firecrawl",
    "tool": "firecrawl_scrape",
    "arguments": {
      "url": "https://modelcontextprotocol.io",
      "formats": ["markdown"],
      "onlyMainContent": true
    }
  }
]
```

### 2. 工具序列

```json
[
  {
    "server": "server_name",
    "tool": "tool_name",
    "arguments": {
      "arg1": "value1",
      "arg2": "value2"
    }
  },
  {
    "server": "server_name",
    "tool": "another_tool",
    "arguments": {
      "arg1": "value1"
    }
  }
]
```

然后使用 `mcp_tool.py` 调用它：

```bash
python mcp_tool.py --conf <your_config_file>
```

或者，如果配置文件中没有指定服务器：

```bash
python mcp_tool.py --server <server_name> --conf <your_config_file>
```

## 总结

MCP-Proxy 项目提供了一套完整的工具，用于与 MCP 服务器交互。通过这些工具，你可以：

1. **管理 MCP 服务器**：连接、查询和管理多个 MCP 服务器
2. **调用工具**：调用 MCP 服务器提供的各种工具，包括单个工具调用和工具序列调用
3. **代理服务器**：启动一个代理服务器，将多个 MCP 服务器的工具整合到一起，提供统一的接口

这个项目特别适合以下场景：

- **开发 AI 应用**：需要访问多种外部工具和资源的 AI 应用
- **集成到 IDE**：如 Cursor 等需要访问多个 MCP 服务器工具的 IDE
- **工具链自动化**：需要将多个工具组合成工作流的自动化场景

### 未来计划

未来，我们计划添加以下功能：

1. **更多传输类型**：支持 SSE 和 streamableHttp 等传输类型，提供更好的稳定性和可扩展性
2. **更多服务器支持**：添加对更多 MCP 服务器的支持
3. **更好的错误处理**：提供更详细的错误信息和恢复机制
4. **更好的日志记录**：提供更详细的日志记录，以便于调试
5. **更好的配置管理**：提供更灵活的配置管理，包括环境变量和占位符支持

### 贡献

欢迎贡献代码、报告问题或提出建议。请通过 GitHub Issues 或 Pull Requests 提交你的贡献。
