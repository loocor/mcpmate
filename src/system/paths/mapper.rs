// Path mapping and template resolution

use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;

/// Path mapper for resolving configuration path templates
pub struct PathMapper {
    variables: HashMap<String, String>,
}

impl PathMapper {
    /// Create a new path mapper with system variables
    pub fn new() -> Result<Self> {
        let mut variables = HashMap::new();

        // Add user home directory
        if let Some(home_dir) = dirs::home_dir() {
            variables.insert(
                "user_home".to_string(),
                home_dir.to_string_lossy().to_string(),
            );
        }

        // Add other common paths based on platform
        #[cfg(target_os = "macos")]
        {
            if let Some(home_dir) = dirs::home_dir() {
                variables.insert(
                    "app_support".to_string(),
                    home_dir
                        .join("Library/Application Support")
                        .to_string_lossy()
                        .to_string(),
                );
                variables.insert(
                    "preferences".to_string(),
                    home_dir
                        .join("Library/Preferences")
                        .to_string_lossy()
                        .to_string(),
                );
            }
        }

        #[cfg(target_os = "windows")]
        {
            if let Some(app_data) = dirs::config_dir() {
                variables.insert(
                    "app_data".to_string(),
                    app_data.to_string_lossy().to_string(),
                );
            }
            if let Some(local_app_data) = dirs::data_local_dir() {
                variables.insert(
                    "local_app_data".to_string(),
                    local_app_data.to_string_lossy().to_string(),
                );
            }
        }

        #[cfg(target_os = "linux")]
        {
            if let Some(config_dir) = dirs::config_dir() {
                variables.insert(
                    "config_dir".to_string(),
                    config_dir.to_string_lossy().to_string(),
                );
            }
            if let Some(data_dir) = dirs::data_dir() {
                variables.insert(
                    "data_dir".to_string(),
                    data_dir.to_string_lossy().to_string(),
                );
            }
        }

        Ok(Self { variables })
    }

    /// Resolve a path template by replacing variables
    pub fn resolve_template(
        &self,
        template: &str,
    ) -> Result<PathBuf> {
        let mut resolved = template.to_string();

        // Replace variables in the format {{variable_name}}
        for (key, value) in &self.variables {
            let pattern = format!("{{{{{}}}}}", key);
            resolved = resolved.replace(&pattern, value);
        }

        // Check if there are any unresolved variables
        if resolved.contains("{{") && resolved.contains("}}") {
            return Err(anyhow::anyhow!(
                "Unresolved variables in path template: {}",
                template
            ));
        }

        Ok(PathBuf::from(resolved))
    }

    /// Add or update a variable
    pub fn set_variable(
        &mut self,
        key: String,
        value: String,
    ) {
        self.variables.insert(key, value);
    }

    /// Get all available variables
    pub fn get_variables(&self) -> &HashMap<String, String> {
        &self.variables
    }

    /// Expand tilde (~) to home directory if present
    pub fn expand_tilde(path: &str) -> Result<PathBuf> {
        if path.starts_with('~') {
            if let Some(home_dir) = dirs::home_dir() {
                let expanded = path.replacen('~', &home_dir.to_string_lossy(), 1);
                Ok(PathBuf::from(expanded))
            } else {
                Err(anyhow::anyhow!("Could not determine home directory"))
            }
        } else {
            Ok(PathBuf::from(path))
        }
    }
}

impl Default for PathMapper {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            variables: HashMap::new(),
        })
    }
}
