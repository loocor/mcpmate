# MCPMate AI Module - 架构设计文档

## 🏗️ 整体架构

MCPMate AI模块采用分层架构设计，确保高性能、可维护性和扩展性。

```
┌─────────────────────────────────────────────────────────────┐
│                    应用层 (Application Layer)                │
├─────────────────────────────────────────────────────────────┤
│  CLI Tool (main.rs)           │  Library API (lib.rs)       │
├─────────────────────────────────────────────────────────────┤
│                    业务层 (Business Layer)                   │
├─────────────────────────────────────────────────────────────┤
│              TextMcpExtractor (extractor.rs)                │
├─────────────────────────────────────────────────────────────┤
│                    服务层 (Service Layer)                    │
├─────────────────────────────────────────────────────────────┤
│ ModelManager │ TokenizerManager │ PromptManager │ DeviceManager │
├─────────────────────────────────────────────────────────────┤
│                    基础层 (Infrastructure Layer)             │
├─────────────────────────────────────────────────────────────┤
│    Candle-rs    │    Tokenizers    │    Metal/CPU    │    Utils    │
└─────────────────────────────────────────────────────────────┘
```

## 📦 模块设计

### 1. 配置管理 (config.rs)
**职责**: 统一管理所有配置参数
```rust
pub struct ExtractorConfig {
    pub model_path: PathBuf,
    pub max_tokens: usize,
    pub temperature: f64,
    pub repeat_penalty: f32,
    pub seed: u64,
    pub debug: bool,
}
```

**关键特性**:
- 命令行参数解析
- 默认配置提供
- 配置验证和转换

### 2. 设备管理 (device.rs)
**职责**: 硬件设备的选择和优化
```rust
pub struct DeviceManager;
impl DeviceManager {
    pub fn create_optimal_device() -> Result<Device>;
    pub fn check_device_capabilities(device: &Device) -> DeviceInfo;
}
```

**设备选择策略**:
- macOS: 优先Metal，失败则报错
- Linux/Windows: 使用CPU
- 未来可扩展CUDA支持

### 3. 模型管理 (model.rs)
**职责**: 模型加载、推理和生命周期管理
```rust
pub struct ModelManager {
    model_path: PathBuf,
    model: Option<Qwen2>,
    device: Device,
}
```

**核心优化**:
- **KV缓存**: 正确实现增量推理
- **内存映射**: 高效模型加载
- **批处理**: 支持未来批量推理

### 4. 分词器管理 (tokenizer.rs)
**职责**: 文本编码解码和分词器生命周期
```rust
pub struct TokenizerManager {
    tokenizer: Option<Tokenizer>,
    model_dir: PathBuf,
}
```

**功能特性**:
- 本地分词器优先
- 自动下载备用方案
- 编码解码性能优化

### 5. 提示词管理 (prompt.rs)
**职责**: 提示词模板和格式化
```rust
pub struct PromptManager {
    prompt_dir: PathBuf,
}
```

**模板系统**:
- 外部文件管理
- Qwen2.5聊天格式
- 动态参数注入

### 6. 核心提取器 (extractor.rs)
**职责**: 整合所有组件，提供统一API
```rust
pub struct TextMcpExtractor {
    config: ExtractorConfig,
    device: Device,
    model_manager: Option<ModelManager>,
    tokenizer_manager: Option<TokenizerManager>,
    prompt_manager: PromptManager,
}
```

**工作流程**:
1. 输入验证和预处理
2. 提示词生成和格式化
3. 文本分词和编码
4. 模型推理和生成
5. 结果解码和验证

### 7. 工具函数 (utils.rs)
**职责**: 通用功能和性能监控
```rust
pub struct TextProcessor;      // 文本预处理
pub struct InputReader;        // 输入获取
pub struct PerformanceMonitor; // 性能监控
```

## 🔄 数据流设计

### 推理流程
```
输入文本 → 预处理 → 提示词生成 → 分词 → 推理 → 解码 → JSON验证 → 输出
    ↓         ↓         ↓        ↓      ↓      ↓        ↓
TextProcessor → PromptManager → Tokenizer → Model → Tokenizer → Validator → Result
```

### 错误处理流
```
Error → anyhow::Error → Context → User-friendly Message
```

## ⚡ 性能优化策略

### 1. KV缓存实现
```rust
// 第一次推理：建立完整上下文
let logits = model.forward(&full_prompt_tokens, 0)?;

// 后续推理：增量生成
for index in 0..max_tokens {
    let logits = model.forward(&[next_token], prompt_len + index)?;
}
```

### 2. Metal加速
```rust
// 强制Metal策略，避免性能回退
let device = Device::new_metal(0)
    .expect("Metal support required on macOS");
```

### 3. 内存优化
- 使用`mmap`加载大模型
- 量化模型减少内存占用
- 智能文本截断避免OOM

### 4. 并发设计
- 模型推理单线程（GPU限制）
- 文本预处理可并行
- 批处理支持多请求

## 🔧 扩展性设计

### 1. 模型支持
```rust
trait ModelBackend {
    fn load(&mut self) -> Result<()>;
    fn generate(&mut self, tokens: &[u32]) -> Result<Vec<u32>>;
}

// 未来可支持:
// - LLaMA模型
// - OpenAI API
// - 其他量化格式
```

### 2. 设备支持
```rust
// 未来扩展:
// - CUDA支持
// - ROCm支持  
// - 分布式推理
```

### 3. 任务扩展
```rust
// 当前: MCP配置提取
// 未来: 
// - 代码生成
// - 文档总结
// - 多语言翻译
```

## 🛡️ 安全性考虑

### 1. 输入验证
- 文本长度限制
- 恶意输入检测
- 资源使用监控

### 2. 模型安全
- 模型文件完整性检查
- 推理结果验证
- 内存安全保障

### 3. 错误处理
- 优雅降级机制
- 详细错误日志
- 用户友好错误信息

## 📊 监控和调试

### 1. 性能监控
```rust
pub struct PerformanceMonitor {
    start_time: Instant,
}

impl PerformanceMonitor {
    pub fn tokens_per_second(&self, count: usize) -> f32;
    pub fn memory_usage(&self) -> usize;
}
```

### 2. 调试系统
```rust
// 分级日志输出
debug_println!("详细调试信息");
println!("用户可见信息");
eprintln!("错误信息");
```

### 3. 指标收集
- 推理延迟
- 吞吐量
- 内存使用
- GPU利用率

## 🔮 未来规划

### 短期目标
1. API服务封装
2. 批处理支持
3. 更多模型格式

### 中期目标
1. 分布式推理
2. 模型微调
3. 多模态支持

### 长期目标
1. 自适应优化
2. 边缘设备部署
3. 实时流式推理

## 📝 开发规范

### 代码组织
- 每个模块职责单一
- 公共API清晰简洁
- 内部实现可替换

### 性能要求
- 推理延迟 < 1秒
- 内存占用 < 4GB
- CPU使用率合理

### 质量保证
- 单元测试覆盖
- 集成测试验证
- 性能基准测试
