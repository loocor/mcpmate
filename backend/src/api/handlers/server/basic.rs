// MCPMate Proxy API handlers for basic MCP server operations
// Contains handler functions for listing and getting servers

use super::{common, shared::*};
use crate::api::models::server::{
    InstanceListData, InstanceListReq, InstanceListResp, InstanceSummary, ServerCapabilitySummary, ServerDetailsData,
    ServerDetailsReq, ServerDetailsResp, ServerListData, ServerListReq, ServerListResp, ServerMetaInfo,
};
use axum::http::StatusCode;
use sqlx::{Pool, Row, Sqlite};
use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

const SERVER_LIST_RUNTIME_SNAPSHOT_TIMEOUT: Duration = Duration::from_millis(200);

/// Macro to extract database pool from app state with early return on error
macro_rules! get_db_pool {
    ($app_state:expr) => {
        match &$app_state.database {
            Some(db) => db.pool.clone(),
            None => return Err(StatusCode::SERVICE_UNAVAILABLE),
        }
    };
}

/// Whether API should include default HTTP headers in responses (redacted)
fn should_expose_headers() -> bool {
    matches!(
        std::env::var("MCPMATE_API_EXPOSE_HEADERS")
            .unwrap_or_else(|_| "false".to_string())
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "on" | "yes"
    )
}

/// Redact sensitive header values while keeping general visibility
fn redact_headers(input: &std::collections::HashMap<String, String>) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    let sensitive = [
        "authorization",
        "proxy-authorization",
        "x-api-key",
        "api-key",
        "apikey",
        "cookie",
        "set-cookie",
        "x-auth-token",
        "authentication",
    ];
    for (k, v) in input.iter() {
        let lower = k.to_ascii_lowercase();
        if sensitive.iter().any(|s| *s == lower) {
            // Show short masked preview for long tokens, else fully masked
            if v.len() > 12 {
                let (head, tail) = (&v[..6], &v[v.len() - 2..]);
                out.insert(k.clone(), format!("{}***{}", head, tail));
            } else {
                out.insert(k.clone(), "***REDACTED***".to_string());
            }
        } else {
            out.insert(k.clone(), v.clone());
        }
    }
    out
}

async fn detect_auth_mode(
    pool: &sqlx::SqlitePool,
    server_id: &str,
    oauth_configured: bool,
) -> Option<String> {
    let has_header_auth = crate::config::server::get_server_headers(pool, server_id)
        .await
        .ok()
        .map(|headers| {
            headers.keys().any(|key| {
                matches!(
                    key.trim().to_ascii_lowercase().as_str(),
                    "authorization"
                        | "proxy-authorization"
                        | "x-api-key"
                        | "api-key"
                        | "apikey"
                        | "x-auth-token"
                        | "authentication"
                )
            })
        })
        .unwrap_or(false);

    if has_header_auth {
        Some("header".to_string())
    } else if oauth_configured {
        Some("oauth".to_string())
    } else {
        None
    }
}

/// Get details for a specific MCP server
///
/// **Endpoint:** `GET /mcp/servers/details?id={server_id}`
pub async fn server_details(
    State(app_state): State<Arc<AppState>>,
    Query(request): Query<ServerDetailsReq>,
) -> Result<Json<ServerDetailsResp>, StatusCode> {
    let db_pool = get_db_pool!(app_state);
    let result = server_details_core(&request, &db_pool, &app_state).await?;
    Ok(Json(result))
}

/// List all MCP servers with optional filtering
///
/// **Endpoint:** `GET /mcp/servers/list?enabled={bool}&server_type={type}&limit={limit}&offset={offset}`
pub async fn server_list(
    State(app_state): State<Arc<AppState>>,
    Query(request): Query<ServerListReq>,
) -> Result<Json<ServerListResp>, StatusCode> {
    let db_pool = get_db_pool!(app_state);
    let result = server_list_core(&request, &db_pool, &app_state).await?;
    Ok(Json(result))
}

/// List instances for servers
///
/// **Endpoint:** `GET /mcp/servers/instances/list?id={server_id}`
pub async fn instance_list(
    State(app_state): State<Arc<AppState>>,
    Query(request): Query<InstanceListReq>,
) -> Result<Json<InstanceListResp>, StatusCode> {
    let db_pool = get_db_pool!(app_state);
    let result = instance_list_core(&request, &db_pool, &app_state).await?;
    Ok(Json(result))
}

// ==================== Core Business Functions ====================

/// Core business logic for server details operation
async fn server_details_core(
    request: &ServerDetailsReq,
    db_pool: &sqlx::SqlitePool,
    state: &Arc<AppState>,
) -> Result<ServerDetailsResp, StatusCode> {
    // Get the server by ID
    let server = crate::config::server::get_server_by_id(db_pool, &request.id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get server: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let id_opt = server.id.clone();
    let server_id = id_opt.as_deref().unwrap_or_default();
    let name = server.name.clone();

    // Get complete server details using unified function
    let details = common::get_complete_server_details(db_pool, server_id, &name, state).await;
    let enabled = details.globally_enabled;
    let created_at = server.created_at.map(|dt| dt.to_rfc3339());
    let updated_at = server.updated_at.map(|dt| dt.to_rfc3339());

    let mut oauth_status = None;
    let mut oauth_configured = false;
    if server.server_type == crate::common::server::ServerType::StreamableHttp {
        let manager = crate::core::oauth::manager::OAuthManager::new(db_pool.clone());
        if let Ok(status) = manager.get_status(server_id).await {
            oauth_configured = status.configured;
            if status.configured {
                oauth_status = Some(status.state);
            }
        }
    }
    let auth_mode = detect_auth_mode(db_pool, server_id, oauth_configured).await;

    // Optionally expose default headers (redacted)
    let headers = if should_expose_headers() {
        if let Some(ref id) = id_opt {
            match crate::config::server::get_server_headers(db_pool, id).await {
                Ok(map) if !map.is_empty() => Some(redact_headers(&map)),
                _ => None,
            }
        } else {
            None
        }
    } else {
        None
    };

    let server_details = ServerDetailsData {
        id: id_opt,
        name,
        registry_server_id: server.registry_server_id.clone(),
        enabled,
        globally_enabled: details.globally_enabled,
        enabled_in_profile: details.enabled_in_profile,
        server_type: server.server_type,
        command: server.command.clone(),
        url: server.url.clone(),
        args: details.args,
        env: details.env,
        headers,
        meta: details.meta,
        capability: details.capability.clone(),
        protocol_version: details.protocol_version.clone(),
        created_at,
        updated_at,
        instances: details.instances,
        auth_mode,
        oauth_status,
    };

    Ok(ServerDetailsResp::success(server_details))
}

/// Core business logic for server list operation
async fn server_list_core(
    request: &ServerListReq,
    db_pool: &sqlx::SqlitePool,
    state: &Arc<AppState>,
) -> Result<ServerListResp, StatusCode> {
    let all_servers = crate::config::server::get_all_servers(db_pool).await.map_err(|e| {
        tracing::error!("Failed to get servers: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let server_ids: Vec<String> = all_servers.iter().filter_map(|server| server.id.clone()).collect();
    let (instance_map, live_protocol_versions) = snapshot_runtime_state(state).await;
    let protocol_versions = load_cached_protocol_versions(state, &server_ids, live_protocol_versions).await;
    let capability_map = load_server_capabilities(db_pool, &server_ids).await;
    let meta_map = load_server_meta_map(db_pool, &server_ids).await;
    let enabled_in_profile = load_enabled_server_ids(db_pool).await;
    let headers_map = if should_expose_headers() {
        load_server_headers_map(db_pool, &server_ids).await
    } else {
        HashMap::new()
    };

    let offset = request.offset.unwrap_or(0) as usize;
    let limit = request.limit.unwrap_or(100) as usize;
    let mut filtered_servers = Vec::new();
    let mut total_count = 0;
    let mut instance_map = instance_map;
    let mut capability_map = capability_map;
    let mut meta_map = meta_map;
    let mut protocol_versions = protocol_versions;
    let mut headers_map = headers_map;

    for server in all_servers {
        if let Some(ref type_filter) = request.server_type {
            if server.server_type.as_str() != type_filter {
                continue;
            }
        }

        let server_id = server.id.clone().unwrap_or_default();
        let globally_enabled = server.enabled.as_bool();
        if let Some(enabled_filter) = request.enabled {
            if globally_enabled != enabled_filter {
                continue;
            }
        }

        total_count += 1;
        if total_count <= offset || filtered_servers.len() >= limit {
            continue;
        }

        let enabled = globally_enabled;
        let created_at = server.created_at.map(|dt| dt.to_rfc3339());
        let updated_at = server.updated_at.map(|dt| dt.to_rfc3339());
        let enabled_in_profile_flag = globally_enabled && enabled_in_profile.contains(&server_id);
        let instances = instance_map.remove(&server_id).unwrap_or_default();
        let capability = capability_map.remove(&server_id);
        let meta = meta_map.remove(&server_id).flatten();
        let protocol_version = protocol_versions.remove(&server_id).flatten();
        let headers = headers_map.remove(&server_id);

        let mut oauth_status = None;
        let mut oauth_configured = false;
        if server.server_type == crate::common::server::ServerType::StreamableHttp {
            let manager = crate::core::oauth::manager::OAuthManager::new(db_pool.clone());
            if let Ok(status) = manager.get_status(&server_id).await {
                oauth_configured = status.configured;
                if status.configured {
                    oauth_status = Some(status.state);
                }
            }
        }
        let auth_mode = detect_auth_mode(db_pool, &server_id, oauth_configured).await;

        filtered_servers.push(ServerDetailsData {
            id: server.id.clone(),
            name: server.name,
            registry_server_id: server.registry_server_id,
            enabled,
            globally_enabled,
            enabled_in_profile: enabled_in_profile_flag,
            server_type: server.server_type,
            command: server.command,
            url: server.url,
            args: None,
            env: None,
            headers,
            meta,
            capability,
            protocol_version,
            created_at,
            updated_at,
            instances,
            auth_mode,
            oauth_status,
        });
    }

    Ok(ServerListResp::success(ServerListData {
        servers: filtered_servers,
    }))
}

async fn snapshot_runtime_state(
    state: &Arc<AppState>
) -> (HashMap<String, Vec<InstanceSummary>>, HashMap<String, Option<String>>) {
    let pool = match state.connection_pool.try_lock() {
        Ok(pool) => pool,
        Err(_) => {
            match tokio::time::timeout(SERVER_LIST_RUNTIME_SNAPSHOT_TIMEOUT, state.connection_pool.lock()).await {
                Ok(pool) => pool,
                Err(_) => {
                    tracing::warn!(
                        timeout_ms = SERVER_LIST_RUNTIME_SNAPSHOT_TIMEOUT.as_millis(),
                        "Connection pool busy; skipping runtime snapshot for server list"
                    );
                    return (HashMap::new(), HashMap::new());
                }
            }
        }
    };

    let now = std::time::SystemTime::now();
    let connection_snapshot = pool.get_connection_snapshot();
    let protocol_snapshot = pool.get_snapshot();

    let instance_map = connection_snapshot
        .into_iter()
        .map(|(server_id, instances)| {
            let summaries = instances
                .into_iter()
                .map(|(id, conn)| build_instance_summary_from_snapshot(id, conn, now))
                .collect();
            (server_id, summaries)
        })
        .collect();

    let protocol_versions = protocol_snapshot
        .into_iter()
        .map(|(server_id, instances)| {
            let version = instances.into_iter().find_map(|(_, _, _, _, peer)| {
                peer.and_then(|peer| peer.peer_info().map(|info| info.protocol_version.to_string()))
            });
            (server_id, version)
        })
        .collect();

    (instance_map, protocol_versions)
}

fn build_instance_summary_from_snapshot(
    id: String,
    conn: crate::core::pool::UpstreamConnection,
    now: std::time::SystemTime,
) -> InstanceSummary {
    let started_at = chrono::DateTime::<chrono::Utc>::from(now - conn.time_since_creation()).to_rfc3339();
    let connected_at = conn
        .is_connected()
        .then(|| chrono::DateTime::<chrono::Utc>::from(now - conn.time_since_last_connection()).to_rfc3339());

    InstanceSummary {
        id,
        status: conn.status_string(),
        started_at: Some(started_at),
        connected_at,
    }
}

async fn load_enabled_server_ids(pool: &Pool<Sqlite>) -> HashSet<String> {
    let active_profiles = match crate::config::profile::get_active_profile(pool).await {
        Ok(profiles) if !profiles.is_empty() => profiles,
        Ok(_) => match crate::config::profile::get_default_profiles(pool).await {
            Ok(defaults) => defaults.into_iter().filter(|profile| profile.is_active).collect(),
            Err(error) => {
                tracing::warn!(error = %error, "Failed to load default profiles for server list");
                Vec::new()
            }
        },
        Err(error) => {
            tracing::warn!(error = %error, "Failed to load active profiles for server list");
            Vec::new()
        }
    };

    let mut enabled = HashSet::new();
    for profile in active_profiles {
        let Some(profile_id) = profile.id else {
            continue;
        };

        match crate::config::profile::get_profile_servers(pool, &profile_id).await {
            Ok(servers) => {
                for server in servers.into_iter().filter(|server| server.enabled) {
                    enabled.insert(server.server_id);
                }
            }
            Err(error) => {
                tracing::warn!(profile_id = %profile_id, error = %error, "Failed to load profile servers for server list");
            }
        }
    }

    if enabled.is_empty() {
        if let Ok(profile_id) = crate::config::profile::ensure_default_anchor_profile_id(pool).await {
            match crate::config::profile::get_profile_servers(pool, &profile_id).await {
                Ok(servers) => {
                    for server in servers.into_iter().filter(|server| server.enabled) {
                        enabled.insert(server.server_id);
                    }
                }
                Err(error) => {
                    tracing::warn!(profile_id = %profile_id, error = %error, "Failed to load anchor profile servers for server list");
                }
            }
        }
    }

    enabled
}

async fn load_server_capabilities(
    pool: &Pool<Sqlite>,
    server_ids: &[String],
) -> HashMap<String, ServerCapabilitySummary> {
    let capability_rows = match sqlx::query(
        r#"
        SELECT id, capabilities
        FROM server_config
        WHERE id IS NOT NULL
        "#,
    )
    .fetch_all(pool)
    .await
    {
        Ok(rows) => rows,
        Err(error) => {
            tracing::warn!(error = %error, "Failed to load capability flags for server list");
            return HashMap::new();
        }
    };

    let mut summaries = HashMap::new();
    for row in capability_rows {
        let server_id: String = row.get("id");
        let tokens: Option<String> = row.get("capabilities");
        summaries.insert(server_id, capability_summary_from_tokens(tokens.as_deref()));
    }

    merge_capability_counts(pool, &mut summaries, "server_tools", server_ids, |summary, count| {
        summary.tools_count = count;
        if count > 0 {
            summary.supports_tools = true;
        }
    })
    .await;
    merge_capability_counts(pool, &mut summaries, "server_prompts", server_ids, |summary, count| {
        summary.prompts_count = count;
        if count > 0 {
            summary.supports_prompts = true;
        }
    })
    .await;
    merge_capability_counts(
        pool,
        &mut summaries,
        "server_resources",
        server_ids,
        |summary, count| {
            summary.resources_count = count;
            if count > 0 {
                summary.supports_resources = true;
            }
        },
    )
    .await;
    merge_capability_counts(
        pool,
        &mut summaries,
        "server_resource_templates",
        server_ids,
        |summary, count| {
            summary.resource_templates_count = count;
            if count > 0 {
                summary.supports_resources = true;
            }
        },
    )
    .await;

    summaries
}

fn capability_summary_from_tokens(tokens: Option<&str>) -> ServerCapabilitySummary {
    let mut supports_tools = false;
    let mut supports_prompts = false;
    let mut supports_resources = false;

    for token in tokens
        .unwrap_or_default()
        .split(',')
        .map(|token| token.trim().to_ascii_lowercase())
    {
        match token.as_str() {
            "tools" => supports_tools = true,
            "prompts" => supports_prompts = true,
            "resources" => supports_resources = true,
            _ => {}
        }
    }

    ServerCapabilitySummary {
        supports_tools,
        supports_prompts,
        supports_resources,
        tools_count: 0,
        prompts_count: 0,
        resources_count: 0,
        resource_templates_count: 0,
    }
}

async fn merge_capability_counts<F>(
    pool: &Pool<Sqlite>,
    summaries: &mut HashMap<String, ServerCapabilitySummary>,
    table: &str,
    server_ids: &[String],
    mut apply: F,
) where
    F: FnMut(&mut ServerCapabilitySummary, u32),
{
    if server_ids.is_empty() {
        return;
    }

    let query = format!("SELECT server_id, COUNT(*) AS count FROM {table} GROUP BY server_id");
    let rows = match sqlx::query(&query).fetch_all(pool).await {
        Ok(rows) => rows,
        Err(error) => {
            tracing::warn!(table = table, error = %error, "Failed to load capability counts for server list");
            return;
        }
    };

    for row in rows {
        let server_id: String = row.get("server_id");
        let count: i64 = row.get("count");
        let summary = summaries
            .entry(server_id)
            .or_insert_with(|| capability_summary_from_tokens(None));
        apply(summary, count.try_into().unwrap_or(u32::MAX));
    }
}

async fn load_server_meta_map(
    pool: &Pool<Sqlite>,
    server_ids: &[String],
) -> HashMap<String, Option<ServerMetaInfo>> {
    if server_ids.is_empty() {
        return HashMap::new();
    }

    let rows = match sqlx::query(
        r#"
        SELECT *
        FROM server_meta
        "#,
    )
    .fetch_all(pool)
    .await
    {
        Ok(rows) => rows,
        Err(error) => {
            tracing::warn!(error = %error, "Failed to load server metadata map for server list");
            return HashMap::new();
        }
    };

    let mut meta_map = HashMap::new();
    for row in rows {
        let server_meta: crate::config::models::ServerMeta = match sqlx::FromRow::from_row(&row) {
            Ok(meta) => meta,
            Err(error) => {
                tracing::warn!(error = %error, "Failed to decode server metadata row for server list");
                continue;
            }
        };
        let server_id = server_meta.server_id.clone();
        meta_map.insert(server_id, build_server_meta_info(server_meta));
    }
    meta_map
}

fn build_server_meta_info(server_meta: crate::config::models::ServerMeta) -> Option<ServerMetaInfo> {
    let icons = match server_meta.icons_json.as_deref() {
        Some(raw) => match crate::api::handlers::server::common::parse_server_icons(raw) {
            Ok(list) => list,
            Err(error) => {
                tracing::warn!(error = %error, server_id = %server_meta.server_id, "Failed to parse icons for server list");
                None
            }
        },
        None => None,
    };

    let repository = server_meta.repository.as_deref().and_then(|raw| match serde_json::from_str(raw) {
        Ok(repo) => Some(repo),
        Err(error) => {
            tracing::warn!(error = %error, server_id = %server_meta.server_id, "Failed to parse repository metadata for server list");
            None
        }
    });

    let registry_meta = server_meta
        .registry_meta_json
        .as_deref()
        .and_then(|raw| match serde_json::from_str(raw) {
            Ok(meta) => Some(meta),
            Err(error) => {
                tracing::warn!(error = %error, server_id = %server_meta.server_id, "Failed to parse registry meta block for server list");
                None
            }
        });

    let mut extras: Option<serde_json::Value> =
        server_meta
            .extras_json
            .as_deref()
            .and_then(|raw| match serde_json::from_str(raw) {
                Ok(value) => Some(value),
                Err(error) => {
                    tracing::warn!(error = %error, server_id = %server_meta.server_id, "Failed to parse extras metadata for server list");
                    None
                }
            });

    if extras.is_none()
        && (server_meta.author.is_some()
            || server_meta.category.is_some()
            || server_meta.recommended_scenario.is_some()
            || server_meta.rating.is_some())
    {
        let mut legacy = serde_json::Map::new();
        if let Some(author) = server_meta.author {
            legacy.insert("author".to_string(), serde_json::Value::String(author));
        }
        if let Some(category) = server_meta.category {
            legacy.insert("category".to_string(), serde_json::Value::String(category));
        }
        if let Some(scene) = server_meta.recommended_scenario {
            legacy.insert("recommended_scenario".to_string(), serde_json::Value::String(scene));
        }
        if let Some(rating) = server_meta.rating {
            legacy.insert("rating".to_string(), serde_json::Value::Number(rating.into()));
        }
        if !legacy.is_empty() {
            let mut wrapper = serde_json::Map::new();
            wrapper.insert("legacy".to_string(), serde_json::Value::Object(legacy));
            extras = Some(serde_json::Value::Object(wrapper));
        }
    }

    Some(ServerMetaInfo {
        description: server_meta.description,
        version: server_meta.registry_version,
        website_url: server_meta.website,
        repository,
        meta: registry_meta,
        extras,
        icons,
    })
}

async fn load_cached_protocol_versions(
    state: &Arc<AppState>,
    server_ids: &[String],
    mut protocol_versions: HashMap<String, Option<String>>,
) -> HashMap<String, Option<String>> {
    for server_id in server_ids {
        if protocol_versions
            .get(server_id)
            .and_then(|version| version.clone())
            .is_some()
        {
            continue;
        }

        let query = crate::core::cache::CacheQuery {
            server_id: server_id.clone(),
            freshness_level: crate::core::cache::FreshnessLevel::Cached,
            include_disabled: true,
            scope: crate::core::cache::CacheScope::shared_raw(),
        };

        let version = state.redb_cache.get_server_data(&query).await.ok().and_then(|result| {
            result.data.and_then(|data| {
                if data.protocol_version.is_empty() {
                    None
                } else {
                    Some(data.protocol_version)
                }
            })
        });

        protocol_versions.insert(server_id.clone(), version);
    }

    protocol_versions
}

async fn load_server_headers_map(
    pool: &Pool<Sqlite>,
    server_ids: &[String],
) -> HashMap<String, HashMap<String, String>> {
    let mut headers_map = HashMap::new();
    for server_id in server_ids {
        match crate::config::server::get_server_headers(pool, server_id).await {
            Ok(headers) if !headers.is_empty() => {
                headers_map.insert(server_id.clone(), redact_headers(&headers));
            }
            Ok(_) => {}
            Err(error) => {
                tracing::warn!(server_id = %server_id, error = %error, "Failed to load server headers for server list");
            }
        }
    }
    headers_map
}

/// Core business logic for instance list operation
async fn instance_list_core(
    request: &InstanceListReq,
    db_pool: &sqlx::SqlitePool,
    state: &Arc<AppState>,
) -> Result<InstanceListResp, StatusCode> {
    if let Some(ref server_id) = request.id {
        // List instances for specific server
        let server = crate::config::server::get_server_by_id(db_pool, server_id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to get server: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .ok_or(StatusCode::NOT_FOUND)?;

        let name = server.name;
        let instance_summaries = common::get_server_instances(state, server_id).await;

        Ok(InstanceListResp::success(InstanceListData {
            name,
            instances: instance_summaries,
        }))
    } else {
        // List all instances for all servers
        use crate::api::handlers::server::common::ConnectionPoolManager;
        let pool = match ConnectionPoolManager::get_pool_for_health_check(state).await {
            Ok(pool) => pool,
            Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
        };

        let mut all_instances = Vec::new();
        for instances in pool.connections.values() {
            for (instance_id, conn) in instances {
                let now = std::time::SystemTime::now();
                all_instances.push(build_instance_summary_from_live_connection(
                    instance_id.clone(),
                    conn,
                    now,
                ));
            }
        }

        Ok(InstanceListResp::success(InstanceListData {
            name: "all".to_string(),
            instances: all_instances,
        }))
    }
}

fn build_instance_summary_from_live_connection(
    id: String,
    conn: &crate::core::pool::UpstreamConnection,
    now: std::time::SystemTime,
) -> InstanceSummary {
    let started_at = chrono::DateTime::<chrono::Utc>::from(now - conn.created_at.elapsed()).to_rfc3339();
    let connected_at = conn
        .is_connected()
        .then(|| chrono::DateTime::<chrono::Utc>::from(now - conn.last_connected.elapsed()).to_rfc3339());

    InstanceSummary {
        id,
        status: conn.status_string(),
        started_at: Some(started_at),
        connected_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        common::{profile::ProfileType, server::ServerType},
        config::{
            database::Database,
            models::{Profile, Server},
            profile, server,
        },
        core::{
            cache::{RedbCacheManager, manager::CacheConfig},
            models::Config,
            pool::{UpstreamConnection, UpstreamConnectionPool},
            profile::ConfigApplicationStateManager,
        },
        inspector::{calls::InspectorCallRegistry, sessions::InspectorSessionManager},
        system::metrics::MetricsCollector,
    };
    use sqlx::sqlite::SqlitePoolOptions;
    use std::{path::PathBuf, sync::Arc, time::Duration};
    use tempfile::TempDir;
    use tokio::{sync::Mutex, time::Instant};

    struct TestContext {
        _temp_dir: TempDir,
        app_state: Arc<AppState>,
        db_pool: sqlx::SqlitePool,
    }

    #[tokio::test]
    async fn server_list_returns_quickly_when_pool_lock_is_held() {
        let context = create_test_context().await;
        seed_servers(&context.db_pool, 24).await;

        let request = ServerListReq {
            enabled: None,
            server_type: None,
            limit: Some(100),
            offset: Some(0),
        };

        let lock_guard = context.app_state.connection_pool.lock().await;
        let started = Instant::now();
        let response = server_list_core(&request, &context.db_pool, &context.app_state)
            .await
            .expect("server list should degrade gracefully");
        let elapsed = started.elapsed();
        drop(lock_guard);

        assert!(response.success);
        let data = response.data.expect("server list data");
        assert_eq!(data.servers.len(), 24);
        assert!(
            elapsed < Duration::from_secs(2),
            "server list should not wait for API lock timeout: {elapsed:?}"
        );
        assert!(data.servers.iter().all(|server| server.instances.is_empty()));
        assert!(data.servers.iter().all(|server| server.protocol_version.is_none()));
    }

    #[tokio::test]
    async fn server_list_waits_briefly_for_runtime_snapshot_before_degrading() {
        let context = create_test_context().await;
        let server_id = seed_servers(&context.db_pool, 1)
            .await
            .into_iter()
            .next()
            .expect("server id");
        seed_runtime_instance(&context.app_state, &server_id).await;

        let request = ServerListReq {
            enabled: None,
            server_type: None,
            limit: Some(10),
            offset: Some(0),
        };

        let app_state = context.app_state.clone();
        let (locked_tx, locked_rx) = tokio::sync::oneshot::channel();
        let holder = tokio::spawn(async move {
            let _guard = app_state.connection_pool.lock().await;
            let _ = locked_tx.send(());
            tokio::time::sleep(Duration::from_millis(50)).await;
        });
        locked_rx.await.expect("lock acquired");

        let started = Instant::now();
        let response = server_list_core(&request, &context.db_pool, &context.app_state)
            .await
            .expect("server list should wait briefly for runtime snapshot");
        let elapsed = started.elapsed();
        holder.await.expect("holder task");

        assert!(response.success);
        let data = response.data.expect("server list data");
        assert_eq!(data.servers.len(), 1);
        assert!(
            elapsed >= Duration::from_millis(40),
            "server list should wait briefly for runtime snapshot: {elapsed:?}"
        );
        assert!(
            elapsed < Duration::from_secs(1),
            "server list should not wait for full API lock timeout: {elapsed:?}"
        );
        assert_eq!(data.servers[0].instances.len(), 1);
    }

    #[tokio::test]
    async fn server_list_preserves_list_metadata_without_full_details() {
        let context = create_test_context().await;
        let server_id = seed_servers(&context.db_pool, 1)
            .await
            .into_iter()
            .next()
            .expect("server id");
        seed_server_meta(&context.db_pool, &server_id).await;
        seed_server_capabilities(&context.db_pool, &server_id).await;
        seed_profile_enablement(&context.db_pool, &server_id).await;
        seed_runtime_instance(&context.app_state, &server_id).await;

        let request = ServerListReq {
            enabled: Some(true),
            server_type: Some("stdio".to_string()),
            limit: Some(10),
            offset: Some(0),
        };

        let response = server_list_core(&request, &context.db_pool, &context.app_state)
            .await
            .expect("server list should succeed");
        let data = response.data.expect("server list data");
        let server = data.servers.into_iter().next().expect("server item");

        assert_eq!(server.id.as_deref(), Some(server_id.as_str()));
        assert!(server.enabled);
        assert!(server.globally_enabled);
        assert!(server.enabled_in_profile);
        assert_eq!(server.args, None);
        assert_eq!(server.env, None);
        assert_eq!(server.instances.len(), 1);
        assert_eq!(server.instances[0].status, "Ready");
        assert_eq!(server.capability.as_ref().map(|summary| summary.tools_count), Some(1));
        assert_eq!(server.capability.as_ref().map(|summary| summary.prompts_count), Some(1));
        assert_eq!(
            server.meta.as_ref().and_then(|meta| meta.description.as_deref()),
            Some("Server description")
        );
        assert_eq!(
            server
                .meta
                .as_ref()
                .and_then(|meta| meta.icons.as_ref())
                .map(|icons| icons.len()),
            Some(1)
        );
    }

    async fn create_test_context() -> TestContext {
        let temp_dir = TempDir::new().expect("temp dir");
        let db_pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("sqlite pool");

        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&db_pool)
            .await
            .expect("enable foreign keys");

        crate::config::server::init::initialize_server_tables(&db_pool)
            .await
            .expect("init server tables");
        crate::config::profile::init::initialize_profile_tables(&db_pool)
            .await
            .expect("init profile tables");
        crate::core::capability::naming::initialize(db_pool.clone());

        let database = Arc::new(Database {
            pool: db_pool.clone(),
            path: PathBuf::from(":memory:"),
        });

        let cache_path = temp_dir.path().join("capability.redb");
        let redb_cache = Arc::new(RedbCacheManager::new(cache_path, CacheConfig::default()).expect("cache manager"));

        let app_state = Arc::new(AppState {
            connection_pool: Arc::new(Mutex::new(UpstreamConnectionPool::new(
                Arc::new(Config::default()),
                Some(database.clone()),
            ))),
            metrics_collector: Arc::new(MetricsCollector::new(Duration::from_secs(5))),
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
            oauth_manager: Some(Arc::new(crate::core::oauth::OAuthManager::new(db_pool.clone()))),
        });

        TestContext {
            _temp_dir: temp_dir,
            app_state,
            db_pool,
        }
    }

    async fn seed_servers(
        pool: &sqlx::SqlitePool,
        count: usize,
    ) -> Vec<String> {
        let mut ids = Vec::with_capacity(count);
        for index in 0..count {
            let server = Server {
                id: None,
                name: format!("server-{index}"),
                server_type: ServerType::Stdio,
                command: Some("echo".to_string()),
                url: None,
                registry_server_id: None,
                capabilities: Some("tools,prompts,resources".to_string()),
                enabled: crate::common::status::EnabledStatus::Enabled,
                created_at: None,
                updated_at: None,
                pending_import: false,
            };
            let id = server::upsert_server(pool, &server).await.expect("insert server");
            ids.push(id);
        }
        ids
    }

    async fn seed_server_meta(
        pool: &sqlx::SqlitePool,
        server_id: &str,
    ) {
        let meta = crate::config::models::ServerMeta {
            id: None,
            server_id: server_id.to_string(),
            description: Some("Server description".to_string()),
            website: Some("https://example.com".to_string()),
            repository: Some(r#"{"url":"https://example.com/repo","source":"github"}"#.to_string()),
            registry_version: Some("1.2.3".to_string()),
            registry_meta_json: None,
            extras_json: None,
            author: None,
            category: None,
            recommended_scenario: None,
            rating: None,
            icons_json: Some(r#"[{"src":"https://example.com/icon.png","mimeType":"image/png"}]"#.to_string()),
            server_version: None,
            protocol_version: None,
            created_at: None,
            updated_at: None,
        };

        server::upsert_server_meta(pool, &meta)
            .await
            .expect("upsert server meta");
    }

    async fn seed_server_capabilities(
        pool: &sqlx::SqlitePool,
        server_id: &str,
    ) {
        server::tools::upsert_server_tool(pool, server_id, "server-0", "tool-a", Some("tool"), None)
            .await
            .expect("insert tool");
        sqlx::query(
            "INSERT INTO server_prompts (id, server_id, server_name, prompt_name, unique_name) VALUES (?, ?, ?, ?, ?)",
        )
        .bind("sprm-test")
        .bind(server_id)
        .bind("server-0")
        .bind("prompt-a")
        .bind("server-0__prompt-a")
        .execute(pool)
        .await
        .expect("insert prompt");
    }

    async fn seed_profile_enablement(
        pool: &sqlx::SqlitePool,
        server_id: &str,
    ) {
        let mut profile = Profile::new_with_description(
            "default".to_string(),
            Some("default profile".to_string()),
            ProfileType::Shared,
        );
        profile.is_active = true;
        profile.is_default = true;
        profile.multi_select = true;
        let profile_id = profile::upsert_profile(pool, &profile).await.expect("upsert profile");
        profile::add_server_to_profile(pool, &profile_id, server_id, true)
            .await
            .expect("attach server to profile");
    }

    async fn seed_runtime_instance(
        app_state: &Arc<AppState>,
        server_id: &str,
    ) {
        let mut pool = app_state.connection_pool.lock().await;
        let mut instance = UpstreamConnection::new("server-0".to_string());
        instance.status = crate::core::foundation::types::ConnectionStatus::Ready;
        pool.connections.insert(
            server_id.to_string(),
            HashMap::from([("instance-1".to_string(), instance)]),
        );
    }
}
