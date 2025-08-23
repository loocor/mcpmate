//! Prompt manager module
//!
//! Responsible for loading and managing AI inference prompts

use anyhow::Result;
use std::path::PathBuf;

/// Prompt manager
pub struct PromptManager {
    _prompt_dir: PathBuf,
}

impl PromptManager {
    /// Create new prompt manager
    pub fn new(prompt_dir: Option<PathBuf>) -> Self {
        let prompt_dir = prompt_dir.unwrap_or_else(|| PathBuf::from("prompts"));
        Self {
            _prompt_dir: prompt_dir,
        }
    }

    /// Get MCP configuration extraction system prompt
    pub fn get_mcp_extract_system_prompt(&self) -> String {
        r#"You are an expert MCP (Model Context Protocol) configuration extractor. Your task is to analyze text content and extract valid MCP server configuration information.

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

Remember: ONLY return the JSON object, nothing else."#.to_string()
    }

    /// Create full chat prompt
    ///
    /// Uses Qwen2.5 standard chat template format
    pub fn create_chat_prompt(
        &self,
        system_message: &str,
        user_message: &str,
    ) -> String {
        format!(
            "<|im_start|>system\n{}\n<|im_end|>\n<|im_start|>user\n{}\n<|im_end|>\n<|im_start|>assistant\n",
            system_message, user_message
        )
    }

    /// Create prompt for MCP configuration extraction with enhanced format control
    pub fn create_mcp_extract_prompt(
        &self,
        input_text: &str,
    ) -> Result<String> {
        let system_message = self.get_mcp_extract_system_prompt();

        // Create structured user message with clear boundaries
        let user_message = format!(
            "### SOURCE TEXT ###\n{}\n### END SOURCE ###\n\nExtract MCP server configuration from the above text. Return only the JSON object.",
            input_text.trim()
        );

        Ok(self.create_chat_prompt(&system_message, &user_message))
    }
}
