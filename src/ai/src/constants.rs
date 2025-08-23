//! Constants and default values for MCPMate AI module
//!
//! Centralized configuration constants for easy adjustment and maintenance

use std::path::PathBuf;

/// Model configuration constants
pub mod model {
    use super::*;

    /// Default model file path (LM Studio location)
    pub const DEFAULT_MODEL_PATH: &str =
        "/Users/Loocor/.lmstudio/models/Qwen/Qwen2.5-0.5B-Instruct-GGUF/qwen2.5-0.5b-instruct-q8_0.gguf";

    /// Alternative model path (MCPMate managed location)
    pub const MCPMATE_MODEL_PATH: &str = "/Users/Loocor/.mcpmate/models/qwen2.5-0.5b-instruct-q4.gguf";

    /// Get default model path as PathBuf
    pub fn default_model_path() -> PathBuf {
        PathBuf::from(DEFAULT_MODEL_PATH)
    }
}

/// Tokenizer configuration constants
pub mod tokenizer {
    /// Tokenizer filename
    pub const TOKENIZER_FILENAME: &str = "tokenizer.json";

    /// HuggingFace model repository for tokenizer download
    pub const HF_MODEL_REPO: &str = "Qwen/Qwen2.5-0.5B-Instruct";

    /// HuggingFace tokenizer download URL
    pub const HF_TOKENIZER_URL: &str = "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct/resolve/main/tokenizer.json";
}

/// Inference parameters - Optimized for structured JSON output and reduced hallucination
pub mod inference {
    /// Temperature: Controls randomness (0.0 = deterministic, 1.0 = very random)
    /// For structured output, use very low temperature to ensure consistency
    pub const DEFAULT_TEMPERATURE: f64 = 0.1;

    /// Top-K: Number of highest probability tokens to consider
    /// Very low for structured output to focus on most likely tokens
    pub const DEFAULT_TOP_K: usize = 10;

    /// Top-P: Cumulative probability threshold for nucleus sampling
    /// Lower values filter out low-probability tokens more aggressively
    pub const DEFAULT_TOP_P: f64 = 0.8;

    /// Min-P: Minimum probability threshold for token selection
    /// Higher values filter out noise and improve quality
    pub const DEFAULT_MIN_P: f64 = 0.15;

    /// Repeat penalty: Reduces repetitive output
    pub const DEFAULT_REPEAT_PENALTY: f32 = 1.1;

    /// Random seed for reproducible results
    pub const DEFAULT_SEED: u64 = 42;

    /// Maximum tokens to generate
    pub const DEFAULT_MAX_TOKENS: usize = 500;

    /// Debug mode default
    pub const DEFAULT_DEBUG: bool = false;
}

/// Alternative parameter sets for different use cases
pub mod inference_presets {

    /// Structured output preset: Optimized for JSON generation
    pub mod structured {
        pub const TEMPERATURE: f64 = 0.0; // Completely deterministic like LM Studio
        pub const TOP_K: usize = 1; // Most focused possible
        pub const TOP_P: f64 = 1.0; // Disable top-p when using top-k=1
        pub const MIN_P: f64 = 0.0; // Disable min-p when using top-k=1
    }

    /// LM Studio matching preset: Exact parameter matching
    pub mod lm_studio_exact {
        pub const TEMPERATURE: f64 = 0.0; // LM Studio often uses 0 for structured output
        pub const TOP_K: usize = 1;
        pub const TOP_P: f64 = 1.0;
        pub const MIN_P: f64 = 0.0;
    }

    /// Conservative preset: Minimal hallucination, high consistency
    pub mod conservative {
        pub const TEMPERATURE: f64 = 0.1;
        pub const TOP_K: usize = 10;
        pub const TOP_P: f64 = 0.8;
        pub const MIN_P: f64 = 0.15;
    }

    /// Balanced preset: Good balance of creativity and consistency
    pub mod balanced {
        pub const TEMPERATURE: f64 = 0.3;
        pub const TOP_K: usize = 20;
        pub const TOP_P: f64 = 0.9;
        pub const MIN_P: f64 = 0.1;
    }

    /// Creative preset: More creative but potentially less consistent
    pub mod creative {
        pub const TEMPERATURE: f64 = 0.7;
        pub const TOP_K: usize = 40;
        pub const TOP_P: f64 = 0.95;
        pub const MIN_P: f64 = 0.05;
    }

    /// LM Studio matching preset: Matches LM Studio default settings
    pub mod lm_studio {
        pub const TEMPERATURE: f64 = 0.8;
        pub const TOP_K: usize = 40;
        pub const TOP_P: f64 = 0.95;
        pub const MIN_P: f64 = 0.05;
    }
}

/// Input validation constants
pub mod validation {
    /// Minimum input text length to prevent empty/meaningless input
    pub const MIN_INPUT_LENGTH: usize = 10;

    /// Maximum input text length to prevent excessive processing
    pub const MAX_INPUT_LENGTH: usize = 20000;

    /// Keywords that indicate meaningful MCP-related content
    pub const MEANINGFUL_KEYWORDS: &[&str] = &[
        "mcp",
        "server",
        "tool",
        "resource",
        "api",
        "service",
        "config",
        "configuration",
        "figma",
        "github",
        "database",
        "file",
        "管理",
        "配置",
        "服务",
        "工具",
    ];
}

/// EOS (End of Sequence) token IDs for Qwen2.5 model
pub mod tokens {
    /// Primary EOS token ID
    pub const EOS_TOKEN_1: u32 = 151645;

    /// Alternative EOS token IDs
    pub const EOS_TOKEN_2: u32 = 151643;
    pub const EOS_TOKEN_3: u32 = 151644;

    /// All EOS token IDs as array
    pub const EOS_TOKENS: &[u32] = &[EOS_TOKEN_1, EOS_TOKEN_2, EOS_TOKEN_3];
}

/// Performance and optimization constants
pub mod performance {
    /// Context window size for repeat penalty calculation
    pub const REPEAT_PENALTY_CONTEXT_SIZE: usize = 64;

    /// Default model loading timeout (seconds)
    pub const MODEL_LOAD_TIMEOUT_SECS: u64 = 30;

    /// Default inference timeout (seconds)
    pub const INFERENCE_TIMEOUT_SECS: u64 = 60;

    /// Batch size for prompt processing (matching LM Studio's strategy)
    pub const PROMPT_BATCH_SIZE: usize = 512;

    /// Batch size for generation (how many tokens to generate in parallel)
    pub const GENERATION_BATCH_SIZE: usize = 32;

    /// Number of threads for CPU inference
    pub const DEFAULT_NUM_THREADS: usize = 8;
}
