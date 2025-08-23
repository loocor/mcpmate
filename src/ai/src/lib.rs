//! MCPMate AI module: Text MCP configuration extractor
//!
//! Based on Qwen2.5 0.5B model for local inference, converts input text to MCP service configuration JSON

pub mod config;
pub mod constants;
pub mod device;
pub mod extractor;
pub mod model;
pub mod prompt;
pub mod tokenizer;
pub mod utils;

pub use config::ExtractorConfig;
pub use extractor::TextMcpExtractor;

/// Re-export core types
pub use anyhow::{Context, Result};
pub use serde_json::Value;

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");
