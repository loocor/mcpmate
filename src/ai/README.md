# MCPMate AI Module

基于Candle-rs和Qwen2.5模型的高性能MCP配置提取器，提供本地AI推理能力。

## 🚀 核心特性

### 性能优化
- **Metal加速**: macOS平台自动启用Metal GPU加速
- **KV缓存**: 正确实现增量推理，大幅提升生成速度
- **量化模型**: 使用GGUF Q4量化，平衡性能与质量
- **推理速度**: 90+ tokens/s (Metal) / 8+ tokens/s (CPU)

### 架构设计
- **模块化**: 清晰的职责分离，便于维护和扩展
- **类型安全**: 完整的Rust类型系统保障
- **错误处理**: 统一的错误处理和用户友好的错误信息
- **配置管理**: 灵活的配置系统，支持命令行和代码配置

## 📁 项目结构

```
src/
├── lib.rs              # 库入口，导出公共API
├── main.rs             # 命令行工具入口
├── config.rs           # 配置管理（命令行参数、提取器配置）
├── device.rs           # 设备管理（Metal/CPU选择和优化）
├── extractor.rs        # 核心提取器（整合所有组件）
├── model.rs            # 模型管理（加载、推理、KV缓存）
├── prompt.rs           # 提示词管理（模板、加载、格式化）
├── tokenizer.rs        # 分词器管理（编码、解码、下载）
└── utils.rs            # 工具函数（调试、性能监控、文本处理）

prompts/
└── extract_rules.txt  # MCP提取提示词模板
```

## 🛠️ 使用方式

### 命令行工具

```bash
# 从文件提取
./extractor --file input.txt --max-tokens 100

# 从标准输入提取
echo "配置文本" | ./extractor --stdin

# 启用调试模式
./extractor --file input.txt --debug

# 自定义参数
./extractor --file input.txt \
  --max-tokens 200 \
  --temperature 0.8 \
  --repeat-penalty 1.2
```

### 库API

```rust
use mcpmate_ai::{TextMcpExtractor, ExtractorConfig};

// 创建配置
let config = ExtractorConfig::default();

// 创建提取器
let mut extractor = TextMcpExtractor::new(config)?;

// 执行提取
let result = extractor.extract("输入文本")?;
println!("{}", serde_json::to_string_pretty(&result)?);
```

## ⚙️ 配置选项

| 参数 | 默认值 | 说明 |
|------|--------|------|
| `model_path` | `~/.mcpmate/models/qwen2.5-0.5b-instruct-q4.gguf` | 模型文件路径 |
| `max_tokens` | 100 | 最大生成token数 |
| `temperature` | 0.7 | 生成温度（0.0-1.0） |
| `repeat_penalty` | 1.1 | 重复惩罚系数 |
| `seed` | 42 | 随机种子 |
| `debug` | false | 启用调试输出 |

## 🔧 性能调优

### 设备选择策略
```rust
// macOS: 强制使用Metal，失败则报错
let device = Device::new_metal(0)?;

// 其他平台: 使用CPU
let device = Device::Cpu;
```

### KV缓存优化
```rust
// 第一次推理：建立KV缓存
let logits = model.forward(&full_input, 0)?;

// 后续推理：利用缓存
let logits = model.forward(&single_token, position)?;
```

### 内存优化
- 使用内存映射加载模型
- 量化模型减少内存占用
- 智能文本预处理，避免过长输入

## 📊 性能基准

| 环境 | 设备 | 速度 | 内存占用 |
|------|------|------|----------|
| macOS M1 | Metal | 90+ tokens/s | ~2GB |
| macOS M1 | CPU | 8+ tokens/s | ~1.5GB |
| Linux x64 | CPU | 5+ tokens/s | ~1.5GB |

## 🔍 调试模式

启用`--debug`参数可查看详细信息：
- 模型加载时间
- 分词结果
- 推理过程
- 性能指标

## 🚨 故障排除

### 常见问题

1. **Metal初始化失败**
   ```
   解决方案: 确保macOS版本支持Metal，或使用CPU模式
   ```

2. **模型文件未找到**
   ```
   解决方案: 检查模型路径，或下载到指定位置
   ```

3. **分词器下载失败**
   ```
   解决方案: 手动下载tokenizer.json到模型目录
   ```

### 性能问题

1. **推理速度慢**
   - 检查是否启用了Metal加速
   - 确认KV缓存正常工作
   - 减少max_tokens参数

2. **内存占用高**
   - 使用更小的量化模型
   - 减少输入文本长度
   - 检查内存泄漏

## 📝 开发指南

### 添加新功能
1. 在相应模块中实现功能
2. 更新公共API（lib.rs）
3. 添加测试用例
4. 更新文档

### 性能优化
1. 使用`cargo bench`进行基准测试
2. 分析热点函数
3. 优化算法和数据结构
4. 验证优化效果

## 📄 许可证

MIT License - 详见 LICENSE 文件
