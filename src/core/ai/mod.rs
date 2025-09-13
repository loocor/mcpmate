//! AI module for MCP configuration extraction
//!
//! This module provides AI-powered text analysis to extract MCP server configurations
//! from natural language descriptions or technical documentation.

#![allow(clippy::module_inception)]

pub mod ai;

// Re-export main types and functions
pub use ai::{AiConfig, TextMcpExtractor, default_model_path, default_prompt_template_dir, extract_mcp_config};
