//! AI module for MCP configuration extraction
//!
//! Simplified AI module based on Qwen2.5 model for extracting MCP server configurations from text

use anyhow::{Context, Result};
use candle_core::{Device, Tensor};
use candle_transformers::{
    generation::{LogitsProcessor, Sampling},
    models::quantized_qwen2::ModelWeights as Qwen2,
    utils::apply_repeat_penalty,
};
use serde_json::Value;
use std::path::PathBuf;
use tokenizers::Tokenizer;

// ============================================================================
// Configuration
// ============================================================================

/// AI configuration
#[derive(Debug, Clone)]
pub struct AiConfig {
    pub model_path: PathBuf,
    pub max_tokens: usize,
    pub temperature: f64,
    pub debug: bool,
    pub prompt_template_dir: PathBuf,
    pub system_prompt_file: String,
    pub user_template_file: String,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            model_path: default_model_path(),
            max_tokens: 500,
            temperature: 0.1, // Low temperature for structured output
            debug: false,
            prompt_template_dir: default_prompt_template_dir(),
            system_prompt_file: "system.txt".to_string(),
            user_template_file: "template.txt".to_string(),
        }
    }
}

impl AiConfig {
    /// Create new config with custom model path
    pub fn with_model_path(model_path: PathBuf) -> Self {
        Self {
            model_path,
            ..Default::default()
        }
    }

    /// Enable debug mode
    pub fn with_debug(
        mut self,
        debug: bool,
    ) -> Self {
        self.debug = debug;
        self
    }

    /// Set max tokens
    pub fn with_max_tokens(
        mut self,
        max_tokens: usize,
    ) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Set temperature
    pub fn with_temperature(
        mut self,
        temperature: f64,
    ) -> Self {
        self.temperature = temperature;
        self
    }

    /// Set prompt template directory
    pub fn with_prompt_template_dir(
        mut self,
        dir: PathBuf,
    ) -> Self {
        self.prompt_template_dir = dir;
        self
    }

    /// Set system prompt file name
    pub fn with_system_prompt_file(
        mut self,
        filename: String,
    ) -> Self {
        self.system_prompt_file = filename;
        self
    }

    /// Get full path to system prompt file
    pub fn system_prompt_path(&self) -> PathBuf {
        self.prompt_template_dir.join(&self.system_prompt_file)
    }

    /// Get full path to user template file
    pub fn user_template_path(&self) -> PathBuf {
        self.prompt_template_dir.join(&self.user_template_file)
    }
}

/// Get default model path
pub fn default_model_path() -> PathBuf {
    // Try LM Studio location first
    let lm_studio_path =
        PathBuf::from("/Users/Loocor/.lmstudio/models/Qwen/Qwen2.5-0.5B-Instruct-GGUF/qwen2.5-0.5b-instruct-q8_0.gguf");
    if lm_studio_path.exists() {
        return lm_studio_path;
    }

    // Fallback to MCPMate managed location
    PathBuf::from("/Users/Loocor/.mcpmate/models/qwen2.5-0.5b-instruct-q4.gguf")
}

/// Get default prompt template directory
pub fn default_prompt_template_dir() -> PathBuf {
    PathBuf::from("/Users/Loocor/.mcpmate/models/prompts")
}

/// EOS tokens for Qwen2.5 model
const EOS_TOKENS: &[u32] = &[151645, 151643, 151644];

/// Check if token is EOS
fn is_eos_token(token: u32) -> bool {
    EOS_TOKENS.contains(&token)
}

/// Load prompt template from file with fallback to default
fn load_prompt_template(
    file_path: &PathBuf,
    default_content: &str,
) -> String {
    match std::fs::read_to_string(file_path) {
        Ok(content) => {
            tracing::debug!("Loaded prompt template from: {:?}", file_path);
            content
        }
        Err(e) => {
            tracing::warn!(
                "Failed to load prompt template from {:?}: {}. Using default.",
                file_path,
                e
            );
            default_content.to_string()
        }
    }
}

/// Replace variables in template string
fn replace_template_variables(
    template: &str,
    input_text: &str,
) -> String {
    template.replace("{{INPUT_TEXT}}", input_text)
}

// ============================================================================
// Model Manager
// ============================================================================

/// Simplified model wrapper
struct ModelManager {
    model: Qwen2,
    tokenizer: Tokenizer,
    device: Device,
}

impl ModelManager {
    /// Load model and tokenizer
    async fn load(config: &AiConfig) -> Result<Self> {
        if config.debug {
            println!("🔄 Loading model: {:?}", config.model_path);
        }

        // Check model exists
        if !config.model_path.exists() {
            anyhow::bail!("Model file not found: {:?}", config.model_path);
        }

        // Create device
        let device = if cfg!(target_os = "macos") {
            Device::new_metal(0).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to initialize Metal device: {}. Metal support is required on macOS.",
                    e
                )
            })?
        } else {
            Device::Cpu
        };

        if config.debug {
            println!("🚀 Using device: {:?}", device);
        }

        // Load model
        let mut file = std::fs::File::open(&config.model_path)
            .with_context(|| format!("Failed to open model file: {:?}", config.model_path))?;

        let content = candle_core::quantized::gguf_file::Content::read(&mut file)
            .with_context(|| "Failed to read GGUF file content")?;

        let model = Qwen2::from_gguf(content, &mut file, &device)
            .with_context(|| "Failed to create quantized model from GGUF")?;

        // Load tokenizer
        let tokenizer_path = config
            .model_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("tokenizer.json");

        let tokenizer = if tokenizer_path.exists() {
            Tokenizer::from_file(&tokenizer_path).map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {}", e))?
        } else {
            // Try to download from HuggingFace
            let api = hf_hub::api::sync::Api::new().map_err(|e| anyhow::anyhow!("Failed to create HF API: {}", e))?;
            let repo = api.model("Qwen/Qwen2.5-0.5B-Instruct".to_string());
            let downloaded_tokenizer = repo
                .get("tokenizer.json")
                .map_err(|e| anyhow::anyhow!("Failed to download tokenizer: {}", e))?;

            Tokenizer::from_file(&downloaded_tokenizer)
                .map_err(|e| anyhow::anyhow!("Failed to load downloaded tokenizer: {}", e))?
        };

        if config.debug {
            println!("✅ Model and tokenizer loaded successfully");
        }

        Ok(Self {
            model,
            tokenizer,
            device,
        })
    }

    /// Generate text from prompt
    async fn generate(
        &mut self,
        prompt: &str,
        config: &AiConfig,
    ) -> Result<String> {
        if config.debug {
            println!("🔥 Starting inference");
        }

        // Encode prompt
        let prompt_tokens = self
            .tokenizer
            .encode(prompt, true)
            .map_err(|e| anyhow::anyhow!("Tokenization failed: {}", e))?
            .get_ids()
            .to_vec();

        if config.debug {
            println!("📊 Input tokens: {}", prompt_tokens.len());
        }

        // Generate tokens
        let generated_tokens = self.generate_tokens(&prompt_tokens, config)?;

        // Decode
        let generated_text = self
            .tokenizer
            .decode(&generated_tokens, true)
            .map_err(|e| anyhow::anyhow!("Decoding failed: {}", e))?;

        if config.debug {
            println!("✅ Generated {} tokens", generated_tokens.len());
        }

        Ok(generated_text)
    }

    /// Generate tokens using the model
    fn generate_tokens(
        &mut self,
        input_tokens: &[u32],
        config: &AiConfig,
    ) -> Result<Vec<u32>> {
        // Setup sampling
        let sampling = if config.temperature <= 0.0 {
            Sampling::ArgMax
        } else {
            Sampling::TopKThenTopP {
                k: 10,
                p: 0.8,
                temperature: config.temperature,
            }
        };
        let mut logits_processor = LogitsProcessor::from_sampling(42, sampling);

        // Process initial prompt
        let input = Tensor::new(input_tokens, &self.device)?.unsqueeze(0)?;
        let logits = self.model.forward(&input, 0)?;
        let logits = logits.squeeze(0)?;
        let mut next_token = logits_processor.sample(&logits)?;

        let mut generated_tokens = vec![next_token];
        let mut all_tokens = input_tokens.to_vec();
        all_tokens.push(next_token);

        // Generate remaining tokens
        for index in 0..config.max_tokens.saturating_sub(1) {
            let input = Tensor::new(&[next_token], &self.device)?.unsqueeze(0)?;
            let logits = self.model.forward(&input, input_tokens.len() + index)?;
            let logits = logits.squeeze(0)?;

            // Apply repeat penalty
            let logits = apply_repeat_penalty(&logits, 1.1, &all_tokens[all_tokens.len().saturating_sub(64)..])?;

            next_token = logits_processor.sample(&logits)?;

            // Check for EOS
            if is_eos_token(next_token) {
                if config.debug {
                    println!("🔍 EOS token detected, stopping generation");
                }
                break;
            }

            generated_tokens.push(next_token);
            all_tokens.push(next_token);
        }

        Ok(generated_tokens)
    }
}

// ============================================================================
// Text MCP Extractor
// ============================================================================

/// Text MCP configuration extractor
pub struct TextMcpExtractor {
    config: AiConfig,
    model: Option<ModelManager>,
}

impl TextMcpExtractor {
    /// Create new extractor
    pub fn new(config: AiConfig) -> Result<Self> {
        Ok(Self { config, model: None })
    }

    /// Extract MCP configuration from text
    pub async fn extract(
        &mut self,
        input_text: &str,
    ) -> Result<Value> {
        // Validate input
        self.validate_input(input_text)?;

        // Ensure model is loaded
        if self.model.is_none() {
            if self.config.debug {
                println!("🚀 Loading model...");
            }
            self.model = Some(ModelManager::load(&self.config).await?);
        }

        // Generate prompt
        let prompt = self.create_prompt(input_text);

        // Generate response
        let response = self.model.as_mut().unwrap().generate(&prompt, &self.config).await?;

        // Parse and validate response
        self.parse_response(&response)
    }

    /// Validate input text
    fn validate_input(
        &self,
        text: &str,
    ) -> Result<()> {
        let trimmed = text.trim();

        if trimmed.is_empty() {
            anyhow::bail!("Input text cannot be empty");
        }

        if trimmed.len() < 10 {
            anyhow::bail!("Input text too short, minimum 10 characters required");
        }

        if text.len() > 20000 {
            anyhow::bail!("Input text too long, maximum 20000 characters supported");
        }

        // Check for meaningful content
        let text_lower = trimmed.to_lowercase();
        let meaningful_keywords = ["mcp", "server", "tool", "resource", "api", "service", "config"];
        let has_meaningful_content = meaningful_keywords.iter().any(|keyword| text_lower.contains(keyword));

        if !has_meaningful_content {
            anyhow::bail!("Input does not contain meaningful MCP-related content");
        }

        Ok(())
    }

    /// Create extraction prompt
    fn create_prompt(
        &self,
        input_text: &str,
    ) -> String {
        // Default system message (fallback if file not found)
        let default_system_message = r#"You are an expert MCP (Model Context Protocol) configuration extractor. Your task is to analyze text content and extract valid MCP server configuration information.

### CRITICAL OUTPUT REQUIREMENTS ###
1. ONLY return valid JSON - no explanations, no markdown, no code blocks
2. Use EXACTLY this structure: {"mcpServers": {...}}
3. If no MCP configuration is found, return: {"mcpServers": {}}

### MCP Server Configuration Format ###
Each server must have:
- "command": string (executable command)
- "args": array of strings (command arguments)
- Optional: "env", "cwd", "transport"

### Example Valid Output ###
{"mcpServers": {"example-server": {"command": "node", "args": ["server.js"]}}}

### Analysis Rules ###
- Look for server commands, tool descriptions, API endpoints
- Extract meaningful server names and configurations
- Ignore non-MCP related content
- Ensure all JSON is properly formatted and parseable

Remember: ONLY return the JSON object, nothing else."#;

        // Default user message template (fallback if file not found)
        let default_user_template = "### SOURCE TEXT ###\n{{INPUT_TEXT}}\n### END SOURCE ###\n\nExtract MCP server configuration from the above text. Return only the JSON object.";

        // Load system prompt from file or use default
        let system_message = load_prompt_template(&self.config.system_prompt_path(), default_system_message);

        // Load user template from file or use default
        let user_template = load_prompt_template(&self.config.user_template_path(), default_user_template);

        // Replace variables in user template
        let user_message = replace_template_variables(&user_template, input_text.trim());

        // Use Qwen2.5 chat template
        format!(
            "<|im_start|>system\n{}\n<|im_end|>\n<|im_start|>user\n{}\n<|im_end|>\n<|im_start|>assistant\n",
            system_message, user_message
        )
    }

    /// Parse and validate response
    fn parse_response(
        &self,
        response: &str,
    ) -> Result<Value> {
        let trimmed = response.trim();

        if self.config.debug {
            println!("🔍 Raw response: {}", trimmed);
        }

        // Handle empty or "no config" responses
        if trimmed.is_empty() || trimmed == "No MCP config" {
            return Ok(serde_json::json!({"mcpServers": {}}));
        }

        // Try to parse JSON
        let parsed: Value =
            serde_json::from_str(trimmed).with_context(|| format!("Failed to parse JSON: {}", trimmed))?;

        // Validate structure
        self.validate_mcp_config(&parsed)?;

        if self.config.debug {
            println!("✅ Successfully extracted MCP configuration");
        }

        Ok(parsed)
    }

    /// Validate MCP configuration structure
    fn validate_mcp_config(
        &self,
        config: &Value,
    ) -> Result<()> {
        // Check root structure
        let servers = config
            .get("mcpServers")
            .ok_or_else(|| anyhow::anyhow!("Missing 'mcpServers' field"))?;

        if !servers.is_object() {
            anyhow::bail!("'mcpServers' must be an object");
        }

        let servers_obj = servers.as_object().unwrap();

        // Validate each server
        for (name, server_config) in servers_obj {
            if !server_config.is_object() {
                anyhow::bail!("Server '{}' configuration must be an object", name);
            }

            let config_obj = server_config.as_object().unwrap();

            // Check required command field
            if !config_obj.contains_key("command") {
                anyhow::bail!("Server '{}' missing required 'command' field", name);
            }

            // Validate args if present
            if let Some(args) = config_obj.get("args") {
                if !args.is_array() {
                    anyhow::bail!("Server '{}' 'args' field must be an array", name);
                }
            }
        }

        Ok(())
    }
}

// ============================================================================
// Public API
// ============================================================================

/// Extract MCP configuration from text using AI
pub async fn extract_mcp_config(
    text: &str,
    config: Option<AiConfig>,
) -> Result<Value> {
    let config = config.unwrap_or_default();
    let mut extractor = TextMcpExtractor::new(config)?;
    extractor.extract(text).await
}
