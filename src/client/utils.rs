use crate::client::CallToolInput;
use rmcp::model::CallToolRequestParam;
use rmcp::model::Tool;
use serde_json;
use std::env;
use tokio::process::Command;

/// Format the schema parameters into a human-readable string
pub fn schema_formater(schema: &serde_json::Map<String, serde_json::Value>) -> String {
    // Convert to Value for easier processing
    let schema_value: serde_json::Value =
        serde_json::to_value(schema).unwrap_or_else(|_| serde_json::json!({}));

    // Extract and format parameter information
    if let Some(properties) = schema_value.get("properties").and_then(|p| p.as_object()) {
        let mut param_info = Vec::new();

        for (param_name, param_details) in properties {
            let param_type = param_details
                .get("type")
                .and_then(|t| t.as_str())
                .unwrap_or("unknown");
            let param_desc = param_details
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("");
            let required = schema_value
                .get("required")
                .and_then(|r| r.as_array())
                .map(|r| r.iter().any(|v| v.as_str() == Some(param_name)))
                .unwrap_or(false);

            param_info.push(format!(
                "       - {}{}: {} ({})",
                param_name,
                if required { " [required]" } else { "" },
                param_type,
                param_desc
            ));

            // Handle nested properties
            if let Some(sub_properties) =
                param_details.get("properties").and_then(|p| p.as_object())
            {
                for (sub_name, sub_details) in sub_properties {
                    let sub_type = sub_details
                        .get("type")
                        .and_then(|t| t.as_str())
                        .unwrap_or("unknown");
                    let sub_desc = sub_details
                        .get("description")
                        .and_then(|d| d.as_str())
                        .unwrap_or("");
                    param_info.push(format!(
                        "         • {}: {} ({})",
                        sub_name, sub_type, sub_desc
                    ));
                }
            }
        }

        param_info.join("\n")
    } else {
        "       No parameters required".to_string()
    }
}

/// prepare command env for different commands
pub fn prepare_command_env(command: &mut Command, command_str: &str) {
    // 1. bin path
    let bin_var = match command_str {
        "npx" => "NPX_BIN_PATH",
        "uvx" => "UVX_BIN_PATH",
        _ => "MCP_RUNTIME_BIN",
    };
    let bin_path = env::var(bin_var)
        .or_else(|_| env::var("MCP_RUNTIME_BIN"))
        .ok();
    if let Some(bin_path) = bin_path {
        let old_path = env::var("PATH").unwrap_or_default();
        let new_path = format!("{}:{}", bin_path, old_path);
        command.env("PATH", new_path);
    }

    // 2. cache env
    let cache_var = match command_str {
        "npx" => "NPM_CONFIG_CACHE",
        "uvx" => "UV_CACHE_DIR",
        _ => "",
    };
    if !cache_var.is_empty() {
        if let Ok(cache_val) = env::var(cache_var) {
            command.env(cache_var, cache_val);
        }
    }
}

/// Print content from tool call results
pub fn print_content(idx: usize, content: &rmcp::model::Content) {
    match &content.raw {
        rmcp::model::RawContent::Text(txt) => {
            println!("[{}] Text: {}", idx + 1, txt.text);
        }
        rmcp::model::RawContent::Image(img) => {
            println!(
                "[{}] Image: [mime: {}, {} bytes]",
                idx + 1,
                img.mime_type,
                img.data.len()
            );
        }
        rmcp::model::RawContent::Resource(res) => match &res.resource {
            rmcp::model::ResourceContents::TextResourceContents { text, .. } => {
                println!("[{}] Embedded Text Resource: {}", idx + 1, text);
            }
            rmcp::model::ResourceContents::BlobResourceContents { blob, .. } => {
                println!(
                    "[{}] Embedded Blob Resource: [base64, {} bytes]",
                    idx + 1,
                    blob.len()
                );
            }
        },
    }
}

/// find the tool, check the arguments and return the tool and the request parameters
pub fn prepare_tool_call<'a>(
    tools: &'a [Tool],
    input: &CallToolInput<'_>,
) -> Result<(&'a Tool, CallToolRequestParam), String> {
    let tool = tools.iter().find(|t| t.name == input.tool_name);
    if tool.is_none() {
        return Err(format!("Tool '{}' not found on server.", input.tool_name));
    }
    let tool = tool.unwrap();
    let arguments = match &input.arguments {
        Some(serde_json::Value::Object(map)) => Some(map.clone()),
        Some(_) => {
            return Err("Tool arguments must be a JSON object.".to_string());
        }
        None => None,
    };
    let req = CallToolRequestParam {
        name: input.tool_name.clone().into(),
        arguments,
    };
    Ok((tool, req))
}
