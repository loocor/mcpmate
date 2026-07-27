//! Authoritative metadata for MCPMate built-in MCP server and capabilities.

use rmcp::model::{Implementation, InitializeResult, Prompt, ProtocolVersion, ServerCapabilities, Tool};

use crate::common::constants::branding;

use super::names::{
    MCPMATE_CLIENT_CUSTOM_PROFILE_DETAILS_TOOL, MCPMATE_PROFILE_ADD_TOOL, MCPMATE_PROFILE_DETAILS_TOOL,
    MCPMATE_PROFILE_GET_TOOL, MCPMATE_PROFILE_LIST_TOOL, MCPMATE_PROFILE_REMOVE_TOOL, MCPMATE_PROFILE_SET_TOOL,
    MCPMATE_UCAN_CALL_TOOL, MCPMATE_UCAN_CATALOG_TOOL, MCPMATE_UCAN_DETAILS_TOOL,
};

/// Stable catalog source server identity for the synthetic `mcpmate-builtin` observation.
pub const BUILTIN_CATALOG_SERVER_NAME: &str = "mcpmate-builtin";

const MCPMATE_UNIFY_MODE_GUIDE_PROMPT: &str = "mcpmate_unify_mode_guide";
const MCPMATE_UNIFY_MODE_NEXT_ACTIONS_PROMPT: &str = "mcpmate_unify_mode_next_actions";

/// Human-readable title for a built-in tool by machine name.
pub fn builtin_tool_title(name: &str) -> Option<&'static str> {
    match name {
        MCPMATE_UCAN_CATALOG_TOOL => Some("Browse MCPMate Capabilities"),
        MCPMATE_UCAN_DETAILS_TOOL => Some("Inspect MCPMate Capability"),
        MCPMATE_UCAN_CALL_TOOL => Some("Call MCPMate Capability"),
        MCPMATE_PROFILE_GET_TOOL => Some("Get Active Profiles"),
        MCPMATE_PROFILE_SET_TOOL => Some("Set Active Profiles"),
        MCPMATE_PROFILE_ADD_TOOL => Some("Add Active Profiles"),
        MCPMATE_PROFILE_REMOVE_TOOL => Some("Remove Active Profiles"),
        MCPMATE_CLIENT_CUSTOM_PROFILE_DETAILS_TOOL => Some("Get Custom Profile Details"),
        MCPMATE_PROFILE_LIST_TOOL => Some("List Profiles"),
        MCPMATE_PROFILE_DETAILS_TOOL => Some("Inspect Profile"),
        _ => None,
    }
}

/// Human-readable title for a built-in prompt by machine name.
pub fn builtin_prompt_title(name: &str) -> Option<&'static str> {
    match name {
        MCPMATE_UNIFY_MODE_GUIDE_PROMPT => Some("MCPMate Unify Mode Guide"),
        MCPMATE_UNIFY_MODE_NEXT_ACTIONS_PROMPT => Some("MCPMate Unify Mode Next Steps"),
        _ => None,
    }
}

/// Attach the canonical built-in title when one is defined for the tool name.
pub fn with_builtin_tool_title(mut tool: Tool) -> Tool {
    if let Some(title) = builtin_tool_title(tool.name.as_ref()) {
        tool.title = Some(title.to_string());
    }
    tool
}

/// Attach the canonical built-in title when one is defined for the prompt name.
pub fn with_builtin_prompt_title(mut prompt: Prompt) -> Prompt {
    if let Some(title) = builtin_prompt_title(&prompt.name) {
        prompt.title = Some(title.to_string());
    }
    prompt
}

/// `serverInfo` for the synthetic built-in catalog source (`mcpmate-builtin`).
pub fn create_catalog_implementation() -> Implementation {
    Implementation::new(BUILTIN_CATALOG_SERVER_NAME, env!("CARGO_PKG_VERSION"))
        .with_title(branding::DISPLAY_TITLE)
        .with_icons(vec![branding::create_logo_icon()])
        .with_website_url(branding::WEBSITE_URL)
}

/// Initialize payload observed for the synthetic built-in catalog source.
pub fn create_catalog_initialize_result() -> InitializeResult {
    let capabilities = ServerCapabilities::builder()
        .enable_tools()
        .enable_tool_list_changed()
        .enable_prompts()
        .enable_prompts_list_changed()
        .build();

    InitializeResult::new(capabilities)
        .with_protocol_version(ProtocolVersion::V_2025_11_25)
        .with_server_info(create_catalog_implementation())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::constants::branding;
    use crate::mcper::builtin::names::{HOSTED_BUILTIN_TOOL_NAMES, UNIFY_BUILTIN_TOOL_NAMES};

    #[test]
    fn catalog_metadata_matches_branding_contract() {
        let server_info = create_catalog_implementation();

        assert_eq!(server_info.name, BUILTIN_CATALOG_SERVER_NAME);
        assert_eq!(server_info.title.as_deref(), Some(branding::DISPLAY_TITLE));
        assert_eq!(server_info.version, env!("CARGO_PKG_VERSION"));
        let icons = server_info.icons.expect("catalog server info should include icons");
        assert_eq!(icons.len(), 1);
        assert_eq!(icons[0].src, branding::LOGO_URL);
        assert_eq!(icons[0].mime_type.as_deref(), Some(branding::LOGO_MIME_TYPE));
        assert_eq!(server_info.website_url.as_deref(), Some(branding::WEBSITE_URL));

        let initialize = create_catalog_initialize_result();
        assert_eq!(initialize.protocol_version, ProtocolVersion::V_2025_11_25);
        assert_eq!(initialize.server_info.name, BUILTIN_CATALOG_SERVER_NAME);
    }

    #[test]
    fn builtin_titles_cover_registered_names() {
        for name in UNIFY_BUILTIN_TOOL_NAMES
            .iter()
            .chain(HOSTED_BUILTIN_TOOL_NAMES.iter())
            .copied()
        {
            assert!(
                builtin_tool_title(name).is_some(),
                "missing title for builtin tool {name}"
            );
        }
        for name in [MCPMATE_PROFILE_LIST_TOOL, MCPMATE_PROFILE_DETAILS_TOOL] {
            assert!(
                builtin_tool_title(name).is_some(),
                "missing title for internal builtin tool {name}"
            );
        }
        for name in [MCPMATE_UNIFY_MODE_GUIDE_PROMPT, MCPMATE_UNIFY_MODE_NEXT_ACTIONS_PROMPT] {
            assert!(
                builtin_prompt_title(name).is_some(),
                "missing title for builtin prompt {name}"
            );
        }
    }

    #[test]
    fn with_builtin_title_preserves_machine_name() {
        let tool = with_builtin_tool_title(Tool::new(
            MCPMATE_UCAN_CATALOG_TOOL,
            "catalog description",
            std::sync::Arc::new(serde_json::Map::new()),
        ));

        assert_eq!(tool.name.as_ref(), MCPMATE_UCAN_CATALOG_TOOL);
        assert_eq!(tool.title.as_deref(), Some("Browse MCPMate Capabilities"));
        assert_eq!(tool.description.as_deref(), Some("catalog description"));
    }
}
