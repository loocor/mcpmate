//! Token estimation utilities for MCP capabilities.
//!
//! Provides approximate token counting for MCP tools, prompts, resources,
//! and resource templates using a simple chars/4 heuristic.
//!
//! This module is used to estimate token savings when MCPMate filters
//! capabilities based on profiles and client configurations.

use std::sync::Arc;

use rmcp::model::{Prompt, PromptArgument, Resource, ResourceTemplate, Tool};
use serde::Serialize;

use crate::clients::models::CapabilitySource;

/// Result of builtin overhead calculation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltinOverhead {
    /// Estimated token count for builtin tools.
    pub tokens: u32,
    /// Number of builtin tools.
    pub tool_count: u32,
    /// Capability source mode name.
    pub mode: String,
}

/// Estimate token count from a JSON string using chars/4 approximation.
///
/// This is a simple heuristic that assumes approximately 4 characters per token,
/// which works reasonably well for English text and JSON structures.
///
/// # Arguments
/// * `json_str` - JSON string to estimate tokens for.
///
/// # Returns
/// Estimated token count (chars.len() / 4).
pub fn estimate_capability_tokens(json_str: &str) -> u32 {
    (json_str.chars().count() / 4) as u32
}

fn estimate_json_tokens<T: Serialize>(value: &T) -> u32 {
    serde_json::to_string(value)
        .map(|json| estimate_capability_tokens(&json))
        .unwrap_or(0)
}

/// Estimate token count for a tool definition.
///
/// Serializes the tool's name, description, and input_schema to JSON,
/// then estimates tokens using the chars/4 heuristic.
///
/// # Arguments
/// * `tool` - The tool to estimate.
///
/// # Returns
/// Estimated token count for the tool definition.
pub fn estimate_tool_tokens(tool: &Tool) -> u32 {
    #[derive(Serialize)]
    struct ToolForEstimate<'a> {
        name: &'a str,
        description: &'a Option<std::borrow::Cow<'a, str>>,
        input_schema: &'a Arc<rmcp::model::JsonObject>,
    }

    let estimate = ToolForEstimate {
        name: &tool.name,
        description: &tool.description,
        input_schema: &tool.input_schema,
    };

    estimate_json_tokens(&estimate)
}

/// Estimate token count for a prompt definition.
///
/// Serializes the prompt's name, description, and arguments to JSON,
/// then estimates tokens using the chars/4 heuristic.
///
/// # Arguments
/// * `prompt` - The prompt to estimate.
///
/// # Returns
/// Estimated token count for the prompt definition.
pub fn estimate_prompt_tokens(prompt: &Prompt) -> u32 {
    #[derive(Serialize)]
    struct PromptForEstimate<'a> {
        name: &'a str,
        description: &'a Option<String>,
        arguments: &'a Option<Vec<PromptArgument>>,
    }

    let estimate = PromptForEstimate {
        name: &prompt.name,
        description: &prompt.description,
        arguments: &prompt.arguments,
    };

    estimate_json_tokens(&estimate)
}

/// Estimate token count for a resource definition.
///
/// Serializes the resource's uri, name, description, and mime_type to JSON,
/// then estimates tokens using the chars/4 heuristic.
///
/// # Arguments
/// * `resource` - The resource to estimate.
///
/// # Returns
/// Estimated token count for the resource definition.
pub fn estimate_resource_tokens(resource: &Resource) -> u32 {
    #[derive(Serialize)]
    struct ResourceForEstimate<'a> {
        uri: &'a str,
        name: &'a str,
        description: &'a Option<String>,
        mime_type: &'a Option<String>,
    }

    let estimate = ResourceForEstimate {
        uri: &resource.raw.uri,
        name: &resource.raw.name,
        description: &resource.raw.description,
        mime_type: &resource.raw.mime_type,
    };

    estimate_json_tokens(&estimate)
}

/// Estimate token count for a resource template definition.
///
/// Serializes the template's uri_template, name, description, and mime_type to JSON,
/// then estimates tokens using the chars/4 heuristic.
///
/// # Arguments
/// * `template` - The resource template to estimate.
///
/// # Returns
/// Estimated token count for the resource template definition.
pub fn estimate_resource_template_tokens(template: &ResourceTemplate) -> u32 {
    #[derive(Serialize)]
    struct TemplateForEstimate<'a> {
        uri_template: &'a str,
        name: &'a str,
        description: &'a Option<String>,
        mime_type: &'a Option<String>,
    }

    let estimate = TemplateForEstimate {
        uri_template: &template.raw.uri_template,
        name: &template.raw.name,
        description: &template.raw.description,
        mime_type: &template.raw.mime_type,
    };

    estimate_json_tokens(&estimate)
}

/// Calculate builtin tool overhead for a given capability source mode.
///
/// # Arguments
/// * `capability_source` - The capability source mode.
/// * `has_custom_profile` - Whether a custom profile exists (only relevant for Profiles mode).
///
/// # Returns
/// `BuiltinOverhead` with token count, tool count, and mode name.
pub fn calculate_builtin_overhead(
    capability_source: &CapabilitySource,
    has_custom_profile: bool,
) -> BuiltinOverhead {
    let tools = create_builtin_tools_for_mode(capability_source, has_custom_profile);
    let tokens: u32 = tools.iter().map(estimate_tool_tokens).sum();
    let tool_count = tools.len() as u32;

    BuiltinOverhead {
        tokens,
        tool_count,
        mode: capability_source.as_str().to_string(),
    }
}

/// Create builtin tools for a given capability source mode.
fn create_builtin_tools_for_mode(
    capability_source: &CapabilitySource,
    has_custom_profile: bool,
) -> Vec<Tool> {
    // Profile tools (always included)
    let profile_tools = vec![
        create_profile_list_tool(),
        create_profile_preview_tool(),
        create_profile_enable_tool(),
        create_profile_disable_tool(),
        create_profile_activate_only_tool(),
    ];

    match capability_source {
        CapabilitySource::Activated => profile_tools,
        CapabilitySource::Profiles => {
            let mut tools = profile_tools;
            tools.push(create_scope_get_tool());
            tools.push(create_scope_set_tool());
            tools.push(create_scope_add_tool());
            tools.push(create_scope_remove_tool());
            if has_custom_profile {
                tools.push(create_client_custom_profile_details_tool());
            }
            tools
        }
        CapabilitySource::Custom => {
            let mut tools = profile_tools;
            tools.push(create_scope_get_tool());
            tools.push(create_client_custom_profile_details_tool());
            tools
        }
    }
}

// --- Builtin tool definitions ---

fn create_profile_list_tool() -> Tool {
    Tool::new(
        "mcpmate_profile_list",
        "List profiles with capability counts",
        Arc::new(
            serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            })
            .as_object()
            .unwrap()
            .clone(),
        ),
    )
}

fn create_profile_preview_tool() -> Tool {
    Tool::new(
        "mcpmate_profile_preview",
        "Preview a profile with lightweight capability details for one reusable scene.",
        Arc::new(
            serde_json::json!({
                "type": "object",
                "properties": {
                    "profile_id": {
                        "type": "string",
                        "description": "Profile ID to inspect"
                    }
                },
                "required": ["profile_id"]
            })
            .as_object()
            .unwrap()
            .clone(),
        ),
    )
}

fn create_profile_enable_tool() -> Tool {
    Tool::new(
        "mcpmate_profile_enable",
        "Enable a profile. If the target profile is exclusive, other non-default profiles may be disabled by profile rules.",
        Arc::new(
            serde_json::json!({
                "type": "object",
                "properties": {
                    "profile_id": {
                        "type": "string",
                        "description": "Profile ID to enable"
                    }
                },
                "required": ["profile_id"]
            })
            .as_object()
            .unwrap()
            .clone(),
        ),
    )
}

fn create_profile_disable_tool() -> Tool {
    Tool::new(
        "mcpmate_profile_disable",
        "Disable a profile and remove it from the active working set.",
        Arc::new(
            serde_json::json!({
                "type": "object",
                "properties": {
                    "profile_id": {
                        "type": "string",
                        "description": "Profile ID to disable"
                    }
                },
                "required": ["profile_id"]
            })
            .as_object()
            .unwrap()
            .clone(),
        ),
    )
}

fn create_profile_activate_only_tool() -> Tool {
    Tool::new(
        "mcpmate_profile_activate_only",
        "Switch to a single shared scene by keeping only this profile active among non-default profiles.",
        Arc::new(
            serde_json::json!({
                "type": "object",
                "properties": {
                    "profile_id": {
                        "type": "string",
                        "description": "Profile ID to keep as the only active non-default profile"
                    }
                },
                "required": ["profile_id"]
            })
            .as_object()
            .unwrap()
            .clone(),
        ),
    )
}

fn create_scope_get_tool() -> Tool {
    Tool::new(
        "mcpmate_scope_get",
        "Get the current scope for this client session, including mode, source, selected profiles, and custom profile ID when present.",
        Arc::new(
            serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            })
            .as_object()
            .unwrap()
            .clone(),
        ),
    )
}

fn create_scope_set_tool() -> Tool {
    Tool::new(
        "mcpmate_scope_set",
        "Replace the current scope with an exact list of shared profiles. Use this to switch to a single scene or exact set.",
        Arc::new(
            serde_json::json!({
                "type": "object",
                "properties": {
                    "profile_ids": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Shared profile IDs to keep in the working set"
                    }
                },
                "required": ["profile_ids"]
            })
            .as_object()
            .unwrap()
            .clone(),
        ),
    )
}

fn create_scope_add_tool() -> Tool {
    Tool::new(
        "mcpmate_scope_add",
        "Add shared profiles to the current scope without replacing the existing selection.",
        Arc::new(
            serde_json::json!({
                "type": "object",
                "properties": {
                    "profile_ids": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Shared profile IDs to add to the working set"
                    }
                },
                "required": ["profile_ids"]
            })
            .as_object()
            .unwrap()
            .clone(),
        ),
    )
}

fn create_scope_remove_tool() -> Tool {
    Tool::new(
        "mcpmate_scope_remove",
        "Remove shared profiles from the current scope without deleting the profile definitions themselves.",
        Arc::new(
            serde_json::json!({
                "type": "object",
                "properties": {
                    "profile_ids": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Shared profile IDs to remove from the working set"
                    }
                },
                "required": ["profile_ids"]
            })
            .as_object()
            .unwrap()
            .clone(),
        ),
    )
}

fn create_client_custom_profile_details_tool() -> Tool {
    Tool::new(
        "mcpmate_client_custom_profile_details",
        "Get custom profile details: servers, tools, prompts, resources (custom mode only)",
        Arc::new(
            serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            })
            .as_object()
            .unwrap()
            .clone(),
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_capability_tokens_empty_string() {
        assert_eq!(estimate_capability_tokens(""), 0);
    }

    #[test]
    fn test_estimate_capability_tokens_simple_json() {
        // 15 chars / 4 = 3 tokens (integer division)
        let json = r#"{"name":"test"}"#;
        assert_eq!(estimate_capability_tokens(json), 3);
    }

    #[test]
    fn test_estimate_capability_tokens_longer_json() {
        // 79 chars / 4 = 19 tokens (integer division)
        let json = r#"{"name":"test_tool","description":"A test tool for testing","input_schema":{}}"#;
        assert_eq!(estimate_capability_tokens(json), 19);
    }

    #[test]
    fn test_estimate_tool_tokens_known_tool() {
        let tool = create_profile_list_tool();
        let tokens = estimate_tool_tokens(&tool);

        // The tool should have a reasonable token count
        // Profile list tool: name + description + empty schema
        // Expected: roughly 30-50 tokens based on the JSON structure
        assert!(tokens > 0, "Tool tokens should be positive");
        assert!(tokens < 100, "Tool tokens should be reasonable (< 100)");
    }

    #[test]
    fn test_estimate_tool_tokens_complex_schema() {
        let tool = create_profile_activate_only_tool();
        let tokens = estimate_tool_tokens(&tool);

        // Profile switch tool has a more complex schema with two properties
        assert!(tokens > 0, "Tool tokens should be positive");
        assert!(tokens > 30, "Complex tool should have more tokens than simple tool");
    }

    #[test]
    fn test_estimate_prompt_tokens() {
        let prompt = Prompt::new(
            "test_prompt",
            Some("A test prompt for testing"),
            Some(vec![
                PromptArgument::new("arg1")
                    .with_description("First argument")
                    .with_required(true),
            ]),
        );

        let tokens = estimate_prompt_tokens(&prompt);
        assert!(tokens > 0, "Prompt tokens should be positive");
    }

    #[test]
    fn test_estimate_resource_tokens() {
        let resource = Resource {
            raw: rmcp::model::RawResource {
                uri: "file:///test/resource.txt".to_string(),
                name: "Test Resource".to_string(),
                title: None,
                description: Some("A test resource".to_string()),
                mime_type: Some("text/plain".to_string()),
                size: None,
                icons: None,
                meta: None,
            },
            annotations: None,
        };

        let tokens = estimate_resource_tokens(&resource);
        assert!(tokens > 0, "Resource tokens should be positive");
    }

    #[test]
    fn test_estimate_resource_template_tokens() {
        let template = ResourceTemplate {
            raw: rmcp::model::RawResourceTemplate {
                uri_template: "file:///test/{path}".to_string(),
                name: "Test Template".to_string(),
                title: None,
                description: Some("A test template".to_string()),
                mime_type: Some("text/plain".to_string()),
                icons: None,
            },
            annotations: None,
        };

        let tokens = estimate_resource_template_tokens(&template);
        assert!(tokens > 0, "Resource template tokens should be positive");
    }

    #[test]
    fn test_builtin_overhead_activated_mode() {
        let overhead = calculate_builtin_overhead(&CapabilitySource::Activated, false);

        assert_eq!(overhead.tool_count, 5, "Activated mode should have 5 tools");
        assert_eq!(overhead.mode, "activated");
        assert!(overhead.tokens > 0, "Token count should be positive");
    }

    #[test]
    fn test_builtin_overhead_profiles_mode_without_custom() {
        let overhead = calculate_builtin_overhead(&CapabilitySource::Profiles, false);

        assert_eq!(
            overhead.tool_count, 9,
            "Profiles mode without custom profile should have 9 tools"
        );
        assert_eq!(overhead.mode, "profiles");
        assert!(overhead.tokens > 0, "Token count should be positive");
    }

    #[test]
    fn test_builtin_overhead_profiles_mode_with_custom() {
        let overhead = calculate_builtin_overhead(&CapabilitySource::Profiles, true);

        assert_eq!(
            overhead.tool_count, 10,
            "Profiles mode with custom profile should have 10 tools"
        );
        assert_eq!(overhead.mode, "profiles");
        assert!(overhead.tokens > 0, "Token count should be positive");
    }

    #[test]
    fn test_builtin_overhead_custom_mode() {
        let overhead = calculate_builtin_overhead(&CapabilitySource::Custom, false);

        assert_eq!(overhead.tool_count, 7, "Custom mode should have 7 tools");
        assert_eq!(overhead.mode, "custom");
        assert!(overhead.tokens > 0, "Token count should be positive");
    }

    #[test]
    fn test_builtin_overhead_token_consistency() {
        // Verify that more tools = more tokens
        let activated = calculate_builtin_overhead(&CapabilitySource::Activated, false);
        let profiles_no_custom = calculate_builtin_overhead(&CapabilitySource::Profiles, false);
        let profiles_with_custom = calculate_builtin_overhead(&CapabilitySource::Profiles, true);
        let custom = calculate_builtin_overhead(&CapabilitySource::Custom, false);

        assert!(
            activated.tokens < profiles_no_custom.tokens,
            "Activated should have fewer tokens than Profiles without custom"
        );
        assert!(
            profiles_no_custom.tokens < profiles_with_custom.tokens,
            "Profiles without custom should have fewer tokens than Profiles with custom"
        );
        assert!(
            custom.tokens < profiles_with_custom.tokens,
            "Custom should have fewer tokens than Profiles with custom"
        );
    }

    #[test]
    fn test_tool_token_estimation_range() {
        // Test that all builtin tools fall within expected token ranges
        let tools = vec![
            create_profile_list_tool(),
            create_profile_preview_tool(),
            create_profile_enable_tool(),
            create_profile_disable_tool(),
            create_profile_activate_only_tool(),
            create_scope_get_tool(),
            create_scope_set_tool(),
            create_scope_add_tool(),
            create_scope_remove_tool(),
            create_client_custom_profile_details_tool(),
        ];

        for tool in &tools {
            let tokens = estimate_tool_tokens(tool);
            assert!(
                (20..=100).contains(&tokens),
                "Tool {} has {} tokens, expected 20-100",
                tool.name,
                tokens
            );
        }
    }
}
