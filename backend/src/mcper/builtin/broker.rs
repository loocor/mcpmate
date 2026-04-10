use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex as StdMutex, RwLock as StdRwLock};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};
use futures::StreamExt;
use json5;
use once_cell::sync::Lazy;
use rmcp::model::{
    CallToolRequest, CallToolRequestParams, CallToolResult, ClientRequest, Content, Resource, ResourceTemplate, Tool,
};
use rmcp::service::PeerRequestOptions;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::config::database::Database;
use crate::config::registry::RegistryCacheService;
use crate::core::cache::manager::RedbCacheManager;
use crate::core::capability::naming::{NamingKind, resolve_unique_name};
use crate::core::foundation::types::ConnectionStatus;
use crate::core::pool::UpstreamConnectionPool;
use crate::core::profile::visibility::ProfileVisibilityService;
use crate::core::proxy::server::{ClientContext, ClientIdentitySource, ClientTransport};
use crate::system::paths::PathService;

use super::{ClientBuiltinContext, registry::BuiltinService};

/// Structured error response for UCAN tools, designed for LLM parsing and recovery.
#[derive(Debug, Clone, Serialize)]
pub struct UcanError {
    /// Error code for programmatic handling (e.g., "capability_not_found")
    pub error_code: String,
    /// Human-readable error message
    pub message: String,
    /// Actionable guidance for recovery
    pub recovery_hint: String,
    /// Similar capability names (fuzzy match suggestions)
    pub alternatives: Vec<String>,
    /// Whether retry makes sense
    pub retry_eligible: bool,
}

impl UcanError {
    /// Creates a capability_not_found error with fuzzy-matched alternatives.
    pub fn capability_not_found(
        capability_kind: &str,
        capability_name: &str,
        catalog_names: &[String],
    ) -> Self {
        let alternatives = find_similar_names(capability_name, catalog_names, 3);
        Self {
            error_code: "capability_not_found".to_string(),
            message: format!(
                "{} '{}' is not available in the current catalog.",
                capitalize_kind(capability_kind),
                capability_name
            ),
            recovery_hint: if alternatives.is_empty() {
                "Use mcpmate_ucan_catalog to list available capabilities.".to_string()
            } else {
                "Check the 'alternatives' field for similar capability names, or use mcpmate_ucan_catalog to browse all available capabilities.".to_string()
            },
            alternatives,
            retry_eligible: false,
        }
    }

    /// Creates a server_unreachable error for connection failures.
    pub fn server_unreachable(
        server_id: &str,
        server_name: &str,
    ) -> Self {
        Self {
            error_code: "server_unreachable".to_string(),
            message: format!(
                "Server '{}' ({}) is not reachable or not ready.",
                server_name, server_id
            ),
            recovery_hint: "Check if the server process is running. Verify server configuration and network connectivity. The server may need time to start up.".to_string(),
            alternatives: Vec::new(),
            retry_eligible: true,
        }
    }

    /// Creates a visibility_denied error when capability is disabled.
    pub fn visibility_denied(
        capability_kind: &str,
        capability_name: &str,
    ) -> Self {
        Self {
            error_code: "visibility_denied".to_string(),
            message: format!(
                "{} '{}' is not available in the current catalog.",
                capitalize_kind(capability_kind),
                capability_name
            ),
            recovery_hint: "Re-run mcpmate_ucan_catalog to refresh visibility, then choose a capability that appears in the latest catalog.".to_string(),
            alternatives: Vec::new(),
            retry_eligible: false,
        }
    }

    /// Creates an invalid_parameters error.
    pub fn invalid_parameters(
        tool_name: &str,
        details: &str,
    ) -> Self {
        Self {
            error_code: "invalid_parameters".to_string(),
            message: format!("Invalid parameters for {}: {}", tool_name, details),
            recovery_hint: "Check the tool schema for required parameters and their types. Use mcpmate_ucan_details to inspect capability schemas.".to_string(),
            alternatives: Vec::new(),
            retry_eligible: false,
        }
    }

    pub fn missing_required_arguments(
        capability_kind: &str,
        capability_name: &str,
        missing: &[String],
    ) -> Self {
        let missing_joined = missing.join(", ");
        Self {
            error_code: "missing_required_arguments".to_string(),
            message: format!(
                "{} '{}' is missing required arguments: {}",
                capitalize_kind(capability_kind),
                capability_name,
                missing_joined
            ),
            recovery_hint: "Call mcpmate_ucan_details with detail_level=full for this capability, then retry mcpmate_ucan_call with all required arguments.".to_string(),
            alternatives: Vec::new(),
            retry_eligible: true,
        }
    }

    pub fn resource_arguments_not_supported(capability_name: &str) -> Self {
        Self {
            error_code: "resource_arguments_not_supported".to_string(),
            message: format!(
                "Resource '{}' does not accept call arguments.",
                capability_name
            ),
            recovery_hint: "Call mcpmate_ucan_call again with capability_kind=resource and arguments={}. If you started from a template, resolve it via mcpmate_ucan_details first.".to_string(),
            alternatives: Vec::new(),
            retry_eligible: true,
        }
    }

    /// Creates an upstream_error for errors from the upstream MCP server.
    pub fn upstream_error(
        capability_kind: &str,
        capability_name: &str,
        error_details: &str,
    ) -> Self {
        Self {
            error_code: "upstream_error".to_string(),
            message: format!(
                "Upstream server returned an error for {} '{}': {}",
                capability_kind, capability_name, error_details
            ),
            recovery_hint: "The upstream MCP server encountered an error. Check the server logs for details. Verify the capability is correctly implemented and the arguments are valid.".to_string(),
            alternatives: Vec::new(),
            retry_eligible: false,
        }
    }

    /// Creates a timeout error for operation timeouts.
    pub fn timeout(
        capability_kind: &str,
        capability_name: &str,
        timeout_secs: u64,
    ) -> Self {
        Self {
            error_code: "timeout".to_string(),
            message: format!(
                "{} '{}' execution timed out after {} seconds.",
                capitalize_kind(capability_kind),
                capability_name,
                timeout_secs
            ),
            recovery_hint: "The operation took longer than the configured timeout. Consider increasing MCPMATE_TOOL_CALL_TIMEOUT_SECS environment variable, or check if the upstream server is responsive.".to_string(),
            alternatives: Vec::new(),
            retry_eligible: true,
        }
    }

    /// Creates a resource_template_not_invocable error.
    pub fn resource_template_not_invocable(template_name: &str) -> Self {
        Self {
            error_code: "resource_template_not_invocable".to_string(),
            message: format!(
                "Resource template '{}' cannot be invoked directly.",
                template_name
            ),
            recovery_hint: "Use mcpmate_ucan_details to inspect the template and extract URI construction rules. Template-derived URIs are not directly invocable through mcpmate_ucan_call unless they appear in catalog as concrete resources.".to_string(),
            alternatives: Vec::new(),
            retry_eligible: false,
        }
    }

    /// Creates a context_required error.
    pub fn context_required(tool_name: &str) -> Self {
        Self {
            error_code: "context_required".to_string(),
            message: format!("Tool '{}' requires client context.", tool_name),
            recovery_hint:
                "This tool must be called through a client session. Use call_tool_with_context instead of call_tool."
                    .to_string(),
            alternatives: Vec::new(),
            retry_eligible: false,
        }
    }

    /// Creates an unknown_tool error.
    pub fn unknown_tool(tool_name: &str) -> Self {
        Self {
            error_code: "unknown_tool".to_string(),
            message: format!("Unknown broker tool: {}", tool_name),
            recovery_hint: "Available tools are: mcpmate_ucan_catalog, mcpmate_ucan_details, mcpmate_ucan_call."
                .to_string(),
            alternatives: vec![
                "mcpmate_ucan_catalog".to_string(),
                "mcpmate_ucan_details".to_string(),
                "mcpmate_ucan_call".to_string(),
            ],
            retry_eligible: false,
        }
    }

    /// Converts the error to a JSON string for MCP tool response.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| {
            serde_json::to_string_pretty(&serde_json::json!({
                "error_code": "serialization_error",
                "message": "Failed to serialize error response",
                "recovery_hint": "An internal error occurred while formatting the error response.",
                "alternatives": [],
                "retry_eligible": false
            }))
            .unwrap_or_else(|_| "{}".to_string())
        })
    }

    /// Converts the error to a failed CallToolResult.
    pub fn to_call_tool_result(&self) -> CallToolResult {
        CallToolResult::error(vec![Content::text(self.to_json())])
    }
}

/// Capitalizes the capability kind for display.
fn capitalize_kind(kind: &str) -> &'static str {
    match kind {
        "tool" => "Tool",
        "prompt" => "Prompt",
        "resource" => "Resource",
        "resource_template" => "Resource template",
        _ => "Capability",
    }
}

/// Finds similar names using Levenshtein distance.
/// Returns up to `limit` names sorted by similarity (closest first).
fn find_similar_names(
    query: &str,
    candidates: &[String],
    limit: usize,
) -> Vec<String> {
    if candidates.is_empty() || limit == 0 {
        return Vec::new();
    }

    let query_lower = query.to_lowercase();
    let mut scored: Vec<(usize, &String)> = candidates
        .iter()
        .filter(|c| !c.is_empty())
        .map(|candidate| {
            let candidate_lower = candidate.to_lowercase();
            let distance = levenshtein_distance(&query_lower, &candidate_lower);
            (distance, candidate)
        })
        .collect();

    scored.sort_by_key(|(distance, _)| *distance);

    scored
        .into_iter()
        .take(limit)
        .map(|(_, candidate)| candidate.clone())
        .collect()
}

/// Computes the Levenshtein distance between two strings.
/// Uses dynamic programming with O(min(m,n)) space.
fn levenshtein_distance(
    a: &str,
    b: &str,
) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let a_len = a_chars.len();
    let b_len = b_chars.len();

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    // Use the shorter string for the inner loop to minimize space
    let (longer, shorter) = if a_len > b_len {
        (&a_chars, &b_chars)
    } else {
        (&b_chars, &a_chars)
    };

    let mut prev_row: Vec<usize> = (0..=shorter.len()).collect();
    let mut curr_row: Vec<usize> = vec![0; shorter.len() + 1];

    for (i, long_char) in longer.iter().enumerate() {
        curr_row[0] = i + 1;
        for (j, short_char) in shorter.iter().enumerate() {
            let cost = if long_char == short_char { 0 } else { 1 };
            curr_row[j + 1] = (prev_row[j + 1] + 1).min(curr_row[j] + 1).min(prev_row[j] + cost);
        }
        std::mem::swap(&mut prev_row, &mut curr_row);
    }

    prev_row[shorter.len()]
}

pub struct BrokerService {
    database: Arc<Database>,
    connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
}

const UCAN_RELOAD_TTL: Duration = Duration::from_secs(2);

static UCAN_PROMPT_REPO: Lazy<UcanPromptRepository> = Lazy::new(UcanPromptRepository::new);

#[derive(Debug, Clone, Serialize)]
struct CatalogToolSummary {
    capability_name: String,
    capability_kind: UcanCapabilityKind,
    summary: Option<String>,
    action: &'static str,
    next_step: &'static str,
    server_id: String,
    server_name: String,
    interaction_mode: &'static str,
    detail_hint: &'static str,
    /// Whether this capability comes from a server installed from the registry
    registry_enriched: bool,
    /// Category from registry metadata (if available)
    registry_category: Option<String>,
}

#[derive(Debug, Serialize)]
struct CatalogPageResponse {
    format: Vec<String>,
    page: usize,
    page_size: usize,
    total_items: usize,
    total_pages: usize,
    has_next_page: bool,
    next_page: Option<usize>,
    usage: String,
    stale_hint: String,
    error_recovery_hint: String,
    items: Vec<CatalogToolSummary>,
}

#[derive(Debug, Clone, Deserialize)]
struct UcanPromptConfig {
    catalog_tool_description: String,
    details_tool_description: String,
    call_tool_description: String,
    catalog_usage: String,
    #[serde(default = "default_catalog_stale_hint")]
    catalog_stale_hint: String,
    #[serde(default = "default_error_recovery_hint")]
    error_recovery_hint: String,
    catalog_format: Vec<String>,
    catalog_page_size_default: usize,
    catalog_page_size_max: usize,
    #[serde(default)]
    catalog_sort_weights: CatalogSortWeights,
    #[serde(default)]
    workflow_hints: WorkflowHints,
    #[serde(default = "default_catalog_enrich_from_registry")]
    catalog_enrich_from_registry: bool,
}

fn default_catalog_enrich_from_registry() -> bool {
    true
}

fn default_catalog_stale_hint() -> String {
    "Catalog data may be stale if server status changed recently. Re-run mcpmate_ucan_catalog before deciding."
        .to_string()
}

fn default_error_recovery_hint() -> String {
    "If a call fails, verify capability_name and capability_kind from catalog, inspect details with detail_level=full, then retry with corrected arguments.".to_string()
}

#[derive(Debug, Clone, Deserialize)]
struct WorkflowHints {
    #[serde(default = "default_workflow_hints_tool")]
    tool: Vec<String>,
    #[serde(default = "default_workflow_hints_prompt")]
    prompt: Vec<String>,
    #[serde(default = "default_workflow_hints_resource")]
    resource: Vec<String>,
    #[serde(default = "default_workflow_hints_resource_template")]
    resource_template: Vec<String>,
}

impl Default for WorkflowHints {
    fn default() -> Self {
        Self {
            tool: default_workflow_hints_tool(),
            prompt: default_workflow_hints_prompt(),
            resource: default_workflow_hints_resource(),
            resource_template: default_workflow_hints_resource_template(),
        }
    }
}

impl WorkflowHints {
    fn normalize(&mut self) {
        self.tool = normalize_string_list(std::mem::take(&mut self.tool));
        self.prompt = normalize_string_list(std::mem::take(&mut self.prompt));
        self.resource = normalize_string_list(std::mem::take(&mut self.resource));
        self.resource_template = normalize_string_list(std::mem::take(&mut self.resource_template));
    }
}

fn default_workflow_hints_tool() -> Vec<String> {
    vec![
        "Use detail_level=summary first, then detail_level=full when arguments remain unclear.".to_string(),
        "Fill all required arguments before mcpmate_ucan_call.".to_string(),
    ]
}

fn default_workflow_hints_prompt() -> Vec<String> {
    vec![
        "Inspect prompt arguments first, then call through mcpmate_ucan_call.".to_string(),
        "Use required arguments only; avoid speculative fields.".to_string(),
    ]
}

fn default_workflow_hints_resource() -> Vec<String> {
    vec![
        "Inspect resource details first to confirm URI intent, then call via mcpmate_ucan_call without arguments."
            .to_string(),
    ]
}

fn default_workflow_hints_resource_template() -> Vec<String> {
    vec![
        "Resource templates are not directly invocable.".to_string(),
        "Use template output as guidance and call only concrete resources listed in catalog.".to_string(),
    ]
}

/// Multi-factor sorting weights for catalog relevance ranking.
/// Lower values = higher priority (sorted first).
#[derive(Debug, Clone, Deserialize, Default)]
struct CatalogSortWeights {
    #[serde(default)]
    kind: KindWeights,
    #[serde(default)]
    health: HealthWeights,
}

#[derive(Debug, Clone, Deserialize)]
struct KindWeights {
    #[serde(default = "default_kind_weight_tool")]
    tool: u32,
    #[serde(default = "default_kind_weight_prompt")]
    prompt: u32,
    #[serde(default = "default_kind_weight_resource")]
    resource: u32,
    #[serde(default = "default_kind_weight_resource_template")]
    resource_template: u32,
}

impl Default for KindWeights {
    fn default() -> Self {
        Self {
            tool: default_kind_weight_tool(),
            prompt: default_kind_weight_prompt(),
            resource: default_kind_weight_resource(),
            resource_template: default_kind_weight_resource_template(),
        }
    }
}

fn default_kind_weight_tool() -> u32 {
    0
}
fn default_kind_weight_prompt() -> u32 {
    1
}
fn default_kind_weight_resource() -> u32 {
    2
}
fn default_kind_weight_resource_template() -> u32 {
    3
}

#[derive(Debug, Clone, Deserialize)]
struct HealthWeights {
    #[serde(default = "default_health_weight_ready")]
    ready: u32,
    #[serde(default = "default_health_weight_reconnecting")]
    reconnecting: u32,
    #[serde(default = "default_health_weight_other")]
    other: u32,
}

impl Default for HealthWeights {
    fn default() -> Self {
        Self {
            ready: default_health_weight_ready(),
            reconnecting: default_health_weight_reconnecting(),
            other: default_health_weight_other(),
        }
    }
}

fn default_health_weight_ready() -> u32 {
    0
}
fn default_health_weight_reconnecting() -> u32 {
    1
}
fn default_health_weight_other() -> u32 {
    2
}

#[derive(Debug, Clone)]
struct UcanPromptState {
    config: UcanPromptConfig,
    last_loaded_at: Instant,
}

struct UcanPromptRepository {
    state: StdRwLock<Option<UcanPromptState>>,
    reload_lock: StdMutex<()>,
}

impl UcanPromptRepository {
    fn new() -> Self {
        Self {
            state: StdRwLock::new(None),
            reload_lock: StdMutex::new(()),
        }
    }

    async fn get(&self) -> UcanPromptConfig {
        self.get_blocking()
    }

    fn get_blocking(&self) -> UcanPromptConfig {
        {
            let guard = self.state.read().expect("ucan prompt state read lock");
            if let Some(state) = guard.as_ref()
                && state.last_loaded_at.elapsed() < UCAN_RELOAD_TTL
            {
                return state.config.clone();
            }
        }

        let _lock = self.reload_lock.lock().expect("ucan prompt reload lock");
        {
            let guard = self.state.read().expect("ucan prompt state read lock");
            if let Some(state) = guard.as_ref()
                && state.last_loaded_at.elapsed() < UCAN_RELOAD_TTL
            {
                return state.config.clone();
            }
        }

        let next_config = load_ucan_prompt_config_blocking().unwrap_or_else(|error| {
            tracing::warn!("Failed to reload UCAN prompt config: {error}");
            default_ucan_prompt_config()
        });

        let mut guard = self.state.write().expect("ucan prompt state write lock");
        let effective = next_config.clone();
        *guard = Some(UcanPromptState {
            config: next_config,
            last_loaded_at: Instant::now(),
        });
        effective
    }
}

/// Related capability reference for cross-linking in details response.
#[derive(Debug, Clone, Serialize)]
struct RelatedCapability {
    capability_name: String,
    capability_kind: UcanCapabilityKind,
    summary: Option<String>,
}

/// Argument tip extracted from schema for LLM guidance.
#[derive(Debug, Clone, Serialize)]
struct ArgumentTip {
    name: String,
    required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    type_hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

#[derive(Debug, Serialize)]
struct CapabilityDetailsResponse {
    capability_kind: UcanCapabilityKind,
    capability_name: String,
    server_id: String,
    server_name: String,
    detail_level: UcanDetailLevel,
    details: serde_json::Value,
    /// Workflow hints for LLM to understand how to use this capability.
    workflow_hints: Vec<String>,
    /// Related capabilities from the same server (max 5).
    related_capabilities: Vec<RelatedCapability>,
    /// Argument tips extracted from schema (for tools and prompts).
    argument_tips: Vec<ArgumentTip>,
    call_requirements: CallRequirements,
    error_recovery_hint: String,
}

#[derive(Debug, Clone, Serialize)]
struct CallRequirements {
    accepts_arguments: bool,
    required_arguments: Vec<String>,
    call_ready_without_arguments: bool,
}

#[derive(Debug, Deserialize)]
struct CapabilityLookupParams {
    capability_kind: UcanCapabilityKind,
    capability_name: String,
    #[serde(default)]
    detail_level: UcanDetailLevel,
}

#[derive(Debug, Deserialize)]
struct CatalogParams {
    #[serde(default = "default_catalog_page")]
    page: usize,
    #[serde(default = "default_catalog_page_size")]
    page_size: usize,
    /// Case-insensitive substring search in capability_name and summary.
    #[serde(default)]
    search: Option<String>,
    /// Filter by capability_kind. Valid values: "tool", "prompt", "resource", "resource_template".
    #[serde(default)]
    kind_filter: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct BrokerCapabilityCallParams {
    capability_kind: UcanCapabilityKind,
    capability_name: String,
    #[serde(default)]
    arguments: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum UcanCapabilityKind {
    Tool,
    Prompt,
    Resource,
    ResourceTemplate,
}

impl UcanCapabilityKind {
    fn weight(
        &self,
        weights: &KindWeights,
    ) -> u32 {
        match self {
            UcanCapabilityKind::Tool => weights.tool,
            UcanCapabilityKind::Prompt => weights.prompt,
            UcanCapabilityKind::Resource => weights.resource,
            UcanCapabilityKind::ResourceTemplate => weights.resource_template,
        }
    }
}

type InstanceSnapshot = (
    String,
    ConnectionStatus,
    bool,
    bool,
    Option<rmcp::service::Peer<rmcp::RoleClient>>,
);
type PoolSnapshot = std::collections::HashMap<String, Vec<InstanceSnapshot>>;

fn health_weight_for_server(
    server_id: &str,
    snapshot: &PoolSnapshot,
    weights: &HealthWeights,
) -> u32 {
    if let Some(instances) = snapshot.get(server_id) {
        for (_, status, _, _, _) in instances {
            if matches!(status, ConnectionStatus::Ready) {
                return weights.ready;
            }
            if matches!(status, ConnectionStatus::Initializing) {
                return weights.reconnecting;
            }
        }
    }
    weights.other
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
enum UcanDetailLevel {
    #[default]
    Summary,
    Full,
}

#[derive(Clone)]
struct VisibleToolEntry {
    server_id: String,
    server_name: String,
    raw_tool_name: String,
    tool: Tool,
}

#[derive(Clone)]
struct VisiblePromptEntry {
    server_id: String,
    server_name: String,
    raw_prompt_name: String,
    prompt: rmcp::model::Prompt,
}

#[derive(Clone)]
struct VisibleResourceEntry {
    server_id: String,
    server_name: String,
    raw_resource_uri: String,
    resource: Resource,
}

#[derive(Clone)]
struct VisibleResourceTemplateEntry {
    server_id: String,
    server_name: String,
    raw_uri_template: String,
    resource_template: ResourceTemplate,
}

fn retain_brokered_tools(
    context: &ClientBuiltinContext,
    eligible_server_ids: &HashSet<String>,
    visible: &mut Vec<VisibleToolEntry>,
) {
    visible.retain(|entry| {
        !crate::core::proxy::server::unify_directly_exposed_tool_allowed(
            context.unify_workspace.as_ref(),
            eligible_server_ids,
            &entry.server_id,
            &entry.raw_tool_name,
        )
    });
}

fn retain_brokered_prompts(
    context: &ClientBuiltinContext,
    eligible_server_ids: &HashSet<String>,
    visible: &mut Vec<VisiblePromptEntry>,
) {
    visible.retain(|entry| {
        !crate::core::proxy::server::unify_directly_exposed_prompt_allowed(
            context.unify_workspace.as_ref(),
            eligible_server_ids,
            &entry.server_id,
            &entry.raw_prompt_name,
        )
    });
}

fn retain_brokered_resources(
    context: &ClientBuiltinContext,
    eligible_server_ids: &HashSet<String>,
    visible: &mut Vec<VisibleResourceEntry>,
) {
    visible.retain(|entry| {
        !crate::core::proxy::server::unify_directly_exposed_resource_allowed(
            context.unify_workspace.as_ref(),
            eligible_server_ids,
            &entry.server_id,
            &entry.raw_resource_uri,
        )
    });
}

fn retain_brokered_resource_templates(
    context: &ClientBuiltinContext,
    eligible_server_ids: &HashSet<String>,
    visible: &mut Vec<VisibleResourceTemplateEntry>,
) {
    visible.retain(|entry| {
        !crate::core::proxy::server::unify_directly_exposed_template_allowed(
            context.unify_workspace.as_ref(),
            eligible_server_ids,
            &entry.server_id,
            &entry.raw_uri_template,
        )
    });
}

impl BrokerService {
    pub fn new(
        database: Arc<Database>,
        connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
    ) -> Self {
        Self {
            database,
            connection_pool,
        }
    }

    async fn ucan_prompt_config(&self) -> UcanPromptConfig {
        UCAN_PROMPT_REPO.get().await
    }

    async fn load_enabled_servers(
        &self,
        context: &'static str,
    ) -> Result<Vec<(String, String, Option<String>, bool)>> {
        sqlx::query_as(
            r#"
            SELECT sc.id, sc.name, sc.capabilities, sc.unify_direct_exposure_eligible
            FROM server_config sc
            WHERE sc.enabled = 1
            ORDER BY sc.name, sc.id
            "#,
        )
        .fetch_all(&self.database.pool)
        .await
        .context(context)
    }

    async fn fetch_registry_enrichment(
        &self,
        server_ids: &[String],
    ) -> Result<HashMap<String, (bool, Option<String>)>> {
        if server_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let placeholders: Vec<String> = server_ids.iter().map(|_| "?".to_string()).collect();
        let placeholders_str = placeholders.join(",");
        let query_str = format!(
            r#"
            SELECT sc.id, sc.registry_server_id
            FROM server_config sc
            WHERE sc.id IN ({})
            "#,
            placeholders_str
        );

        let mut query = sqlx::query_as::<_, (String, Option<String>)>(&query_str);
        for id in server_ids {
            query = query.bind(id);
        }
        let rows = query
            .fetch_all(&self.database.pool)
            .await
            .context("Failed to fetch registry server IDs")?;

        let server_registry_map: HashMap<String, Option<String>> = rows.into_iter().collect();

        let registry_names: Vec<String> = server_registry_map
            .values()
            .filter_map(|r| r.as_ref())
            .cloned()
            .collect();

        if registry_names.is_empty() {
            let result: HashMap<String, (bool, Option<String>)> = server_ids
                .iter()
                .map(|id| {
                    let enriched = server_registry_map.get(id).and_then(|r| r.as_ref()).is_some();
                    (id.clone(), (enriched, None))
                })
                .collect();
            return Ok(result);
        }

        let cache_service = RegistryCacheService::new(self.database.pool.clone());
        let mut enrichment_map: HashMap<String, (bool, Option<String>)> = HashMap::new();

        for server_id in server_ids {
            if let Some(Some(registry_name)) = server_registry_map.get(server_id) {
                match cache_service.get_by_name(registry_name).await {
                    Ok(Some(entry)) => {
                        let category = entry
                            .meta_json
                            .as_ref()
                            .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok())
                            .and_then(|v| v.get("category").and_then(|c| c.as_str()).map(|s| s.to_string()));
                        enrichment_map.insert(server_id.clone(), (true, category));
                    }
                    _ => {
                        enrichment_map.insert(server_id.clone(), (true, None));
                    }
                }
            } else {
                enrichment_map.insert(server_id.clone(), (false, None));
            }
        }

        Ok(enrichment_map)
    }

    async fn tool_catalog(
        &self,
        context: &ClientBuiltinContext,
        page: usize,
        page_size: usize,
        search: Option<&str>,
        kind_filter: Option<&[String]>,
    ) -> Result<CallToolResult> {
        let prompt_config = self.ucan_prompt_config().await;
        let enrich_enabled = prompt_config.catalog_enrich_from_registry;

        let tools = self.visible_tools(context).await?;
        let prompts = self.visible_prompts(context).await?;
        let resources = self.visible_resources(context).await?;
        let resource_templates = self.visible_resource_templates(context).await?;

        let all_server_ids: Vec<String> = {
            let mut ids = HashSet::new();
            for entry in &tools {
                ids.insert(entry.server_id.clone());
            }
            for entry in &prompts {
                ids.insert(entry.server_id.clone());
            }
            for entry in &resources {
                ids.insert(entry.server_id.clone());
            }
            for entry in &resource_templates {
                ids.insert(entry.server_id.clone());
            }
            ids.into_iter().collect()
        };

        let enrichment_map = if enrich_enabled {
            self.fetch_registry_enrichment(&all_server_ids).await?
        } else {
            HashMap::new()
        };

        let get_enrichment = |server_id: &str| -> (bool, Option<String>) {
            enrichment_map.get(server_id).cloned().unwrap_or((false, None))
        };

        let mut summaries: Vec<CatalogToolSummary> = tools
            .into_iter()
            .map(|entry| {
                let (registry_enriched, registry_category) = get_enrichment(&entry.server_id);
                CatalogToolSummary {
                    capability_name: entry.tool.name.to_string(),
                    capability_kind: UcanCapabilityKind::Tool,
                    summary: compact_description(entry.tool.description.as_deref()),
                    action: "inspect_first",
                    next_step: "details",
                    server_id: entry.server_id,
                    server_name: entry.server_name,
                    interaction_mode: "model_controlled",
                    detail_hint: "Use mcpmate_ucan_details with detail_level=summary first; switch to full before constructing arguments if needed.",
                    registry_enriched,
                    registry_category,
                }
            })
            .collect();

        summaries.extend(
            prompts.into_iter().map(|entry| {
                let (registry_enriched, registry_category) = get_enrichment(&entry.server_id);
                CatalogToolSummary {
                    capability_name: entry.prompt.name.to_string(),
                    capability_kind: UcanCapabilityKind::Prompt,
                    summary: compact_description(extract_description_from_value(&entry.prompt).as_deref()),
                    action: "inspect_first",
                    next_step: "details",
                    server_id: entry.server_id,
                    server_name: entry.server_name,
                    interaction_mode: "user_controlled_template",
                    detail_hint: "Prompt results are brokered through mcpmate_ucan_call; inspect arguments with mcpmate_ucan_details first.",
                    registry_enriched,
                    registry_category,
                }
            }),
        );

        summaries.extend(resources.into_iter().map(|entry| {
            let (registry_enriched, registry_category) = get_enrichment(&entry.server_id);
            CatalogToolSummary {
                capability_name: entry.resource.uri.to_string(),
                    capability_kind: UcanCapabilityKind::Resource,
                    summary: compact_description(extract_description_from_value(&entry.resource).as_deref()),
                    action: "inspect_first",
                    next_step: "details",
                    server_id: entry.server_id,
                    server_name: entry.server_name,
                    interaction_mode: "application_context",
                    detail_hint: "Inspect resource details first, then call mcpmate_ucan_call with capability_kind=resource and arguments={}",
                    registry_enriched,
                    registry_category,
                }
        }));

        summaries.extend(
            resource_templates.into_iter().map(|entry| {
                let (registry_enriched, registry_category) = get_enrichment(&entry.server_id);
                CatalogToolSummary {
                    capability_name: entry.resource_template.name.to_string(),
                    capability_kind: UcanCapabilityKind::ResourceTemplate,
                    summary: compact_description(extract_description_from_value(&entry.resource_template).as_deref()),
                    action: "inspect_first",
                    next_step: "details",
                    server_id: entry.server_id,
                    server_name: entry.server_name,
                    interaction_mode: "application_context_template",
                    detail_hint: "Inspect template rules first. Template-derived URIs are only callable if they appear as concrete resources in catalog.",
                    registry_enriched,
                    registry_category,
                }
            }),
        );

        let weights = &prompt_config.catalog_sort_weights;
        let pool_snapshot = self.connection_pool.lock().await.get_snapshot();

        summaries.sort_by(|left, right| {
            let left_kind_weight = left.capability_kind.weight(&weights.kind);
            let right_kind_weight = right.capability_kind.weight(&weights.kind);

            let left_health_weight = health_weight_for_server(&left.server_id, &pool_snapshot, &weights.health);
            let right_health_weight = health_weight_for_server(&right.server_id, &pool_snapshot, &weights.health);

            let left_score = left_kind_weight.saturating_add(left_health_weight);
            let right_score = right_kind_weight.saturating_add(right_health_weight);

            left_score
                .cmp(&right_score)
                .then_with(|| left.capability_name.cmp(&right.capability_name))
        });

        // Apply filters before pagination
        if let Some(search_term) = search {
            let search_lower = search_term.to_lowercase();
            summaries.retain(|item| {
                let name_match = item.capability_name.to_lowercase().contains(&search_lower);
                let summary_match = item
                    .summary
                    .as_ref()
                    .map(|s| s.to_lowercase().contains(&search_lower))
                    .unwrap_or(false);
                name_match || summary_match
            });
        }

        if let Some(kinds) = kind_filter {
            let allowed_kinds: HashSet<&str> = kinds.iter().map(|s| s.as_str()).collect();
            summaries.retain(|item| {
                let kind_str = match item.capability_kind {
                    UcanCapabilityKind::Tool => "tool",
                    UcanCapabilityKind::Prompt => "prompt",
                    UcanCapabilityKind::Resource => "resource",
                    UcanCapabilityKind::ResourceTemplate => "resource_template",
                };
                allowed_kinds.contains(kind_str)
            });
        }

        let page_size = page_size.clamp(1, 50);
        let page_size = page_size.clamp(1, prompt_config.catalog_page_size_max.max(1));
        let total_items = summaries.len();
        let total_pages = if total_items == 0 {
            1
        } else {
            total_items.div_ceil(page_size)
        };
        let page = page.clamp(1, total_pages);
        let start = (page - 1) * page_size;
        let end = (start + page_size).min(total_items);
        let items = if start < total_items {
            summaries[start..end].to_vec()
        } else {
            Vec::new()
        };
        let response = CatalogPageResponse {
            format: if prompt_config.catalog_format.is_empty() {
                vec![
                    "capability_name".to_string(),
                    "capability_kind".to_string(),
                    "summary".to_string(),
                    "action".to_string(),
                    "next_step".to_string(),
                    "server_id".to_string(),
                    "server_name".to_string(),
                    "interaction_mode".to_string(),
                    "detail_hint".to_string(),
                    "registry_enriched".to_string(),
                    "registry_category".to_string(),
                ]
            } else {
                prompt_config.catalog_format.clone()
            },
            page,
            page_size,
            total_items,
            total_pages,
            has_next_page: page < total_pages,
            next_page: (page < total_pages).then_some(page + 1),
            usage: prompt_config.catalog_usage.clone(),
            stale_hint: prompt_config.catalog_stale_hint.clone(),
            error_recovery_hint: prompt_config.error_recovery_hint.clone(),
            items,
        };

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&response).context("Failed to serialize Unify tool catalog")?,
        )]))
    }

    async fn collect_capability_names_for_kind(
        &self,
        context: &ClientBuiltinContext,
        kind: UcanCapabilityKind,
    ) -> Result<Vec<String>> {
        let names = match kind {
            UcanCapabilityKind::Tool => self
                .visible_tools(context)
                .await?
                .into_iter()
                .map(|entry| entry.tool.name.to_string())
                .collect(),
            UcanCapabilityKind::Prompt => self
                .visible_prompts(context)
                .await?
                .into_iter()
                .map(|entry| entry.prompt.name.to_string())
                .collect(),
            UcanCapabilityKind::Resource => self
                .visible_resources(context)
                .await?
                .into_iter()
                .map(|entry| entry.resource.uri.to_string())
                .collect(),
            UcanCapabilityKind::ResourceTemplate => self
                .visible_resource_templates(context)
                .await?
                .into_iter()
                .map(|entry| entry.resource_template.name.to_string())
                .collect(),
        };
        Ok(names)
    }

    async fn tool_details(
        &self,
        context: &ClientBuiltinContext,
        capability_kind: UcanCapabilityKind,
        capability_name: &str,
        detail_level: UcanDetailLevel,
    ) -> Result<CallToolResult> {
        let prompt_config = self.ucan_prompt_config().await;
        let workflow_hints = match capability_kind {
            UcanCapabilityKind::Tool => prompt_config.workflow_hints.tool.clone(),
            UcanCapabilityKind::Prompt => prompt_config.workflow_hints.prompt.clone(),
            UcanCapabilityKind::Resource => prompt_config.workflow_hints.resource.clone(),
            UcanCapabilityKind::ResourceTemplate => prompt_config.workflow_hints.resource_template.clone(),
        };

        let response = match capability_kind {
            UcanCapabilityKind::Tool => match self.find_visible_tool(context, capability_name).await? {
                Some(tool) => {
                    let related = self
                        .find_related_capabilities(context, &tool.server_id, capability_name, capability_kind)
                        .await;
                    let argument_tips = extract_argument_tips_from_tool(&tool.tool);
                    CapabilityDetailsResponse {
                        capability_kind,
                        capability_name: tool.tool.name.to_string(),
                        server_id: tool.server_id,
                        server_name: tool.server_name,
                        detail_level,
                        details: tool_details_value(&tool.tool, detail_level),
                        workflow_hints,
                        related_capabilities: related,
                        argument_tips,
                        call_requirements: call_requirements_for_tool(&tool.tool),
                        error_recovery_hint: prompt_config.error_recovery_hint.clone(),
                    }
                }
                None => {
                    let catalog_names = self.collect_capability_names_for_kind(context, capability_kind).await?;
                    return Ok(
                        UcanError::capability_not_found("tool", capability_name, &catalog_names).to_call_tool_result()
                    );
                }
            },
            UcanCapabilityKind::Prompt => match self.find_visible_prompt(context, capability_name).await? {
                Some(prompt) => {
                    let related = self
                        .find_related_capabilities(context, &prompt.server_id, capability_name, capability_kind)
                        .await;
                    let argument_tips = extract_argument_tips_from_prompt(&prompt.prompt);
                    CapabilityDetailsResponse {
                        capability_kind,
                        capability_name: prompt.prompt.name.to_string(),
                        server_id: prompt.server_id,
                        server_name: prompt.server_name,
                        detail_level,
                        details: prompt_details_value(&prompt.prompt, detail_level)
                            .context("Failed to serialize Unify prompt details")?,
                        workflow_hints,
                        related_capabilities: related,
                        argument_tips,
                        call_requirements: call_requirements_for_prompt(&prompt.prompt),
                        error_recovery_hint: prompt_config.error_recovery_hint.clone(),
                    }
                }
                None => {
                    let catalog_names = self.collect_capability_names_for_kind(context, capability_kind).await?;
                    return Ok(
                        UcanError::capability_not_found("prompt", capability_name, &catalog_names)
                            .to_call_tool_result(),
                    );
                }
            },
            UcanCapabilityKind::Resource => match self.find_visible_resource(context, capability_name).await? {
                Some(resource) => {
                    let related = self
                        .find_related_capabilities(context, &resource.server_id, capability_name, capability_kind)
                        .await;
                    CapabilityDetailsResponse {
                        capability_kind,
                        capability_name: resource.resource.uri.to_string(),
                        server_id: resource.server_id,
                        server_name: resource.server_name,
                        detail_level,
                        details: resource_details_value(&resource.resource, detail_level)
                            .context("Failed to serialize Unify resource details")?,
                        workflow_hints,
                        related_capabilities: related,
                        argument_tips: Vec::new(),
                        call_requirements: CallRequirements {
                            accepts_arguments: false,
                            required_arguments: Vec::new(),
                            call_ready_without_arguments: true,
                        },
                        error_recovery_hint: prompt_config.error_recovery_hint.clone(),
                    }
                }
                None => {
                    let catalog_names = self.collect_capability_names_for_kind(context, capability_kind).await?;
                    return Ok(
                        UcanError::capability_not_found("resource", capability_name, &catalog_names)
                            .to_call_tool_result(),
                    );
                }
            },
            UcanCapabilityKind::ResourceTemplate => {
                match self.find_visible_resource_template(context, capability_name).await? {
                    Some(template) => {
                        let related = self
                            .find_related_capabilities(context, &template.server_id, capability_name, capability_kind)
                            .await;
                        CapabilityDetailsResponse {
                            capability_kind,
                            capability_name: template.resource_template.name.to_string(),
                            server_id: template.server_id,
                            server_name: template.server_name,
                            detail_level,
                            details: resource_template_details_value(&template.resource_template, detail_level)
                                .context("Failed to serialize Unify resource template details")?,
                            workflow_hints,
                            related_capabilities: related,
                            argument_tips: Vec::new(),
                            call_requirements: CallRequirements {
                                accepts_arguments: false,
                                required_arguments: Vec::new(),
                                call_ready_without_arguments: false,
                            },
                            error_recovery_hint: prompt_config.error_recovery_hint.clone(),
                        }
                    }
                    None => {
                        let catalog_names = self.collect_capability_names_for_kind(context, capability_kind).await?;
                        return Ok(UcanError::capability_not_found(
                            "resource_template",
                            capability_name,
                            &catalog_names,
                        )
                        .to_call_tool_result());
                    }
                }
            }
        };

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&response).context("Failed to serialize Unify tool details")?,
        )]))
    }

    async fn find_related_capabilities(
        &self,
        context: &ClientBuiltinContext,
        server_id: &str,
        exclude_name: &str,
        exclude_kind: UcanCapabilityKind,
    ) -> Vec<RelatedCapability> {
        let mut related = Vec::new();

        if exclude_kind != UcanCapabilityKind::Tool {
            if let Ok(tools) = self.visible_tools(context).await {
                for entry in tools.iter().take(50) {
                    if entry.server_id == server_id && entry.tool.name.as_ref() != exclude_name {
                        related.push(RelatedCapability {
                            capability_name: entry.tool.name.to_string(),
                            capability_kind: UcanCapabilityKind::Tool,
                            summary: compact_description(entry.tool.description.as_deref()),
                        });
                        if related.len() >= 5 {
                            break;
                        }
                    }
                }
            }
        }

        if related.len() < 5 && exclude_kind != UcanCapabilityKind::Prompt {
            if let Ok(prompts) = self.visible_prompts(context).await {
                for entry in prompts.iter().take(50) {
                    if entry.server_id == server_id && entry.prompt.name.as_str() != exclude_name {
                        related.push(RelatedCapability {
                            capability_name: entry.prompt.name.to_string(),
                            capability_kind: UcanCapabilityKind::Prompt,
                            summary: compact_description(extract_description_from_value(&entry.prompt).as_deref()),
                        });
                        if related.len() >= 5 {
                            break;
                        }
                    }
                }
            }
        }

        if related.len() < 5 && exclude_kind != UcanCapabilityKind::Resource {
            if let Ok(resources) = self.visible_resources(context).await {
                for entry in resources.iter().take(50) {
                    if entry.server_id == server_id && entry.resource.uri.as_str() != exclude_name {
                        related.push(RelatedCapability {
                            capability_name: entry.resource.uri.to_string(),
                            capability_kind: UcanCapabilityKind::Resource,
                            summary: compact_description(extract_description_from_value(&entry.resource).as_deref()),
                        });
                        if related.len() >= 5 {
                            break;
                        }
                    }
                }
            }
        }

        if related.len() < 5 && exclude_kind != UcanCapabilityKind::ResourceTemplate {
            if let Ok(templates) = self.visible_resource_templates(context).await {
                for entry in templates.iter().take(50) {
                    if entry.server_id == server_id && entry.resource_template.name.as_str() != exclude_name {
                        related.push(RelatedCapability {
                            capability_name: entry.resource_template.name.to_string(),
                            capability_kind: UcanCapabilityKind::ResourceTemplate,
                            summary: compact_description(
                                extract_description_from_value(&entry.resource_template).as_deref(),
                            ),
                        });
                        if related.len() >= 5 {
                            break;
                        }
                    }
                }
            }
        }

        related
    }

    async fn broker_tool_call(
        &self,
        context: &ClientBuiltinContext,
        capability_kind: UcanCapabilityKind,
        capability_name: &str,
        arguments: serde_json::Map<String, serde_json::Value>,
    ) -> Result<CallToolResult> {
        match capability_kind {
            UcanCapabilityKind::Tool => self.broker_tool_call_inner(context, capability_name, arguments).await,
            UcanCapabilityKind::Prompt => self.broker_prompt_call(context, capability_name, arguments).await,
            UcanCapabilityKind::Resource => {
                if !arguments.is_empty() {
                    return Ok(UcanError::resource_arguments_not_supported(capability_name).to_call_tool_result());
                }
                self.broker_resource_read(context, capability_name).await
            }
            UcanCapabilityKind::ResourceTemplate => {
                Ok(UcanError::resource_template_not_invocable(capability_name).to_call_tool_result())
            }
        }
    }

    async fn broker_tool_call_inner(
        &self,
        context: &ClientBuiltinContext,
        tool_name: &str,
        arguments: serde_json::Map<String, serde_json::Value>,
    ) -> Result<CallToolResult> {
        if let Some(tool_entry) = self.find_visible_tool(context, tool_name).await? {
            let required_arguments = required_arguments_from_tool(&tool_entry.tool);
            let missing_required: Vec<String> = required_arguments
                .into_iter()
                .filter(|name| !arguments.contains_key(name))
                .collect();
            if !missing_required.is_empty() {
                return Ok(
                    UcanError::missing_required_arguments("tool", tool_name, &missing_required).to_call_tool_result(),
                );
            }
        }

        let client_context = context.as_client_context();
        let visibility = ProfileVisibilityService::new(Some(self.database.clone()), None);
        let snapshot = visibility
            .resolve_snapshot_for_client(&client_context)
            .await
            .context("Failed to resolve Unify visibility snapshot")?;

        if visibility
            .assert_tool_allowed_with_snapshot(&snapshot, tool_name)
            .await
            .is_err()
        {
            return Ok(UcanError::visibility_denied("tool", tool_name).to_call_tool_result());
        }

        let (server_name, original_tool_name) = resolve_unique_name(NamingKind::Tool, tool_name)
            .await
            .with_context(|| format!("Failed to resolve unique tool '{}'", tool_name))?;
        let server_id = match crate::core::capability::resolver::to_id(&server_name)
            .await
            .ok()
            .flatten()
        {
            Some(id) => id,
            None => {
                return Ok(UcanError::server_unreachable("unknown", &server_name).to_call_tool_result());
            }
        };

        let peer = match self.acquire_peer(&client_context, &server_id).await {
            Ok(p) => p,
            Err(_) => {
                return Ok(UcanError::server_unreachable(&server_id, &server_name).to_call_tool_result());
            }
        };
        let request = ClientRequest::CallToolRequest(CallToolRequest::new(
            CallToolRequestParams::new(original_tool_name).with_arguments(arguments),
        ));
        let timeout_secs = std::env::var("MCPMATE_TOOL_CALL_TIMEOUT_SECS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(60);
        let mut options = PeerRequestOptions::no_options();
        options.timeout = Some(std::time::Duration::from_secs(timeout_secs));
        let handle = peer
            .send_cancellable_request(request, options)
            .await
            .context("Failed to send Unify broker tool call")?;

        match handle.await_response().await {
            Ok(rmcp::model::ServerResult::CallToolResult(result)) => Ok(result),
            Ok(other) => {
                Ok(UcanError::upstream_error("tool", tool_name, &format!("{:?}", other)).to_call_tool_result())
            }
            Err(error) => {
                let error_str = error.to_string();
                if error_str.contains("timeout") || error_str.contains("Timeout") || error_str.contains("timed out") {
                    Ok(UcanError::timeout("tool", tool_name, timeout_secs).to_call_tool_result())
                } else {
                    Ok(UcanError::upstream_error("tool", tool_name, &error_str).to_call_tool_result())
                }
            }
        }
    }

    async fn visible_tools(
        &self,
        context: &ClientBuiltinContext,
    ) -> Result<Vec<VisibleToolEntry>> {
        let client_context = context.as_client_context();
        let visibility = ProfileVisibilityService::new(Some(self.database.clone()), None);
        let snapshot = visibility
            .resolve_snapshot_for_client(&client_context)
            .await
            .context("Failed to resolve Unify visibility snapshot")?;
        let visible_server_ids = snapshot.server_ids.iter().cloned().collect::<HashSet<_>>();

        let enabled_servers = self
            .load_enabled_servers("Failed to load enabled servers for Unify catalog")
            .await?;

        let redb = RedbCacheManager::global().context("REDB cache is not initialized")?;
        let database = self.database.clone();
        let connection_pool = self.connection_pool.clone();
        let runtime_identity = client_context.runtime_identity();

        let mut tasks = Vec::new();
        let mut eligible_server_ids = HashSet::new();
        for (server_id, server_name, capabilities, unify_direct_exposure_eligible) in enabled_servers {
            if !visible_server_ids.contains(&server_id) {
                continue;
            }
            if !crate::core::proxy::server::supports_capability(
                capabilities.as_deref(),
                crate::core::capability::CapabilityType::Tools,
            ) {
                continue;
            }
            if unify_direct_exposure_eligible {
                eligible_server_ids.insert(server_id.clone());
            }

            let ctx = crate::core::capability::runtime::ListCtx {
                capability: crate::core::capability::CapabilityType::Tools,
                server_id: server_id.clone(),
                refresh: Some(crate::core::capability::runtime::RefreshStrategy::CacheFirst),
                timeout: Some(std::time::Duration::from_secs(10)),
                validation_session: None,
                runtime_identity: runtime_identity.clone(),
                connection_selection: client_context.connection_selection(server_id.clone()),
            };
            let redb = redb.clone();
            let database = database.clone();
            let connection_pool = connection_pool.clone();
            tasks.push(async move {
                let result = crate::core::capability::runtime::list(&ctx, &redb, &connection_pool, &database).await;
                (server_id, server_name, result)
            });
        }

        let mut visible = Vec::new();
        for (server_id, server_name, result) in futures::stream::iter(tasks)
            .buffer_unordered(crate::core::capability::facade::concurrency_limit())
            .collect::<Vec<_>>()
            .await
        {
            if let Ok(result) = result {
                if let Some(tools) = result.items.into_tools() {
                    for tool in tools {
                        let raw_tool_name = crate::core::proxy::server::resolve_direct_surface_value(
                            NamingKind::Tool,
                            &server_name,
                            tool.name.as_ref(),
                        )
                        .await;
                        visible.push(VisibleToolEntry {
                            server_id: server_id.clone(),
                            server_name: server_name.clone(),
                            raw_tool_name,
                            tool,
                        });
                    }
                }
            }
        }

        let filtered_names = visibility
            .filter_tools_with_snapshot(&snapshot, visible.iter().map(|entry| entry.tool.clone()).collect())
            .into_iter()
            .map(|tool| tool.name.to_string())
            .collect::<HashSet<_>>();
        visible.retain(|entry| filtered_names.contains(entry.tool.name.as_ref()));
        retain_brokered_tools(context, &eligible_server_ids, &mut visible);
        visible.sort_by(|left, right| {
            left.server_name
                .cmp(&right.server_name)
                .then_with(|| left.tool.name.as_ref().cmp(right.tool.name.as_ref()))
        });

        Ok(visible)
    }

    async fn find_visible_tool(
        &self,
        context: &ClientBuiltinContext,
        tool_name: &str,
    ) -> Result<Option<VisibleToolEntry>> {
        let tools = self.visible_tools(context).await?;
        Ok(tools.into_iter().find(|entry| entry.tool.name.as_ref() == tool_name))
    }

    async fn visible_prompts(
        &self,
        context: &ClientBuiltinContext,
    ) -> Result<Vec<VisiblePromptEntry>> {
        let client_context = context.as_client_context();
        let visibility = ProfileVisibilityService::new(Some(self.database.clone()), None);
        let snapshot = visibility
            .resolve_snapshot_for_client(&client_context)
            .await
            .context("Failed to resolve Unify visibility snapshot")?;
        let visible_server_ids = snapshot.server_ids.iter().cloned().collect::<HashSet<_>>();

        let enabled_servers = self
            .load_enabled_servers("Failed to load enabled servers for Unify prompt catalog")
            .await?;
        let eligible_server_ids = enabled_servers
            .iter()
            .filter_map(|(server_id, _server_name, _capabilities, eligible)| eligible.then_some(server_id.clone()))
            .collect::<HashSet<_>>();

        let redb = RedbCacheManager::global().context("REDB cache is not initialized")?;
        let database = self.database.clone();
        let connection_pool = self.connection_pool.clone();
        let runtime_identity = client_context.runtime_identity();

        let mut tasks = Vec::new();
        for (server_id, server_name, capabilities, _unify_direct_exposure_eligible) in enabled_servers {
            if !visible_server_ids.contains(&server_id) {
                continue;
            }
            if !crate::core::proxy::server::supports_capability(
                capabilities.as_deref(),
                crate::core::capability::CapabilityType::Prompts,
            ) {
                continue;
            }

            let ctx = crate::core::capability::runtime::ListCtx {
                capability: crate::core::capability::CapabilityType::Prompts,
                server_id: server_id.clone(),
                refresh: Some(crate::core::capability::runtime::RefreshStrategy::CacheFirst),
                timeout: Some(std::time::Duration::from_secs(10)),
                validation_session: None,
                runtime_identity: runtime_identity.clone(),
                connection_selection: client_context.connection_selection(server_id.clone()),
            };
            let redb = redb.clone();
            let database = database.clone();
            let connection_pool = connection_pool.clone();
            let server_name_cloned = server_name.clone();
            tasks.push(async move {
                let result = crate::core::capability::runtime::list(&ctx, &redb, &connection_pool, &database).await;
                (server_id, server_name_cloned, result)
            });
        }

        let mut visible = Vec::new();
        for (server_id, server_name, result) in futures::stream::iter(tasks)
            .buffer_unordered(crate::core::capability::facade::concurrency_limit())
            .collect::<Vec<_>>()
            .await
        {
            if let Ok(result) = result {
                if let Some(prompts) = result.items.into_prompts() {
                    for mut prompt in prompts {
                        let raw_prompt_name = prompt.name.to_string();
                        prompt.name = crate::core::capability::naming::generate_unique_name(
                            NamingKind::Prompt,
                            &server_name,
                            &raw_prompt_name,
                        );
                        visible.push(VisiblePromptEntry {
                            server_id: server_id.clone(),
                            server_name: server_name.clone(),
                            raw_prompt_name,
                            prompt,
                        });
                    }
                }
            }
        }

        let filtered_names = visibility
            .filter_prompts_with_snapshot(&snapshot, visible.iter().map(|entry| entry.prompt.clone()).collect())
            .into_iter()
            .map(|prompt| prompt.name.to_string())
            .collect::<HashSet<_>>();
        visible.retain(|entry| filtered_names.contains(entry.prompt.name.as_str()));
        retain_brokered_prompts(context, &eligible_server_ids, &mut visible);
        visible.sort_by(|left, right| {
            left.server_name
                .cmp(&right.server_name)
                .then_with(|| left.prompt.name.as_str().cmp(right.prompt.name.as_str()))
        });

        Ok(visible)
    }

    async fn find_visible_prompt(
        &self,
        context: &ClientBuiltinContext,
        prompt_name: &str,
    ) -> Result<Option<VisiblePromptEntry>> {
        let prompts = self.visible_prompts(context).await?;
        Ok(prompts
            .into_iter()
            .find(|entry| entry.prompt.name.as_str() == prompt_name))
    }

    async fn visible_resources(
        &self,
        context: &ClientBuiltinContext,
    ) -> Result<Vec<VisibleResourceEntry>> {
        let client_context = context.as_client_context();
        let visibility = ProfileVisibilityService::new(Some(self.database.clone()), None);
        let snapshot = visibility
            .resolve_snapshot_for_client(&client_context)
            .await
            .context("Failed to resolve Unify visibility snapshot")?;
        let visible_server_ids = snapshot.server_ids.iter().cloned().collect::<HashSet<_>>();

        let enabled_servers = self
            .load_enabled_servers("Failed to load enabled servers for Unify resource catalog")
            .await?;
        let eligible_server_ids = enabled_servers
            .iter()
            .filter_map(|(server_id, _server_name, _capabilities, eligible)| eligible.then_some(server_id.clone()))
            .collect::<HashSet<_>>();

        let redb = RedbCacheManager::global().context("REDB cache is not initialized")?;
        let database = self.database.clone();
        let connection_pool = self.connection_pool.clone();
        let runtime_identity = client_context.runtime_identity();

        let mut tasks = Vec::new();
        for (server_id, server_name, capabilities, _unify_direct_exposure_eligible) in enabled_servers {
            if !visible_server_ids.contains(&server_id) {
                continue;
            }
            if !crate::core::proxy::server::supports_capability(
                capabilities.as_deref(),
                crate::core::capability::CapabilityType::Resources,
            ) {
                continue;
            }

            let ctx = crate::core::capability::runtime::ListCtx {
                capability: crate::core::capability::CapabilityType::Resources,
                server_id: server_id.clone(),
                refresh: Some(crate::core::capability::runtime::RefreshStrategy::CacheFirst),
                timeout: Some(std::time::Duration::from_secs(10)),
                validation_session: None,
                runtime_identity: runtime_identity.clone(),
                connection_selection: client_context.connection_selection(server_id.clone()),
            };
            let redb = redb.clone();
            let database = database.clone();
            let connection_pool = connection_pool.clone();
            let server_name_cloned = server_name.clone();
            tasks.push(async move {
                let result = crate::core::capability::runtime::list(&ctx, &redb, &connection_pool, &database).await;
                (server_id, server_name_cloned, result)
            });
        }

        let mut visible = Vec::new();
        for (server_id, server_name, result) in futures::stream::iter(tasks)
            .buffer_unordered(crate::core::capability::facade::concurrency_limit())
            .collect::<Vec<_>>()
            .await
        {
            if let Ok(result) = result {
                if let Some(resources) = result.items.into_resources() {
                    for mut resource in resources {
                        let raw_resource_uri = resource.uri.to_string();
                        resource.raw.uri = crate::core::capability::naming::generate_unique_name(
                            NamingKind::Resource,
                            &server_name,
                            &raw_resource_uri,
                        );
                        visible.push(VisibleResourceEntry {
                            server_id: server_id.clone(),
                            server_name: server_name.clone(),
                            raw_resource_uri,
                            resource,
                        });
                    }
                }
            }
        }

        let filtered_names = visibility
            .filter_resources_with_snapshot(
                &snapshot,
                visible.iter().map(|entry| entry.resource.clone()).collect(),
                Vec::new(),
            )
            .0
            .into_iter()
            .map(|resource| resource.uri.to_string())
            .collect::<HashSet<_>>();
        visible.retain(|entry| filtered_names.contains(entry.resource.uri.as_str()));
        retain_brokered_resources(context, &eligible_server_ids, &mut visible);
        visible.sort_by(|left, right| {
            left.server_name
                .cmp(&right.server_name)
                .then_with(|| left.resource.uri.as_str().cmp(right.resource.uri.as_str()))
        });

        Ok(visible)
    }

    async fn find_visible_resource(
        &self,
        context: &ClientBuiltinContext,
        resource_name: &str,
    ) -> Result<Option<VisibleResourceEntry>> {
        let resources = self.visible_resources(context).await?;
        Ok(resources
            .into_iter()
            .find(|entry| entry.resource.uri.as_str() == resource_name))
    }

    async fn visible_resource_templates(
        &self,
        context: &ClientBuiltinContext,
    ) -> Result<Vec<VisibleResourceTemplateEntry>> {
        let client_context = context.as_client_context();
        let visibility = ProfileVisibilityService::new(Some(self.database.clone()), None);
        let snapshot = visibility
            .resolve_snapshot_for_client(&client_context)
            .await
            .context("Failed to resolve Unify visibility snapshot")?;
        let visible_server_ids = snapshot.server_ids.iter().cloned().collect::<HashSet<_>>();

        let enabled_servers = self
            .load_enabled_servers("Failed to load enabled servers for Unify resource template catalog")
            .await?;
        let eligible_server_ids = enabled_servers
            .iter()
            .filter_map(|(server_id, _server_name, _capabilities, eligible)| eligible.then_some(server_id.clone()))
            .collect::<HashSet<_>>();

        let redb = RedbCacheManager::global().context("REDB cache is not initialized")?;
        let database = self.database.clone();
        let connection_pool = self.connection_pool.clone();
        let runtime_identity = client_context.runtime_identity();

        let mut tasks = Vec::new();
        for (server_id, server_name, capabilities, _unify_direct_exposure_eligible) in enabled_servers {
            if !visible_server_ids.contains(&server_id) {
                continue;
            }
            if !crate::core::proxy::server::supports_capability(
                capabilities.as_deref(),
                crate::core::capability::CapabilityType::ResourceTemplates,
            ) {
                continue;
            }

            let ctx = crate::core::capability::runtime::ListCtx {
                capability: crate::core::capability::CapabilityType::ResourceTemplates,
                server_id: server_id.clone(),
                refresh: Some(crate::core::capability::runtime::RefreshStrategy::CacheFirst),
                timeout: Some(std::time::Duration::from_secs(10)),
                validation_session: None,
                runtime_identity: runtime_identity.clone(),
                connection_selection: client_context.connection_selection(server_id.clone()),
            };
            let redb = redb.clone();
            let database = database.clone();
            let connection_pool = connection_pool.clone();
            let server_name_cloned = server_name.clone();
            tasks.push(async move {
                let result = crate::core::capability::runtime::list(&ctx, &redb, &connection_pool, &database).await;
                (server_id, server_name_cloned, result)
            });
        }

        let mut visible = Vec::new();
        for (server_id, server_name, result) in futures::stream::iter(tasks)
            .buffer_unordered(crate::core::capability::facade::concurrency_limit())
            .collect::<Vec<_>>()
            .await
        {
            if let Ok(result) = result {
                if let Some(templates) = result.items.into_resource_templates() {
                    for mut resource_template in templates {
                        let raw_uri_template = resource_template.uri_template.to_string();
                        resource_template.raw.name = crate::core::capability::naming::generate_unique_name(
                            NamingKind::ResourceTemplate,
                            &server_name,
                            &raw_uri_template,
                        );
                        visible.push(VisibleResourceTemplateEntry {
                            server_id: server_id.clone(),
                            server_name: server_name.clone(),
                            raw_uri_template,
                            resource_template,
                        });
                    }
                }
            }
        }

        let filtered_names = visibility
            .filter_resources_with_snapshot(
                &snapshot,
                Vec::new(),
                visible.iter().map(|entry| entry.resource_template.clone()).collect(),
            )
            .1
            .into_iter()
            .map(|template| template.name.to_string())
            .collect::<HashSet<_>>();
        visible.retain(|entry| filtered_names.contains(entry.resource_template.name.as_str()));
        retain_brokered_resource_templates(context, &eligible_server_ids, &mut visible);
        visible.sort_by(|left, right| {
            left.server_name.cmp(&right.server_name).then_with(|| {
                left.resource_template
                    .name
                    .as_str()
                    .cmp(right.resource_template.name.as_str())
            })
        });

        Ok(visible)
    }

    async fn find_visible_resource_template(
        &self,
        context: &ClientBuiltinContext,
        template_name: &str,
    ) -> Result<Option<VisibleResourceTemplateEntry>> {
        let templates = self.visible_resource_templates(context).await?;
        Ok(templates
            .into_iter()
            .find(|entry| entry.resource_template.name.as_str() == template_name))
    }

    async fn broker_prompt_call(
        &self,
        context: &ClientBuiltinContext,
        prompt_name: &str,
        arguments: serde_json::Map<String, serde_json::Value>,
    ) -> Result<CallToolResult> {
        if let Some(prompt_entry) = self.find_visible_prompt(context, prompt_name).await? {
            let required_arguments = required_arguments_from_prompt(&prompt_entry.prompt);
            let missing_required: Vec<String> = required_arguments
                .into_iter()
                .filter(|name| !arguments.contains_key(name))
                .collect();
            if !missing_required.is_empty() {
                return Ok(
                    UcanError::missing_required_arguments("prompt", prompt_name, &missing_required)
                        .to_call_tool_result(),
                );
            }
        }

        let client_context = context.as_client_context();
        let visibility = ProfileVisibilityService::new(Some(self.database.clone()), None);
        let snapshot = visibility
            .resolve_snapshot_for_client(&client_context)
            .await
            .context("Failed to resolve Unify visibility snapshot")?;
        if visibility
            .assert_prompt_allowed_with_snapshot(&snapshot, prompt_name)
            .await
            .is_err()
        {
            return Ok(UcanError::visibility_denied("prompt", prompt_name).to_call_tool_result());
        }

        let (server_name, upstream_prompt_name) = resolve_unique_name(NamingKind::Prompt, prompt_name)
            .await
            .with_context(|| format!("Failed to resolve unique prompt '{}'", prompt_name))?;
        let server_id = match crate::core::capability::resolver::to_id(&server_name)
            .await
            .ok()
            .flatten()
        {
            Some(id) => id,
            None => {
                return Ok(UcanError::server_unreachable("unknown", &server_name).to_call_tool_result());
            }
        };

        let prompt_mapping = {
            let filter: HashSet<_> = std::iter::once(server_id.clone()).collect();
            let mapping =
                crate::core::capability::facade::build_prompt_mapping_filtered(&self.connection_pool, Some(&filter))
                    .await;
            if mapping.contains_key(&upstream_prompt_name) {
                mapping
            } else {
                crate::core::capability::facade::build_prompt_mapping(&self.connection_pool).await
            }
        };

        match crate::core::capability::facade::get_upstream_prompt(
            &self.connection_pool,
            &prompt_mapping,
            &upstream_prompt_name,
            Some(arguments),
            Some(server_id.as_str()),
            client_context.connection_selection(server_id.clone()).as_ref(),
        )
        .await
        {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&result).context("Failed to serialize Unify prompt result")?,
            )])),
            Err(e) => Ok(UcanError::upstream_error("prompt", prompt_name, &e.to_string()).to_call_tool_result()),
        }
    }

    async fn broker_resource_read(
        &self,
        context: &ClientBuiltinContext,
        resource_uri: &str,
    ) -> Result<CallToolResult> {
        let client_context = context.as_client_context();
        let visibility = ProfileVisibilityService::new(Some(self.database.clone()), None);
        let snapshot = visibility
            .resolve_snapshot_for_client(&client_context)
            .await
            .context("Failed to resolve Unify visibility snapshot")?;
        if visibility
            .assert_resource_allowed_with_snapshot(&snapshot, resource_uri)
            .await
            .is_err()
        {
            return Ok(UcanError::visibility_denied("resource", resource_uri).to_call_tool_result());
        }

        let (server_name, upstream_resource_uri) = resolve_unique_name(NamingKind::Resource, resource_uri)
            .await
            .with_context(|| format!("Failed to resolve unique resource '{}'", resource_uri))?;
        let server_id = match crate::core::capability::resolver::to_id(&server_name)
            .await
            .ok()
            .flatten()
        {
            Some(id) => id,
            None => {
                return Ok(UcanError::server_unreachable("unknown", &server_name).to_call_tool_result());
            }
        };

        let resource_mapping = {
            let filter: HashSet<_> = std::iter::once(server_id.clone()).collect();
            let mapping = crate::core::capability::facade::build_resource_mapping_filtered(
                &self.connection_pool,
                Some(&self.database),
                Some(&filter),
            )
            .await;
            if mapping.contains_key(&upstream_resource_uri) {
                mapping
            } else {
                crate::core::capability::facade::build_resource_mapping(&self.connection_pool, Some(&self.database))
                    .await
            }
        };

        match crate::core::capability::facade::read_upstream_resource(
            &self.connection_pool,
            &resource_mapping,
            &upstream_resource_uri,
            Some(server_id.as_str()),
            client_context.connection_selection(server_id.clone()).as_ref(),
        )
        .await
        {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&result).context("Failed to serialize Unify resource result")?,
            )])),
            Err(e) => Ok(UcanError::upstream_error("resource", resource_uri, &e.to_string()).to_call_tool_result()),
        }
    }

    async fn acquire_peer(
        &self,
        client_context: &ClientContext,
        server_id: &str,
    ) -> Result<rmcp::service::Peer<rmcp::RoleClient>> {
        let peer_opt = {
            let pool_guard = self.connection_pool.lock().await;
            let snapshot = pool_guard.get_snapshot();
            let mut peer: Option<rmcp::service::Peer<rmcp::RoleClient>> = None;

            if let Some(selection) = client_context.connection_selection(server_id.to_string()) {
                if let Ok(Some(selected_instance_id)) = pool_guard.select_ready_instance_id(&selection) {
                    if let Some(instances) = snapshot.get(server_id) {
                        if let Some((_, _, _, _, selected_peer)) =
                            instances.iter().find(|(candidate_id, _, _, _, peer)| {
                                **candidate_id == selected_instance_id && peer.is_some()
                            })
                        {
                            peer = selected_peer.clone();
                        }
                    }
                }
            }

            if peer.is_none() {
                if let Some(instances) = snapshot.get(server_id) {
                    if let Some((_, _, _, _, selected_peer)) = instances
                        .iter()
                        .find(|(_, status, _, _, peer)| matches!(status, ConnectionStatus::Ready) && peer.is_some())
                    {
                        peer = selected_peer.clone();
                    }
                }
            }

            peer
        };

        if let Some(peer) = peer_opt {
            return Ok(peer);
        }

        {
            let mut pool_guard = self.connection_pool.lock().await;
            if let Some(selection) = client_context.connection_selection(server_id.to_string()) {
                pool_guard
                    .ensure_connected_with_selection(&selection)
                    .await
                    .with_context(|| format!("Failed to connect Unify broker to server '{}'", server_id))?;
            } else {
                pool_guard
                    .ensure_connected(server_id)
                    .await
                    .with_context(|| format!("Failed to connect Unify broker to server '{}'", server_id))?;
            }
        }

        let pool_guard = self.connection_pool.lock().await;
        let snapshot = pool_guard.get_snapshot();
        let instances = snapshot
            .get(server_id)
            .ok_or_else(|| anyhow!("No instance found after connecting to server '{}'", server_id))?;
        let (_, _, _, _, peer) = instances
            .iter()
            .find(|(_, status, _, _, peer)| matches!(status, ConnectionStatus::Ready) && peer.is_some())
            .ok_or_else(|| anyhow!("Ready instance not found for server '{}'", server_id))?;

        peer.clone()
            .ok_or_else(|| anyhow!("Ready peer missing for server '{}'", server_id))
    }
}

#[async_trait::async_trait]
impl BuiltinService for BrokerService {
    fn name(&self) -> &'static str {
        "mcpmate_broker"
    }

    fn tools(&self) -> Vec<Tool> {
        let prompt_config = UCAN_PROMPT_REPO.get_blocking();
        vec![
            Tool::new(
                "mcpmate_ucan_catalog",
                prompt_config.catalog_tool_description.clone(),
                std::sync::Arc::new(
                    serde_json::json!({
                        "type": "object",
                        "properties": {
                            "page": {
                                "type": "integer",
                                "minimum": 1,
                                "description": "Catalog page number. Start with 1."
                            },
                            "page_size": {
                                "type": "integer",
                                "minimum": 1,
                                "maximum": 50,
                                "description": "Number of items per page. Use a small page size to reduce token usage."
                            },
                            "search": {
                                "type": "string",
                                "description": "Case-insensitive substring search in capability_name and summary fields."
                            },
                            "kind_filter": {
                                "type": "array",
                                "items": {
                                    "type": "string",
                                    "enum": ["tool", "prompt", "resource", "resource_template"]
                                },
                                "description": "Filter by capability kind. Returns only matching kinds."
                            }
                        },
                        "required": []
                    })
                    .as_object()
                    .unwrap()
                    .clone(),
                ),
            ),
            Tool::new(
                "mcpmate_ucan_details",
                prompt_config.details_tool_description.clone(),
                std::sync::Arc::new(
                    serde_json::json!({
                        "type": "object",
                        "properties": {
                            "capability_kind": {
                                "type": "string",
                                "enum": ["tool", "prompt", "resource", "resource_template"],
                                "description": "Capability kind returned by mcpmate_ucan_catalog"
                            },
                            "capability_name": {
                                "type": "string",
                                "description": "Capability name returned by mcpmate_ucan_catalog"
                            },
                            "detail_level": {
                                "type": "string",
                                "enum": ["summary", "full"],
                                "description": "Use summary first. Use full only when summary is not enough for safe execution."
                            }
                        },
                        "required": ["capability_kind", "capability_name"]
                    })
                    .as_object()
                    .unwrap()
                    .clone(),
                ),
            ),
            Tool::new(
                "mcpmate_ucan_call",
                prompt_config.call_tool_description.clone(),
                std::sync::Arc::new(
                    serde_json::json!({
                        "type": "object",
                        "properties": {
                            "capability_kind": {
                                "type": "string",
                                "enum": ["tool", "prompt", "resource", "resource_template"],
                                "description": "Capability kind returned by mcpmate_ucan_catalog"
                            },
                            "capability_name": {
                                "type": "string",
                                "description": "Capability name returned by mcpmate_ucan_catalog"
                            },
                            "arguments": {
                                "type": "object",
                                "description": "Arguments for tool/prompt capabilities. Omit or pass {} for resources."
                            }
                        },
                        "required": ["capability_kind", "capability_name"]
                    })
                    .as_object()
                    .unwrap()
                    .clone(),
                ),
            ),
        ]
    }

    async fn call_tool(
        &self,
        request: &CallToolRequestParams,
    ) -> Result<CallToolResult> {
        Ok(UcanError::context_required(&request.name).to_call_tool_result())
    }

    async fn call_tool_with_context(
        &self,
        request: &CallToolRequestParams,
        context: Option<&ClientBuiltinContext>,
    ) -> Result<CallToolResult> {
        let context = match context {
            Some(ctx) => ctx,
            None => {
                return Ok(UcanError::context_required(&request.name).to_call_tool_result());
            }
        };

        match request.name.as_ref() {
            "mcpmate_ucan_catalog" => {
                let args = serde_json::Value::Object(request.arguments.clone().unwrap_or_default());
                match serde_json::from_value::<CatalogParams>(args) {
                    Ok(params) => {
                        self.tool_catalog(
                            context,
                            params.page,
                            params.page_size,
                            params.search.as_deref(),
                            params.kind_filter.as_deref(),
                        )
                        .await
                    }
                    Err(e) => {
                        Ok(UcanError::invalid_parameters("mcpmate_ucan_catalog", &e.to_string()).to_call_tool_result())
                    }
                }
            }
            "mcpmate_ucan_details" => {
                let args = serde_json::Value::Object(request.arguments.clone().unwrap_or_default());
                match serde_json::from_value::<CapabilityLookupParams>(args) {
                    Ok(params) => {
                        self.tool_details(
                            context,
                            params.capability_kind,
                            &params.capability_name,
                            params.detail_level,
                        )
                        .await
                    }
                    Err(e) => {
                        Ok(UcanError::invalid_parameters("mcpmate_ucan_details", &e.to_string()).to_call_tool_result())
                    }
                }
            }
            "mcpmate_ucan_call" => {
                let args = serde_json::Value::Object(request.arguments.clone().unwrap_or_default());
                match serde_json::from_value::<BrokerCapabilityCallParams>(args) {
                    Ok(params) => {
                        self.broker_tool_call(
                            context,
                            params.capability_kind,
                            &params.capability_name,
                            params.arguments,
                        )
                        .await
                    }
                    Err(e) => {
                        Ok(UcanError::invalid_parameters("mcpmate_ucan_call", &e.to_string()).to_call_tool_result())
                    }
                }
            }
            _ => Ok(UcanError::unknown_tool(&request.name).to_call_tool_result()),
        }
    }
}

impl ClientBuiltinContext {
    pub(crate) fn as_client_context(&self) -> ClientContext {
        ClientContext {
            client_id: self.client_id.clone(),
            session_id: self.session_id.clone(),
            profile_id: None,
            config_mode: self.config_mode.clone(),
            unify_workspace: self.unify_workspace.clone(),
            rules_fingerprint: None,
            transport: ClientTransport::Other,
            source: ClientIdentitySource::SessionBinding,
            observed_client_info: None,
        }
    }
}

fn default_catalog_page() -> usize {
    1
}

fn default_catalog_page_size() -> usize {
    default_ucan_prompt_config().catalog_page_size_default
}

fn default_ucan_prompt_config() -> UcanPromptConfig {
    UcanPromptConfig {
        catalog_tool_description: "MCPMATE_UCAN_CATALOG\nROLE: Unified capability entry for MCPMate.\nUSE_WHEN: Before starting any task, call this first to find the most relevant capability.\nRETURNS: A paginated capability catalog with lightweight summaries.\nWORKFLOW: catalog -> details -> call.\nRULES: Use the current page first. If you still have not found a good match, request the next page instead of expanding everything at once.".to_string(),
        details_tool_description: "MCPMATE_UCAN_DETAILS\nROLE: Explain how to use one capability selected from MCPMate's catalog.\nUSE_WHEN: After catalog, before call.\nRETURNS: Summary or full details for the selected capability.\nWORKFLOW: Use summary first for quick judgment. Use full only when you need complete metadata.\nRULES: Do not inspect unrelated capabilities in full.".to_string(),
        call_tool_description: "MCPMATE_UCAN_CALL\nROLE: Execute one capability selected from MCPMate's catalog.\nUSE_WHEN: Only after you already know which capability to use.\nRETURNS: The execution result produced by the selected capability.\nWORKFLOW: catalog -> details -> call.\nRULES: Call only the capability you intentionally selected. Use details first when arguments or behavior are unclear.".to_string(),
        catalog_usage: "Before starting any task, call mcpmate_ucan_catalog first. Pick the most relevant capability from the current page. If the current page is not enough, request the next page instead of expanding everything at once. Then use mcpmate_ucan_details to inspect the selected capability, and use mcpmate_ucan_call only after you understand how to use it.".to_string(),
        catalog_stale_hint: default_catalog_stale_hint(),
        error_recovery_hint: default_error_recovery_hint(),
        catalog_format: vec![
            "capability_name".to_string(),
            "capability_kind".to_string(),
            "summary".to_string(),
            "action".to_string(),
            "next_step".to_string(),
            "server_id".to_string(),
            "server_name".to_string(),
            "interaction_mode".to_string(),
            "detail_hint".to_string(),
            "registry_enriched".to_string(),
            "registry_category".to_string(),
        ],
        catalog_page_size_default: 20,
        catalog_page_size_max: 50,
        catalog_sort_weights: CatalogSortWeights::default(),
        workflow_hints: WorkflowHints::default(),
        catalog_enrich_from_registry: true,
    }
}

fn resolve_ucan_prompt_config_path() -> Result<PathBuf> {
    let path_service = PathService::new().context("Create PathService for UCAN prompt config")?;
    let path_hint = std::env::var("MCPMATE_UCAN_CONFIG")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| format!("{}/config/ucan.json5", env!("CARGO_MANIFEST_DIR")));
    path_service
        .resolve_user_path(&path_hint)
        .context("Resolve UCAN prompt config path")
}

fn load_ucan_prompt_config_blocking() -> Result<UcanPromptConfig> {
    let path = resolve_ucan_prompt_config_path()?;
    let content =
        std::fs::read_to_string(&path).with_context(|| format!("Read UCAN prompt config from {}", path.display()))?;
    let value: serde_json::Value =
        json5::from_str(&content).with_context(|| format!("Parse UCAN prompt config from {}", path.display()))?;
    if !value.is_object() {
        return Err(anyhow!(
            "UCAN prompt config at {} must be a JSON5 object",
            path.display()
        ));
    }
    let config: UcanPromptConfig =
        serde_json::from_value(value).with_context(|| format!("Decode UCAN prompt config from {}", path.display()))?;
    Ok(normalize_ucan_prompt_config(config))
}

fn normalize_multiline_text(raw: &str) -> String {
    let normalized = raw.replace("\r\n", "\n").replace('\r', "\n");
    let lines: Vec<String> = normalized.lines().map(|line| line.trim_end().to_string()).collect();
    lines.join("\n").trim().to_string()
}

fn normalize_string_list(items: Vec<String>) -> Vec<String> {
    items
        .into_iter()
        .map(|item| normalize_multiline_text(&item))
        .filter(|item| !item.is_empty())
        .collect()
}

fn normalize_ucan_prompt_config(mut config: UcanPromptConfig) -> UcanPromptConfig {
    config.catalog_tool_description = normalize_multiline_text(&config.catalog_tool_description);
    config.details_tool_description = normalize_multiline_text(&config.details_tool_description);
    config.call_tool_description = normalize_multiline_text(&config.call_tool_description);
    config.catalog_usage = normalize_multiline_text(&config.catalog_usage);
    config.catalog_stale_hint = normalize_multiline_text(&config.catalog_stale_hint);
    config.error_recovery_hint = normalize_multiline_text(&config.error_recovery_hint);
    config.catalog_format = normalize_string_list(config.catalog_format);
    config.workflow_hints.normalize();
    config
}

fn compact_description(description: Option<&str>) -> Option<String> {
    let description = description?.trim();
    if description.is_empty() {
        return None;
    }

    let first_line = description
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())?
        .to_string();

    Some(first_line)
}

fn extract_description_from_value<T: Serialize>(value: &T) -> Option<String> {
    let json = serde_json::to_value(value).ok()?;
    let object = json.as_object()?;

    ["description", "title", "name", "uri_template", "uri"]
        .into_iter()
        .find_map(|key| object.get(key).and_then(|value| value.as_str()).map(ToOwned::to_owned))
}

fn tool_details_value(
    tool: &Tool,
    detail_level: UcanDetailLevel,
) -> serde_json::Value {
    match detail_level {
        UcanDetailLevel::Summary => {
            let schema = tool.schema_as_json_value();
            let properties = schema
                .get("properties")
                .and_then(|value| value.as_object())
                .map(|properties| properties.keys().cloned().collect::<Vec<_>>())
                .unwrap_or_default();
            let required = schema
                .get("required")
                .and_then(|value| value.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str().map(ToOwned::to_owned))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            serde_json::json!({
                "description": tool.description.clone().map(|d| d.into_owned()),
                "input_fields": properties,
                "required_fields": required,
                "has_output_schema": tool.output_schema.is_some(),
            })
        }
        UcanDetailLevel::Full => serde_json::json!({
            "description": tool.description.clone().map(|d| d.into_owned()),
            "input_schema": tool.schema_as_json_value(),
            "output_schema": tool
                .output_schema
                .as_ref()
                .map(|schema| serde_json::Value::Object((**schema).clone())),
        }),
    }
}

fn prompt_details_value(
    prompt: &rmcp::model::Prompt,
    detail_level: UcanDetailLevel,
) -> Result<serde_json::Value> {
    match detail_level {
        UcanDetailLevel::Summary => Ok(serde_json::json!({
            "description": extract_description_from_value(prompt),
            "argument_names": prompt
                .arguments
                .as_ref()
                .map(|arguments| arguments.iter().map(|arg| arg.name.clone()).collect::<Vec<_>>())
                .unwrap_or_default(),
            "required_arguments": prompt
                .arguments
                .as_ref()
                .map(|arguments| {
                    arguments
                        .iter()
                        .filter(|arg| arg.required.unwrap_or(false))
                        .map(|arg| arg.name.clone())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
        })),
        UcanDetailLevel::Full => serde_json::to_value(prompt).context("Serialize prompt detail"),
    }
}

fn resource_details_value(
    resource: &Resource,
    detail_level: UcanDetailLevel,
) -> Result<serde_json::Value> {
    match detail_level {
        UcanDetailLevel::Summary => Ok(serde_json::json!({
            "description": extract_description_from_value(resource),
            "mime_type": resource.mime_type,
            "annotations": resource.annotations,
        })),
        UcanDetailLevel::Full => serde_json::to_value(resource).context("Serialize resource detail"),
    }
}

fn resource_template_details_value(
    resource_template: &ResourceTemplate,
    detail_level: UcanDetailLevel,
) -> Result<serde_json::Value> {
    match detail_level {
        UcanDetailLevel::Summary => Ok(serde_json::json!({
            "description": extract_description_from_value(resource_template),
            "uri_template": resource_template.uri_template,
            "usage": "Inspect URI construction rules from this template. Template-derived URIs are not directly invocable unless listed in catalog as concrete resources.",
        })),
        UcanDetailLevel::Full => Ok(serde_json::json!({
            "template": serde_json::to_value(resource_template).context("Serialize resource template detail")?,
            "usage": "Use this template as URI construction guidance. Broker calls require concrete resources that are present in catalog."
        })),
    }
}

fn extract_argument_tips_from_tool(tool: &Tool) -> Vec<ArgumentTip> {
    let schema = tool.schema_as_json_value();
    let properties = match schema.get("properties").and_then(|v| v.as_object()) {
        Some(props) => props,
        None => return Vec::new(),
    };

    let required: std::collections::HashSet<&str> = schema
        .get("required")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|item| item.as_str()).collect())
        .unwrap_or_default();

    properties
        .iter()
        .map(|(name, prop)| {
            let type_hint = prop.get("type").and_then(|v| v.as_str()).map(ToOwned::to_owned);
            let description = prop.get("description").and_then(|v| v.as_str()).map(ToOwned::to_owned);
            ArgumentTip {
                name: name.clone(),
                required: required.contains(name.as_str()),
                type_hint,
                description,
            }
        })
        .collect()
}

fn required_arguments_from_tool(tool: &Tool) -> Vec<String> {
    let schema = tool.schema_as_json_value();
    schema
        .get("required")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| item.as_str().map(ToOwned::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

fn extract_argument_tips_from_prompt(prompt: &rmcp::model::Prompt) -> Vec<ArgumentTip> {
    prompt
        .arguments
        .as_ref()
        .map(|args| {
            args.iter()
                .map(|arg| ArgumentTip {
                    name: arg.name.clone(),
                    required: arg.required.unwrap_or(false),
                    type_hint: None,
                    description: arg.description.clone(),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn required_arguments_from_prompt(prompt: &rmcp::model::Prompt) -> Vec<String> {
    prompt
        .arguments
        .as_ref()
        .map(|args| {
            args.iter()
                .filter(|arg| arg.required.unwrap_or(false))
                .map(|arg| arg.name.clone())
                .collect()
        })
        .unwrap_or_default()
}

fn call_requirements_for_tool(tool: &Tool) -> CallRequirements {
    let required_arguments = required_arguments_from_tool(tool);
    CallRequirements {
        accepts_arguments: true,
        call_ready_without_arguments: required_arguments.is_empty(),
        required_arguments,
    }
}

fn call_requirements_for_prompt(prompt: &rmcp::model::Prompt) -> CallRequirements {
    let required_arguments = required_arguments_from_prompt(prompt);
    CallRequirements {
        accepts_arguments: true,
        call_ready_without_arguments: required_arguments.is_empty(),
        required_arguments,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ClientBuiltinContext, UcanDetailLevel, UcanError, UcanPromptRepository, VisiblePromptEntry,
        VisibleResourceEntry, VisibleResourceTemplateEntry, VisibleToolEntry, capitalize_kind, compact_description,
        extract_description_from_value, find_similar_names, levenshtein_distance, retain_brokered_prompts,
        retain_brokered_resource_templates, retain_brokered_resources, retain_brokered_tools, tool_details_value,
    };
    use crate::clients::models::{
        CapabilitySource, UnifyDirectExposureConfig, UnifyDirectPromptSurface, UnifyDirectResourceSurface,
        UnifyDirectToolSurface, UnifyRouteMode,
    };
    use rmcp::model::{Prompt, Resource, ResourceTemplate, Tool};
    use std::collections::HashSet;
    use std::sync::{Arc, Mutex as StdMutex};
    use std::time::Duration;
    use tempfile::tempdir;

    static ENV_LOCK: StdMutex<()> = StdMutex::new(());

    #[test]
    fn compact_description_keeps_first_non_empty_line() {
        let value = compact_description(Some("\n\nFirst line.\nSecond line."));
        assert_eq!(value.as_deref(), Some("First line."));
    }

    #[test]
    fn compact_description_keeps_long_lines() {
        let source = format!("{} extra", "a".repeat(200));
        let value = compact_description(Some(&source)).expect("summary");
        assert_eq!(value, source);
    }

    #[test]
    fn extract_description_from_value_prefers_description_field() {
        let value = serde_json::json!({"description": "hello", "name": "fallback"});
        assert_eq!(extract_description_from_value(&value).as_deref(), Some("hello"));
    }

    #[test]
    fn tool_details_summary_is_lighter_than_full() {
        let tool = Tool::new(
            "demo",
            "Demo tool",
            Arc::new(
                serde_json::json!({
                    "type": "object",
                    "properties": {"path": {"type": "string"}, "force": {"type": "boolean"}},
                    "required": ["path"]
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
        );
        let summary = tool_details_value(&tool, UcanDetailLevel::Summary);
        let full = tool_details_value(&tool, UcanDetailLevel::Full);
        assert!(summary.get("input_fields").is_some());
        assert!(summary.get("input_schema").is_none());
        assert!(full.get("input_schema").is_some());
    }

    #[test]
    fn levenshtein_distance_basic_cases() {
        assert_eq!(levenshtein_distance("", ""), 0);
        assert_eq!(levenshtein_distance("a", ""), 1);
        assert_eq!(levenshtein_distance("", "a"), 1);
        assert_eq!(levenshtein_distance("abc", "abc"), 0);
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
        assert_eq!(levenshtein_distance("saturday", "sunday"), 3);
    }

    #[test]
    fn levenshtein_distance_is_case_sensitive() {
        assert_eq!(levenshtein_distance("Tool", "tool"), 1);
        assert_eq!(levenshtein_distance("TOOL", "tool"), 4);
    }

    #[test]
    fn find_similar_names_is_case_insensitive() {
        let candidates = vec!["Tool".to_string(), "TOOL".to_string()];
        let result = find_similar_names("tool", &candidates, 2);
        assert!(!result.is_empty());
    }

    #[test]
    fn find_similar_names_returns_closest_matches() {
        let candidates = vec![
            "mcpmate_ucan_catalog".to_string(),
            "mcpmate_ucan_details".to_string(),
            "mcpmate_ucan_call".to_string(),
            "other_tool".to_string(),
        ];
        let result = find_similar_names("mcpmate_ucan_catlog", &candidates, 3);
        assert_eq!(result.len(), 3);
        assert!(result.contains(&"mcpmate_ucan_catalog".to_string()));
    }

    #[test]
    fn find_similar_names_respects_limit() {
        let candidates = vec![
            "tool_a".to_string(),
            "tool_b".to_string(),
            "tool_c".to_string(),
            "tool_d".to_string(),
            "tool_e".to_string(),
        ];
        let result = find_similar_names("tool_x", &candidates, 2);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn find_similar_names_handles_empty_candidates() {
        let candidates: Vec<String> = vec![];
        let result = find_similar_names("any_tool", &candidates, 3);
        assert!(result.is_empty());
    }

    #[test]
    fn test_error_capability_not_found_includes_alternatives() {
        let catalog_names = vec![
            "mcpmate_ucan_catalog".to_string(),
            "mcpmate_ucan_details".to_string(),
            "mcpmate_ucan_call".to_string(),
        ];
        let error = UcanError::capability_not_found("tool", "mcpmate_ucan_catlog", &catalog_names);

        assert_eq!(error.error_code, "capability_not_found");
        assert!(error.message.contains("Tool"));
        assert!(error.message.contains("mcpmate_ucan_catlog"));
        assert!(!error.alternatives.is_empty());
        assert!(error.alternatives.contains(&"mcpmate_ucan_catalog".to_string()));
        assert!(!error.retry_eligible);
        assert!(error.recovery_hint.contains("alternatives"));
    }

    #[test]
    fn test_error_capability_not_found_empty_catalog() {
        let catalog_names: Vec<String> = vec![];
        let error = UcanError::capability_not_found("prompt", "unknown_prompt", &catalog_names);

        assert_eq!(error.error_code, "capability_not_found");
        assert!(error.alternatives.is_empty());
        assert!(error.recovery_hint.contains("mcpmate_ucan_catalog"));
    }

    #[test]
    fn test_error_server_unreachable_includes_retry_hint() {
        let error = UcanError::server_unreachable("server-123", "my-server");

        assert_eq!(error.error_code, "server_unreachable");
        assert!(error.message.contains("my-server"));
        assert!(error.message.contains("server-123"));
        assert!(error.retry_eligible);
        assert!(error.recovery_hint.contains("running"));
        assert!(error.alternatives.is_empty());
    }

    #[test]
    fn test_error_visibility_denied() {
        let error = UcanError::visibility_denied("tool", "sensitive_tool");

        assert_eq!(error.error_code, "visibility_denied");
        assert!(error.message.contains("not available"));
        assert!(!error.retry_eligible);
        assert!(error.recovery_hint.contains("mcpmate_ucan_catalog"));
    }

    #[test]
    fn test_error_missing_required_arguments() {
        let missing = vec!["path".to_string(), "mode".to_string()];
        let error = UcanError::missing_required_arguments("tool", "read_file", &missing);

        assert_eq!(error.error_code, "missing_required_arguments");
        assert!(error.message.contains("read_file"));
        assert!(error.message.contains("path"));
        assert!(error.retry_eligible);
        assert!(error.recovery_hint.contains("detail_level=full"));
    }

    #[test]
    fn test_error_resource_arguments_not_supported() {
        let error = UcanError::resource_arguments_not_supported("filesystem://root");

        assert_eq!(error.error_code, "resource_arguments_not_supported");
        assert!(error.message.contains("does not accept call arguments"));
        assert!(error.retry_eligible);
    }

    #[test]
    fn test_error_upstream_error() {
        let error = UcanError::upstream_error("tool", "my_tool", "connection refused");

        assert_eq!(error.error_code, "upstream_error");
        assert!(error.message.contains("connection refused"));
        assert!(!error.retry_eligible);
        assert!(error.recovery_hint.contains("upstream"));
    }

    #[test]
    fn test_error_timeout() {
        let error = UcanError::timeout("tool", "slow_tool", 60);

        assert_eq!(error.error_code, "timeout");
        assert!(error.message.contains("60 seconds"));
        assert!(error.retry_eligible);
        assert!(error.recovery_hint.contains("MCPMATE_TOOL_CALL_TIMEOUT_SECS"));
    }

    #[test]
    fn test_error_resource_template_not_invocable() {
        let error = UcanError::resource_template_not_invocable("file:///{path}");

        assert_eq!(error.error_code, "resource_template_not_invocable");
        assert!(error.message.contains("file:///{path}"));
        assert!(!error.retry_eligible);
        assert!(error.recovery_hint.contains("not directly invocable"));
    }

    #[test]
    fn test_error_context_required() {
        let error = UcanError::context_required("mcpmate_ucan_catalog");

        assert_eq!(error.error_code, "context_required");
        assert!(error.message.contains("mcpmate_ucan_catalog"));
        assert!(!error.retry_eligible);
    }

    #[test]
    fn test_error_unknown_tool() {
        let error = UcanError::unknown_tool("invalid_tool");

        assert_eq!(error.error_code, "unknown_tool");
        assert!(error.message.contains("invalid_tool"));
        assert!(!error.retry_eligible);
        assert!(error.alternatives.contains(&"mcpmate_ucan_catalog".to_string()));
        assert!(error.alternatives.contains(&"mcpmate_ucan_details".to_string()));
        assert!(error.alternatives.contains(&"mcpmate_ucan_call".to_string()));
    }

    #[test]
    fn test_error_invalid_parameters() {
        let error = UcanError::invalid_parameters("mcpmate_ucan_call", "missing capability_kind");

        assert_eq!(error.error_code, "invalid_parameters");
        assert!(error.message.contains("missing capability_kind"));
        assert!(!error.retry_eligible);
    }

    #[test]
    fn test_error_response_is_valid_json() {
        let error = UcanError::capability_not_found("tool", "test_tool", &["alt1".to_string(), "alt2".to_string()]);
        let json = error.to_json();

        let parsed: serde_json::Value = serde_json::from_str(&json).expect("JSON should be valid");
        assert_eq!(parsed["error_code"], "capability_not_found");
        assert!(parsed["message"].is_string());
        assert!(parsed["recovery_hint"].is_string());
        assert!(parsed["alternatives"].is_array());
        assert!(parsed["retry_eligible"].is_boolean());
    }

    #[test]
    fn test_error_to_call_tool_result() {
        let error = UcanError::server_unreachable("server-1", "test-server");
        let result = error.to_call_tool_result();

        assert_eq!(result.is_error, Some(true));
        assert!(!result.content.is_empty());
    }

    #[test]
    fn capitalize_kind_returns_correct_strings() {
        assert_eq!(capitalize_kind("tool"), "Tool");
        assert_eq!(capitalize_kind("prompt"), "Prompt");
        assert_eq!(capitalize_kind("resource"), "Resource");
        assert_eq!(capitalize_kind("resource_template"), "Resource template");
        assert_eq!(capitalize_kind("unknown"), "Capability");
    }

    #[test]
    fn unify_direct_exposure_catalog_exclusion_keeps_broker_only_tools() {
        let context = ClientBuiltinContext {
            client_id: "client-1".to_string(),
            session_id: Some("session-1".to_string()),
            config_mode: Some("unify".to_string()),
            capability_source: CapabilitySource::Profiles,
            selected_profile_ids: Vec::new(),
            custom_profile_id: None,
            unify_workspace: Some(UnifyDirectExposureConfig {
                route_mode: UnifyRouteMode::BrokerOnly,
                selected_server_ids: vec!["server-a".to_string()],
                selected_tool_surfaces: vec![UnifyDirectToolSurface {
                    server_id: "server-a".to_string(),
                    tool_name: "tool-one".to_string(),
                }],
                selected_prompt_surfaces: Vec::new(),
                selected_resource_surfaces: Vec::new(),
                selected_template_surfaces: Vec::new(),
            }),
        };
        let mut visible = vec![VisibleToolEntry {
            server_id: "server-a".to_string(),
            server_name: "Server A".to_string(),
            raw_tool_name: "tool-one".to_string(),
            tool: Tool::new("server-a__tool-one", "demo", Arc::new(serde_json::Map::new())),
        }];

        retain_brokered_tools(&context, &HashSet::from(["server-a".to_string()]), &mut visible);

        assert_eq!(visible.len(), 1);
    }

    #[test]
    fn unify_direct_exposure_catalog_exclusion_removes_server_live_tools() {
        let context = ClientBuiltinContext {
            client_id: "client-1".to_string(),
            session_id: Some("session-1".to_string()),
            config_mode: Some("unify".to_string()),
            capability_source: CapabilitySource::Profiles,
            selected_profile_ids: Vec::new(),
            custom_profile_id: None,
            unify_workspace: Some(UnifyDirectExposureConfig {
                route_mode: UnifyRouteMode::ServerLive,
                selected_server_ids: vec!["server-a".to_string()],
                selected_tool_surfaces: Vec::new(),
                selected_prompt_surfaces: Vec::new(),
                selected_resource_surfaces: Vec::new(),
                selected_template_surfaces: Vec::new(),
            }),
        };
        let mut visible = vec![
            VisibleToolEntry {
                server_id: "server-a".to_string(),
                server_name: "Server A".to_string(),
                raw_tool_name: "tool-one".to_string(),
                tool: Tool::new("server-a__tool-one", "demo", Arc::new(serde_json::Map::new())),
            },
            VisibleToolEntry {
                server_id: "server-b".to_string(),
                server_name: "Server B".to_string(),
                raw_tool_name: "tool-two".to_string(),
                tool: Tool::new("server-b__tool-two", "demo", Arc::new(serde_json::Map::new())),
            },
        ];

        retain_brokered_tools(&context, &HashSet::from(["server-a".to_string()]), &mut visible);

        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].tool.name.as_ref(), "server-b__tool-two");
    }

    #[test]
    fn unify_direct_exposure_catalog_exclusion_removes_only_selected_capability_level_tools() {
        let context = ClientBuiltinContext {
            client_id: "client-1".to_string(),
            session_id: Some("session-1".to_string()),
            config_mode: Some("unify".to_string()),
            capability_source: CapabilitySource::Profiles,
            selected_profile_ids: Vec::new(),
            custom_profile_id: None,
            unify_workspace: Some(UnifyDirectExposureConfig {
                route_mode: UnifyRouteMode::CapabilityLevel,
                selected_server_ids: vec!["server-a".to_string()],
                selected_tool_surfaces: vec![UnifyDirectToolSurface {
                    server_id: "server-a".to_string(),
                    tool_name: "tool-one".to_string(),
                }],
                selected_prompt_surfaces: Vec::new(),
                selected_resource_surfaces: Vec::new(),
                selected_template_surfaces: Vec::new(),
            }),
        };
        let mut visible = vec![
            VisibleToolEntry {
                server_id: "server-a".to_string(),
                server_name: "Server A".to_string(),
                raw_tool_name: "tool-one".to_string(),
                tool: Tool::new("server-a__tool-one", "demo", Arc::new(serde_json::Map::new())),
            },
            VisibleToolEntry {
                server_id: "server-a".to_string(),
                server_name: "Server A".to_string(),
                raw_tool_name: "tool-two".to_string(),
                tool: Tool::new("server-a__tool-two", "demo", Arc::new(serde_json::Map::new())),
            },
        ];

        retain_brokered_tools(&context, &HashSet::from(["server-a".to_string()]), &mut visible);

        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].tool.name.as_ref(), "server-a__tool-two");
    }


    #[test]
    fn unify_direct_exposure_catalog_exclusion_removes_server_live_prompts() {
        let context = ClientBuiltinContext {
            client_id: "client-1".to_string(),
            session_id: Some("session-1".to_string()),
            config_mode: Some("unify".to_string()),
            capability_source: CapabilitySource::Profiles,
            selected_profile_ids: Vec::new(),
            custom_profile_id: None,
            unify_workspace: Some(UnifyDirectExposureConfig {
                route_mode: UnifyRouteMode::ServerLive,
                selected_server_ids: vec!["server-a".to_string()],
                selected_tool_surfaces: Vec::new(),
                selected_prompt_surfaces: Vec::new(),
                selected_resource_surfaces: Vec::new(),
                selected_template_surfaces: Vec::new(),
            }),
        };
        let mut visible = vec![
            VisiblePromptEntry {
                server_id: "server-a".to_string(),
                server_name: "Server A".to_string(),
                raw_prompt_name: "prompt-one".to_string(),
                prompt: Prompt::new(
                    "server-a__prompt-one",
                    Some("demo".to_string()),
                    None::<Vec<rmcp::model::PromptArgument>>,
                ),
            },
            VisiblePromptEntry {
                server_id: "server-b".to_string(),
                server_name: "Server B".to_string(),
                raw_prompt_name: "prompt-two".to_string(),
                prompt: Prompt::new(
                    "server-b__prompt-two",
                    Some("demo".to_string()),
                    None::<Vec<rmcp::model::PromptArgument>>,
                ),
            },
        ];

        retain_brokered_prompts(&context, &HashSet::from(["server-a".to_string()]), &mut visible);

        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].prompt.name.as_str(), "server-b__prompt-two");
    }

    #[test]
    fn unify_direct_exposure_catalog_exclusion_removes_only_selected_capability_level_prompts() {
        let context = ClientBuiltinContext {
            client_id: "client-1".to_string(),
            session_id: Some("session-1".to_string()),
            config_mode: Some("unify".to_string()),
            capability_source: CapabilitySource::Profiles,
            selected_profile_ids: Vec::new(),
            custom_profile_id: None,
            unify_workspace: Some(UnifyDirectExposureConfig {
                route_mode: UnifyRouteMode::CapabilityLevel,
                selected_server_ids: vec!["server-a".to_string()],
                selected_tool_surfaces: Vec::new(),
                selected_prompt_surfaces: vec![UnifyDirectPromptSurface {
                    server_id: "server-a".to_string(),
                    prompt_name: "prompt-one".to_string(),
                }],
                selected_resource_surfaces: Vec::new(),
                selected_template_surfaces: Vec::new(),
            }),
        };
        let mut visible = vec![
            VisiblePromptEntry {
                server_id: "server-a".to_string(),
                server_name: "Server A".to_string(),
                raw_prompt_name: "prompt-one".to_string(),
                prompt: Prompt::new(
                    "server-a__prompt-one",
                    Some("demo".to_string()),
                    None::<Vec<rmcp::model::PromptArgument>>,
                ),
            },
            VisiblePromptEntry {
                server_id: "server-a".to_string(),
                server_name: "Server A".to_string(),
                raw_prompt_name: "prompt-two".to_string(),
                prompt: Prompt::new(
                    "server-a__prompt-two",
                    Some("demo".to_string()),
                    None::<Vec<rmcp::model::PromptArgument>>,
                ),
            },
        ];

        retain_brokered_prompts(&context, &HashSet::from(["server-a".to_string()]), &mut visible);

        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].raw_prompt_name, "prompt-two");
    }


    #[test]
    fn unify_direct_exposure_catalog_exclusion_removes_server_live_resources() {
        let context = ClientBuiltinContext {
            client_id: "client-1".to_string(),
            session_id: Some("session-1".to_string()),
            config_mode: Some("unify".to_string()),
            capability_source: CapabilitySource::Profiles,
            selected_profile_ids: Vec::new(),
            custom_profile_id: None,
            unify_workspace: Some(UnifyDirectExposureConfig {
                route_mode: UnifyRouteMode::ServerLive,
                selected_server_ids: vec!["server-a".to_string()],
                selected_tool_surfaces: Vec::new(),
                selected_prompt_surfaces: Vec::new(),
                selected_resource_surfaces: Vec::new(),
                selected_template_surfaces: Vec::new(),
            }),
        };
        let mut visible = vec![
            VisibleResourceEntry {
                server_id: "server-a".to_string(),
                server_name: "Server A".to_string(),
                raw_resource_uri: "resource-one".to_string(),
                resource: Resource {
                    raw: rmcp::model::RawResource::new("server-a://resource-one", "server-a://resource-one"),
                    annotations: None,
                },
            },
            VisibleResourceEntry {
                server_id: "server-b".to_string(),
                server_name: "Server B".to_string(),
                raw_resource_uri: "resource-two".to_string(),
                resource: Resource {
                    raw: rmcp::model::RawResource::new("server-b://resource-two", "server-b://resource-two"),
                    annotations: None,
                },
            },
        ];

        retain_brokered_resources(&context, &HashSet::from(["server-a".to_string()]), &mut visible);

        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].resource.uri.as_str(), "server-b://resource-two");
    }

    #[test]
    fn unify_direct_exposure_catalog_exclusion_removes_only_selected_capability_level_resources() {
        let context = ClientBuiltinContext {
            client_id: "client-1".to_string(),
            session_id: Some("session-1".to_string()),
            config_mode: Some("unify".to_string()),
            capability_source: CapabilitySource::Profiles,
            selected_profile_ids: Vec::new(),
            custom_profile_id: None,
            unify_workspace: Some(UnifyDirectExposureConfig {
                route_mode: UnifyRouteMode::CapabilityLevel,
                selected_server_ids: vec!["server-a".to_string()],
                selected_tool_surfaces: Vec::new(),
                selected_prompt_surfaces: Vec::new(),
                selected_resource_surfaces: vec![UnifyDirectResourceSurface {
                    server_id: "server-a".to_string(),
                    resource_uri: "resource-one".to_string(),
                }],
                selected_template_surfaces: Vec::new(),
            }),
        };
        let mut visible = vec![
            VisibleResourceEntry {
                server_id: "server-a".to_string(),
                server_name: "Server A".to_string(),
                raw_resource_uri: "resource-one".to_string(),
                resource: Resource {
                    raw: rmcp::model::RawResource::new("server-a://resource-one", "server-a://resource-one"),
                    annotations: None,
                },
            },
            VisibleResourceEntry {
                server_id: "server-a".to_string(),
                server_name: "Server A".to_string(),
                raw_resource_uri: "resource-two".to_string(),
                resource: Resource {
                    raw: rmcp::model::RawResource::new("server-a://resource-two", "server-a://resource-two"),
                    annotations: None,
                },
            },
        ];

        retain_brokered_resources(&context, &HashSet::from(["server-a".to_string()]), &mut visible);

        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].raw_resource_uri, "resource-two");
    }

    #[test]
    fn unify_direct_exposure_catalog_exclusion_removes_server_live_resource_templates() {
        let context = ClientBuiltinContext {
            client_id: "client-1".to_string(),
            session_id: Some("session-1".to_string()),
            config_mode: Some("unify".to_string()),
            capability_source: CapabilitySource::Profiles,
            selected_profile_ids: Vec::new(),
            custom_profile_id: None,
            unify_workspace: Some(UnifyDirectExposureConfig {
                route_mode: UnifyRouteMode::ServerLive,
                selected_server_ids: vec!["server-a".to_string()],
                selected_tool_surfaces: Vec::new(),
                selected_prompt_surfaces: Vec::new(),
                selected_resource_surfaces: Vec::new(),
                selected_template_surfaces: Vec::new(),
            }),
        };
        let mut visible = vec![
            VisibleResourceTemplateEntry {
                server_id: "server-a".to_string(),
                server_name: "Server A".to_string(),
                raw_uri_template: "server-a://{id}".to_string(),
                resource_template: ResourceTemplate {
                    raw: rmcp::model::RawResourceTemplate::new("server-a://{id}", "server-a://{id}"),
                    annotations: None,
                },
            },
            VisibleResourceTemplateEntry {
                server_id: "server-b".to_string(),
                server_name: "Server B".to_string(),
                raw_uri_template: "server-b://{id}".to_string(),
                resource_template: ResourceTemplate {
                    raw: rmcp::model::RawResourceTemplate::new("server-b://{id}", "server-b://{id}"),
                    annotations: None,
                },
            },
        ];

        retain_brokered_resource_templates(&context, &HashSet::from(["server-a".to_string()]), &mut visible);

        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].resource_template.uri_template.as_str(), "server-b://{id}");
    }

    #[test]
    fn unify_direct_exposure_catalog_exclusion_removes_only_selected_capability_level_resource_templates() {
        let context = ClientBuiltinContext {
            client_id: "client-1".to_string(),
            session_id: Some("session-1".to_string()),
            config_mode: Some("unify".to_string()),
            capability_source: CapabilitySource::Profiles,
            selected_profile_ids: Vec::new(),
            custom_profile_id: None,
            unify_workspace: Some(UnifyDirectExposureConfig {
                route_mode: UnifyRouteMode::CapabilityLevel,
                selected_server_ids: vec!["server-a".to_string()],
                selected_tool_surfaces: Vec::new(),
                selected_prompt_surfaces: Vec::new(),
                selected_resource_surfaces: Vec::new(),
                selected_template_surfaces: vec![crate::clients::models::UnifyDirectTemplateSurface {
                    server_id: "server-a".to_string(),
                    uri_template: "server-a://{id}".to_string(),
                }],
            }),
        };
        let mut visible = vec![
            VisibleResourceTemplateEntry {
                server_id: "server-a".to_string(),
                server_name: "Server A".to_string(),
                raw_uri_template: "server-a://{id}".to_string(),
                resource_template: ResourceTemplate {
                    raw: rmcp::model::RawResourceTemplate::new("server-a://{id}", "server-a://{id}"),
                    annotations: None,
                },
            },
            VisibleResourceTemplateEntry {
                server_id: "server-a".to_string(),
                server_name: "Server A".to_string(),
                raw_uri_template: "server-a://{name}".to_string(),
                resource_template: ResourceTemplate {
                    raw: rmcp::model::RawResourceTemplate::new("server-a://{name}", "server-a://{name}"),
                    annotations: None,
                },
            },
        ];

        retain_brokered_resource_templates(&context, &HashSet::from(["server-a".to_string()]), &mut visible);

        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].raw_uri_template, "server-a://{name}");
    }

    #[allow(clippy::await_holding_lock)]
    #[tokio::test]
    async fn ucan_prompt_repository_hot_reloads_from_disk() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        let dir = tempdir().expect("tempdir");
        let config_path = dir.path().join("ucan.json5");
        std::fs::write(
            &config_path,
            r#"{
                catalog_tool_description: "first",
                details_tool_description: "details",
                call_tool_description: "call",
                catalog_usage: "usage",
                catalog_format: [
                    "capability_name",
                    "capability_kind",
                    "summary",
                    "action",
                    "next_step",
                    "server_id",
                    "server_name",
                    "interaction_mode",
                ],
                catalog_page_size_default: 20,
                catalog_page_size_max: 50,
            }"#,
        )
        .expect("write config");

        unsafe { std::env::set_var("MCPMATE_UCAN_CONFIG", &config_path) };

        let repo = UcanPromptRepository::new();
        let first = repo.get().await;
        assert_eq!(first.catalog_tool_description, "first");

        std::fs::write(
            &config_path,
            r#"{
                catalog_tool_description: "second",
                details_tool_description: "details",
                call_tool_description: "call",
                catalog_usage: "usage",
                catalog_format: [
                    "capability_name",
                    "capability_kind",
                    "summary",
                    "action",
                    "next_step",
                    "server_id",
                    "server_name",
                    "interaction_mode",
                ],
                catalog_page_size_default: 20,
                catalog_page_size_max: 50,
            }"#,
        )
        .expect("rewrite config");

        tokio::time::sleep(Duration::from_secs(3)).await;
        let second = repo.get().await;
        assert_eq!(second.catalog_tool_description, "second");

        unsafe { std::env::remove_var("MCPMATE_UCAN_CONFIG") };
    }

    #[test]
    fn normalize_multiline_text_cleans_line_endings_and_edges() {
        let input = "\r\n  line one\r\nline two  \r\n\r\n";
        let normalized = super::normalize_multiline_text(input);
        assert_eq!(normalized, "line one\nline two");
        assert!(!normalized.contains('\r'));
    }

    #[test]
    fn load_ucan_prompt_config_requires_object_root() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        let dir = tempdir().expect("tempdir");
        let config_path = dir.path().join("ucan.json5");
        std::fs::write(&config_path, "[1, 2, 3]").expect("write invalid root");

        unsafe { std::env::set_var("MCPMATE_UCAN_CONFIG", &config_path) };
        let result = super::load_ucan_prompt_config_blocking();
        assert!(result.is_err());
        let message = format!("{}", result.expect_err("load error"));
        assert!(message.contains("must be a JSON5 object"));
        unsafe { std::env::remove_var("MCPMATE_UCAN_CONFIG") };
    }

    #[test]
    fn bundled_ucan_json5_is_well_formed_object() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        unsafe { std::env::remove_var("MCPMATE_UCAN_CONFIG") };
        let path = super::resolve_ucan_prompt_config_path().expect("resolve config path");
        let content = std::fs::read_to_string(&path).expect("read ucan config");
        let value: serde_json::Value = json5::from_str(&content).expect("parse json5");
        assert!(value.is_object(), "ucan.json5 root must be object");
    }

    #[test]
    fn test_catalog_sort_tools_before_prompts() {
        let weights = super::KindWeights {
            tool: 0,
            prompt: 1,
            resource: 2,
            resource_template: 3,
        };

        let tool_weight = super::UcanCapabilityKind::Tool.weight(&weights);
        let prompt_weight = super::UcanCapabilityKind::Prompt.weight(&weights);
        let resource_weight = super::UcanCapabilityKind::Resource.weight(&weights);
        let template_weight = super::UcanCapabilityKind::ResourceTemplate.weight(&weights);

        assert!(
            tool_weight < prompt_weight,
            "tools should have lower weight than prompts"
        );
        assert!(
            prompt_weight < resource_weight,
            "prompts should have lower weight than resources"
        );
        assert!(
            resource_weight < template_weight,
            "resources should have lower weight than templates"
        );
    }

    #[test]
    fn test_catalog_sort_healthy_servers_first() {
        use super::{HealthWeights, PoolSnapshot, health_weight_for_server};
        use crate::core::foundation::types::ConnectionStatus;

        let weights = HealthWeights {
            ready: 0,
            reconnecting: 1,
            other: 2,
        };

        let mut snapshot: PoolSnapshot = std::collections::HashMap::new();
        snapshot.insert(
            "ready-server".to_string(),
            vec![("inst-1".to_string(), ConnectionStatus::Ready, false, false, None)],
        );
        snapshot.insert(
            "reconnecting-server".to_string(),
            vec![("inst-2".to_string(), ConnectionStatus::Initializing, false, false, None)],
        );
        snapshot.insert(
            "error-server".to_string(),
            vec![(
                "inst-3".to_string(),
                ConnectionStatus::Error(crate::core::foundation::types::ErrorDetails {
                    message: "error".to_string(),
                    error_type: crate::core::foundation::types::ErrorType::Temporary,
                    failure_count: 1,
                    first_failure_time: 0,
                    last_failure_time: 0,
                }),
                false,
                false,
                None,
            )],
        );

        let ready_weight = health_weight_for_server("ready-server", &snapshot, &weights);
        let reconnecting_weight = health_weight_for_server("reconnecting-server", &snapshot, &weights);
        let error_weight = health_weight_for_server("error-server", &snapshot, &weights);
        let unknown_weight = health_weight_for_server("unknown-server", &snapshot, &weights);

        assert_eq!(ready_weight, 0, "ready server should have weight 0");
        assert_eq!(reconnecting_weight, 1, "reconnecting server should have weight 1");
        assert_eq!(error_weight, 2, "error server should have weight 2");
        assert_eq!(unknown_weight, 2, "unknown server should have weight 2 (other)");
    }

    #[test]
    fn test_catalog_sort_stable_with_equal_scores() {
        let weights = super::KindWeights {
            tool: 0,
            prompt: 1,
            resource: 2,
            resource_template: 3,
        };

        let tool_a = super::UcanCapabilityKind::Tool.weight(&weights);
        let tool_b = super::UcanCapabilityKind::Tool.weight(&weights);

        assert_eq!(tool_a, tool_b, "same capability kinds should have equal weights");
    }

    #[test]
    fn test_default_catalog_sort_weights() {
        let weights = super::CatalogSortWeights::default();

        assert_eq!(weights.kind.tool, 0);
        assert_eq!(weights.kind.prompt, 1);
        assert_eq!(weights.kind.resource, 2);
        assert_eq!(weights.kind.resource_template, 3);

        assert_eq!(weights.health.ready, 0);
        assert_eq!(weights.health.reconnecting, 1);
        assert_eq!(weights.health.other, 2);
    }

    #[test]
    fn test_catalog_params_deserialize_search() {
        let json = serde_json::json!({
            "page": 1,
            "page_size": 10,
            "search": "file"
        });
        let params: super::CatalogParams = serde_json::from_value(json).expect("deserialize");
        assert_eq!(params.page, 1);
        assert_eq!(params.page_size, 10);
        assert_eq!(params.search, Some("file".to_string()));
        assert!(params.kind_filter.is_none());
    }

    #[test]
    fn test_catalog_params_deserialize_kind_filter() {
        let json = serde_json::json!({
            "page": 1,
            "page_size": 10,
            "kind_filter": ["tool", "prompt"]
        });
        let params: super::CatalogParams = serde_json::from_value(json).expect("deserialize");
        assert_eq!(params.page, 1);
        assert_eq!(params.page_size, 10);
        assert!(params.search.is_none());
        assert_eq!(params.kind_filter, Some(vec!["tool".to_string(), "prompt".to_string()]));
    }

    #[test]
    fn test_catalog_params_deserialize_both_filters() {
        let json = serde_json::json!({
            "page": 2,
            "page_size": 5,
            "search": "read",
            "kind_filter": ["resource"]
        });
        let params: super::CatalogParams = serde_json::from_value(json).expect("deserialize");
        assert_eq!(params.page, 2);
        assert_eq!(params.page_size, 5);
        assert_eq!(params.search, Some("read".to_string()));
        assert_eq!(params.kind_filter, Some(vec!["resource".to_string()]));
    }

    #[test]
    fn test_catalog_params_deserialize_defaults() {
        let json = serde_json::json!({});
        let params: super::CatalogParams = serde_json::from_value(json).expect("deserialize");
        assert_eq!(params.page, 1);
        assert_eq!(params.page_size, 20);
        assert!(params.search.is_none());
        assert!(params.kind_filter.is_none());
    }

    #[test]
    fn test_catalog_search_filter_logic() {
        use super::{CatalogToolSummary, UcanCapabilityKind};

        let summaries = [
            CatalogToolSummary {
                capability_name: "read_file".to_string(),
                capability_kind: UcanCapabilityKind::Tool,
                summary: Some("Read file contents".to_string()),
                action: "inspect_first",
                next_step: "details",
                server_id: "server-1".to_string(),
                server_name: "filesystem".to_string(),
                interaction_mode: "model_controlled",
                detail_hint: "Use details first.",
                registry_enriched: false,
                registry_category: None,
            },
            CatalogToolSummary {
                capability_name: "write_file".to_string(),
                capability_kind: UcanCapabilityKind::Tool,
                summary: Some("Write content to file".to_string()),
                action: "inspect_first",
                next_step: "details",
                server_id: "server-1".to_string(),
                server_name: "filesystem".to_string(),
                interaction_mode: "model_controlled",
                detail_hint: "Use details first.",
                registry_enriched: false,
                registry_category: None,
            },
            CatalogToolSummary {
                capability_name: "list_directory".to_string(),
                capability_kind: UcanCapabilityKind::Tool,
                summary: Some("List directory contents".to_string()),
                action: "inspect_first",
                next_step: "details",
                server_id: "server-1".to_string(),
                server_name: "filesystem".to_string(),
                interaction_mode: "model_controlled",
                detail_hint: "Use details first.",
                registry_enriched: false,
                registry_category: None,
            },
        ];

        let search_lower = "file".to_lowercase();
        let filtered: Vec<_> = summaries
            .iter()
            .filter(|item| {
                let name_match = item.capability_name.to_lowercase().contains(&search_lower);
                let summary_match = item
                    .summary
                    .as_ref()
                    .map(|s| s.to_lowercase().contains(&search_lower))
                    .unwrap_or(false);
                name_match || summary_match
            })
            .collect();

        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().any(|s| s.capability_name == "read_file"));
        assert!(filtered.iter().any(|s| s.capability_name == "write_file"));
        assert!(!filtered.iter().any(|s| s.capability_name == "list_directory"));
    }

    #[test]
    fn test_catalog_search_filter_by_summary() {
        use super::{CatalogToolSummary, UcanCapabilityKind};

        let summaries = [
            CatalogToolSummary {
                capability_name: "execute_command".to_string(),
                capability_kind: UcanCapabilityKind::Tool,
                summary: Some("Run shell commands in terminal".to_string()),
                action: "inspect_first",
                next_step: "details",
                server_id: "server-2".to_string(),
                server_name: "shell".to_string(),
                interaction_mode: "model_controlled",
                detail_hint: "Use details first.",
                registry_enriched: false,
                registry_category: None,
            },
            CatalogToolSummary {
                capability_name: "get_weather".to_string(),
                capability_kind: UcanCapabilityKind::Tool,
                summary: Some("Fetch weather data from API".to_string()),
                action: "inspect_first",
                next_step: "details",
                server_id: "server-3".to_string(),
                server_name: "weather".to_string(),
                interaction_mode: "model_controlled",
                detail_hint: "Use details first.",
                registry_enriched: false,
                registry_category: None,
            },
        ];

        let search_lower = "shell".to_lowercase();
        let filtered: Vec<_> = summaries
            .iter()
            .filter(|item| {
                let name_match = item.capability_name.to_lowercase().contains(&search_lower);
                let summary_match = item
                    .summary
                    .as_ref()
                    .map(|s| s.to_lowercase().contains(&search_lower))
                    .unwrap_or(false);
                name_match || summary_match
            })
            .collect();

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].capability_name, "execute_command");
    }

    #[test]
    fn test_catalog_kind_filter_logic() {
        use super::{CatalogToolSummary, UcanCapabilityKind};
        use std::collections::HashSet;

        let summaries = [
            CatalogToolSummary {
                capability_name: "tool_one".to_string(),
                capability_kind: UcanCapabilityKind::Tool,
                summary: None,
                action: "inspect_first",
                next_step: "details",
                server_id: "s1".to_string(),
                server_name: "server".to_string(),
                interaction_mode: "model_controlled",
                detail_hint: "",
                registry_enriched: false,
                registry_category: None,
            },
            CatalogToolSummary {
                capability_name: "prompt_one".to_string(),
                capability_kind: UcanCapabilityKind::Prompt,
                summary: None,
                action: "inspect_first",
                next_step: "details",
                server_id: "s1".to_string(),
                server_name: "server".to_string(),
                interaction_mode: "user_controlled_template",
                detail_hint: "",
                registry_enriched: false,
                registry_category: None,
            },
            CatalogToolSummary {
                capability_name: "resource_one".to_string(),
                capability_kind: UcanCapabilityKind::Resource,
                summary: None,
                action: "inspect_first",
                next_step: "details",
                server_id: "s1".to_string(),
                server_name: "server".to_string(),
                interaction_mode: "application_context",
                detail_hint: "",
                registry_enriched: false,
                registry_category: None,
            },
            CatalogToolSummary {
                capability_name: "template_one".to_string(),
                capability_kind: UcanCapabilityKind::ResourceTemplate,
                summary: None,
                action: "inspect_first",
                next_step: "details",
                server_id: "s1".to_string(),
                server_name: "server".to_string(),
                interaction_mode: "application_context_template",
                detail_hint: "",
                registry_enriched: false,
                registry_category: None,
            },
        ];

        let kinds = ["tool", "resource"];
        let allowed_kinds: HashSet<&str> = kinds.iter().copied().collect();

        let filtered: Vec<_> = summaries
            .iter()
            .filter(|item| {
                let kind_str = match item.capability_kind {
                    UcanCapabilityKind::Tool => "tool",
                    UcanCapabilityKind::Prompt => "prompt",
                    UcanCapabilityKind::Resource => "resource",
                    UcanCapabilityKind::ResourceTemplate => "resource_template",
                };
                allowed_kinds.contains(kind_str)
            })
            .collect();

        assert_eq!(filtered.len(), 2);
        assert!(
            filtered
                .iter()
                .any(|s| matches!(s.capability_kind, UcanCapabilityKind::Tool))
        );
        assert!(
            filtered
                .iter()
                .any(|s| matches!(s.capability_kind, UcanCapabilityKind::Resource))
        );
    }

    #[test]
    fn test_catalog_search_empty_returns_all() {
        use super::{CatalogToolSummary, UcanCapabilityKind};

        let summaries = [
            CatalogToolSummary {
                capability_name: "tool_a".to_string(),
                capability_kind: UcanCapabilityKind::Tool,
                summary: Some("Tool A".to_string()),
                action: "inspect_first",
                next_step: "details",
                server_id: "s1".to_string(),
                server_name: "server".to_string(),
                interaction_mode: "model_controlled",
                detail_hint: "",
                registry_enriched: false,
                registry_category: None,
            },
            CatalogToolSummary {
                capability_name: "tool_b".to_string(),
                capability_kind: UcanCapabilityKind::Tool,
                summary: Some("Tool B".to_string()),
                action: "inspect_first",
                next_step: "details",
                server_id: "s1".to_string(),
                server_name: "server".to_string(),
                interaction_mode: "model_controlled",
                detail_hint: "",
                registry_enriched: false,
                registry_category: None,
            },
        ];

        let search: Option<&str> = None;
        let filtered: Vec<_> = if let Some(search_term) = search {
            let search_lower = search_term.to_lowercase();
            summaries
                .iter()
                .filter(|item| {
                    let name_match = item.capability_name.to_lowercase().contains(&search_lower);
                    let summary_match = item
                        .summary
                        .as_ref()
                        .map(|s| s.to_lowercase().contains(&search_lower))
                        .unwrap_or(false);
                    name_match || summary_match
                })
                .collect()
        } else {
            summaries.iter().collect()
        };

        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_catalog_search_case_insensitive() {
        use super::{CatalogToolSummary, UcanCapabilityKind};

        let summaries = [CatalogToolSummary {
            capability_name: "Read_File".to_string(),
            capability_kind: UcanCapabilityKind::Tool,
            summary: Some("READ FILE CONTENTS".to_string()),
            action: "inspect_first",
            next_step: "details",
            server_id: "s1".to_string(),
            server_name: "server".to_string(),
            interaction_mode: "model_controlled",
            detail_hint: "",
            registry_enriched: false,
            registry_category: None,
        }];

        let search_lower = "FILE".to_lowercase();
        let filtered: Vec<_> = summaries
            .iter()
            .filter(|item| {
                let name_match = item.capability_name.to_lowercase().contains(&search_lower);
                let summary_match = item
                    .summary
                    .as_ref()
                    .map(|s| s.to_lowercase().contains(&search_lower))
                    .unwrap_or(false);
                name_match || summary_match
            })
            .collect();

        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_details_includes_workflow_hints_for_tool() {
        use super::{UcanDetailLevel, extract_argument_tips_from_tool};

        let tool = Tool::new(
            "test_tool",
            "A test tool for workflow hints",
            Arc::new(
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "File path to read"},
                        "encoding": {"type": "string", "description": "File encoding"}
                    },
                    "required": ["path"]
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
        );

        let details = tool_details_value(&tool, UcanDetailLevel::Summary);
        assert!(details.get("input_fields").is_some());
        assert!(details.get("required_fields").is_some());

        let tips = extract_argument_tips_from_tool(&tool);
        assert_eq!(tips.len(), 2);

        let path_tip = tips.iter().find(|t| t.name == "path").expect("path tip");
        assert!(path_tip.required);
        assert_eq!(path_tip.type_hint, Some("string".to_string()));
        assert_eq!(path_tip.description, Some("File path to read".to_string()));

        let encoding_tip = tips.iter().find(|t| t.name == "encoding").expect("encoding tip");
        assert!(!encoding_tip.required);
    }

    #[test]
    fn test_details_includes_related_capabilities() {
        use super::{RelatedCapability, UcanCapabilityKind};

        let related = RelatedCapability {
            capability_name: "related_tool".to_string(),
            capability_kind: UcanCapabilityKind::Tool,
            summary: Some("A related tool".to_string()),
        };

        let json = serde_json::to_string(&related).expect("serialize");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse");
        assert_eq!(parsed["capability_name"], "related_tool");
        assert_eq!(parsed["capability_kind"], "tool");
        assert_eq!(parsed["summary"], "A related tool");
    }

    #[test]
    fn test_details_resource_template_explains_uri_construction() {
        use super::resource_template_details_value;
        use rmcp::model::ResourceTemplate;

        let template = ResourceTemplate::new(
            rmcp::model::RawResourceTemplate {
                name: "file_template".into(),
                uri_template: "file:///{path}".into(),
                title: None,
                description: Some("File template".into()),
                mime_type: None,
                icons: None,
            },
            None,
        );

        let summary = resource_template_details_value(&template, UcanDetailLevel::Summary).expect("summary details");
        assert!(summary.get("uri_template").is_some());
        assert!(summary.get("usage").is_some());
        let usage = summary.get("usage").and_then(|v| v.as_str()).expect("usage string");
        assert!(usage.contains("not directly invocable"));

        let full = resource_template_details_value(&template, UcanDetailLevel::Full).expect("full details");
        assert!(full.get("template").is_some());
        assert!(full.get("usage").is_some());
    }

    #[test]
    fn test_argument_tips_from_tool() {
        use super::extract_argument_tips_from_tool;

        let tool = Tool::new(
            "complex_tool",
            "A tool with complex arguments",
            Arc::new(
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string", "description": "Search query"},
                        "limit": {"type": "integer", "description": "Max results"},
                        "verbose": {"type": "boolean"}
                    },
                    "required": ["query"]
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
        );

        let tips = extract_argument_tips_from_tool(&tool);
        assert_eq!(tips.len(), 3);

        let query_tip = tips.iter().find(|t| t.name == "query").expect("query tip");
        assert!(query_tip.required);
        assert_eq!(query_tip.type_hint, Some("string".to_string()));

        let limit_tip = tips.iter().find(|t| t.name == "limit").expect("limit tip");
        assert!(!limit_tip.required);
        assert_eq!(limit_tip.type_hint, Some("integer".to_string()));

        let verbose_tip = tips.iter().find(|t| t.name == "verbose").expect("verbose tip");
        assert!(!verbose_tip.required);
        assert_eq!(verbose_tip.type_hint, Some("boolean".to_string()));
        assert!(verbose_tip.description.is_none());
    }

    #[test]
    fn test_required_arguments_from_tool_and_call_requirements() {
        use super::{call_requirements_for_tool, required_arguments_from_tool};

        let tool = Tool::new(
            "validate_input",
            "A tool requiring query and format",
            Arc::new(
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string"},
                        "format": {"type": "string"},
                        "limit": {"type": "integer"}
                    },
                    "required": ["query", "format"]
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
        );

        let required = required_arguments_from_tool(&tool);
        assert_eq!(required.len(), 2);
        assert!(required.contains(&"query".to_string()));
        assert!(required.contains(&"format".to_string()));

        let requirements = call_requirements_for_tool(&tool);
        assert!(requirements.accepts_arguments);
        assert!(!requirements.call_ready_without_arguments);
        assert_eq!(requirements.required_arguments.len(), 2);
    }

    #[test]
    fn test_required_arguments_from_prompt_and_call_requirements() {
        use super::{call_requirements_for_prompt, required_arguments_from_prompt};
        use rmcp::model::{Prompt, PromptArgument};

        let arguments = vec![
            PromptArgument::new("topic")
                .with_description("Topic")
                .with_required(true),
            PromptArgument::new("tone")
                .with_description("Tone")
                .with_required(false),
        ];

        let prompt = Prompt::new("compose", Some("Compose text"), Some(arguments));
        let required = required_arguments_from_prompt(&prompt);
        assert_eq!(required, vec!["topic".to_string()]);

        let requirements = call_requirements_for_prompt(&prompt);
        assert!(requirements.accepts_arguments);
        assert!(!requirements.call_ready_without_arguments);
        assert_eq!(requirements.required_arguments, vec!["topic".to_string()]);
    }

    #[test]
    fn test_argument_tips_from_prompt() {
        use super::extract_argument_tips_from_prompt;
        use rmcp::model::{Prompt, PromptArgument};

        let arguments = vec![
            PromptArgument::new("code")
                .with_description("Code to review")
                .with_required(true),
            PromptArgument::new("language")
                .with_description("Programming language")
                .with_required(false),
        ];

        let prompt = Prompt::new("code_review", Some("Review code"), Some(arguments));

        let tips = extract_argument_tips_from_prompt(&prompt);
        assert_eq!(tips.len(), 2);

        let code_tip = tips.iter().find(|t| t.name == "code").expect("code tip");
        assert!(code_tip.required);
        assert_eq!(code_tip.description, Some("Code to review".to_string()));
        assert!(code_tip.type_hint.is_none());

        let lang_tip = tips.iter().find(|t| t.name == "language").expect("language tip");
        assert!(!lang_tip.required);
    }

    #[test]
    fn test_workflow_hints_default_values() {
        use super::{
            WorkflowHints, default_workflow_hints_prompt, default_workflow_hints_resource,
            default_workflow_hints_resource_template, default_workflow_hints_tool,
        };

        let hints = WorkflowHints::default();

        assert!(!hints.tool.is_empty());
        assert!(!hints.prompt.is_empty());
        assert!(!hints.resource.is_empty());
        assert!(!hints.resource_template.is_empty());

        let tool_hints = default_workflow_hints_tool();
        assert!(tool_hints.iter().any(|hint| hint.contains("required arguments")));

        let prompt_hints = default_workflow_hints_prompt();
        assert!(prompt_hints.iter().any(|hint| hint.contains("mcpmate_ucan_call")));

        let resource_hints = default_workflow_hints_resource();
        assert!(resource_hints[0].contains("without arguments"));

        let template_hints = default_workflow_hints_resource_template();
        assert!(
            template_hints
                .iter()
                .any(|hint| hint.contains("not directly invocable"))
        );
    }

    #[test]
    fn test_catalog_enrichment_fields_in_summary() {
        use super::{CatalogToolSummary, UcanCapabilityKind};

        let summary_with_enrichment = CatalogToolSummary {
            capability_name: "registry_tool".to_string(),
            capability_kind: UcanCapabilityKind::Tool,
            summary: Some("A tool from registry".to_string()),
            action: "inspect_first",
            next_step: "details",
            server_id: "server-1".to_string(),
            server_name: "registry-server".to_string(),
            interaction_mode: "model_controlled",
            detail_hint: "Use details first.",
            registry_enriched: true,
            registry_category: Some("filesystem".to_string()),
        };

        assert!(summary_with_enrichment.registry_enriched);
        assert_eq!(
            summary_with_enrichment.registry_category,
            Some("filesystem".to_string())
        );

        let summary_without_enrichment = CatalogToolSummary {
            capability_name: "local_tool".to_string(),
            capability_kind: UcanCapabilityKind::Tool,
            summary: Some("A local tool".to_string()),
            action: "inspect_first",
            next_step: "details",
            server_id: "server-2".to_string(),
            server_name: "local-server".to_string(),
            interaction_mode: "model_controlled",
            detail_hint: "Use details first.",
            registry_enriched: false,
            registry_category: None,
        };

        assert!(!summary_without_enrichment.registry_enriched);
        assert!(summary_without_enrichment.registry_category.is_none());
    }

    #[test]
    fn test_catalog_enrichment_config_default() {
        use super::default_catalog_enrich_from_registry;

        assert!(default_catalog_enrich_from_registry());
    }
}
