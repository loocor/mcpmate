# MCPMan 测试目录

本目录包含 MCPMan 项目的测试文件和示例配置。

## 目录结构

- `config/`: 测试用配置文件
  - `mcp.json`: MCP 服务器配置文件
  - `rule.json5`: 规则配置文件
- `samples/`: 示例工具调用配置
  - `firecrawl_scrape.json`: Firecrawl 抓取工具示例
  - `playwright_screenshot.json`: Playwright 截图工具示例
  - `thinking_sequence.json`: Sequential Thinking 工具示例
- `config_tests.rs`: 配置加载测试

## 运行测试

使用以下命令运行测试：

```bash
cargo test
```

## 示例用法

示例配置文件可以用于测试 MCP 工具调用。例如：

```bash
cargo run --bin mcpman-proxy -- --tool --conf tests/samples/firecrawl_scrape.json
```
