use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use tokio::sync::Mutex;

use crate::{config::database::Database, core::pool::UpstreamConnectionPool};

use super::{internal, prompts, resources};

pub use super::internal::{
    CapabilityFetchFailure, CapabilityFetchOutcome, collect_capability_from_instance_peer, is_method_not_supported,
};
pub use prompts::{PromptMapping, PromptTemplateMapping};
pub use resources::{ResourceMapping, ResourceTemplateMapping};

/// Shared concurrency limit derived from host CPU parallelism
pub fn concurrency_limit() -> usize {
    internal::concurrency_limit()
}

/// Parse capability declaration strings to determine whether a capability token is enabled
pub fn capability_declared(
    capabilities: Option<&str>,
    token: &str,
) -> bool {
    internal::capability_declared(capabilities, token)
}

pub async fn build_resource_mapping(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>,
    database: Option<&Arc<Database>>,
) -> HashMap<String, ResourceMapping> {
    resources::build_resource_mapping(connection_pool, database).await
}

pub async fn build_resource_mapping_filtered(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>,
    database: Option<&Arc<Database>>,
    enabled_server_ids: Option<&HashSet<String>>,
) -> HashMap<String, ResourceMapping> {
    resources::build_resource_mapping_filtered(connection_pool, database, enabled_server_ids).await
}

pub async fn build_resource_template_mapping(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>
) -> Vec<ResourceTemplateMapping> {
    resources::build_resource_template_mapping(connection_pool).await
}

pub async fn read_upstream_resource(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>,
    mapping: &HashMap<String, ResourceMapping>,
    uri: &str,
    target_server_id: Option<&str>,
    connection_selection: Option<&crate::core::capability::ConnectionSelection>,
) -> anyhow::Result<rmcp::model::ReadResourceResult> {
    resources::read_upstream_resource(connection_pool, mapping, uri, target_server_id, connection_selection).await
}

pub async fn build_prompt_mapping(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>
) -> HashMap<String, PromptMapping> {
    prompts::build_prompt_mapping(connection_pool).await
}

pub async fn build_prompt_mapping_filtered(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>,
    enabled_server_ids: Option<&HashSet<String>>,
) -> HashMap<String, PromptMapping> {
    prompts::build_prompt_mapping_filtered(connection_pool, enabled_server_ids).await
}

pub async fn build_prompt_template_mapping(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>
) -> Vec<PromptTemplateMapping> {
    prompts::build_prompt_template_mapping(connection_pool).await
}

pub async fn get_upstream_prompt(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>,
    mapping: &HashMap<String, PromptMapping>,
    name: &str,
    arguments: Option<serde_json::Map<String, serde_json::Value>>,
    target_server_id: Option<&str>,
    connection_selection: Option<&crate::core::capability::ConnectionSelection>,
) -> anyhow::Result<rmcp::model::GetPromptResult> {
    prompts::get_upstream_prompt(connection_pool, mapping, name, arguments, target_server_id, connection_selection).await
}
