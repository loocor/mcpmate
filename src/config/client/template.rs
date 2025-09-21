// Template engine for configuration generation
// Handles template processing and variable substitution

use anyhow::Result;
use serde_json::{Value, json};
use std::collections::HashMap;

use super::loader::ServerInfo;

/// Template processing engine
pub struct TemplateEngine;

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateEngine {
    /// Create a new template engine
    pub fn new() -> Self {
        Self
    }

    /// Apply template with actual server values
    /// Handles both string templates and nested object templates
    pub async fn apply_template(
        &self,
        template: &Value,
        server: &ServerInfo,
        _transport: &str,
    ) -> Result<Value> {
        // Directly use the template value since it's already a serde_json::Value
        Box::pin(self.apply_template_recursive(template, server)).await
    }

    /// Apply template recursively for nested objects
    async fn apply_template_recursive(
        &self,
        template: &Value,
        server: &ServerInfo,
    ) -> Result<Value> {
        match template {
            Value::String(s) => {
                // Handle string templates with replacements
                let mut result = s.clone();

                // Handle different server types
                match server.server_type.as_str() {
                    "stdio" => {
                        // For stdio servers, use command and args
                        if let Some(command) = &server.command {
                            // Special handling for command with args concatenation (for Augment style)
                            if result.contains("{{command}}{{args}}") {
                                // For Augment, we need to combine command and args into a single string
                                let mut cmd_string = command.clone();
                                for arg in &server.args {
                                    cmd_string.push(' ');
                                    // Add quotes if the arg contains spaces
                                    if arg.contains(' ') {
                                        cmd_string.push_str(&format!("\"{}\"", arg));
                                    } else {
                                        cmd_string.push_str(arg);
                                    }
                                }
                                result = result.replace("{{command}}{{args}}", &cmd_string);
                            } else {
                                // Normal command/args separation for other client
                                let (final_command, final_args) = self.apply_platform_wrapper(command, &server.args)?;

                                result = result
                                    .replace("{{command}}", &final_command)
                                    .replace("{{args}}", &serde_json::to_string(&final_args)?);
                            }
                        } else {
                            result = result
                                .replace("{{command}}", "")
                                .replace("{{args}}", "[]")
                                .replace("{{command}}{{args}}", "");
                        }
                    }
                    "sse" => {
                        // For SSE servers, use URL
                        let url = server.url.as_deref().unwrap_or("");
                        result = result.replace("{{url}}", url);
                    }
                    _ => {
                        // Default handling
                        result = result
                            .replace("{{command}}", server.command.as_deref().unwrap_or(""))
                            .replace("{{args}}", "[]");
                    }
                }

                // Apply common replacements
                result = result
                    .replace("{{env}}", &serde_json::to_string(&server.env)?)
                    .replace("{{runtime}}", &server.runtime)
                    .replace("{{headers}}", "{}");

                // Try to parse result as JSON, fallback to string
                serde_json::from_str(&result).or_else(|_| Ok(json!(result)))
            }
            Value::Object(obj) => {
                // Recursively apply template for each key-value pair
                let mut result = serde_json::Map::new();
                for (key, value) in obj {
                    result.insert(
                        key.clone(),
                        Box::pin(self.apply_template_recursive(value, server)).await?,
                    );
                }
                Ok(Value::Object(result))
            }
            Value::Array(arr) => {
                // Recursively apply template for each array element
                let mut result = Vec::new();
                for item in arr {
                    result.push(Box::pin(self.apply_template_recursive(item, server)).await?);
                }
                Ok(Value::Array(result))
            }
            _ => {
                // For other types (null, bool, number), return as-is
                Ok(template.clone())
            }
        }
    }

    /// Apply platform-specific command wrapping (Windows cmd /c, etc.)
    fn apply_platform_wrapper(
        &self,
        command: &str,
        args: &[String],
    ) -> Result<(String, Vec<String>)> {
        match std::env::consts::OS {
            "windows" => {
                // Windows: wrap with cmd /c
                let mut wrapped_args = vec!["/c".to_string(), command.to_string()];
                wrapped_args.extend_from_slice(args);
                Ok(("cmd".to_string(), wrapped_args))
            }
            _ => {
                // Unix-like systems: use command directly
                Ok((command.to_string(), args.to_vec()))
            }
        }
    }

    /// Create a mock server info for template processing with specific values
    pub fn create_mock_server(
        id: &str,
        name: &str,
        command: Option<String>,
        url: Option<String>,
        args: Vec<String>,
        env: HashMap<String, String>,
        server_type: &str,
    ) -> ServerInfo {
        ServerInfo {
            id: id.to_string(),
            name: name.to_string(),
            command,
            url,
            args,
            env,
            runtime: server_type.to_string(),
            server_type: server_type.to_string(),
        }
    }
}
