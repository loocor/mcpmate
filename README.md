# MCP Demo Tools

这个目录包含了一系列用于演示 MCP（Model Context Protocol）功能的工具。

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

#### 工具序列调用

```bash
python mcp_tool.py --server <server_name> --sequence <sequence_file>
```

例如：

```bash
python mcp_tool.py --server playwright --sequence sequences/playwright_screenshot.json
```

#### 输出格式

默认情况下，`mcp_tool.py` 会输出详细的结果。如果你只想要简洁的输出，可以使用 `--output compact` 选项：

```bash
python mcp_tool.py --server firecrawl --tool firecrawl_scrape --args '{"url": "https://modelcontextprotocol.io"}' --output compact
```

## 特定工具

以下是一些针对特定 MCP 服务器的工具：

### mcp_info.py

这个工具连接到所有配置的 MCP 服务器并获取它们的信息。

```bash
python mcp_info.py
```

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

## 工具序列

`sequences` 目录包含了一些预定义的工具序列：

- `playwright_screenshot.json`: 使用 playwright 服务器导航到网页并截图
- `firecrawl_scrape.json`: 使用 firecrawl 服务器抓取网页内容
- `thinking_sequence.json`: 使用 thinking 服务器进行顺序思考

你可以创建自己的工具序列，只需要按照以下格式：

```json
[
  {
    "name": "tool_name",
    "arguments": {
      "arg1": "value1",
      "arg2": "value2"
    }
  },
  {
    "name": "another_tool",
    "arguments": {
      "arg1": "value1"
    }
  }
]
```

然后使用 `mcp_tool.py` 调用它：

```bash
python mcp_tool.py --server <server_name> --sequence <your_sequence_file>
```
