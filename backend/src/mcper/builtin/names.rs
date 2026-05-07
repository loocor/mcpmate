pub const MCPMATE_UCAN_CATALOG_TOOL: &str = "mcpmate_ucan_catalog";
pub const MCPMATE_UCAN_DETAILS_TOOL: &str = "mcpmate_ucan_details";
pub const MCPMATE_UCAN_CALL_TOOL: &str = "mcpmate_ucan_call";

pub const MCPMATE_PROFILE_GET_TOOL: &str = "mcpmate_profile_get";
pub const MCPMATE_PROFILE_SET_TOOL: &str = "mcpmate_profile_set";
pub const MCPMATE_PROFILE_ADD_TOOL: &str = "mcpmate_profile_add";
pub const MCPMATE_PROFILE_REMOVE_TOOL: &str = "mcpmate_profile_remove";

pub const MCPMATE_CLIENT_CUSTOM_PROFILE_DETAILS_TOOL: &str = "mcpmate_client_custom_profile_details";

// Kept for ProfileService internal registration — not exposed to downstream clients.
pub(crate) const MCPMATE_PROFILE_LIST_TOOL: &str = "mcpmate_profile_list";
pub(crate) const MCPMATE_PROFILE_DETAILS_TOOL: &str = "mcpmate_profile_details";

/// Unified discovery tools shared by both Unify and Hosted modes.
pub const SHARED_DISCOVERY_TOOL_NAMES: [&str; 3] = [
    MCPMATE_UCAN_CATALOG_TOOL,
    MCPMATE_UCAN_DETAILS_TOOL,
    MCPMATE_UCAN_CALL_TOOL,
];

/// Tools exposed in Unify mode (discovery only).
pub const UNIFY_BUILTIN_TOOL_NAMES: [&str; 3] = SHARED_DISCOVERY_TOOL_NAMES;

/// Tools exposed in Hosted mode (discovery + profile management).
pub const HOSTED_BUILTIN_TOOL_NAMES: [&str; 7] = [
    MCPMATE_UCAN_CATALOG_TOOL,
    MCPMATE_UCAN_DETAILS_TOOL,
    MCPMATE_UCAN_CALL_TOOL,
    MCPMATE_PROFILE_GET_TOOL,
    MCPMATE_PROFILE_SET_TOOL,
    MCPMATE_PROFILE_ADD_TOOL,
    MCPMATE_PROFILE_REMOVE_TOOL,
];

/// Legacy alias — prefer HOSTED_BUILTIN_TOOL_NAMES.
pub const PROFILE_MODE_BUILTIN_TOOL_NAMES: [&str; 7] = HOSTED_BUILTIN_TOOL_NAMES;
