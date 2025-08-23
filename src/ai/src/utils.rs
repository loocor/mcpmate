//! Utility functions module
//!
//! Provides debug output, text processing, etc.

use anyhow::{Context, Result};
use std::path::PathBuf;

/// Global debug flag
pub static mut DEBUG_MODE: bool = false;

/// Set debug mode
pub fn set_debug_mode(enabled: bool) {
    unsafe {
        DEBUG_MODE = enabled;
    }
}

/// Debug output macro
#[macro_export]
macro_rules! debug_println {
    ($($arg:tt)*) => {
        unsafe {
            if crate::utils::DEBUG_MODE {
                println!($($arg)*);
            }
        }
    };
}

/// Re-export macro for internal use
pub use debug_println;

/// Text preprocessing tool
pub struct TextProcessor;

impl TextProcessor {
    /// Preprocess input text for consistent tokenization
    ///
    /// 1. Clean invisible characters and normalize whitespace
    /// 2. Detect and extract JSON code block
    /// 3. Limit text length
    pub fn preprocess(input_text: &str) -> String {
        // First, clean and normalize the text for consistent tokenization
        let cleaned_text = Self::normalize_text_for_tokenization(input_text);

        // Check text length, if too long, try to extract JSON portion
        if cleaned_text.len() > 20000 {
            println!("⚠️ Input text very long, attempting to extract JSON portion");

            // Try to find JSON code block
            if let Some(json_part) = Self::extract_json_block(&cleaned_text) {
                return Self::normalize_text_for_tokenization(&json_part);
            }

            // If no JSON found, take first 20000 characters
            cleaned_text.chars().take(20000).collect()
        } else {
            cleaned_text
        }
    }

    /// Normalize text for consistent tokenization across different sources
    fn normalize_text_for_tokenization(text: &str) -> String {
        // Step 1: Clean invisible and problematic characters
        let cleaned = Self::clean_invisible_chars(text);

        // Step 2: Normalize whitespace
        Self::normalize_whitespace(&cleaned)
    }

    /// Clean invisible and problematic characters from web content
    fn clean_invisible_chars(text: &str) -> String {
        text
            // Replace various line endings with standard \n
            .replace('\r', "\n")
            .replace("\r\n", "\n")
            // Replace various space characters with standard space
            .replace('\u{00A0}', " ") // Non-breaking space
            .replace('\u{2000}', " ") // En quad
            .replace('\u{2001}', " ") // Em quad
            .replace('\u{2002}', " ") // En space
            .replace('\u{2003}', " ") // Em space
            .replace('\u{2004}', " ") // Three-per-em space
            .replace('\u{2005}', " ") // Four-per-em space
            .replace('\u{2006}', " ") // Six-per-em space
            .replace('\u{2007}', " ") // Figure space
            .replace('\u{2008}', " ") // Punctuation space
            .replace('\u{2009}', " ") // Thin space
            .replace('\u{200A}', " ") // Hair space
            .replace('\u{200B}', "") // Zero-width space
            .replace('\u{200C}', "") // Zero-width non-joiner
            .replace('\u{200D}', "") // Zero-width joiner
            .replace('\u{FEFF}', "") // Zero-width no-break space (BOM)
            // Replace tabs with spaces
            .replace('\t', " ")
            // Clean HTML entities that might leak through
            .replace("&nbsp;", " ")
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
    }

    /// Add retry mechanism for failed JSON parsing
    pub fn extract_with_retry(
        text: &str,
        max_retries: usize,
    ) -> Result<serde_json::Value, String> {
        for attempt in 0..=max_retries {
            let _processed_text = if attempt == 0 {
                Self::preprocess(text)
            } else {
                // Add stronger instruction for retry
                format!(
                    "IMPORTANT: Return ONLY valid JSON, no explanations.\n\n{}",
                    Self::preprocess(text)
                )
            };

            // Here you would call your model inference
            // For now, just return a placeholder
            // This should be integrated with your actual inference pipeline
            return Err("Integration with inference pipeline needed".to_string());
        }
        Err("Max retries exceeded".to_string())
    }

    /// Normalize whitespace for consistent tokenization
    fn normalize_whitespace(text: &str) -> String {
        // Remove trailing spaces from each line and normalize multiple spaces
        let lines: Vec<String> = text
            .lines()
            .map(|line| {
                // Replace multiple spaces with single space
                let mut normalized = String::new();
                let mut prev_was_space = false;

                for ch in line.chars() {
                    if ch == ' ' {
                        if !prev_was_space {
                            normalized.push(' ');
                            prev_was_space = true;
                        }
                    } else {
                        normalized.push(ch);
                        prev_was_space = false;
                    }
                }

                normalized.trim_end().to_string()
            })
            .collect();

        // Join lines and normalize multiple newlines
        let joined = lines.join("\n");

        // Replace 3+ consecutive newlines with exactly 2 newlines
        let mut result = String::new();
        let mut newline_count = 0;

        for ch in joined.chars() {
            if ch == '\n' {
                newline_count += 1;
                if newline_count <= 2 {
                    result.push(ch);
                }
            } else {
                newline_count = 0;
                result.push(ch);
            }
        }

        result.trim().to_string()
    }

    /// Extract JSON code block
    fn extract_json_block(text: &str) -> Option<String> {
        // Find JSON block containing "mcpServers"
        if let Some(start) = text.find('{') {
            if let Some(end) = Self::find_matching_brace(text, start) {
                let json_candidate = &text[start..=end];
                if json_candidate.contains("mcpServers") {
                    return Some(json_candidate.to_string());
                }
            }
        }
        None
    }

    /// Find matching right brace
    fn find_matching_brace(
        text: &str,
        start: usize,
    ) -> Option<usize> {
        let chars: Vec<char> = text.chars().collect();
        let mut brace_count = 0;

        for (i, &ch) in chars.iter().enumerate().skip(start) {
            match ch {
                '{' => brace_count += 1,
                '}' => {
                    brace_count -= 1;
                    if brace_count == 0 {
                        return Some(i);
                    }
                }
                _ => {}
            }
        }
        None
    }
}

/// Input reader
pub struct InputReader;

impl InputReader {
    /// Get input text from command line arguments
    pub fn get_input_text(
        text: Option<&String>,
        file: Option<&PathBuf>,
        stdin: bool,
    ) -> Result<String> {
        if let Some(text) = text {
            // Use text parameter directly
            Ok(text.clone())
        } else if let Some(file_path) = file {
            // Read from file
            debug_println!("📁 Reading input from file: {:?}", file_path);
            std::fs::read_to_string(file_path).with_context(|| format!("Failed to read file: {:?}", file_path))
        } else if stdin {
            // Read from standard input
            println!("📝 Reading input from stdin (press Ctrl+D when finished):");
            Self::read_from_stdin()
        } else {
            // Interactive input
            println!("📝 Please enter your text (press Ctrl+D when finished):");
            Self::read_from_stdin()
        }
    }

    /// Read from standard input
    fn read_from_stdin() -> Result<String> {
        use std::io::Read;
        let mut buffer = String::new();
        std::io::stdin()
            .read_to_string(&mut buffer)
            .with_context(|| "Failed to read from stdin")?;
        Ok(buffer.trim().to_string())
    }
}

/// Performance monitor
pub struct PerformanceMonitor {
    start_time: std::time::Instant,
}

impl PerformanceMonitor {
    /// Start monitoring
    pub fn start() -> Self {
        Self {
            start_time: std::time::Instant::now(),
        }
    }

    /// Get elapsed time
    pub fn elapsed(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }

    /// Calculate tokens per second
    pub fn tokens_per_second(
        &self,
        token_count: usize,
    ) -> f32 {
        let duration = self.elapsed().as_secs_f32();
        if duration > 0.0 {
            token_count as f32 / duration
        } else {
            0.0
        }
    }
}
