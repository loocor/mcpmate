//! MCPMate AI module: Text MCP configuration extractor
//!
//! Based on Qwen2.5 0.5B model for local inference, converts input text to MCP service configuration JSON

use anyhow::{Context, Result};
use candle_core::Device;
use serde_json::Value;
use std::path::PathBuf;

use crate::config::ExtractorConfig;
use crate::constants::validation;
use crate::device::DeviceManager;
use crate::model::ModelManager;
use crate::prompt::PromptManager;
use crate::tokenizer::TokenizerManager;
use crate::{
    debug_println,
    utils::{PerformanceMonitor, TextProcessor},
};

/// MCPMate AI module: Text MCP configuration extractor
///
/// Based on Qwen2.5 0.5B model for local inference, converts input text to MCP service configuration JSON
pub struct TextMcpExtractor {
    config: ExtractorConfig,
    device: Device,
    model_manager: Option<ModelManager>,
    tokenizer_manager: Option<TokenizerManager>,
    prompt_manager: PromptManager,
}

impl TextMcpExtractor {
    /// Create new extractor instance
    pub fn new(config: ExtractorConfig) -> Result<Self> {
        // Create optimal device
        let device = DeviceManager::create_optimal_device()?;

        // Create prompt manager
        let prompt_manager = PromptManager::new(None);

        Ok(Self {
            config,
            device,
            model_manager: None,
            tokenizer_manager: None,
            prompt_manager,
        })
    }

    /// Create extractor from model path
    pub fn from_model_path(model_path: PathBuf) -> Result<Self> {
        let config = ExtractorConfig {
            model_path,
            ..ExtractorConfig::default()
        };
        Self::new(config)
    }

    /// Initialize extractor (load model and tokenizer)
    pub fn initialize(&mut self) -> Result<()> {
        println!("🚀 Initializing MCP configuration extractor...");

        // Initialize model manager
        let mut model_manager = ModelManager::new(self.config.model_path.clone(), self.device.clone());
        model_manager.load_model()?;
        self.model_manager = Some(model_manager);

        // Initialize tokenizer manager
        let mut tokenizer_manager = TokenizerManager::new(&self.config.model_path);
        tokenizer_manager.load_tokenizer()?;
        self.tokenizer_manager = Some(tokenizer_manager);

        println!("✅ Extractor initialized successfully");
        Ok(())
    }

    /// Extract MCP configuration
    pub fn extract(
        &mut self,
        input_text: &str,
    ) -> Result<Value> {
        println!("🚀 Starting MCP configuration extraction workflow");
        debug_println!("📄 Input text: {}", input_text);

        // Input validation
        self.validate_input(input_text)?;

        // Ensure initialized
        if self.model_manager.is_none() || self.tokenizer_manager.is_none() {
            self.initialize()?;
        }

        let monitor = PerformanceMonitor::start();

        // Step 1: Preprocess text
        let processed_text = TextProcessor::preprocess(input_text);

        // Step 2: Generate prompt
        let prompt = self.prompt_manager.create_mcp_extract_prompt(&processed_text)?;
        debug_println!("📝 Prompt length: {} characters", prompt.len());

        // Step 3: Tokenize
        let tokenizer = self.tokenizer_manager.as_ref().unwrap();
        let prompt_tokens = tokenizer.encode(&prompt)?;
        debug_println!("📊 Input tokens: {}", prompt_tokens.len());

        // Step 4: Inference
        let model = self.model_manager.as_mut().unwrap();
        let generated_tokens = model.generate(
            &prompt_tokens,
            self.config.max_tokens,
            self.config.temperature,
            self.config.top_k,
            self.config.top_p,
            self.config.min_p,
            self.config.repeat_penalty,
            self.config.seed,
        )?;

        // Step 5: Decode
        let generated_text = tokenizer.decode(&generated_tokens)?;
        debug_println!("📝 Generated text: {}", generated_text.trim());

        // Step 6: Parse and validate
        let config = self.parse_and_validate_output(&generated_text)?;

        let total_duration = monitor.elapsed();
        println!("🎉 Extraction completed in {:.2}s", total_duration.as_secs_f32());

        Ok(config)
    }

    /// Validate input
    fn validate_input(
        &self,
        text: &str,
    ) -> Result<()> {
        let trimmed = text.trim();

        if trimmed.is_empty() {
            anyhow::bail!("Input text cannot be empty");
        }

        if trimmed.len() < validation::MIN_INPUT_LENGTH {
            anyhow::bail!(
                "Input text too short, minimum {} characters required",
                validation::MIN_INPUT_LENGTH
            );
        }

        if text.len() > validation::MAX_INPUT_LENGTH {
            anyhow::bail!(
                "Input text too long, maximum {} characters supported",
                validation::MAX_INPUT_LENGTH
            );
        }

        // Check for meaningful content
        let text_lower = trimmed.to_lowercase();
        let has_meaningful_content = validation::MEANINGFUL_KEYWORDS
            .iter()
            .any(|keyword| text_lower.contains(keyword));

        if !has_meaningful_content {
            anyhow::bail!(
                "Input does not contain meaningful MCP-related content. Please include keywords like 'mcp', 'server', 'tool', 'config', etc."
            );
        }

        Ok(())
    }

    /// Parse and validate output
    fn parse_and_validate_output(
        &self,
        raw_output: &str,
    ) -> Result<Value> {
        println!("🔍 Parsing and validating JSON output...");

        let trimmed_output = raw_output.trim();

        // Check if it's "无MCP配置"
        if trimmed_output == "无MCP配置" || trimmed_output.is_empty() {
            return Ok(serde_json::json!({
                "mcpServers": {}
            }));
        }

        // Try to parse JSON
        let parsed: Value = serde_json::from_str(trimmed_output)
            .with_context(|| format!("Failed to parse JSON: {}", trimmed_output))?;

        // Validate JSON structure
        self.validate_mcp_config(&parsed)?;

        println!("✅ JSON validation passed");
        Ok(parsed)
    }

    /// Validate MCP configuration structure
    fn validate_mcp_config(
        &self,
        config: &Value,
    ) -> Result<()> {
        // Check if root object contains mcpServers
        let servers = config
            .get("mcpServers")
            .ok_or_else(|| anyhow::anyhow!("Missing 'mcpServers' field"))?;

        if !servers.is_object() {
            anyhow::bail!("'mcpServers' must be an object");
        }

        let servers_obj = servers.as_object().unwrap();

        // Validate each server configuration
        for (server_name, server_config) in servers_obj {
            self.validate_server_config(server_name, server_config)?;
        }

        println!("✅ Found {} server configurations", servers_obj.len());
        Ok(())
    }

    /// Validate single server configuration
    fn validate_server_config(
        &self,
        name: &str,
        config: &Value,
    ) -> Result<()> {
        if !config.is_object() {
            anyhow::bail!("Server '{}' configuration must be an object", name);
        }

        let config_obj = config.as_object().unwrap();

        // Check required command field
        if !config_obj.contains_key("command") {
            anyhow::bail!("Server '{}' missing required 'command' field", name);
        }

        // Check args field (if exists)
        if let Some(args) = config_obj.get("args") {
            if !args.is_array() {
                anyhow::bail!("Server '{}' 'args' field must be an array", name);
            }
        }

        Ok(())
    }

    /// Get extractor status
    pub fn status(&self) -> ExtractorStatus {
        ExtractorStatus {
            model_loaded: self.model_manager.as_ref().map_or(false, |m| m.is_loaded()),
            tokenizer_loaded: self.tokenizer_manager.as_ref().map_or(false, |t| t.is_loaded()),
            device: format!("{:?}", self.device),
            config: self.config.clone(),
        }
    }
}

/// Extractor status information
#[derive(Debug, Clone)]
pub struct ExtractorStatus {
    pub model_loaded: bool,
    pub tokenizer_loaded: bool,
    pub device: String,
    pub config: ExtractorConfig,
}

impl ExtractorStatus {
    /// Check if ready
    pub fn is_ready(&self) -> bool {
        self.model_loaded && self.tokenizer_loaded
    }

    /// Get status summary
    pub fn summary(&self) -> String {
        format!(
            "Model: {}, Tokenizer: {}, Device: {}",
            if self.model_loaded { "✅" } else { "❌" },
            if self.tokenizer_loaded { "✅" } else { "❌" },
            self.device
        )
    }
}
