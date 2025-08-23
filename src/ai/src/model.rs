//! Model manager module
//!
//! Responsible for loading, inference, and managing Qwen2.5 model

use crate::{
    constants::{performance, tokens},
    debug_println,
    utils::PerformanceMonitor,
};
use anyhow::{Context, Result};
use candle_core::{Device, Tensor};
use candle_transformers::generation::{LogitsProcessor, Sampling};
use candle_transformers::models::quantized_qwen2::ModelWeights as Qwen2;
use candle_transformers::utils::apply_repeat_penalty;
use std::path::PathBuf;

/// Model manager
pub struct ModelManager {
    model_path: PathBuf,
    model: Option<Qwen2>,
    device: Device,
}

impl ModelManager {
    /// Create new model manager
    pub fn new(
        model_path: PathBuf,
        device: Device,
    ) -> Self {
        Self {
            model_path,
            model: None,
            device,
        }
    }

    /// Load model
    pub fn load_model(&mut self) -> Result<()> {
        debug_println!("🔄 Loading model: {:?}", self.model_path);

        // Check if model file exists
        if !self.model_path.exists() {
            anyhow::bail!("Model file not found: {:?}", self.model_path);
        }

        let monitor = PerformanceMonitor::start();

        // Open model file
        let mut file = std::fs::File::open(&self.model_path)
            .with_context(|| format!("Failed to open model file: {:?}", self.model_path))?;

        // Read GGUF file content
        let content = candle_core::quantized::gguf_file::Content::read(&mut file)
            .with_context(|| "Failed to read GGUF file content")?;

        // Create quantized model instance
        let model = Qwen2::from_gguf(content, &mut file, &self.device)
            .with_context(|| "Failed to create quantized model from GGUF")?;

        self.model = Some(model);

        debug_println!(
            "✅ Model loaded successfully in {:.2}s",
            monitor.elapsed().as_secs_f32()
        );
        Ok(())
    }

    /// Execute inference
    pub fn generate(
        &mut self,
        input_tokens: &[u32],
        max_tokens: usize,
        temperature: f64,
        top_k: usize,
        top_p: f64,
        _min_p: f64, // Currently not used by candle-rs sampling
        repeat_penalty: f32,
        seed: u64,
    ) -> Result<Vec<u32>> {
        let model = self.model.as_mut().ok_or_else(|| anyhow::anyhow!("Model not loaded"))?;

        debug_println!("🔥 Starting inference with {} input tokens", input_tokens.len());

        let monitor = PerformanceMonitor::start();

        // Set sampling strategy with Top-K, Top-P, and Min-P
        let sampling = if temperature <= 0.0 {
            Sampling::ArgMax
        } else {
            Sampling::TopKThenTopP {
                k: top_k,
                p: top_p,
                temperature,
            }
        };
        let mut logits_processor = LogitsProcessor::from_sampling(seed, sampling);

        // Process full prompt in one go
        let input = Tensor::new(input_tokens, &self.device)?.unsqueeze(0)?;
        debug_println!("📐 Input tensor shape: {:?}", input.dims());

        let logits = model.forward(&input, 0)?;
        let logits = logits.squeeze(0)?;
        let mut next_token = logits_processor.sample(&logits)?;

        let mut generated_tokens = vec![next_token];
        let mut all_tokens = input_tokens.to_vec();
        all_tokens.push(next_token);

        // Optimized generation loop - minimize tensor creation overhead
        for index in 0..max_tokens.saturating_sub(1) {
            // Create tensor for single token (unavoidable for autoregressive generation)
            let input = Tensor::new(&[next_token], &self.device)?.unsqueeze(0)?;
            let logits = model.forward(&input, input_tokens.len() + index)?;
            let logits = logits.squeeze(0)?;

            // Apply repeat penalty
            let logits = if repeat_penalty != 1.0 {
                let start_at = all_tokens
                    .len()
                    .saturating_sub(performance::REPEAT_PENALTY_CONTEXT_SIZE);
                apply_repeat_penalty(&logits, repeat_penalty, &all_tokens[start_at..])?
            } else {
                logits
            };

            next_token = logits_processor.sample(&logits)?;

            // Check end condition (EOS tokens for Qwen2.5) - optimized comparison
            if next_token == tokens::EOS_TOKEN_1
                || next_token == tokens::EOS_TOKEN_2
                || next_token == tokens::EOS_TOKEN_3
            {
                debug_println!("🔍 EOS token detected, stopping generation");
                break;
            }

            generated_tokens.push(next_token);
            all_tokens.push(next_token);
        }

        let duration = monitor.elapsed();
        let tokens_per_sec = monitor.tokens_per_second(generated_tokens.len());

        println!(
            "✅ Generation completed! {} tokens in {:.2}s ({:.1} tokens/s)",
            generated_tokens.len(),
            duration.as_secs_f32(),
            tokens_per_sec
        );

        Ok(generated_tokens)
    }

    /// Check if model is loaded
    pub fn is_loaded(&self) -> bool {
        self.model.is_some()
    }

    /// Get model information
    pub fn model_info(&self) -> ModelInfo {
        ModelInfo {
            path: self.model_path.clone(),
            loaded: self.is_loaded(),
            device: format!("{:?}", self.device),
        }
    }
}

/// Model information
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub path: PathBuf,
    pub loaded: bool,
    pub device: String,
}

impl ModelInfo {
    /// Get model file size (MB)
    pub fn file_size_mb(&self) -> Result<f64> {
        let metadata = std::fs::metadata(&self.path)?;
        Ok(metadata.len() as f64 / (1024.0 * 1024.0))
    }

    /// Check if model file exists
    pub fn file_exists(&self) -> bool {
        self.path.exists()
    }
}
