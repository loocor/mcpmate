//! Server capability handling utilities
//!
//! This module provides comprehensive functionality for managing MCP Server capabilities including:
//! - Database mapping and persistence for tools, prompts, resources, and resource templates
//! - Data enrichment with unique identifiers and database relationships
//! - JSON formatting for API responses
//! - Capability extraction from live server instances
//! - Refresh mechanisms for cache invalidation and temporary instances
//!
//! All capability types (Tools, Prompts, Resources, ResourceTemplates) follow unified patterns
//! for consistent handling across the API.

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use rmcp::model::Icon;
use serde_json::{Map, Value};
use sqlx::{Pool, Sqlite};
use std::{collections::HashMap, sync::Arc};

use crate::api::handlers::ApiError;
use crate::api::handlers::server::common::{
    InspectParams, InspectQuery, RefreshStrategy, ServerIdentification, get_database_from_state,
    get_server_info_for_inspect,
};
use crate::api::models::cache::{
    CacheDetailsData, CacheDetailsReq, CacheDetailsResp, CacheKeyItem, CacheMetricsStats, CacheResetData,
    CacheResetResp, CacheStorageStats, CacheTablesCount, CacheViewType,
};
use crate::api::models::server::{
    ServerCapabilityDetailData, ServerCapabilityDetailReq, ServerCapabilityDetailResp, ServerCapabilityMeta,
};
use crate::api::routes::AppState;
use crate::audit::{AuditAction, AuditStatus};
use crate::core::cache::{CacheQuery, CacheScope, FreshnessLevel};
use crate::core::capability::naming::{NamingKind, resolve_capability_route};

#[derive(Debug, Clone, Copy)]
pub enum CapabilityType {
    Tools,
    Prompts,
    Resources,
    ResourceTemplates,
}

#[cfg(test)]
mod tests {
    use super::{
        CapabilityType, ExtractedCapability, enrich_prompt_item, enrich_resource_item, enrich_resource_template_item,
        enrich_tool_item, persist_extracted_inventory, prompt_json, resource_json, resource_template_json,
        resource_template_json_from_cached, tool_json,
    };
    use crate::{
        api::{handlers::server::common::ServerIdentification, routes::AppState},
        config::database::Database,
        core::{
            cache::{
                CachedPromptInfo, CachedResourceInfo, CachedResourceTemplateInfo, CachedToolInfo, RedbCacheManager,
                manager::CacheConfig,
            },
            models::Config,
            pool::UpstreamConnectionPool,
            profile::ConfigApplicationStateManager,
        },
        inspector::{calls::InspectorCallRegistry, sessions::InspectorSessionManager},
        system::metrics::MetricsCollector,
    };
    use chrono::Utc;
    use serde_json::json;
    use sqlx::sqlite::SqlitePoolOptions;
    use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Duration};
    use tempfile::TempDir;
    use tokio::sync::{Mutex, RwLock};

    fn mapping(
        upstream: &str,
        external: &str,
    ) -> HashMap<String, (String, String)> {
        HashMap::from([(
            upstream.to_string(),
            ("capability-id".to_string(), external.to_string()),
        )])
    }

    #[test]
    fn cached_detail_json_uses_standard_mcp_wire_fields() {
        let tool = tool_json(
            "everything_get-tiny-image",
            Some("Returns a tiny image".to_string()),
            json!({ "type": "object" }),
            Some(json!({ "type": "object" })),
            None,
        );
        assert_eq!(tool["name"], "everything_get-tiny-image");
        assert_eq!(tool["inputSchema"], json!({ "type": "object" }));
        assert_eq!(tool["outputSchema"], json!({ "type": "object" }));
        assert!(tool.get("input_schema").is_none());
        assert!(tool.get("output_schema").is_none());
        assert!(tool.get("unique_name").is_none());
        assert!(tool.get("id").is_none());

        let resource = resource_json(
            "mcpmate://resources/everything/demo/static/document/architecture.md",
            Some("architecture.md".to_string()),
            Some("Architecture document".to_string()),
            Some("text/markdown".to_string()),
            None,
        );
        assert_eq!(resource["mimeType"], "text/markdown");
        assert!(resource.get("mime_type").is_none());
        assert!(resource.get("unique_uri").is_none());
        assert!(resource.get("id").is_none());

        let template = resource_template_json(
            "mcpmate://resources/template/everything/demo/dynamic/text/{resourceId}",
            Some("Dynamic Text Resource".to_string()),
            Some("Dynamic text".to_string()),
            Some("text/plain".to_string()),
        );
        assert_eq!(
            template["uriTemplate"],
            "mcpmate://resources/template/everything/demo/dynamic/text/{resourceId}"
        );
        assert_eq!(template["mimeType"], "text/plain");
        assert!(template.get("uri_template").is_none());
        assert!(template.get("unique_uri_template").is_none());
        assert!(template.get("id").is_none());

        let prompt = prompt_json(
            "everything_args-prompt",
            Some("Prompt with arguments".to_string()),
            vec![crate::core::cache::PromptArgument {
                name: "message".to_string(),
                description: Some("Message".to_string()),
                required: true,
            }],
            None,
        );
        assert_eq!(prompt["name"], "everything_args-prompt");
        assert!(prompt.get("unique_name").is_none());
        assert!(prompt.get("id").is_none());
    }

    #[test]
    fn cached_detail_projection_uses_external_identifiers() {
        let cached_at = Utc::now();
        let tool = super::tool_json_from_cached(
            &CachedToolInfo {
                name: "get-tiny-image".to_string(),
                description: None,
                input_schema_json: r#"{"type":"object"}"#.to_string(),
                output_schema_json: None,
                unique_name: None,
                icons: None,
                enabled: true,
                cached_at,
            },
            "everything_get-tiny-image",
        );
        let prompt = super::prompt_json_from_cached(
            CachedPromptInfo {
                name: "args-prompt".to_string(),
                description: None,
                arguments: Vec::new(),
                icons: None,
                enabled: true,
                cached_at,
            },
            "everything_args-prompt",
        );
        let resource = super::resource_json_from_cached(
            CachedResourceInfo {
                uri: "demo://resource/static/document/architecture.md".to_string(),
                name: Some("architecture.md".to_string()),
                description: None,
                mime_type: Some("text/markdown".to_string()),
                icons: None,
                enabled: true,
                cached_at,
            },
            "mcpmate://resources/everything/demo/static/document/architecture.md",
        );
        let template = resource_template_json_from_cached(
            CachedResourceTemplateInfo {
                uri_template: "demo://resource/dynamic/text/{resourceId}".to_string(),
                name: Some("Dynamic Text Resource".to_string()),
                description: None,
                mime_type: Some("text/plain".to_string()),
                enabled: true,
                cached_at,
            },
            "mcpmate://resources/template/everything/demo/dynamic/text/{resourceId}",
        );

        assert_eq!(tool["name"], "everything_get-tiny-image");
        assert_eq!(prompt["name"], "everything_args-prompt");
        assert_eq!(
            resource["uri"],
            "mcpmate://resources/everything/demo/static/document/architecture.md"
        );
        assert_eq!(
            template["uriTemplate"],
            "mcpmate://resources/template/everything/demo/dynamic/text/{resourceId}"
        );
    }

    #[test]
    fn management_tool_projection_uses_external_name_and_keeps_upstream_metadata() {
        let projected = enrich_tool_item(
            json!({ "name": "get_searxng_status" }),
            &mapping("get_searxng_status", "searxng_get_status"),
        )
        .expect("project tool");

        assert_eq!(projected["name"], "searxng_get_status");
        assert_eq!(projected["unique_name"], "searxng_get_status");
        assert_eq!(projected["tool_name"], "get_searxng_status");
        assert_eq!(projected["id"], "capability-id");
    }

    #[test]
    fn management_projections_resolve_already_external_values() {
        let canonical_resource = crate::core::capability::resource_uri::encode_resource_uri("docs", "file:///guide.md")
            .expect("encode resource");
        let upstream_template = "demo://resource/lookup/{id}";
        let canonical_template =
            crate::core::capability::resource_uri::encode_resource_template("docs", upstream_template)
                .expect("encode resource template");
        let prompt =
            enrich_prompt_item(json!({ "name": "docs_help" }), &mapping("help", "docs_help")).expect("project prompt");
        let resource = enrich_resource_item(
            json!({ "uri": canonical_resource }),
            &mapping("file:///guide.md", &canonical_resource),
        )
        .expect("project resource");
        let template = enrich_resource_template_item(
            json!({ "name": "Lookup", "uri_template": upstream_template }),
            &mapping(upstream_template, &canonical_template),
        )
        .expect("project resource template");

        assert_eq!(prompt["name"], "docs_help");
        assert_eq!(prompt["prompt_name"], "help");
        assert_eq!(resource["uri"], canonical_resource);
        assert_eq!(resource["resource_uri"], "file:///guide.md");
        assert_eq!(template["name"], "Lookup");
        assert_eq!(template["uri_template"], upstream_template);
        assert_eq!(template["unique_uri_template"], canonical_template);
    }

    #[test]
    fn management_resource_template_projection_accepts_rmcp_wire_shape() {
        let upstream_template = "demo://resource/dynamic/text/{resourceId}";
        let canonical_template = "mcpmate://resources/template/everything/demo/dynamic/text/{resourceId}".to_string();
        let template = rmcp::model::ResourceTemplate {
            raw: rmcp::model::RawResourceTemplate::new(canonical_template.clone(), "Dynamic Text Resource"),
            annotations: None,
        };
        let wire_item = serde_json::to_value(template).expect("serialize RMCP resource template");

        assert_eq!(wire_item["uriTemplate"], canonical_template);
        let projected = enrich_resource_template_item(wire_item, &mapping(upstream_template, &canonical_template))
            .expect("project RMCP resource template");

        assert_eq!(projected["name"], "Dynamic Text Resource");
        assert_eq!(projected["uri_template"], upstream_template);
        assert_eq!(projected["unique_uri_template"], canonical_template);
        assert_eq!(projected["id"], "capability-id");
    }

    #[test]
    fn management_resource_template_projection_rejects_display_name_as_identity() {
        let display_name = "demo://resource/dynamic/text/{resourceId}";
        let result = enrich_resource_template_item(
            json!({ "name": display_name }),
            &mapping(
                display_name,
                "mcpmate://resources/template/everything/demo/dynamic/text/{resourceId}",
            ),
        );

        assert!(result.is_err());
    }

    #[test]
    fn cached_resource_template_protocol_projection_accepts_external_identifier() {
        let fixtures = [
            (
                "demo://resource/dynamic/blob/{resourceId}",
                "mcpmate://resources/template/everything/demo/dynamic/blob/{resourceId}",
            ),
            (
                "demo://resource/dynamic/text/{resourceId}",
                "mcpmate://resources/template/everything/demo/dynamic/text/{resourceId}",
            ),
        ];

        for (upstream_template, external_template) in fixtures {
            let cached_payload = resource_template_json_from_cached(
                CachedResourceTemplateInfo {
                    uri_template: upstream_template.to_string(),
                    name: Some("Dynamic resource".to_string()),
                    description: Some("Read a dynamic resource".to_string()),
                    mime_type: Some("application/octet-stream".to_string()),
                    enabled: true,
                    cached_at: Utc::now(),
                },
                external_template,
            );
            assert_eq!(cached_payload["uriTemplate"], external_template);

            let projected =
                enrich_resource_template_item(cached_payload, &mapping(upstream_template, external_template))
                    .expect("project cached resource template");

            assert_eq!(projected["uri_template"], upstream_template);
            assert_eq!(projected["unique_uri_template"], external_template);
            assert_eq!(projected["id"], "capability-id");
        }
    }

    #[tokio::test]
    async fn naming_projection_fails_when_catalog_mapping_cannot_be_loaded() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:")
            .await
            .expect("create test database");

        let error = super::enrich_capability_items(
            super::CapabilityType::Tools,
            &pool,
            "server-1",
            vec![json!({ "name": "upstream_tool" })],
        )
        .await
        .expect_err("missing capability catalog must not expose an upstream name");

        assert!(error.to_string().contains("Failed to load tool naming mappings"));
    }

    #[tokio::test]
    async fn naming_projection_rejects_an_unmapped_upstream_value() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:")
            .await
            .expect("create test database");
        sqlx::query("CREATE TABLE server_tools (server_id TEXT, tool_name TEXT, id TEXT, unique_name TEXT)")
            .execute(&pool)
            .await
            .expect("create tool catalog");

        let error = super::enrich_capability_items(
            super::CapabilityType::Tools,
            &pool,
            "server-1",
            vec![json!({ "name": "unmapped_upstream_tool" })],
        )
        .await
        .expect_err("unmapped upstream values must not escape the connection pool");

        assert!(error.to_string().contains("unmapped_upstream_tool"));
    }

    #[tokio::test]
    async fn force_refresh_collision_records_namespace_remediation_issue() {
        let temp_dir = TempDir::new().expect("temp dir");
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("connect database");
        crate::config::initialization::run_initialization(&pool)
            .await
            .expect("initialize schema");
        for (server_id, namespace) in [("server-owner", "a"), ("server-challenger", "a_b")] {
            sqlx::query("INSERT INTO server_config (id, name, server_type) VALUES (?, ?, 'stdio')")
                .bind(server_id)
                .bind(namespace)
                .execute(&pool)
                .await
                .expect("insert server");
        }
        crate::config::server::tools::upsert_server_tool(&pool, "server-owner", "a", "b_c", None)
            .await
            .expect("insert owner tool");

        let database = Arc::new(Database {
            pool: pool.clone(),
            path: PathBuf::from(":memory:"),
        });
        let redb_cache = Arc::new(
            RedbCacheManager::new(temp_dir.path().join("capability.redb"), CacheConfig::default())
                .expect("create cache"),
        );
        let state = Arc::new(AppState {
            connection_pool: Arc::new(Mutex::new(UpstreamConnectionPool::new(
                Arc::new(Config::default()),
                Some(database.clone()),
            ))),
            metrics_collector: Arc::new(MetricsCollector::new(Duration::from_secs(1))),
            http_proxy: None,
            profile_merge_service: None,
            database: Some(database),
            audit_database: None,
            audit_service: None,
            config_application_state: Arc::new(ConfigApplicationStateManager::new()),
            redb_cache,
            unified_query: None,
            client_service: None,
            inspector_calls: Arc::new(InspectorCallRegistry::new()),
            inspector_sessions: Arc::new(InspectorSessionManager::new()),
            oauth_manager: RwLock::new(None),
            secret_store: RwLock::new(None),
            secret_store_readiness: RwLock::new(crate::api::routes::unavailable_secret_store_readiness(
                "test_unavailable",
            )),
        });
        crate::config::server::capabilities::store_dual_write(
            &pool,
            &state.redb_cache,
            "server-challenger",
            "a_b",
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            None,
        )
        .await
        .expect("store full baseline before scoped force refresh");
        let extracted = ExtractedCapability {
            tools: vec![CachedToolInfo {
                name: "c".to_string(),
                description: None,
                input_schema_json: "{}".to_string(),
                output_schema_json: None,
                unique_name: None,
                icons: None,
                enabled: true,
                cached_at: Utc::now(),
            }],
            ..ExtractedCapability::default()
        };
        let mut events = crate::core::events::EventBus::global().subscribe_async();

        persist_extracted_inventory(
            &state,
            &ServerIdentification {
                server_id: "server-challenger".to_string(),
                server_name: "a_b".to_string(),
            },
            CapabilityType::Tools,
            &extracted,
        )
        .await
        .expect_err("force refresh collision must fail");

        let issue = crate::config::server::namespace_repair::inspect_namespace_issue(&pool, "server-challenger")
            .await
            .expect("inspect issue");
        assert!(
            issue.is_some(),
            "force refresh must record a Board-visible remediation issue"
        );
        let event = tokio::time::timeout(Duration::from_millis(100), events.recv())
            .await
            .expect("force refresh collision must publish a block event")
            .expect("event channel must remain open");
        assert!(matches!(
            event,
            crate::core::events::Event::CapabilityCollisionDetected { server_id, .. }
                if server_id == "server-challenger"
        ));
    }
}

#[derive(Debug, Clone, Default)]
pub struct ExtractedCapability {
    pub data: Vec<serde_json::Value>,
    pub tools: Vec<crate::core::cache::CachedToolInfo>,
    pub prompts: Vec<crate::core::cache::CachedPromptInfo>,
    pub resources: Vec<crate::core::cache::CachedResourceInfo>,
    pub resource_templates: Vec<crate::core::cache::CachedResourceTemplateInfo>,
}

impl ExtractedCapability {
    pub fn empty() -> Self {
        Self::default()
    }
}

/// Load tool name to (id, unique_name) mapping from database
///
/// # Arguments
/// * `pool` - SQLite connection pool
/// * `server_id` - Server identifier to filter tools
///
/// # Returns
/// HashMap mapping tool names to their (id, unique_name) tuples
pub async fn load_tool_mapping(
    pool: &Pool<Sqlite>,
    server_id: &str,
) -> Result<HashMap<String, (String, String)>, sqlx::Error> {
    Ok(sqlx::query_as::<_, (String, String, String)>(
        r#"SELECT tool_name, id, unique_name FROM server_tools WHERE server_id = ?"#,
    )
    .bind(server_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|(name, id, unique_name)| (name, (id, unique_name)))
    .collect())
}

/// Load prompt name to (id, unique_name) mapping from database
///
/// # Arguments
/// * `pool` - SQLite connection pool
/// * `server_id` - Server identifier to filter prompts
///
/// # Returns
/// HashMap mapping prompt names to their (id, unique_name) tuples
pub async fn load_prompt_mapping(
    pool: &Pool<Sqlite>,
    server_id: &str,
) -> Result<HashMap<String, (String, String)>, sqlx::Error> {
    Ok(sqlx::query_as::<_, (String, String, String)>(
        r#"SELECT prompt_name, id, unique_name FROM server_prompts WHERE server_id = ?"#,
    )
    .bind(server_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|(name, id, unique_name)| (name, (id, unique_name)))
    .collect())
}

/// Load resource URI to (id, unique_uri) mapping from database
///
/// # Arguments
/// * `pool` - SQLite connection pool
/// * `server_id` - Server identifier to filter resources
///
/// # Returns
/// HashMap mapping resource URIs to their (id, unique_uri) tuples
pub async fn load_resource_mapping(
    pool: &Pool<Sqlite>,
    server_id: &str,
) -> Result<HashMap<String, (String, String)>, sqlx::Error> {
    Ok(sqlx::query_as::<_, (String, String, String)>(
        r#"SELECT resource_uri, id, unique_uri FROM server_resources WHERE server_id = ?"#,
    )
    .bind(server_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|(uri, id, unique_uri)| (uri, (id, unique_uri)))
    .collect())
}

pub async fn load_resource_template_mapping(
    pool: &Pool<Sqlite>,
    server_id: &str,
) -> Result<HashMap<String, (String, String)>, sqlx::Error> {
    Ok(sqlx::query_as::<_, (String, String, String)>(
        r#"SELECT uri_template, id, unique_name FROM server_resource_templates WHERE server_id = ?"#,
    )
    .bind(server_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|(tpl, id, unique_name)| (tpl, (id, unique_name)))
    .collect())
}

/// Return snapshot of the cache state for MCP server capabilities.
pub async fn server_cache_detail(
    State(state): State<Arc<AppState>>,
    Query(query): Query<CacheDetailsReq>,
) -> Result<Json<CacheDetailsResp>, StatusCode> {
    let result = cache_details_core(&query, &state).await?;
    Ok(Json(result))
}

/// Clear the cached capability data for all servers.
pub async fn server_cache_reset(State(state): State<Arc<AppState>>) -> Result<Json<CacheResetResp>, StatusCode> {
    let started_at = std::time::Instant::now();

    let result = cache_reset_core(&state).await;

    let (audit_status, audit_error) = match &result {
        Ok(_) => (AuditStatus::Success, None),
        Err(e) => (AuditStatus::Failed, Some(e.to_string())),
    };

    let mut data = Map::new();
    if let Ok(ref response) = result {
        if let Some(ref inner) = response.data {
            data.insert("success".to_string(), Value::Bool(inner.success));
            if let Some(ref msg) = inner.message {
                data.insert("message".to_string(), Value::String(msg.clone()));
            }
        }
    }

    crate::audit::interceptor::emit_event(
        state.audit_service.as_ref(),
        crate::audit::interceptor::build_rest_event(
            AuditAction::ServerCacheReset,
            audit_status,
            "POST",
            "/api/mcp/servers/cache/reset",
            Some(started_at.elapsed().as_millis() as u64),
            None,
            None,
            Some(data),
            audit_error,
        ),
    )
    .await;

    result.map(Json)
}

struct CapabilityDetailLookup {
    item: Option<serde_json::Value>,
    cache_hit: bool,
    source: String,
}

/// Return a single capability item for lazy detail expansion.
pub async fn server_capability_detail(
    State(state): State<Arc<AppState>>,
    Query(request): Query<ServerCapabilityDetailReq>,
) -> Result<Json<ServerCapabilityDetailResp>, StatusCode> {
    let key = request.key.trim();
    if key.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let query = InspectQuery {
        refresh: None,
        format: None,
        include_meta: None,
        timeout: None,
    };
    let (_db, server_info, _) = get_server_info_for_inspect(&state, &request.id, &query).await?;
    let capability_type = parse_capability_detail_type(&request.kind)?;
    let lookup = match cached_capability_detail_item(&state, &server_info, capability_type, key).await {
        Ok(lookup) => lookup,
        Err(error) => {
            tracing::error!(
                server_id = %server_info.server_id,
                kind = %request.kind,
                key = %key,
                error = %error,
                "Cached capability detail lookup failed"
            );
            return Err(error);
        }
    };

    let item = lookup.item;

    let state_name = if item.is_some() { "ok" } else { "missing" };
    Ok(Json(ServerCapabilityDetailResp::success(ServerCapabilityDetailData {
        item,
        state: state_name.to_string(),
        meta: ServerCapabilityMeta {
            cache_hit: lookup.cache_hit,
            strategy: "cache".to_string(),
            source: lookup.source,
        },
    })))
}

async fn cached_capability_detail_item(
    state: &Arc<AppState>,
    server_info: &ServerIdentification,
    capability_type: CapabilityType,
    key: &str,
) -> Result<CapabilityDetailLookup, StatusCode> {
    let query = CacheQuery {
        server_id: server_info.server_id.clone(),
        freshness_level: FreshnessLevel::Cached,
        include_disabled: true,
        scope: CacheScope::shared_raw(),
    };
    let cached = state
        .redb_cache
        .get_server_data(&query)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let Some(data) = cached.data else {
        return Ok(CapabilityDetailLookup {
            item: None,
            cache_hit: false,
            source: "cache".to_string(),
        });
    };

    let naming_kind = match capability_type {
        CapabilityType::Tools => NamingKind::Tool,
        CapabilityType::Prompts => NamingKind::Prompt,
        CapabilityType::Resources => NamingKind::Resource,
        CapabilityType::ResourceTemplates => NamingKind::ResourceTemplate,
    };
    let route = resolve_capability_route(naming_kind, key)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    if route.server_id != server_info.server_id {
        return Ok(CapabilityDetailLookup {
            item: None,
            cache_hit: true,
            source: "cache".to_string(),
        });
    }
    let upstream_key = route.upstream_value;
    let item = match capability_type {
        CapabilityType::Tools => data
            .tools
            .into_iter()
            .find(|tool| capability_key_matches(&tool.name, &upstream_key))
            .map(|tool| tool_json_from_cached(&tool, key)),
        CapabilityType::Resources => data
            .resources
            .into_iter()
            .find(|resource| capability_key_matches(&resource.uri, &upstream_key))
            .map(|resource| resource_json_from_cached(resource, key)),
        CapabilityType::Prompts => data
            .prompts
            .into_iter()
            .find(|prompt| capability_key_matches(&prompt.name, &upstream_key))
            .map(|prompt| prompt_json_from_cached(prompt, key)),
        CapabilityType::ResourceTemplates => data
            .resource_templates
            .into_iter()
            .find(|template| capability_key_matches(&template.uri_template, &upstream_key))
            .map(|template| resource_template_json_from_cached(template, key)),
    };

    Ok(CapabilityDetailLookup {
        item,
        cache_hit: true,
        source: "cache".to_string(),
    })
}

fn parse_capability_detail_type(kind: &str) -> Result<CapabilityType, StatusCode> {
    match kind.trim().to_ascii_lowercase().as_str() {
        "tool" | "tools" => Ok(CapabilityType::Tools),
        "resource" | "resources" => Ok(CapabilityType::Resources),
        "prompt" | "prompts" => Ok(CapabilityType::Prompts),
        "template" | "templates" | "resource_template" | "resource_templates" => Ok(CapabilityType::ResourceTemplates),
        _ => Err(StatusCode::BAD_REQUEST),
    }
}

fn capability_key_matches(
    candidate: &str,
    upstream_key: &str,
) -> bool {
    let candidate = candidate.trim();
    candidate == upstream_key.trim()
}

const DEFAULT_LIMIT: usize = 50;
const MAX_LIMIT: usize = 1000;

async fn cache_details_core(
    query: &CacheDetailsReq,
    state: &Arc<AppState>,
) -> Result<CacheDetailsResp, StatusCode> {
    match query.view {
        CacheViewType::Keys => {
            let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);

            let entries = state
                .redb_cache
                .list_server_entries(query.server_id.as_deref(), limit)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to list cache entries: {e}");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

            let keys: Vec<CacheKeyItem> = entries
                .into_iter()
                .map(|e| CacheKeyItem {
                    key: e.key,
                    server_id: e.server_id,
                    approx_value_size_bytes: e.approx_value_size_bytes,
                    cached_at: e.cached_at.map(|t| t.to_rfc3339()),
                })
                .collect();

            let total = keys.len();
            let response = CacheDetailsData {
                keys: Some(keys),
                storage: None,
                metrics: None,
                total: Some(total),
                generated_at: None,
            };

            Ok(CacheDetailsResp::success(response))
        }
        CacheViewType::Stats => {
            let stats = state.redb_cache.get_stats().await;
            let live = state.redb_cache.get_metrics().await;
            let db_path = state.redb_cache.database_path();
            let last_cleanup = state.redb_cache.get_last_cleanup_time();

            let storage = CacheStorageStats {
                db_path: db_path.to_string_lossy().to_string(),
                cache_size_bytes: stats.cache_size_bytes,
                tables: CacheTablesCount {
                    servers: stats.total_servers,
                    tools: stats.total_tools,
                    resources: stats.total_resources,
                    prompts: stats.total_prompts,
                    resource_templates: stats.total_resource_templates,
                },
                last_cleanup,
            };

            let hit_ratio = live.hit_ratio();
            let hit_ratio = (hit_ratio * 10_000.0).round() / 10_000.0;

            let metrics = CacheMetricsStats {
                total_queries: live.total_queries,
                cache_hits: live.cache_hits,
                cache_misses: live.cache_misses,
                hit_ratio,
                read_operations: live.read_operations,
                write_operations: live.write_operations,
                cache_invalidations: live.cache_invalidations,
            };

            let response = CacheDetailsData {
                keys: None,
                storage: Some(storage),
                metrics: Some(metrics),
                total: None,
                generated_at: Some(stats.last_updated.to_rfc3339()),
            };

            Ok(CacheDetailsResp::success(response))
        }
    }
}

async fn cache_reset_core(state: &Arc<AppState>) -> Result<CacheResetResp, StatusCode> {
    state.redb_cache.clear_all().await.map_err(|e| {
        tracing::error!("Failed to clear cache: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let response = CacheResetData {
        success: true,
        message: Some("Cache cleared successfully".to_string()),
    };

    Ok(CacheResetResp::success(response))
}

/// Enrich tool items with database identifiers
#[inline]
fn enrich_tool_item(
    item: serde_json::Value,
    mapping: &HashMap<String, (String, String)>,
) -> Result<serde_json::Value, ApiError> {
    let mut item = item;
    let presented_name = item
        .get("tool_name")
        .or_else(|| item.get("name"))
        .and_then(|value| value.as_str())
        .ok_or_else(|| ApiError::InternalError("Tool response is missing a capability name".to_string()))?;
    let (upstream_name, id, unique_name) = find_capability_mapping(mapping, presented_name).ok_or_else(|| {
        ApiError::InternalError(format!("Tool '{presented_name}' has no persisted external identifier"))
    })?;
    let obj = item
        .as_object_mut()
        .ok_or_else(|| ApiError::InternalError("Tool response is not an object".to_string()))?;
    obj.insert("name".to_string(), serde_json::json!(unique_name));
    obj.insert("tool_name".to_string(), serde_json::json!(upstream_name));
    obj.insert("unique_name".to_string(), serde_json::json!(unique_name));
    obj.insert("id".to_string(), serde_json::json!(id));
    Ok(item)
}

/// Enrich prompt items with database identifiers
#[inline]
fn enrich_prompt_item(
    item: serde_json::Value,
    mapping: &HashMap<String, (String, String)>,
) -> Result<serde_json::Value, ApiError> {
    let mut item = item;
    let presented_name = item
        .get("prompt_name")
        .or_else(|| item.get("name"))
        .and_then(|value| value.as_str())
        .ok_or_else(|| ApiError::InternalError("Prompt response is missing a capability name".to_string()))?;
    let (upstream_name, id, unique_name) = find_capability_mapping(mapping, presented_name).ok_or_else(|| {
        ApiError::InternalError(format!(
            "Prompt '{presented_name}' has no persisted external identifier"
        ))
    })?;
    let obj = item
        .as_object_mut()
        .ok_or_else(|| ApiError::InternalError("Prompt response is not an object".to_string()))?;
    obj.insert("name".to_string(), serde_json::json!(unique_name));
    obj.insert("prompt_name".to_string(), serde_json::json!(upstream_name));
    obj.insert("unique_name".to_string(), serde_json::json!(unique_name));
    obj.insert("id".to_string(), serde_json::json!(id));
    Ok(item)
}

/// Enrich resource items with database identifiers
#[inline]
fn enrich_resource_item(
    item: serde_json::Value,
    mapping: &HashMap<String, (String, String)>,
) -> Result<serde_json::Value, ApiError> {
    let mut item = item;
    let presented_uri = item
        .get("resource_uri")
        .or_else(|| item.get("uri"))
        .and_then(|value| value.as_str())
        .ok_or_else(|| ApiError::InternalError("Resource response is missing a URI".to_string()))?;
    let (upstream_uri, id, unique_uri) = find_capability_mapping(mapping, presented_uri).ok_or_else(|| {
        ApiError::InternalError(format!(
            "Resource '{presented_uri}' has no persisted external identifier"
        ))
    })?;
    let obj = item
        .as_object_mut()
        .ok_or_else(|| ApiError::InternalError("Resource response is not an object".to_string()))?;
    obj.insert("uri".to_string(), serde_json::json!(unique_uri));
    obj.insert("resource_uri".to_string(), serde_json::json!(upstream_uri));
    obj.insert("unique_uri".to_string(), serde_json::json!(unique_uri));
    obj.insert("id".to_string(), serde_json::json!(id));
    Ok(item)
}

/// Enrich resource template items with database identifiers
#[inline]
fn enrich_resource_template_item(
    item: serde_json::Value,
    mapping: &HashMap<String, (String, String)>,
) -> Result<serde_json::Value, ApiError> {
    let mut item = item;
    let presented_template = ["unique_uri_template", "uri_template", "uriTemplate"]
        .into_iter()
        .filter_map(|field| item.get(field).and_then(|value| value.as_str()))
        .find(|value| !value.trim().is_empty())
        .ok_or_else(|| ApiError::InternalError("Resource template response is missing a template".to_string()))?;
    let (upstream_template, id, unique_name) =
        find_capability_mapping(mapping, presented_template).ok_or_else(|| {
            ApiError::InternalError(format!(
                "Resource template '{presented_template}' has no persisted external identifier"
            ))
        })?;
    let obj = item
        .as_object_mut()
        .ok_or_else(|| ApiError::InternalError("Resource template response is not an object".to_string()))?;
    obj.insert("uri_template".to_string(), serde_json::json!(upstream_template));
    obj.insert("unique_uri_template".to_string(), serde_json::json!(unique_name));
    obj.insert("id".to_string(), serde_json::json!(id));
    Ok(item)
}

fn find_capability_mapping<'a>(
    mapping: &'a HashMap<String, (String, String)>,
    presented_value: &str,
) -> Option<(&'a str, &'a str, &'a str)> {
    mapping
        .iter()
        .find(|(upstream_value, (_, external_value))| {
            upstream_value.as_str() == presented_value || external_value == presented_value
        })
        .map(|(upstream_value, (id, external_value))| (upstream_value.as_str(), id.as_str(), external_value.as_str()))
}

/// Enrich capability items with database-stored identifiers
///
/// Adds `id` and `unique_name` fields to capability items by looking up
/// the corresponding records in the database.
///
/// # Arguments
/// * `capability_type` - Type of capability (Tools, Prompts, Resources, ResourceTemplates)
/// * `pool` - SQLite connection pool
/// * `server_id` - Server identifier to filter records
/// * `items` - JSON objects representing capability items
///
/// # Returns
/// Enhanced JSON objects with `id` and `unique_name` fields added
pub async fn enrich_capability_items(
    capability_type: CapabilityType,
    pool: &Pool<Sqlite>,
    server_id: &str,
    items: Vec<serde_json::Value>,
) -> Result<Vec<serde_json::Value>, ApiError> {
    let enriched = match capability_type {
        CapabilityType::Tools => {
            let mapping = load_tool_mapping(pool, server_id)
                .await
                .map_err(|error| ApiError::InternalError(format!("Failed to load tool naming mappings: {error}")))?;
            items
                .into_iter()
                .map(|item| enrich_tool_item(item, &mapping))
                .collect::<Result<Vec<_>, _>>()?
        }
        CapabilityType::Prompts => {
            let mapping = load_prompt_mapping(pool, server_id)
                .await
                .map_err(|error| ApiError::InternalError(format!("Failed to load prompt naming mappings: {error}")))?;
            items
                .into_iter()
                .map(|item| enrich_prompt_item(item, &mapping))
                .collect::<Result<Vec<_>, _>>()?
        }
        CapabilityType::Resources => {
            let mapping = load_resource_mapping(pool, server_id).await.map_err(|error| {
                ApiError::InternalError(format!("Failed to load resource naming mappings: {error}"))
            })?;
            items
                .into_iter()
                .map(|item| enrich_resource_item(item, &mapping))
                .collect::<Result<Vec<_>, _>>()?
        }
        CapabilityType::ResourceTemplates => {
            let mapping = load_resource_template_mapping(pool, server_id).await.map_err(|error| {
                ApiError::InternalError(format!("Failed to load resource template naming mappings: {error}"))
            })?;
            items
                .into_iter()
                .map(|item| enrich_resource_template_item(item, &mapping))
                .collect::<Result<Vec<_>, _>>()?
        }
    };
    Ok(enriched)
}

pub fn respond_with_enriched(
    data: Vec<serde_json::Value>,
    cache_hit: bool,
    refresh_strategy: Option<RefreshStrategy>,
    source: &str,
) -> Json<serde_json::Value> {
    crate::api::handlers::server::common::create_inspect_response(data, cache_hit, refresh_strategy, source)
}

/// Create standardized JSON representation of a tool
pub fn tool_json(
    name: &str,
    description: Option<String>,
    input_schema: serde_json::Value,
    output_schema: Option<serde_json::Value>,
    icons: Option<Vec<Icon>>,
) -> serde_json::Value {
    let mut item = Map::from_iter([
        ("name".to_string(), name.into()),
        ("inputSchema".to_string(), input_schema),
    ]);
    insert_optional_json(&mut item, "description", description);
    insert_optional_json(&mut item, "outputSchema", output_schema);
    insert_optional_json(&mut item, "icons", icons);
    Value::Object(item)
}

pub fn tool_json_from_cached(
    t: &crate::core::cache::CachedToolInfo,
    external_name: &str,
) -> serde_json::Value {
    let schema = t.input_schema().unwrap_or_else(|_| serde_json::json!({}));
    let out = t.output_schema();
    tool_json(external_name, t.description.clone(), schema, out, t.icons.clone())
}

pub fn tool_management_json_from_cached(t: &crate::core::cache::CachedToolInfo) -> serde_json::Value {
    serde_json::json!({
        "name": t.name,
        "description": t.description,
        "input_schema": t.input_schema().unwrap_or_else(|_| serde_json::json!({})),
        "output_schema": t.output_schema(),
        "unique_name": t.unique_name,
        "icons": t.icons,
    })
}

pub fn resource_json(
    uri: &str,
    name: Option<String>,
    description: Option<String>,
    mime_type: Option<String>,
    icons: Option<Vec<Icon>>,
) -> serde_json::Value {
    let mut item = Map::from_iter([
        ("uri".to_string(), uri.into()),
        ("name".to_string(), name.unwrap_or_else(|| uri.to_string()).into()),
    ]);
    insert_optional_json(&mut item, "description", description);
    insert_optional_json(&mut item, "mimeType", mime_type);
    insert_optional_json(&mut item, "icons", icons);
    Value::Object(item)
}

pub fn resource_json_from_cached(
    r: crate::core::cache::CachedResourceInfo,
    external_uri: &str,
) -> serde_json::Value {
    resource_json(external_uri, r.name, r.description, r.mime_type, r.icons)
}

pub fn resource_template_json(
    uri_template: &str,
    name: Option<String>,
    description: Option<String>,
    mime_type: Option<String>,
) -> serde_json::Value {
    let mut item = Map::from_iter([
        ("uriTemplate".to_string(), uri_template.into()),
        (
            "name".to_string(),
            name.unwrap_or_else(|| uri_template.to_string()).into(),
        ),
    ]);
    insert_optional_json(&mut item, "description", description);
    insert_optional_json(&mut item, "mimeType", mime_type);
    Value::Object(item)
}

pub fn resource_template_json_from_cached(
    t: crate::core::cache::CachedResourceTemplateInfo,
    external_uri_template: &str,
) -> serde_json::Value {
    resource_template_json(external_uri_template, t.name, t.description, t.mime_type)
}

pub fn prompt_json(
    name: &str,
    description: Option<String>,
    arguments: Vec<crate::core::cache::PromptArgument>,
    icons: Option<Vec<Icon>>,
) -> serde_json::Value {
    let args: Vec<serde_json::Value> = arguments
        .into_iter()
        .map(|a| {
            serde_json::json!({
                "name": a.name,
                "description": a.description,
                "required": a.required,
            })
        })
        .collect();

    let mut item = Map::from_iter([("name".to_string(), name.into())]);
    insert_optional_json(&mut item, "description", description);
    if !args.is_empty() {
        item.insert("arguments".to_string(), args.into());
    }
    insert_optional_json(&mut item, "icons", icons);
    Value::Object(item)
}

pub fn prompt_json_from_cached(
    p: crate::core::cache::CachedPromptInfo,
    external_name: &str,
) -> serde_json::Value {
    prompt_json(
        external_name,
        p.description.clone(),
        p.arguments.clone(),
        p.icons.clone(),
    )
}

fn insert_optional_json<T>(
    item: &mut Map<String, Value>,
    key: &str,
    value: Option<T>,
) where
    T: serde::Serialize,
{
    if let Some(value) = value {
        item.insert(
            key.to_string(),
            serde_json::to_value(value).expect("MCP capability fields must serialize"),
        );
    }
}

pub async fn extract_tools_capability(
    conn: &crate::core::pool::UpstreamConnection
) -> Result<ExtractedCapability, ApiError> {
    let now = chrono::Utc::now();

    let (data, tools): (Vec<_>, Vec<_>) = conn
        .tools
        .iter()
        .map(|t| {
            let schema = t.schema_as_json_value();
            let data_item = serde_json::json!({
                "name": t.name,
                "description": t.description,
                "input_schema": schema,
                "output_schema": t.output_schema.as_ref().map(|s| serde_json::Value::Object((**s).clone())),
                "unique_name": serde_json::Value::Null,
                "icons": t.icons.clone(),
            });

            let input_schema_json = serde_json::to_string(&schema).unwrap_or_else(|_| "{}".to_string());
            let tool_info = crate::core::cache::CachedToolInfo {
                name: t.name.to_string(),
                description: t.description.clone().map(|d| d.into_owned()),
                input_schema_json,
                output_schema_json: t.output_schema.as_ref().map(|s| {
                    serde_json::to_string(&serde_json::Value::Object((**s).clone()))
                        .unwrap_or_else(|_| "{}".to_string())
                }),
                unique_name: None,
                icons: t.icons.clone(),
                enabled: true,
                cached_at: now,
            };

            (data_item, tool_info)
        })
        .unzip();

    Ok(ExtractedCapability {
        data,
        tools,
        prompts: Vec::new(),
        resources: Vec::new(),
        resource_templates: Vec::new(),
    })
}

pub async fn extract_prompts_capability(
    conn: &crate::core::pool::UpstreamConnection
) -> Result<ExtractedCapability, ApiError> {
    if !conn.supports_prompts() {
        return Ok(ExtractedCapability::empty());
    }

    let service = match &conn.service {
        Some(service) => service,
        None => return Ok(ExtractedCapability::empty()),
    };

    let list_result = service
        .list_prompts(None)
        .await
        .map_err(|_| ApiError::InternalError("Failed to list prompts".to_string()))?;

    let now = chrono::Utc::now();
    let (data, prompts): (Vec<_>, Vec<_>) = list_result
        .prompts
        .into_iter()
        .map(|p| {
            let arguments = p.arguments.unwrap_or_default();

            let prompt_info = crate::core::cache::CachedPromptInfo {
                name: p.name,
                description: p.description,
                arguments: arguments
                    .clone()
                    .into_iter()
                    .map(|arg| crate::core::cache::PromptArgument {
                        name: arg.name,
                        description: arg.description,
                        required: arg.required.unwrap_or(false),
                    })
                    .collect(),
                icons: p.icons.clone(),
                enabled: true,
                cached_at: now,
            };

            let data_item = serde_json::json!({
                "name": prompt_info.name,
                "description": prompt_info.description,
                "arguments": arguments,
                "icons": prompt_info.icons.clone(),
            });

            (data_item, prompt_info)
        })
        .unzip();

    Ok(ExtractedCapability {
        data,
        tools: Vec::new(),
        prompts,
        resources: Vec::new(),
        resource_templates: Vec::new(),
    })
}

pub async fn extract_resources_capability(
    conn: &crate::core::pool::UpstreamConnection
) -> Result<ExtractedCapability, ApiError> {
    if !conn.supports_resources() {
        return Ok(ExtractedCapability::empty());
    }

    let service = match &conn.service {
        Some(service) => service,
        None => return Ok(ExtractedCapability::empty()),
    };

    let list_result = service
        .list_resources(None)
        .await
        .map_err(|_| ApiError::InternalError("Failed to list resources".to_string()))?;

    let now = chrono::Utc::now();
    let (data, resources): (Vec<_>, Vec<_>) = list_result
        .resources
        .into_iter()
        .map(|r| {
            let raw = &*r;
            let resource_info = crate::core::cache::CachedResourceInfo {
                uri: raw.uri.clone(),
                name: Some(raw.name.clone()),
                description: raw.description.clone(),
                mime_type: raw.mime_type.clone(),
                icons: raw.icons.clone(),
                enabled: true,
                cached_at: now,
            };

            let data_item = serde_json::json!({
                "uri": resource_info.uri,
                "name": resource_info.name,
                "description": resource_info.description,
                "mime_type": resource_info.mime_type,
                "icons": resource_info.icons.clone(),
            });

            (data_item, resource_info)
        })
        .unzip();

    Ok(ExtractedCapability {
        data,
        tools: Vec::new(),
        prompts: Vec::new(),
        resources,
        resource_templates: Vec::new(),
    })
}

pub async fn extract_resource_templates_capability(
    conn: &crate::core::pool::UpstreamConnection
) -> Result<ExtractedCapability, ApiError> {
    if !conn.supports_resources() {
        return Ok(ExtractedCapability::empty());
    }

    let service = match &conn.service {
        Some(service) => service,
        None => return Ok(ExtractedCapability::empty()),
    };

    let now = chrono::Utc::now();
    let mut all_templates = Vec::new();
    let mut cursor = None;

    // Paginated resource template collection
    loop {
        let list_result = service
            .list_resource_templates(Some(rmcp::model::PaginatedRequestParams::default().with_cursor(cursor)))
            .await
            .map_err(|_| ApiError::InternalError("Failed to list resource templates".to_string()))?;

        all_templates.extend(list_result.resource_templates);
        cursor = list_result.next_cursor;

        if cursor.is_none() {
            break;
        }
    }

    let (data, resource_templates): (Vec<_>, Vec<_>) = all_templates
        .into_iter()
        .map(|t| {
            let data_item = serde_json::json!({
                "uri_template": t.uri_template,
                "name": t.name,
                "description": t.description,
                "mime_type": t.mime_type,
            });

            let template_info = crate::core::cache::CachedResourceTemplateInfo {
                uri_template: t.uri_template.clone(),
                name: Some(t.name.clone()),
                description: t.description.clone(),
                mime_type: t.mime_type.clone(),
                enabled: true,
                cached_at: now,
            };

            (data_item, template_info)
        })
        .unzip();

    Ok(ExtractedCapability {
        data,
        tools: Vec::new(),
        prompts: Vec::new(),
        resources: Vec::new(),
        resource_templates,
    })
}

/// Persist the authoritative inventory for one explicitly refreshed kind.
async fn persist_extracted_inventory(
    state: &Arc<AppState>,
    server_info: &ServerIdentification,
    capability_type: CapabilityType,
    extracted: &ExtractedCapability,
) -> Result<(), ApiError> {
    let kinds = match capability_type {
        CapabilityType::Tools => crate::core::pool::CapSyncFlags::TOOLS,
        CapabilityType::Prompts => crate::core::pool::CapSyncFlags::PROMPTS,
        CapabilityType::Resources => crate::core::pool::CapSyncFlags::RESOURCES,
        CapabilityType::ResourceTemplates => crate::core::pool::CapSyncFlags::RESOURCE_TEMPLATES,
    };
    let db = get_database_from_state(state)?;
    if let Err(error) = crate::config::server::capabilities::store_dual_write_for_kinds(
        &db.pool,
        &state.redb_cache,
        &server_info.server_id,
        &server_info.server_name,
        extracted.tools.clone(),
        extracted.resources.clone(),
        extracted.prompts.clone(),
        extracted.resource_templates.clone(),
        None,
        kinds,
    )
    .await
    {
        crate::config::server::namespace_repair::record_capability_collision_from_error(&db.pool, &error).await?;
        return Err(error.into());
    }
    Ok(())
}

/// Create a temporary server instance for capability extraction during force refresh.
pub async fn create_temporary_instance_for_capability(
    state: &Arc<AppState>,
    server_info: &ServerIdentification,
    params: &InspectParams,
    capability_type: CapabilityType,
    allow_without_force: bool,
) -> Result<Option<Json<serde_json::Value>>, ApiError> {
    if params.refresh != Some(RefreshStrategy::Force) && !allow_without_force {
        return Ok(None);
    }

    // Try to reuse an existing connected instance first
    use crate::api::handlers::server::common::ConnectionPoolManager;
    let mut pool = match ConnectionPoolManager::get_pool_for_capability(state).await {
        Ok(pool) => pool,
        Err(_) => return Ok(None),
    };

    if let Some(instances) = pool.connections.get(&server_info.server_id) {
        if let Some(conn) = instances.values().find(|c| c.is_connected()) {
            let extracted = match capability_type {
                CapabilityType::Tools => extract_tools_capability(conn).await?,
                CapabilityType::Prompts => extract_prompts_capability(conn).await?,
                CapabilityType::Resources => extract_resources_capability(conn).await?,
                CapabilityType::ResourceTemplates => extract_resource_templates_capability(conn).await?,
            };

            persist_extracted_inventory(state, server_info, capability_type, &extracted).await?;

            let db = get_database_from_state(state)?;
            let enriched =
                enrich_capability_items(capability_type, &db.pool, &server_info.server_id, extracted.data).await?;
            return Ok(Some(respond_with_enriched(enriched, false, params.refresh, "runtime")));
        }
    }

    // Create temporary validation instance
    match pool
        .get_or_create_validation_instance(&server_info.server_id, "api", std::time::Duration::from_secs(5 * 60))
        .await
    {
        Ok(Some(validation_conn)) => {
            let extracted = match capability_type {
                CapabilityType::Tools => extract_tools_capability(validation_conn).await?,
                CapabilityType::Prompts => extract_prompts_capability(validation_conn).await?,
                CapabilityType::Resources => extract_resources_capability(validation_conn).await?,
                CapabilityType::ResourceTemplates => extract_resource_templates_capability(validation_conn).await?,
            };

            persist_extracted_inventory(state, server_info, capability_type, &extracted).await?;

            let db = get_database_from_state(state)?;
            let items =
                enrich_capability_items(capability_type, &db.pool, &server_info.server_id, extracted.data).await?;

            Ok(Some(respond_with_enriched(items, false, params.refresh, "temporary")))
        }
        _ => Ok(None),
    }
}
