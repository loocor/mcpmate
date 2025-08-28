//! Unified configuration checking module
//!
//! Provides standardized configuration file existence and content checking functionality, eliminating repetitive configuration checking logic

use std::path::Path;

/// Configuration checker
///
/// Provides unified configuration file existence and content checking functionality
pub struct ConfigChecker {
    /// List of patterns to check
    patterns: Vec<String>,
}

impl ConfigChecker {
    /// Create a new configuration checker, using the default MCP configuration pattern
    pub fn new() -> Self {
        Self {
            patterns: vec!["mcpServers".to_string(), "mcp_servers".to_string()],
        }
    }

    /// Create a configuration checker, using custom patterns
    pub fn with_patterns(patterns: Vec<String>) -> Self {
        Self { patterns }
    }

    /// Add a check pattern
    pub fn add_pattern(
        &mut self,
        pattern: String,
    ) {
        self.patterns.push(pattern);
    }

    /// Check if MCP configuration exists
    ///
    /// Check if the configuration file exists at the specified path and contains MCP-related configuration
    pub async fn check_mcp_config_exists(
        &self,
        config_path: &Path,
    ) -> bool {
        // Check if the file exists
        if !config_path.exists() {
            return false;
        }

        // Read the file content and check the patterns
        match std::fs::read_to_string(config_path) {
            Ok(content) => self.check_content_patterns(&content),
            Err(_) => false,
        }
    }

    /// Check if the configuration content contains the specified pattern
    pub fn check_content_patterns(
        &self,
        content: &str,
    ) -> bool {
        self.patterns.iter().any(|pattern| content.contains(pattern))
    }

    /// Check the configuration file and return detailed information
    ///
    /// Return the detailed check result of the configuration file
    pub async fn check_config_detailed(
        &self,
        config_path: &Path,
    ) -> ConfigCheckResult {
        if !config_path.exists() {
            return ConfigCheckResult {
                exists: false,
                readable: false,
                has_mcp_config: false,
                matched_patterns: vec![],
                error: None,
            };
        }

        match std::fs::read_to_string(config_path) {
            Ok(content) => {
                let matched_patterns: Vec<String> = self
                    .patterns
                    .iter()
                    .filter(|pattern| content.contains(*pattern))
                    .cloned()
                    .collect();

                ConfigCheckResult {
                    exists: true,
                    readable: true,
                    has_mcp_config: !matched_patterns.is_empty(),
                    matched_patterns,
                    error: None,
                }
            }
            Err(e) => ConfigCheckResult {
                exists: true,
                readable: false,
                has_mcp_config: false,
                matched_patterns: vec![],
                error: Some(e.to_string()),
            },
        }
    }

    /// Get the current check pattern
    pub fn get_patterns(&self) -> &[String] {
        &self.patterns
    }
}

impl Default for ConfigChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration check result
#[derive(Debug, Clone)]
pub struct ConfigCheckResult {
    /// File exists
    pub exists: bool,
    /// File is readable
    pub readable: bool,
    /// Whether the configuration file contains MCP configuration
    pub has_mcp_config: bool,
    /// List of matched patterns
    pub matched_patterns: Vec<String>,
    /// Error message (if any)
    pub error: Option<String>,
}

impl ConfigCheckResult {
    /// Check if the configuration is valid (file exists, readable, and contains MCP configuration)
    pub fn is_valid(&self) -> bool {
        self.exists && self.readable && self.has_mcp_config
    }

    /// Get the error description
    pub fn get_error_description(&self) -> Option<String> {
        if let Some(ref error) = self.error {
            return Some(format!("File read error: {}", error));
        }

        if !self.exists {
            return Some("Configuration file does not exist".to_string());
        }

        if !self.readable {
            return Some("Configuration file is not readable".to_string());
        }

        if !self.has_mcp_config {
            return Some("Configuration file does not contain MCP configuration".to_string());
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_config_checker_new() {
        let checker = ConfigChecker::new();
        assert_eq!(checker.get_patterns(), &["mcpServers", "mcp_servers"]);
    }

    #[tokio::test]
    async fn test_config_checker_with_patterns() {
        let patterns = vec!["custom_pattern".to_string()];
        let checker = ConfigChecker::with_patterns(patterns.clone());
        assert_eq!(checker.get_patterns(), &patterns);
    }

    #[tokio::test]
    async fn test_check_content_patterns() {
        let checker = ConfigChecker::new();

        assert!(checker.check_content_patterns(r#"{"mcpServers": {}}"#));
        assert!(checker.check_content_patterns(r#"{"mcp_servers": {}}"#));
        assert!(!checker.check_content_patterns(r#"{"other": {}}"#));
    }

    #[tokio::test]
    async fn test_check_mcp_config_exists_file_not_found() {
        let checker = ConfigChecker::new();
        let result = checker.check_mcp_config_exists(Path::new("/nonexistent/file")).await;
        assert!(!result);
    }

    #[tokio::test]
    async fn test_check_mcp_config_exists_with_valid_content() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("config.json");

        fs::write(&file_path, r#"{"mcpServers": {"test": {}}}"#).unwrap();

        let checker = ConfigChecker::new();
        let result = checker.check_mcp_config_exists(&file_path).await;
        assert!(result);
    }

    #[tokio::test]
    async fn test_check_mcp_config_exists_with_invalid_content() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("config.json");

        fs::write(&file_path, r#"{"other": {}}"#).unwrap();

        let checker = ConfigChecker::new();
        let result = checker.check_mcp_config_exists(&file_path).await;
        assert!(!result);
    }

    #[tokio::test]
    async fn test_check_config_detailed() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("config.json");

        fs::write(&file_path, r#"{"mcpServers": {"test": {}}}"#).unwrap();

        let checker = ConfigChecker::new();
        let result = checker.check_config_detailed(&file_path).await;

        assert!(result.exists);
        assert!(result.readable);
        assert!(result.has_mcp_config);
        assert!(result.matched_patterns.contains(&"mcpServers".to_string()));
        assert!(result.is_valid());
        assert!(result.get_error_description().is_none());
    }

    #[tokio::test]
    async fn test_add_pattern() {
        let mut checker = ConfigChecker::new();
        checker.add_pattern("new_pattern".to_string());

        assert!(checker.get_patterns().contains(&"new_pattern".to_string()));
        assert!(checker.check_content_patterns("content with new_pattern"));
    }
}
