//! Tokenizer manager module
//!
//! Responsible for text encoding and decoding, supporting local and remote tokenizers

use crate::{constants::tokenizer as tokenizer_constants, debug_println};
use anyhow::{Context, Result};
use std::path::PathBuf;
use tokenizers::Tokenizer;

/// Tokenizer manager
pub struct TokenizerManager {
    tokenizer: Option<Tokenizer>,
    model_dir: PathBuf,
}

impl TokenizerManager {
    /// Create new tokenizer manager
    pub fn new(model_path: &PathBuf) -> Self {
        let model_dir = model_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .to_path_buf();

        Self {
            tokenizer: None,
            model_dir,
        }
    }

    /// Load tokenizer
    pub fn load_tokenizer(&mut self) -> Result<()> {
        // 1. First try to use tokenizer.json in the same directory
        let local_tokenizer_path = self.model_dir.join(tokenizer_constants::TOKENIZER_FILENAME);

        if local_tokenizer_path.exists() {
            debug_println!("📝 Loading local tokenizer: {:?}", local_tokenizer_path);
            let tokenizer = Tokenizer::from_file(&local_tokenizer_path)
                .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {}", e))?;
            self.tokenizer = Some(tokenizer);
            debug_println!("✅ Tokenizer loaded successfully");
            return Ok(());
        }

        // 2. Try to download tokenizer from HuggingFace
        println!("⚠️ Local tokenizer not found, downloading from HuggingFace...");
        let cached_tokenizer = self.model_dir.join(tokenizer_constants::TOKENIZER_FILENAME);

        match self.download_tokenizer(&cached_tokenizer) {
            Ok(()) => {
                let tokenizer = Tokenizer::from_file(&cached_tokenizer)
                    .map_err(|e| anyhow::anyhow!("Failed to load downloaded tokenizer: {}", e))?;
                self.tokenizer = Some(tokenizer);
                println!("✅ Tokenizer downloaded and loaded successfully");
            }
            Err(e) => {
                println!("❌ Failed to download tokenizer: {}", e);
                println!("💡 Please manually download tokenizer.json from:");
                println!("   {}", tokenizer_constants::HF_TOKENIZER_URL);
                println!("   And place it in: {:?}", self.model_dir);
                anyhow::bail!("Tokenizer not available");
            }
        }

        Ok(())
    }

    /// Encode text to token IDs
    pub fn encode(
        &self,
        text: &str,
    ) -> Result<Vec<u32>> {
        let tokenizer = self
            .tokenizer
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Tokenizer not loaded"))?;

        let encoding = tokenizer
            .encode(text, true)
            .map_err(|e| anyhow::anyhow!("Tokenization failed: {}", e))?;

        Ok(encoding.get_ids().to_vec())
    }

    /// Decode token IDs to text
    pub fn decode(
        &self,
        token_ids: &[u32],
    ) -> Result<String> {
        let tokenizer = self
            .tokenizer
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Tokenizer not loaded"))?;

        let text = tokenizer
            .decode(token_ids, true)
            .map_err(|e| anyhow::anyhow!("Decoding failed: {}", e))?;

        Ok(text)
    }

    /// Check if tokenizer is loaded
    pub fn is_loaded(&self) -> bool {
        self.tokenizer.is_some()
    }

    /// Get vocabulary size
    pub fn vocab_size(&self) -> Option<usize> {
        self.tokenizer.as_ref().map(|t| t.get_vocab_size(true))
    }

    /// Download tokenizer (simplified implementation)
    fn download_tokenizer(
        &self,
        target_path: &PathBuf,
    ) -> Result<()> {
        use hf_hub::api::sync::Api;

        let api = Api::new().map_err(|e| anyhow::anyhow!("Failed to create HF API: {}", e))?;

        let repo = api.model(tokenizer_constants::HF_MODEL_REPO.to_string());

        let tokenizer_path = repo
            .get(tokenizer_constants::TOKENIZER_FILENAME)
            .map_err(|e| anyhow::anyhow!("Failed to download tokenizer: {}", e))?;

        // Copy to target location
        std::fs::copy(&tokenizer_path, target_path)
            .with_context(|| format!("Failed to copy tokenizer to {:?}", target_path))?;

        Ok(())
    }
}

/// Tokenizer information
#[derive(Debug, Clone)]
pub struct TokenizerInfo {
    pub loaded: bool,
    pub vocab_size: Option<usize>,
    pub model_dir: PathBuf,
}

impl TokenizerInfo {
    /// Create information from manager
    pub fn from_manager(manager: &TokenizerManager) -> Self {
        Self {
            loaded: manager.is_loaded(),
            vocab_size: manager.vocab_size(),
            model_dir: manager.model_dir.clone(),
        }
    }

    /// Check if local tokenizer file exists
    pub fn local_file_exists(&self) -> bool {
        self.model_dir.join(tokenizer_constants::TOKENIZER_FILENAME).exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_tokenizer_manager_creation() {
        let model_path = PathBuf::from("/test/model.gguf");
        let manager = TokenizerManager::new(&model_path);

        assert!(!manager.is_loaded());
        assert_eq!(manager.model_dir, PathBuf::from("/test"));
    }

    #[test]
    fn test_tokenizer_info() {
        let model_path = PathBuf::from("/test/model.gguf");
        let manager = TokenizerManager::new(&model_path);
        let info = TokenizerInfo::from_manager(&manager);

        assert!(!info.loaded);
        assert_eq!(info.vocab_size, None);
    }
}
