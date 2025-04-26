# MCP Client

这个目录包含了一系列用于演示 MCP（Model Context Protocol）功能的工具。

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
      ]
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
