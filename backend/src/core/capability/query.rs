pub(crate) const DEFAULT_QUERY_TIMEOUT_SECS: u64 = 120;

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use serde_json::Value;
use tokio::sync::Mutex;

use crate::api::handlers::server::common::{InspectParams, RefreshStrategy};
use crate::config::database::Database;
use crate::config::models::Server;
use crate::config::server;
use crate::core::capability::domain::{
    CapabilityError, CapabilityItem, CapabilityResult, CapabilityType, DataSource,
    PromptArgument as DomainPromptArgument, PromptCapability, ResourceCapability, ResourceTemplateCapability,
    ResponseMetadata, ToolCapability,
};
use crate::core::capability::read_service::CapabilityReadService;
use crate::core::capability::runtime::{
    CapabilityItems, ListCtx, ListResult, Meta, RefreshStrategy as RuntimeRefreshStrategy,
};
use crate::core::pool::UpstreamConnectionPool;

/// Performance metrics collection trait used by capability query helpers.
pub trait MetricsCollector {
    fn record_capability_query_duration(
        &self,
        capability_type: CapabilityType,
        duration: std::time::Duration,
    );

    fn record_query_source(
        &self,
        source: DataSource,
    );

    fn record_query_result(
        &self,
        capability_type: CapabilityType,
        success: bool,
        item_count: usize,
    );
}

/// Unified entry for profile token metrics and capability ledger reads.
pub async fn query_capabilities(
    pool: Arc<Mutex<UpstreamConnectionPool>>,
    database: Arc<Database>,
    server_id: &str,
    capability_type: CapabilityType,
    params: &InspectParams,
) -> Result<CapabilityResult, CapabilityError> {
    let server = load_server(&database, server_id).await?;
    ensure_server_enabled(&server, server_id)?;

    let timeout = params
        .timeout
        .map(Duration::from_secs)
        .unwrap_or_else(|| Duration::from_secs(DEFAULT_QUERY_TIMEOUT_SECS));
    let list_ctx = ListCtx {
        capability: capability_type,
        server_id: server_id.to_string(),
        refresh: Some(map_refresh_strategy(params.refresh)),
        operation_timeout: timeout,
        validation_session: None,
        runtime_identity: None,
        connection_selection: None,
        visibility_snapshot: None,
        name_domain: crate::core::capability::runtime::NameDomain::External,
    };
    let capability_service = CapabilityReadService::from_runtime(database, pool);
    let list_result = capability_service
        .list(&list_ctx)
        .await
        .map_err(|err| CapabilityError::RuntimeError(err.to_string()))?;

    Ok(list_to_capability_result(list_result, capability_type))
}

async fn load_server(
    database: &Database,
    server_id: &str,
) -> Result<Server, CapabilityError> {
    server::get_server_by_id(&database.pool, server_id)
        .await
        .map_err(|err| CapabilityError::InternalError(err.to_string()))?
        .ok_or_else(|| CapabilityError::InternalError(format!("Server {server_id} not found")))
}

fn ensure_server_enabled(
    server: &Server,
    server_id: &str,
) -> Result<(), CapabilityError> {
    if !server.enabled.as_bool() {
        return Err(CapabilityError::ServerDisabled {
            server_id: server_id.to_string(),
        });
    }

    Ok(())
}

fn map_refresh_strategy(refresh: Option<RefreshStrategy>) -> RuntimeRefreshStrategy {
    match refresh.unwrap_or(RefreshStrategy::CacheFirst) {
        RefreshStrategy::Force => RuntimeRefreshStrategy::Force,
        _ => RuntimeRefreshStrategy::CacheFirst,
    }
}

/// Convert runtime list result into domain capability result
pub fn list_to_capability_result(
    list_result: ListResult,
    capability_type: CapabilityType,
) -> CapabilityResult {
    let items = match list_result.items {
        CapabilityItems::Tools(tools) => tools.into_iter().map(tool_to_capability).collect::<Vec<_>>(),
        CapabilityItems::Resources(resources) => resources.into_iter().map(resource_to_capability).collect::<Vec<_>>(),
        CapabilityItems::Prompts(prompts) => prompts.into_iter().map(prompt_to_capability).collect::<Vec<_>>(),
        CapabilityItems::ResourceTemplates(templates) => {
            templates.into_iter().map(template_to_capability).collect::<Vec<_>>()
        }
    };

    let metadata = build_metadata(&list_result.meta, items.len(), capability_type);

    CapabilityResult { items, metadata }
}

fn build_metadata(
    meta: &Meta,
    item_count: usize,
    capability_type: CapabilityType,
) -> ResponseMetadata {
    ResponseMetadata {
        cache_hit: meta.cache_hit,
        source: to_data_source(meta, capability_type),
        duration_ms: meta.duration_ms,
        item_count,
        timestamp: Utc::now(),
    }
}

fn to_data_source(
    meta: &Meta,
    capability_type: CapabilityType,
) -> DataSource {
    match meta.source.as_str() {
        "memory_cache" => DataSource::CacheL1,
        "sqlite_catalog" => DataSource::CacheL2,
        "live" => match capability_type {
            CapabilityType::Tools | CapabilityType::Prompts | CapabilityType::Resources => DataSource::Runtime,
            CapabilityType::ResourceTemplates => {
                if meta.had_peer {
                    DataSource::Runtime
                } else {
                    DataSource::None
                }
            }
        },
        other => {
            tracing::debug!(source = other, "Unknown capability data source");
            DataSource::None
        }
    }
}

fn tool_to_capability(tool: rmcp::model::Tool) -> CapabilityItem {
    let name = tool.name.to_string();
    let schema = Value::Object((*tool.input_schema).clone());
    CapabilityItem::Tool(ToolCapability {
        name: name.clone(),
        description: tool.description.map(|d| d.into_owned()),
        input_schema: schema,
        unique_name: name,
        enabled: true,
        icons: tool.icons,
    })
}

fn resource_to_capability(resource: rmcp::model::Resource) -> CapabilityItem {
    let unique_uri = resource.uri.clone();
    CapabilityItem::Resource(ResourceCapability {
        uri: resource.uri,
        name: Some(resource.name),
        description: resource.description,
        mime_type: resource.mime_type,
        unique_uri,
        enabled: true,
        icons: resource.icons,
    })
}

fn prompt_to_capability(prompt: rmcp::model::Prompt) -> CapabilityItem {
    let rmcp::model::Prompt {
        name,
        description,
        arguments,
        ..
    } = prompt;

    let unique_name = name.clone();
    CapabilityItem::Prompt(PromptCapability {
        name,
        description,
        arguments: arguments.map(|args| {
            args.into_iter()
                .map(|arg| DomainPromptArgument {
                    name: arg.name,
                    description: arg.description,
                    required: arg.required,
                })
                .collect()
        }),
        unique_name,
        enabled: true,
        icons: prompt.icons,
    })
}

fn template_to_capability(template: rmcp::model::ResourceTemplate) -> CapabilityItem {
    let unique_template = template.uri_template.clone();
    CapabilityItem::ResourceTemplate(ResourceTemplateCapability {
        uri_template: template.uri_template,
        name: Some(template.name),
        description: template.description,
        mime_type: template.mime_type,
        unique_template,
        enabled: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_template_identity_uses_uri_template_not_display_name() {
        let external_template = "mcpmate://resources/template/docs/file/{path}";
        let template = rmcp::model::ResourceTemplate::new(external_template, "File");

        let CapabilityItem::ResourceTemplate(projected) = template_to_capability(template) else {
            panic!("expected resource template capability");
        };

        assert_eq!(projected.uri_template, external_template);
        assert_eq!(projected.unique_template, external_template);
        assert_eq!(projected.name.as_deref(), Some("File"));
    }
}
