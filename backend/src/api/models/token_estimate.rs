// MCP Proxy API models for Token Estimation
// Contains data models for token estimate endpoints

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Estimate breakdown for a single capability type (tools, prompts, resources, templates)
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Token estimate for a single capability type")]
pub struct CapTypeEstimate {
    /// Total number of available capabilities
    #[schemars(description = "Total number of available capabilities")]
    pub available_count: u32,

    /// Number of capabilities visible to the client
    #[schemars(description = "Number of capabilities visible to the client")]
    pub visible_count: u32,

    /// Number of capabilities disabled
    #[schemars(description = "Number of capabilities disabled")]
    pub disabled_count: u32,

    /// Estimated tokens for all available capabilities
    #[schemars(description = "Estimated tokens for all available capabilities")]
    pub available_tokens: u32,

    /// Estimated tokens for visible capabilities
    #[schemars(description = "Estimated tokens for visible capabilities")]
    pub visible_tokens: u32,

    /// Token savings from disabled capabilities
    #[schemars(description = "Token savings from disabled capabilities")]
    pub savings_tokens: u32,
}

/// Token breakdown response showing estimates per capability type
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Token breakdown by capability type")]
pub struct TokenBreakdownResponse {
    /// Tools breakdown
    #[schemars(description = "Tools token breakdown")]
    pub tools: CapTypeEstimate,

    /// Prompts breakdown
    #[schemars(description = "Prompts token breakdown")]
    pub prompts: CapTypeEstimate,

    /// Resources breakdown
    #[schemars(description = "Resources token breakdown")]
    pub resources: CapTypeEstimate,

    /// Resource templates breakdown
    #[schemars(description = "Resource templates token breakdown")]
    pub templates: CapTypeEstimate,
}

/// Query parameters for token estimate endpoint
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[schemars(description = "Query parameters for token estimate")]
pub struct TokenEstimateQuery {
    /// Profile ID to estimate tokens for (required)
    #[schemars(description = "Profile ID to estimate tokens for")]
    pub profile_id: String,

    /// Source of capabilities: "activated" (profile-activated servers), "profiles" (all profile servers),
    /// "custom" (custom from request body). Defaults to "activated" if not specified.
    #[schemars(description = "Capability source: activated, profiles, or custom")]
    #[serde(default = "default_activated")]
    pub capability_source: String,
}

fn default_activated() -> String {
    "activated".to_string()
}

/// One profile-bound capability row for client-side token counting (e.g. gpt-tokenizer).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Profile capability payload for client tokenizer")]
pub struct CapabilityTokenLedgerRow {
    #[schemars(description = "Profile component id (matches tools/list, prompts/list, etc.)")]
    pub profile_row_id: String,
    #[schemars(description = "tool | prompt | resource | template")]
    pub kind: String,
    pub server_id: String,
    #[schemars(description = "Whether the server is enabled in this profile")]
    pub server_enabled_in_profile: bool,
    /// JSON serialization compatible with `CapabilityItem` shapes (tool/prompt/resource/template).
    #[schemars(description = "JSON text to pass to cl100k tokenizer on the client")]
    pub payload_json: String,
}

/// Response for capability token ledger (client-side trimming math).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Per-capability JSON blobs for client token estimation")]
pub struct CapabilityTokenLedgerResponse {
    pub items: Vec<CapabilityTokenLedgerRow>,
    #[schemars(description = "Hint for dashboard: use cl100k_base on payload_json UTF-8 bytes")]
    pub tokenizer_note: String,
}

/// Full token estimate response
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Complete token estimate response")]
pub struct TokenEstimateResponse {
    /// Total estimated tokens for all available capabilities
    #[schemars(description = "Total estimated tokens for all available capabilities")]
    pub total_available_tokens: u32,

    /// Estimated tokens for only visible capabilities
    #[schemars(description = "Estimated tokens for visible capabilities")]
    pub visible_tokens: u32,

    /// Token savings from disabled capabilities
    #[schemars(description = "Token savings from disabling capabilities")]
    pub savings_tokens: u32,

    /// Breakdown by capability type
    #[schemars(description = "Breakdown by capability type")]
    pub breakdown: TokenBreakdownResponse,

    /// Method used for estimation (always "chars_div_4" for now)
    #[schemars(description = "Estimation method (always 'chars_div_4')")]
    pub estimation_method: String,

    /// Whether this is an approximation (always true)
    #[schemars(description = "Whether this is an approximation (always true)")]
    pub approximate: bool,
}
