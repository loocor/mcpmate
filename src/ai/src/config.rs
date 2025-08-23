//! Configuration management module
//!
//! Manages AI module configuration parameters, including model paths, inference parameters, etc.

use crate::constants::{inference, model};
use clap::Parser;
use std::path::PathBuf;

/// MCPMate AI module: Text MCP configuration extractor
///
/// Based on Qwen2.5 0.5B model for local inference, converts input text to MCP service configuration JSON
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Input text content (mutually exclusive with --file)
    #[arg(short, long, conflicts_with = "file")]
    pub text: Option<String>,

    /// Read input text from file (mutually exclusive with --text)
    #[arg(short, long, conflicts_with = "text")]
    pub file: Option<PathBuf>,

    /// Read text from standard input
    #[arg(long, conflicts_with_all = ["text", "file"])]
    pub stdin: bool,

    /// Model file path (optional, defaults to predefined path)
    #[arg(short, long)]
    pub model_path: Option<PathBuf>,

    /// Maximum number of tokens to generate
    #[arg(short = 'n', long, default_value_t = inference::DEFAULT_MAX_TOKENS)]
    pub max_tokens: usize,

    /// Temperature parameter, controls randomness of generation
    #[arg(long, default_value_t = inference::DEFAULT_TEMPERATURE)]
    pub temperature: f64,

    /// Top-K sampling parameter
    #[arg(long, default_value_t = inference::DEFAULT_TOP_K)]
    pub top_k: usize,

    /// Top-P sampling parameter
    #[arg(long, default_value_t = inference::DEFAULT_TOP_P)]
    pub top_p: f64,

    /// Min-P sampling parameter
    #[arg(long, default_value_t = inference::DEFAULT_MIN_P)]
    pub min_p: f64,

    /// Repeat penalty coefficient
    #[arg(long, default_value_t = inference::DEFAULT_REPEAT_PENALTY)]
    pub repeat_penalty: f32,

    /// Random seed
    #[arg(long, default_value_t = inference::DEFAULT_SEED)]
    pub seed: u64,

    /// Enable detailed debug output
    #[arg(short, long)]
    pub debug: bool,
}

/// Extractor configuration
#[derive(Debug, Clone)]
pub struct ExtractorConfig {
    pub model_path: PathBuf,
    pub max_tokens: usize,
    pub temperature: f64,
    pub top_k: usize,
    pub top_p: f64,
    pub min_p: f64,
    pub repeat_penalty: f32,
    pub seed: u64,
    pub debug: bool,
}

impl ExtractorConfig {
    /// Create configuration from command line arguments
    pub fn from_args(args: &Args) -> Self {
        let model_path = args.model_path.clone().unwrap_or_else(|| model::default_model_path());

        Self {
            model_path,
            max_tokens: args.max_tokens,
            temperature: args.temperature,
            top_k: args.top_k,
            top_p: args.top_p,
            min_p: args.min_p,
            repeat_penalty: args.repeat_penalty,
            seed: args.seed,
            debug: args.debug,
        }
    }

    /// Get default configuration
    pub fn default() -> Self {
        Self {
            model_path: model::default_model_path(),
            max_tokens: inference::DEFAULT_MAX_TOKENS,
            temperature: inference::DEFAULT_TEMPERATURE,
            top_k: inference::DEFAULT_TOP_K,
            top_p: inference::DEFAULT_TOP_P,
            min_p: inference::DEFAULT_MIN_P,
            repeat_penalty: inference::DEFAULT_REPEAT_PENALTY,
            seed: inference::DEFAULT_SEED,
            debug: inference::DEFAULT_DEBUG,
        }
    }
}
