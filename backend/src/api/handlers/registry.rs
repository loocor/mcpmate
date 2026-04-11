use std::{sync::Arc, time::Duration};

use axum::{Json, extract::Query, extract::State};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::ApiError;
use crate::api::models::server::{ServerDetailsData, ServerDetailsResp, ServerIdReq, ServersImportData};
use crate::api::routes::AppState;
use crate::config::registry::cache::RegistryCacheEntry;
use crate::config::registry::sync::{RegistryPackage as CachedRegistryPackage, RegistryRemote as CachedRegistryRemote};
use crate::config::registry::{RegistryCacheService, RegistrySyncService};
use crate::config::server::import::{ImportOptions, build_meta_from_entry, import_from_registry, upsert_import_meta};

#[derive(Debug, Deserialize, Clone)]
pub struct RegistryServersQuery {
    pub limit: Option<u32>,
    pub cursor: Option<String>,
    pub search: Option<String>,
    pub version: Option<String>,
    pub updated_since: Option<String>,
    pub include_deleted: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RegistryInstallRequest {
    pub name: String,
    pub version: Option<String>,
    #[serde(default)]
    pub target_profile_id: Option<String>,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Serialize)]
pub struct RegistryInstallResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<ServersImportData>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CachedRegistryServersQuery {
    pub limit: Option<u32>,
    pub cursor: Option<String>,
    pub search: Option<String>,
    pub include_deleted: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct CachedRegistryServersResponse {
    pub servers: Vec<CachedRegistryServerWrapper>,
    pub metadata: CachedRegistryMetadata,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_synced_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CachedRegistryMetadata {
    #[serde(rename = "nextCursor", skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct CachedRegistryServerWrapper {
    pub server: CachedRegistryServer,
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CachedRegistryServer {
    pub name: String,
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub description: Option<String>,
    pub version: String,
    pub status: Option<String>,
    #[serde(rename = "websiteUrl", skip_serializing_if = "Option::is_none")]
    pub website_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icons: Option<Vec<CachedRegistryIcon>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remotes: Option<Vec<CachedRegistryTransport>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub packages: Option<Vec<CachedRegistryPackagePayload>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CachedRegistryIcon {
    pub src: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CachedRegistryTransport {
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CachedRegistryPackagePayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport: Option<CachedRegistryPackageTransport>,
}

#[derive(Debug, Serialize)]
pub struct CachedRegistryPackageTransport {
    pub r#type: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrySyncResponse {
    pub success: bool,
    pub updated_count: usize,
    pub last_synced_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RegistryOfficialMeta {
    pub server_id: String,
    pub version_id: String,
    pub published_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_changed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_latest: Option<bool>,
}

pub async fn list_cached_servers(
    State(state): State<Arc<AppState>>,
    Query(query): Query<CachedRegistryServersQuery>,
) -> Result<Json<CachedRegistryServersResponse>, ApiError> {
    let db = state
        .database
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Database not available".to_string()))?;

    let cache_service = RegistryCacheService::new(db.pool.clone());
    let limit = query.limit.unwrap_or(30).clamp(1, 100) as usize;
    let cursor = query.cursor.as_deref().filter(|value| !value.trim().is_empty());
    let include_deleted = query.include_deleted.unwrap_or(false);

    let (entries, next_cursor, count) = match query.search.as_deref().map(str::trim) {
        Some(search) if !search.is_empty() => {
            let result = cache_service
                .search_local(search, limit as u32, cursor)
                .await
                .map_err(|err| ApiError::InternalError(format!("Failed to query registry cache: {err}")))?;
            let total = if result.total < 0 { 0 } else { result.total as usize };
            (result.servers, result.next_cursor, total)
        }
        _ => {
            let all_entries = cache_service
                .list_all(if include_deleted { None } else { Some("active") })
                .await
                .map_err(|err| ApiError::InternalError(format!("Failed to list registry cache: {err}")))?;

            let mut filtered = all_entries
                .into_iter()
                .filter(|entry| cursor.is_none_or(|current| entry.server_name.as_str() > current));

            let mut page_entries = Vec::with_capacity(limit);
            for entry in filtered.by_ref().take(limit) {
                page_entries.push(entry);
            }
            let has_more = filtered.next().is_some();
            let next = if has_more {
                page_entries.last().map(|entry| entry.server_name.clone())
            } else {
                None
            };

            let page_count = page_entries.len();
            (page_entries, next, page_count)
        }
    };

    let servers = entries.into_iter().map(cache_entry_to_server_wrapper).collect();
    let last_synced_at = cache_service
        .last_sync_time()
        .await
        .map_err(|err| ApiError::InternalError(format!("Failed to load last sync time: {err}")))?
        .map(|ts| ts.to_rfc3339());

    Ok(Json(CachedRegistryServersResponse {
        servers,
        metadata: CachedRegistryMetadata { next_cursor, count },
        last_synced_at,
    }))
}

pub async fn sync_registry(State(state): State<Arc<AppState>>) -> Result<Json<RegistrySyncResponse>, ApiError> {
    let db = state
        .database
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Database not available".to_string()))?;

    let sync_service = RegistrySyncService::new(db.pool.clone());
    let updated_count = sync_service
        .sync_incremental()
        .await
        .map_err(|err| ApiError::InternalError(format!("Failed to sync registry cache: {err}")))?;

    let cache_service = RegistryCacheService::new(db.pool.clone());
    let last_synced_at = cache_service
        .last_sync_time()
        .await
        .map_err(|err| ApiError::InternalError(format!("Failed to load last sync time: {err}")))?
        .unwrap_or_else(Utc::now)
        .to_rfc3339();

    Ok(Json(RegistrySyncResponse {
        success: true,
        updated_count,
        last_synced_at,
    }))
}

fn cache_entry_to_server_wrapper(entry: RegistryCacheEntry) -> CachedRegistryServerWrapper {
    let remotes = parse_remotes(entry.remotes_json.as_deref());
    let packages = parse_packages(entry.packages_json.as_deref());
    let repository = parse_json_value(entry.repository_json.as_deref());
    let icons = parse_icons(entry.icons_json.as_deref());
    let status = if entry.status.is_empty() {
        None
    } else {
        Some(entry.status.clone())
    };

    let official_meta = RegistryOfficialMeta {
        server_id: entry.server_name.clone(),
        version_id: format!("{}@{}", entry.server_name, entry.version),
        published_at: entry
            .published_at
            .or(entry.updated_at)
            .unwrap_or(entry.synced_at)
            .to_rfc3339(),
        status: Some(entry.status.clone()),
        status_changed_at: None,
        updated_at: entry.updated_at.map(|value| value.to_rfc3339()),
        is_latest: Some(true),
    };

    let mut metadata = parse_meta_object(entry.meta_json.as_deref()).unwrap_or_default();
    if let Ok(mut official_value) = serde_json::to_value(official_meta) {
        if let Some(existing) = metadata.get("io.modelcontextprotocol.registry/official") {
            merge_json_values(&mut official_value, existing.clone());
        }
        metadata.insert("io.modelcontextprotocol.registry/official".to_string(), official_value);
    }

    CachedRegistryServerWrapper {
        server: CachedRegistryServer {
            name: entry.server_name,
            schema: entry.schema_url,
            title: entry.title,
            description: entry.description,
            version: entry.version,
            status,
            website_url: entry.website_url,
            repository,
            icons,
            remotes,
            packages,
        },
        meta: if metadata.is_empty() {
            None
        } else {
            Some(Value::Object(metadata))
        },
    }
}

fn merge_json_values(
    target: &mut Value,
    source: Value,
) {
    match (target, source) {
        (Value::Object(target_map), Value::Object(source_map)) => {
            for (key, value) in source_map {
                target_map.entry(key).or_insert(value);
            }
        }
        (target_value, source_value) => {
            *target_value = source_value;
        }
    }
}

fn parse_json_value(raw: Option<&str>) -> Option<Value> {
    raw.and_then(|source| serde_json::from_str::<Value>(source).ok())
}

fn parse_meta_object(raw: Option<&str>) -> Option<serde_json::Map<String, Value>> {
    let value = raw.and_then(|source| serde_json::from_str::<Value>(source).ok())?;
    match value {
        Value::Object(object) => Some(object),
        _ => None,
    }
}

fn parse_remotes(raw: Option<&str>) -> Option<Vec<CachedRegistryTransport>> {
    let parsed = raw.and_then(|source| serde_json::from_str::<Vec<CachedRegistryRemote>>(source).ok())?;
    let remotes: Vec<CachedRegistryTransport> = parsed
        .into_iter()
        .filter_map(|remote| {
            let remote_type = remote
                .r#type
                .and_then(|value| {
                    let trimmed = value.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                })
                .unwrap_or_else(|| "streamable_http".to_string());

            if remote.url.as_deref().is_none_or(str::is_empty) {
                return None;
            }

            Some(CachedRegistryTransport {
                r#type: remote_type,
                url: remote.url,
            })
        })
        .collect();

    if remotes.is_empty() { None } else { Some(remotes) }
}

fn parse_packages(raw: Option<&str>) -> Option<Vec<CachedRegistryPackagePayload>> {
    let parsed = raw.and_then(|source| serde_json::from_str::<Vec<CachedRegistryPackage>>(source).ok())?;
    let packages: Vec<CachedRegistryPackagePayload> = parsed
        .into_iter()
        .map(|package| CachedRegistryPackagePayload {
            registry_type: Some("npm".to_string()),
            identifier: package.name,
            version: package.version,
            transport: Some(CachedRegistryPackageTransport {
                r#type: "stdio".to_string(),
            }),
        })
        .collect();

    if packages.is_empty() { None } else { Some(packages) }
}

fn parse_icons(raw: Option<&str>) -> Option<Vec<CachedRegistryIcon>> {
    let parsed = raw.and_then(|source| serde_json::from_str::<Vec<CachedRegistryRemoteIcon>>(source).ok())?;
    let icons: Vec<CachedRegistryIcon> = parsed
        .into_iter()
        .filter_map(|icon| {
            let src = icon.url?.trim().to_string();
            if src.is_empty() {
                return None;
            }
            Some(CachedRegistryIcon { src, alt: icon.alt })
        })
        .collect();

    if icons.is_empty() { None } else { Some(icons) }
}

#[derive(Debug, Deserialize)]
struct CachedRegistryRemoteIcon {
    url: Option<String>,
    alt: Option<String>,
}

pub async fn list_servers(
    State(_state): State<Arc<AppState>>,
    Query(query): Query<RegistryServersQuery>,
) -> Result<Json<Value>, ApiError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("MCPMate/0.1.0 (+https://mcp.umate.ai)")
        .build()
        .map_err(|err| ApiError::InternalError(format!("Failed to init HTTP client: {err}")))?;

    let mut params: Vec<(String, String)> = Vec::new();

    let limit = query.limit.unwrap_or(30).clamp(1, 100);
    params.push(("limit".to_string(), limit.to_string()));

    let version = query
        .version
        .clone()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "latest".to_string());
    params.push(("version".to_string(), version));

    if let Some(cursor) = query.cursor.filter(|v| !v.trim().is_empty()) {
        params.push(("cursor".to_string(), cursor));
    }

    if let Some(search) = query.search.filter(|v| !v.trim().is_empty()) {
        params.push(("search".to_string(), search));
    }

    if let Some(updated_since) = query.updated_since.filter(|v| !v.trim().is_empty()) {
        params.push(("updated_since".to_string(), updated_since));
    }

    if query.include_deleted.unwrap_or(false) {
        params.push(("include_deleted".to_string(), "true".to_string()));
    }

    let response = client
        .get("https://registry.modelcontextprotocol.io/v0.1/servers")
        .query(&params)
        .send()
        .await
        .map_err(|err| ApiError::InternalError(format!("Registry request failed: {err}")))?;

    if !response.status().is_success() {
        return Err(ApiError::InternalError(format!(
            "Registry responded with status {}",
            response.status()
        )));
    }

    let payload: Value = response
        .json()
        .await
        .map_err(|err| ApiError::InternalError(format!("Failed to decode registry payload: {err}")))?;

    Ok(Json(payload))
}

pub async fn install_server(
    State(state): State<Arc<AppState>>,
    Json(request): Json<RegistryInstallRequest>,
) -> Result<Json<RegistryInstallResponse>, ApiError> {
    let db = state
        .database
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Database not available".to_string()))?;

    let cache_service = RegistryCacheService::new(db.pool.clone());

    let opts = ImportOptions {
        by_name: true,
        by_fingerprint: true,
        conflict_policy: crate::config::server::import::ConflictPolicy::Skip,
        preview: request.dry_run,
        target_profile: request.target_profile_id.clone(),
    };

    let outcome = import_from_registry(
        &db.pool,
        &state.connection_pool,
        &state.redb_cache,
        &cache_service,
        &request.name,
        request.version.as_deref(),
        opts,
    )
    .await
    .map_err(|e| ApiError::BadRequest(format!("Failed to import server: {}", e)))?;

    let imported_count = outcome.imported.len();
    let skipped_count = outcome.skipped.len();
    let failed_count = outcome.failed.len();

    let data = ServersImportData {
        imported_count,
        imported_servers: outcome.imported.iter().map(|s| s.name.clone()).collect(),
        skipped_count,
        skipped_servers: outcome
            .skipped
            .iter()
            .map(|s| crate::api::models::server::SkippedServerData {
                name: s.name.clone(),
                reason: match s.reason {
                    crate::config::server::import::SkipReason::DuplicateName => "duplicate_name".to_string(),
                    crate::config::server::import::SkipReason::DuplicateFingerprint => {
                        "duplicate_fingerprint".to_string()
                    }
                    crate::config::server::import::SkipReason::UrlQueryMismatch { .. } => {
                        "url_query_mismatch".to_string()
                    }
                },
                existing_query: match &s.reason {
                    crate::config::server::import::SkipReason::UrlQueryMismatch { existing_query, .. } => {
                        existing_query.clone()
                    }
                    _ => None,
                },
                incoming_query: match &s.reason {
                    crate::config::server::import::SkipReason::UrlQueryMismatch { incoming_query, .. } => {
                        incoming_query.clone()
                    }
                    _ => None,
                },
            })
            .collect(),
        failed_count,
        failed_servers: outcome.failed.keys().cloned().collect(),
        error_details: if outcome.failed.is_empty() {
            None
        } else {
            Some(outcome.failed)
        },
    };

    let message = if request.dry_run {
        format!(
            "Preview: would import {} server(s), skip {} server(s), fail {} server(s)",
            imported_count, skipped_count, failed_count
        )
    } else if imported_count > 0 {
        format!("Successfully imported {} server(s)", imported_count)
    } else if skipped_count > 0 {
        format!("Skipped {} server(s) (duplicates)", skipped_count)
    } else {
        format!("Failed to import server: {} errors", failed_count)
    };

    Ok(Json(RegistryInstallResponse {
        success: imported_count > 0,
        message,
        data: Some(data),
    }))
}

pub async fn refresh_managed_server_metadata(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ServerIdReq>,
) -> Result<Json<ServerDetailsResp>, ApiError> {
    let db = state
        .database
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Database not available".to_string()))?;

    let server = crate::config::server::get_server_by_id(&db.pool, &request.id)
        .await
        .map_err(|err| ApiError::InternalError(format!("Failed to load managed server: {err}")))?
        .ok_or_else(|| ApiError::NotFound(format!("Server with ID '{}' not found", request.id)))?;

    let server_id = server
        .id
        .clone()
        .ok_or_else(|| ApiError::InternalError("Server found but missing ID".to_string()))?;
    let registry_server_id = server
        .registry_server_id
        .clone()
        .ok_or_else(|| ApiError::BadRequest("Managed server is missing registry_server_id".to_string()))?;

    let sync_service = RegistrySyncService::new(db.pool.clone());
    if let Err(err) = sync_service.sync_incremental().await {
        tracing::warn!(
            registry_server_id = %registry_server_id,
            error = %err,
            "Failed to sync registry before refreshing managed server metadata"
        );
    }

    let cache_service = RegistryCacheService::new(db.pool.clone());
    let entry = cache_service
        .get_by_name(&registry_server_id)
        .await
        .map_err(|err| ApiError::InternalError(format!("Failed to query registry cache: {err}")))?
        .ok_or_else(|| ApiError::NotFound(format!("Registry cache entry '{}' not found", registry_server_id)))?;

    let meta_payload = build_meta_from_entry(&entry);
    upsert_import_meta(&db.pool, &server_id, &meta_payload)
        .await
        .map_err(|err| ApiError::InternalError(format!("Failed to refresh server metadata: {err}")))?;

    let refreshed = crate::config::server::get_server_by_id(&db.pool, &server_id)
        .await
        .map_err(|err| ApiError::InternalError(format!("Failed to reload managed server: {err}")))?
        .ok_or_else(|| ApiError::InternalError("Server disappeared after metadata refresh".to_string()))?;

    let details = crate::api::handlers::server::common::get_complete_server_details(
        &db.pool,
        &server_id,
        &refreshed.name,
        &state,
    )
    .await;

    let mut oauth_status = None;
    if refreshed.server_type == crate::common::server::ServerType::StreamableHttp {
        let manager = crate::core::oauth::manager::OAuthManager::new(db.pool.clone());
        if let Ok(status) = manager.get_status(&server_id).await {
            if status.configured {
                oauth_status = Some(status.state);
            }
        }
    }

    Ok(Json(ServerDetailsResp::success(ServerDetailsData {
        id: Some(server_id),
        name: refreshed.name,
        registry_server_id: refreshed.registry_server_id,
        enabled: details.globally_enabled && details.enabled_in_profile,
        globally_enabled: details.globally_enabled,
        enabled_in_profile: details.enabled_in_profile,
        unify_direct_exposure_eligible: refreshed.unify_direct_exposure_eligible,
        server_type: refreshed.server_type,
        command: refreshed.command,
        url: refreshed.url,
        args: details.args,
        env: details.env,
        headers: None,
        meta: details.meta,
        capability: details.capability,
        protocol_version: details.protocol_version,
        created_at: refreshed.created_at.map(|dt| dt.to_rfc3339()),
        updated_at: refreshed.updated_at.map(|dt| dt.to_rfc3339()),
        instances: details.instances,
        auth_mode: None,
        oauth_status,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{path::PathBuf, sync::Arc, time::Duration};

    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
    };
    use chrono::Utc;
    use sqlx::sqlite::SqlitePoolOptions;
    use tempfile::TempDir;
    use tokio::sync::Mutex;
    use tower::util::ServiceExt;

    use crate::api::routes::AppState;
    use crate::{
        clients::ClientConfigService,
        common::server::ServerType,
        config::{
            database::Database,
            models::Server,
            profile::init::initialize_profile_tables,
            registry::init::initialize_registry_cache_table,
            server::{self, init::initialize_server_tables},
        },
        core::{
            cache::{RedbCacheManager, manager::CacheConfig},
            models::Config,
            pool::UpstreamConnectionPool,
            profile::ConfigApplicationStateManager,
        },
        inspector::{calls::InspectorCallRegistry, sessions::InspectorSessionManager},
        system::metrics::MetricsCollector,
    };

    #[test]
    fn test_registry_query_params_serialization() {
        let query = RegistryServersQuery {
            limit: Some(50),
            cursor: Some("abc123".to_string()),
            search: Some("filesystem".to_string()),
            version: Some("1.0".to_string()),
            updated_since: Some("2025-03-01T00:00:00Z".to_string()),
            include_deleted: Some(true),
        };

        assert_eq!(query.limit, Some(50));
        assert_eq!(query.search, Some("filesystem".to_string()));
        assert_eq!(query.updated_since, Some("2025-03-01T00:00:00Z".to_string()));
        assert_eq!(query.include_deleted, Some(true));
    }

    #[test]
    fn test_registry_query_params_defaults() {
        let query = RegistryServersQuery {
            limit: None,
            cursor: None,
            search: None,
            version: None,
            updated_since: None,
            include_deleted: None,
        };

        assert_eq!(query.limit, None);
        assert_eq!(query.include_deleted, None);
    }

    #[test]
    fn test_registry_query_include_deleted_false() {
        let query = RegistryServersQuery {
            limit: None,
            cursor: None,
            search: None,
            version: None,
            updated_since: None,
            include_deleted: Some(false),
        };

        assert_eq!(query.include_deleted, Some(false));
    }

    #[test]
    fn test_registry_install_request_serialization() {
        let request = RegistryInstallRequest {
            name: "test-server".to_string(),
            version: Some("1.0.0".to_string()),
            target_profile_id: Some("profile-123".to_string()),
            dry_run: true,
        };

        assert_eq!(request.name, "test-server");
        assert_eq!(request.version, Some("1.0.0".to_string()));
        assert_eq!(request.target_profile_id, Some("profile-123".to_string()));
        assert!(request.dry_run);
    }

    #[test]
    fn test_registry_install_request_defaults() {
        let request = RegistryInstallRequest {
            name: "test-server".to_string(),
            version: None,
            target_profile_id: None,
            dry_run: false,
        };

        assert_eq!(request.name, "test-server");
        assert!(request.version.is_none());
        assert!(request.target_profile_id.is_none());
        assert!(!request.dry_run);
    }

    #[test]
    fn test_cached_registry_server_wrapper_maps_meta_and_transport() {
        let entry = RegistryCacheEntry {
            server_name: "filesystem".to_string(),
            version: "1.2.3".to_string(),
            schema_url: Some("https://modelcontextprotocol.io/schema/server.schema.json".to_string()),
            title: Some("Filesystem".to_string()),
            description: Some("File operations".to_string()),
            packages_json: Some(
                r#"[{"name":"@modelcontextprotocol/server-filesystem","version":"1.2.3"}]"#.to_string(),
            ),
            remotes_json: Some(r#"[{"url":"https://example.com/mcp","type":"streamable_http"}]"#.to_string()),
            icons_json: None,
            meta_json: Some(r#"{"custom":true}"#.to_string()),
            website_url: Some("https://example.com/filesystem".to_string()),
            repository_json: Some(r#"{"url":"https://github.com/example/filesystem","source":"github"}"#.to_string()),
            status: "active".to_string(),
            published_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
            synced_at: Utc::now(),
        };

        let wrapper = cache_entry_to_server_wrapper(entry);
        assert_eq!(wrapper.server.name, "filesystem");
        assert!(wrapper.server.remotes.is_some());
        assert!(wrapper.server.packages.is_some());

        let meta = wrapper.meta.expect("wrapper metadata");
        let official = meta
            .get("io.modelcontextprotocol.registry/official")
            .expect("official metadata");
        assert_eq!(official.get("serverId").and_then(Value::as_str), Some("filesystem"));
    }

    #[test]
    fn test_cached_registry_server_wrapper_keeps_server_name_canonical_when_meta_server_id_differs() {
        let entry = RegistryCacheEntry {
            server_name: "filesystem".to_string(),
            version: "1.2.3".to_string(),
            schema_url: Some("https://modelcontextprotocol.io/schema/server.schema.json".to_string()),
            title: Some("Filesystem".to_string()),
            description: Some("File operations".to_string()),
            packages_json: None,
            remotes_json: None,
            icons_json: None,
            meta_json: Some(
                r#"{"io.modelcontextprotocol.registry/official":{"serverId":"different-value","status":"active"}}"#
                    .to_string(),
            ),
            website_url: Some("https://example.com/filesystem".to_string()),
            repository_json: None,
            status: "active".to_string(),
            published_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
            synced_at: Utc::now(),
        };

        let wrapper = cache_entry_to_server_wrapper(entry);
        let meta = wrapper.meta.expect("wrapper metadata");
        let official = meta
            .get("io.modelcontextprotocol.registry/official")
            .expect("official metadata");

        assert_eq!(official.get("serverId").and_then(Value::as_str), Some("filesystem"));
    }

    #[tokio::test]
    async fn test_list_cached_servers_returns_cached_payload() {
        let context = create_test_context().await;
        let cache_service = RegistryCacheService::new(context.pool.clone());
        cache_service
            .upsert(&RegistryCacheEntry {
                server_name: "cached-server".to_string(),
                version: "0.1.0".to_string(),
                schema_url: Some("https://modelcontextprotocol.io/schema/server.schema.json".to_string()),
                title: Some("Cached Server".to_string()),
                description: Some("Cached registry entry".to_string()),
                packages_json: Some(r#"[{"name":"@scope/cached-server","version":"0.1.0"}]"#.to_string()),
                remotes_json: Some(r#"[{"url":"https://cached.example/mcp","type":"streamable_http"}]"#.to_string()),
                icons_json: None,
                meta_json: None,
                website_url: Some("https://cached.example".to_string()),
                repository_json: Some(
                    r#"{"url":"https://github.com/example/cached-server","source":"github"}"#.to_string(),
                ),
                status: "active".to_string(),
                published_at: Some(Utc::now()),
                updated_at: Some(Utc::now()),
                synced_at: Utc::now(),
            })
            .await
            .expect("seed registry cache");

        let response = list_cached_servers(
            State(context.state),
            Query(CachedRegistryServersQuery {
                limit: Some(10),
                cursor: None,
                search: None,
                include_deleted: None,
            }),
        )
        .await
        .expect("cached servers response");

        assert_eq!(response.0.servers.len(), 1);
        assert_eq!(response.0.servers[0].server.name, "cached-server");
        assert!(response.0.last_synced_at.is_some());
        assert_eq!(response.0.metadata.count, 1);
    }

    #[tokio::test]
    async fn test_sync_registry_requires_database() {
        let temp_dir = TempDir::new().expect("temp dir");
        let state = Arc::new(build_app_state(
            temp_dir.path().join("registry-handler-tests-no-db.redb"),
            None,
        ));
        let result = sync_registry(State(state)).await;
        assert!(matches!(result, Err(ApiError::InternalError(message)) if message.contains("Database not available")));
    }

    #[tokio::test]
    async fn test_refresh_managed_server_metadata_round_trips_registry_fields() {
        let context = create_test_context().await;
        let cache_service = RegistryCacheService::new(context.pool.clone());
        cache_service
            .upsert(&RegistryCacheEntry {
                server_name: "official-filesystem".to_string(),
                version: "2.1.0".to_string(),
                schema_url: Some("https://modelcontextprotocol.io/schema/server.schema.json".to_string()),
                title: Some("Filesystem".to_string()),
                description: Some("Official filesystem server".to_string()),
                packages_json: Some(
                    r#"[{"name":"@modelcontextprotocol/server-filesystem","version":"2.1.0"}]"#.to_string(),
                ),
                remotes_json: Some(r#"[{"url":"https://example.com/mcp","type":"streamable_http"}]"#.to_string()),
                icons_json: Some(r#"[{"url":"https://example.com/icon.png"}]"#.to_string()),
                meta_json: Some(r#"{"io.modelcontextprotocol.registry/official":{"status":"active"}}"#.to_string()),
                website_url: Some("https://example.com/filesystem".to_string()),
                repository_json: Some(
                    r#"{"url":"https://github.com/example/filesystem","source":"github"}"#.to_string(),
                ),
                status: "active".to_string(),
                published_at: Some(Utc::now()),
                updated_at: Some(Utc::now()),
                synced_at: Utc::now(),
            })
            .await
            .expect("seed registry cache");

        let server_id = seed_managed_server(&context.pool, "official-filesystem").await;
        let response = refresh_managed_server_metadata(State(context.state), Json(ServerIdReq { id: server_id }))
            .await
            .expect("refresh managed server metadata");

        let meta = response.0.data.expect("response payload").meta.expect("server meta");
        assert_eq!(meta.website_url.as_deref(), Some("https://example.com/filesystem"));
        assert_eq!(
            meta.repository.as_ref().and_then(|repo| repo.source.as_deref()),
            Some("github")
        );
        assert_eq!(meta.icons.as_ref().map(Vec::len), Some(1));
        assert_eq!(meta.version.as_deref(), Some("2.1.0"));
        assert!(meta.meta.is_some());
        assert!(meta.extras.is_some());
    }

    #[tokio::test]
    async fn test_refresh_managed_server_metadata_route_contract() {
        let context = create_test_context().await;
        let cache_service = RegistryCacheService::new(context.pool.clone());
        cache_service
            .upsert(&RegistryCacheEntry {
                server_name: "official-filesystem".to_string(),
                version: "2.1.0".to_string(),
                schema_url: Some("https://modelcontextprotocol.io/schema/server.schema.json".to_string()),
                title: Some("Filesystem".to_string()),
                description: Some("Official filesystem server".to_string()),
                packages_json: None,
                remotes_json: None,
                icons_json: None,
                meta_json: Some(r#"{"io.modelcontextprotocol.registry/official":{"status":"active"}}"#.to_string()),
                website_url: Some("https://example.com/filesystem".to_string()),
                repository_json: Some(
                    r#"{"url":"https://github.com/example/filesystem","source":"github"}"#.to_string(),
                ),
                status: "active".to_string(),
                published_at: Some(Utc::now()),
                updated_at: Some(Utc::now()),
                synced_at: Utc::now(),
            })
            .await
            .expect("seed registry cache");

        let server_id = seed_managed_server(&context.pool, "official-filesystem").await;
        let app = Router::new().merge(crate::api::routes::registry::routes(context.state.clone()));

        let response = app
            .oneshot(
                Request::post("/mcp/registry/servers/refresh")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"id":"{}"}}"#, server_id)))
                    .expect("request"),
            )
            .await
            .expect("route response");

        assert_eq!(response.status(), StatusCode::OK);

        let method_mismatch = Router::new()
            .merge(crate::api::routes::registry::routes(context.state.clone()))
            .oneshot(
                Request::get("/mcp/registry/servers/refresh")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("route response");

        assert_eq!(method_mismatch.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    struct TestContext {
        _temp_dir: TempDir,
        state: Arc<AppState>,
        pool: sqlx::SqlitePool,
    }

    async fn create_test_context() -> TestContext {
        let temp_dir = TempDir::new().expect("temp dir");
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("sqlite pool");

        initialize_registry_cache_table(&pool)
            .await
            .expect("init registry cache table");
        initialize_server_tables(&pool).await.expect("init server tables");
        initialize_profile_tables(&pool).await.expect("init profile tables");

        let database = Arc::new(Database {
            pool: pool.clone(),
            path: PathBuf::from(":memory:"),
        });
        let state = Arc::new(build_app_state(
            temp_dir.path().join("registry-handler-tests.redb"),
            Some(database),
        ));

        TestContext {
            _temp_dir: temp_dir,
            state,
            pool,
        }
    }

    fn build_app_state(
        cache_path: PathBuf,
        database: Option<Arc<Database>>,
    ) -> AppState {
        let redb_cache = Arc::new(RedbCacheManager::new(cache_path, CacheConfig::default()).expect("cache manager"));
        let oauth_manager = database
            .as_ref()
            .map(|db| Arc::new(crate::core::oauth::OAuthManager::new(db.pool.clone())));

        AppState {
            connection_pool: Arc::new(Mutex::new(UpstreamConnectionPool::new(
                Arc::new(Config::default()),
                database.clone(),
            ))),
            metrics_collector: Arc::new(MetricsCollector::new(Duration::from_secs(5))),
            http_proxy: None,
            profile_merge_service: None,
            database,
            audit_database: None,
            audit_service: None,
            config_application_state: Arc::new(ConfigApplicationStateManager::new()),
            redb_cache,
            unified_query: None,
            client_service: None::<Arc<ClientConfigService>>,
            inspector_calls: Arc::new(InspectorCallRegistry::new()),
            inspector_sessions: Arc::new(InspectorSessionManager::new()),
            oauth_manager,
        }
    }

    async fn seed_managed_server(
        pool: &sqlx::SqlitePool,
        registry_server_id: &str,
    ) -> String {
        let server = Server {
            id: None,
            name: "managed-filesystem".to_string(),
            server_type: ServerType::Stdio,
            command: Some("npx".to_string()),
            url: None,
            registry_server_id: Some(registry_server_id.to_string()),
            capabilities: None,
            enabled: crate::common::status::EnabledStatus::Enabled,
            unify_direct_exposure_eligible: false,
            created_at: None,
            updated_at: None,
            pending_import: false,
        };

        server::upsert_server(pool, &server).await.expect("seed managed server")
    }
}
