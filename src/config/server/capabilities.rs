// Server capabilities persistence helpers (shadow tables + REDB dual-write)
// Centralizes insert/update logic so API handlers and migration can reuse.

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};

use crate::core::capability::naming::{NamingKind, generate_unique_name};
use std::sync::Arc;

use crate::common::{capability::CapabilityToken, server::ServerType};
use crate::core::{
    cache::{
        CachedPromptInfo, CachedResourceInfo, CachedResourceTemplateInfo, CachedServerData, CachedToolInfo,
        RedbCacheManager,
    },
    pool::UpstreamConnectionPool,
};
use tokio::time::{Duration, timeout};

/// Internal helpers to deduplicate discovery and application steps
mod discovery_helpers {
    use super::*;

    pub async fn collect_all_prompts(service: &crate::core::transport::ClientService) -> Vec<CachedPromptInfo> {
        let mut out = Vec::new();
        let mut cursor: Option<String> = None;
        while let Ok(result) = service
            .list_prompts(
                cursor
                    .clone()
                    .map(|c| rmcp::model::PaginatedRequestParam { cursor: Some(c) }),
            )
            .await
        {
            for p in result.prompts {
                let converted_args = p
                    .arguments
                    .unwrap_or_default()
                    .into_iter()
                    .map(|arg| crate::core::cache::PromptArgument {
                        name: arg.name,
                        description: arg.description,
                        required: arg.required.unwrap_or(false),
                    })
                    .collect();
                out.push(CachedPromptInfo {
                    name: p.name,
                    description: p.description,
                    arguments: converted_args,
                    enabled: true,
                    cached_at: chrono::Utc::now(),
                });
            }
            cursor = result.next_cursor;
            if cursor.is_none() {
                break;
            }
        }
        out
    }

    pub async fn collect_all_resources(service: &crate::core::transport::ClientService) -> Vec<CachedResourceInfo> {
        let mut out = Vec::new();
        let mut cursor: Option<String> = None;
        while let Ok(result) = service
            .list_resources(
                cursor
                    .clone()
                    .map(|c| rmcp::model::PaginatedRequestParam { cursor: Some(c) }),
            )
            .await
        {
            for r in result.resources {
                out.push(CachedResourceInfo {
                    uri: r.uri.clone(),
                    name: Some(r.name.clone()),
                    description: r.description.clone(),
                    mime_type: r.mime_type.clone(),
                    enabled: true,
                    cached_at: chrono::Utc::now(),
                });
            }
            cursor = result.next_cursor;
            if cursor.is_none() {
                break;
            }
        }
        out
    }

    pub async fn collect_all_resource_templates(
        service: &crate::core::transport::ClientService
    ) -> Vec<CachedResourceTemplateInfo> {
        let mut out = Vec::new();
        let mut cursor: Option<String> = None;
        while let Ok(result) = service
            .list_resource_templates(Some(rmcp::model::PaginatedRequestParam { cursor }))
            .await
        {
            for t in result.resource_templates {
                out.push(CachedResourceTemplateInfo {
                    uri_template: t.uri_template.clone(),
                    name: Some(t.name.clone()),
                    description: t.description.clone(),
                    mime_type: t.mime_type.clone(),
                    enabled: true,
                    cached_at: chrono::Utc::now(),
                });
            }
            cursor = result.next_cursor;
            if cursor.is_none() {
                break;
            }
        }
        out
    }

    pub async fn apply_snapshot(
        db_pool: &Pool<Sqlite>,
        redb: &RedbCacheManager,
        server_id: &str,
        server_name: &str,
        snapshot: &super::CapabilitySnapshot,
        seed_profiles: bool,
    ) -> Result<()> {
        super::store_dual_write(
            db_pool,
            redb,
            server_id,
            server_name,
            snapshot.tools.clone(),
            snapshot.resources.clone(),
            snapshot.prompts.clone(),
            snapshot.resource_templates.clone(),
        )
        .await?;

        let supports_tools = !snapshot.tools.is_empty();
        let supports_prompts = !snapshot.prompts.is_empty();
        let supports_resources = !snapshot.resources.is_empty() || !snapshot.resource_templates.is_empty();

        super::overwrite_capabilities(db_pool, server_id, supports_tools, supports_prompts, supports_resources).await?;

        if seed_profiles {
            if let Err(e) = super::seed_profiles_with_snapshot(db_pool, server_id, snapshot).await {
                tracing::warn!(server_id = %server_id, error = %e, "Failed to seed profiles with snapshot");
            }
        }
        Ok(())
    }
}

/// Cache helpers used by API and startup paths
pub mod cache_utils {
    use super::*;

    /// Create a standard Redb cache manager using the global cache directory
    pub fn get_standard_cache_manager() -> anyhow::Result<Arc<RedbCacheManager>> {
        let cache_path = crate::common::paths::global_paths().cache_dir().join("capability.redb");
        let mgr = RedbCacheManager::new(cache_path, crate::core::cache::manager::CacheConfig::default())?;
        Ok(Arc::new(mgr))
    }
}

/// Unified capability snapshot container
#[derive(Debug, Clone, Default)]
pub struct CapabilitySnapshot {
    pub tools: Vec<CachedToolInfo>,
    pub resources: Vec<CachedResourceInfo>,
    pub prompts: Vec<CachedPromptInfo>,
    pub resource_templates: Vec<CachedResourceTemplateInfo>,
}

/// Discover capabilities from an existing upstream connection (API temporary instance)
pub async fn discover_from_connection(conn: &crate::core::pool::UpstreamConnection) -> Result<CapabilitySnapshot> {
    let mut snap = CapabilitySnapshot::default();

    // Tools
    for t in &conn.tools {
        let schema = t.schema_as_json_value();
        let input_schema_json = serde_json::to_string(&schema).unwrap_or_else(|_| "{}".to_string());
        snap.tools.push(CachedToolInfo {
            name: t.name.to_string(),
            description: t.description.clone().map(|d| d.into_owned()),
            input_schema_json,
            unique_name: None,
            enabled: true,
            cached_at: chrono::Utc::now(),
        });
    }

    // Prompts (paginate defensively)
    if conn.supports_prompts() {
        if let Some(service) = &conn.service {
            let items = discovery_helpers::collect_all_prompts(service).await;
            snap.prompts.extend(items);
        }
    }

    // Resources and templates (paginate fully)
    if conn.supports_resources() {
        if let Some(service) = &conn.service {
            let resources = discovery_helpers::collect_all_resources(service).await;
            let templates = discovery_helpers::collect_all_resource_templates(service).await;
            snap.resources.extend(resources);
            snap.resource_templates.extend(templates);
        }
    }

    Ok(snap)
}

/// Discover capabilities by connecting with the given server config (used by migration)
pub async fn discover_from_config(
    server_name: &str,
    server_config: &crate::core::models::MCPServerConfig,
    server_type: crate::common::server::ServerType,
) -> Result<CapabilitySnapshot> {
    use crate::core::transport::{TransportType, connect_http_server, connect_server_simple};

    let (service, tools, capabilities, _pid) = match server_type {
        crate::common::server::ServerType::Stdio => {
            connect_server_simple(server_name, server_config, server_type, TransportType::Stdio).await?
        }
        crate::common::server::ServerType::Sse => connect_http_server(server_name, server_config, TransportType::Sse)
            .await
            .map(|(s, t, c)| (s, t, c, None))?,
        crate::common::server::ServerType::StreamableHttp => {
            connect_http_server(server_name, server_config, TransportType::StreamableHttp)
                .await
                .map(|(s, t, c)| (s, t, c, None))?
        }
    };

    let mut snap = CapabilitySnapshot::default();

    // Tools
    for t in &tools {
        let schema = t.schema_as_json_value();
        let input_schema_json = serde_json::to_string(&schema).unwrap_or_else(|_| "{}".to_string());
        snap.tools.push(CachedToolInfo {
            name: t.name.to_string(),
            description: t.description.clone().map(|d| d.into_owned()),
            input_schema_json,
            unique_name: None,
            enabled: true,
            cached_at: chrono::Utc::now(),
        });
    }

    // Prompts (paginate defensively)
    if capabilities.as_ref().and_then(|c| c.prompts.as_ref()).is_some() {
        let items = discovery_helpers::collect_all_prompts(&service).await;
        snap.prompts.extend(items);
    }

    // Resources & templates (paginate fully)
    if capabilities.as_ref().and_then(|c| c.resources.as_ref()).is_some() {
        let resources = discovery_helpers::collect_all_resources(&service).await;
        let templates = discovery_helpers::collect_all_resource_templates(&service).await;
        snap.resources.extend(resources);
        snap.resource_templates.extend(templates);
    }

    Ok(snap)
}

/// Upsert shadow prompt row (unique_name uses original prompt_name for now)
pub async fn upsert_shadow_prompt(
    pool: &Pool<Sqlite>,
    server_id: &str,
    server_name: &str,
    prompt_name: &str,
    description: Option<&str>,
) -> Result<()> {
    let id = crate::generate_id!("sprm");
    let unique_name = generate_unique_name(NamingKind::Prompt, server_name, prompt_name);
    sqlx::query(
        r#"
        INSERT INTO server_prompts (id, server_id, server_name, prompt_name, unique_name, description)
        VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT(server_id, prompt_name) DO UPDATE SET
            server_name = excluded.server_name,
            unique_name = excluded.unique_name,
            description = excluded.description,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(&id)
    .bind(server_id)
    .bind(server_name)
    .bind(prompt_name)
    .bind(&unique_name)
    .bind(description)
    .execute(pool)
    .await
    .context("Failed to upsert shadow prompt")?;
    Ok(())
}

/// Upsert shadow resource row (unique_name uses original URI for now)
pub async fn upsert_shadow_resource(
    pool: &Pool<Sqlite>,
    server_id: &str,
    server_name: &str,
    uri: &str,
    name: Option<&str>,
    description: Option<&str>,
    mime_type: Option<&str>,
) -> Result<()> {
    let id = crate::generate_id!("sres");
    let unique_name = generate_unique_name(NamingKind::Resource, server_name, uri);
    sqlx::query(
        r#"
        INSERT INTO server_resources (id, server_id, server_name, resource_uri, unique_uri, name, description, mime_type)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(server_id, resource_uri) DO UPDATE SET
            server_name = excluded.server_name,
            unique_uri = excluded.unique_uri,
            name = excluded.name,
            description = excluded.description,
            mime_type = excluded.mime_type,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(&id)
    .bind(server_id)
    .bind(server_name)
    .bind(uri)
    .bind(&unique_name)
    .bind(name)
    .bind(description)
    .bind(mime_type)
    .execute(pool)
    .await
    .context("Failed to upsert shadow resource")?;
    Ok(())
}

/// Upsert shadow resource template row (unique_name uses original uri_template for now)
pub async fn upsert_shadow_resource_template(
    pool: &Pool<Sqlite>,
    server_id: &str,
    server_name: &str,
    uri_template: &str,
    name: Option<&str>,
    description: Option<&str>,
) -> Result<()> {
    let id = crate::generate_id!("srst");
    let unique_name = generate_unique_name(NamingKind::ResourceTemplate, server_name, uri_template);
    sqlx::query(
        r#"
        INSERT INTO server_resource_templates (id, server_id, server_name, uri_template, unique_name, name, description)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(server_id, uri_template) DO UPDATE SET
            server_name = excluded.server_name,
            unique_name = excluded.unique_name,
            name = excluded.name,
            description = excluded.description,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(&id)
    .bind(server_id)
    .bind(server_name)
    .bind(uri_template)
    .bind(&unique_name)
    .bind(name)
    .bind(description)
    .execute(pool)
    .await
    .context("Failed to upsert shadow resource template")?;
    Ok(())
}

/// Store snapshot in REDB
pub async fn store_redb_snapshot(
    redb: &RedbCacheManager,
    server_id: &str,
    server_name: &str,
    tools: Vec<CachedToolInfo>,
    resources: Vec<CachedResourceInfo>,
    prompts: Vec<CachedPromptInfo>,
    resource_templates: Vec<CachedResourceTemplateInfo>,
) -> Result<()> {
    let server_data = CachedServerData {
        server_id: server_id.to_string(),
        server_name: server_name.to_string(),
        server_version: None,
        protocol_version: "latest".to_string(),
        tools,
        resources,
        prompts,
        resource_templates,
        cached_at: chrono::Utc::now(),
        fingerprint: format!("store:{}:{}", server_id, chrono::Utc::now().timestamp()),
    };
    redb.store_server_data(&server_data)
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))
}

/// Dual-write: REDB full + SQLite shadow tables + server_tools batch upsert
pub async fn store_dual_write(
    pool: &Pool<Sqlite>,
    redb: &RedbCacheManager,
    server_id: &str,
    server_name: &str,
    mut tools: Vec<CachedToolInfo>,
    resources: Vec<CachedResourceInfo>,
    prompts: Vec<CachedPromptInfo>,
    templates: Vec<CachedResourceTemplateInfo>,
) -> Result<()> {
    if !tools.is_empty() {
        crate::config::server::tools::assign_unique_names_to_cached_tools(pool, server_id, server_name, &mut tools)
            .await?;
    }

    // REDB
    store_redb_snapshot(
        redb,
        server_id,
        server_name,
        tools.clone(),
        resources.clone(),
        prompts.clone(),
        templates.clone(),
    )
    .await?;
    // Clear any refreshing marker now that we have a fresh snapshot
    redb.clear_refreshing(server_id).await;

    // SQLite: tools via existing helper
    if !tools.is_empty() {
        let items: Vec<(String, Option<String>)> =
            tools.iter().map(|t| (t.name.clone(), t.description.clone())).collect();
        let server_name_owned = server_name.to_string();
        if let Err(e) =
            crate::config::server::tools::batch_upsert_server_tools(pool, server_id, &server_name_owned, &items).await
        {
            tracing::warn!(
                server_id = %server_id,
                server_name = %server_name,
                error = %e,
                "Failed to batch upsert server tools (SQLite shadow)"
            );
        }
    }

    // SQLite: prompts/resources/templates
    for p in &prompts {
        if let Err(e) = upsert_shadow_prompt(pool, server_id, server_name, &p.name, p.description.as_deref()).await {
            tracing::warn!(
                server_id = %server_id,
                server_name = %server_name,
                prompt = %p.name,
                error = %e,
                "Failed to upsert shadow prompt"
            );
        }
    }

    // SQLite: resources
    for r in &resources {
        if let Err(e) = upsert_shadow_resource(
            pool,
            server_id,
            server_name,
            &r.uri,
            r.name.as_deref(),
            r.description.as_deref(),
            r.mime_type.as_deref(),
        )
        .await
        {
            tracing::warn!(
                server_id = %server_id,
                server_name = %server_name,
                uri = %r.uri,
                error = %e,
                "Failed to upsert shadow resource"
            );
        }
    }

    // SQLite: resource templates
    for t in &templates {
        if let Err(e) = upsert_shadow_resource_template(
            pool,
            server_id,
            server_name,
            &t.uri_template,
            t.name.as_deref(),
            t.description.as_deref(),
        )
        .await
        {
            tracing::warn!(
                server_id = %server_id,
                server_name = %server_name,
                uri_template = %t.uri_template,
                error = %e,
                "Failed to upsert shadow resource template"
            );
        }
    }

    Ok(())
}

/// Seed active profiles with discovered capabilities (enabled=true by default).
/// This honors the REDB-first + seed-profile rule on first-run so that API `/mcp/profile/*`
/// endpoints are not empty immediately after initialization.
pub async fn seed_profiles_with_snapshot(
    pool: &Pool<Sqlite>,
    server_id: &str,
    snapshot: &CapabilitySnapshot,
) -> Result<()> {
    // Get active profiles
    let profiles = crate::config::profile::get_active_profile(pool).await?;
    if profiles.is_empty() {
        return Ok(());
    }

    for profile in profiles {
        let Some(profile_id) = profile.id.as_deref() else {
            continue;
        };

        // Tools
        for t in &snapshot.tools {
            let _ = crate::config::profile::add_tool_to_profile(pool, profile_id, server_id, &t.name, true).await;
        }
        // Resources
        for r in &snapshot.resources {
            let _ = crate::config::profile::add_resource_to_profile(pool, profile_id, server_id, &r.uri, true).await;
        }
        // Prompts
        for p in &snapshot.prompts {
            let _ = crate::config::profile::add_prompt_to_profile(pool, profile_id, server_id, &p.name, true).await;
        }
    }
    Ok(())
}

/// Overwrite server_config.capabilities using protocol-level support flags (full snapshot semantics)
pub async fn overwrite_capabilities(
    pool: &Pool<Sqlite>,
    server_id: &str,
    supports_tools: bool,
    supports_prompts: bool,
    supports_resources: bool,
) -> Result<()> {
    let mut caps: Vec<&str> = Vec::new();
    if supports_tools {
        caps.push(CapabilityToken::Tools.as_str());
    }
    if supports_prompts {
        caps.push(CapabilityToken::Prompts.as_str());
    }
    if supports_resources {
        caps.push(CapabilityToken::Resources.as_str());
    }
    let caps_opt: Option<String> = if caps.is_empty() { None } else { Some(caps.join(",")) };
    sqlx::query(r#"UPDATE server_config SET capabilities = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?"#)
        .bind(caps_opt)
        .bind(server_id)
        .execute(pool)
        .await
        .map_err(|e| anyhow::anyhow!(e))?;
    Ok(())
}

/// Sync capabilities using an upstream connection pool (API path helper)
pub async fn sync_via_connection_pool(
    connection_pool: &tokio::sync::Mutex<UpstreamConnectionPool>,
    redb: &RedbCacheManager,
    db_pool: &Pool<Sqlite>,
    server_id: &str,
    server_name: &str,
    lock_timeout_secs: u64,
) -> Result<()> {
    tracing::info!(
        target: "mcpmate::config::server::capabilities",
        server_id = %server_id,
        server_name = %server_name,
        lock_timeout_secs = lock_timeout_secs,
        "Starting capability sync via connection pool"
    );
    // Acquire pool
    let pool_guard = timeout(Duration::from_secs(lock_timeout_secs), connection_pool.lock())
        .await
        .map_err(|_| anyhow::anyhow!("Timeout acquiring connection pool lock"))?;
    let mut pool = pool_guard;

    // Create temporary validation instance
    let conn = match pool
        .get_or_create_validation_instance(server_name, "api", Duration::from_secs(5 * 60))
        .await
    {
        Ok(Some(c)) => c,
        Ok(None) => {
            tracing::trace!(server_name = %server_name, "No validation instance available for API sync");
            return Ok(());
        }
        Err(e) => {
            tracing::warn!(server_name = %server_name, error = %e, "Failed to create validation instance for API sync");
            return Ok(());
        }
    };

    // Discover and apply (now fully paginated)
    let snap = discover_from_connection(conn).await?;
    discovery_helpers::apply_snapshot(db_pool, redb, server_id, server_name, &snap, true).await?;

    // Cleanup
    if let Err(e) = pool.destroy_validation_instance(server_name, "api").await {
        tracing::trace!(server_name = %server_name, error = %e, "Failed to destroy validation instance (api)");
    }

    Ok(())
}

/// Capability sync strategy
#[derive(Debug, Clone)]
pub enum SyncStrategy {
    /// Use existing connection from pool
    FromConnection,
    /// Create temporary connection for discovery
    FromConfig(crate::core::models::MCPServerConfig, ServerType),
    /// Force refresh capabilities
    ForceRefresh,
}

/// Capability sync result
#[derive(Debug)]
pub struct CapabilitySync {
    pub server_id: String,
    pub server_name: String,
    pub supports_tools: bool,
    pub supports_prompts: bool,
    pub supports_resources: bool,
    pub snapshot: CapabilitySnapshot,
}

/// Unified capability management interface
pub struct CapabilityManager {
    db_pool: Arc<Pool<Sqlite>>,
    redb_cache: Arc<RedbCacheManager>,
    connection_pool: Arc<tokio::sync::Mutex<UpstreamConnectionPool>>,
}

impl CapabilityManager {
    /// Create a new capability manager
    pub fn new(
        db_pool: Arc<Pool<Sqlite>>,
        redb_cache: Arc<RedbCacheManager>,
        connection_pool: Arc<tokio::sync::Mutex<UpstreamConnectionPool>>,
    ) -> Self {
        Self {
            db_pool,
            redb_cache,
            connection_pool,
        }
    }

    /// Sync capabilities for a single server
    pub async fn sync_server_capabilities(
        &self,
        server_id: &str,
        server_name: &str,
        strategy: SyncStrategy,
    ) -> Result<CapabilitySync> {
        tracing::debug!(
            "Syncing capabilities for server '{}' using strategy {:?}",
            server_name,
            strategy
        );

        // Discover capabilities
        let snapshot = match strategy {
            SyncStrategy::FromConnection => self.discover_from_existing_connection(server_name).await?,
            SyncStrategy::FromConfig(config, server_type) => {
                discover_from_config(server_name, &config, server_type).await?
            }
            SyncStrategy::ForceRefresh => {
                // Get server config from database and use FromConfig strategy
                let server_row = crate::config::models::Server::find_by_name(&self.db_pool, server_name)
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("Server '{}' not found", server_name))?;

                let config = crate::core::models::MCPServerConfig {
                    kind: server_row.server_type,
                    command: server_row.command,
                    url: server_row.url,
                    args: None,
                    env: None,
                    transport_type: server_row.transport_type,
                };

                discover_from_config(server_name, &config, server_row.server_type).await?
            }
        };

        // Store capabilities and overwrite flags (no seeding here)
        discovery_helpers::apply_snapshot(
            &self.db_pool,
            &self.redb_cache,
            server_id,
            server_name,
            &snapshot,
            false,
        )
        .await?;

        let supports_tools = !snapshot.tools.is_empty();
        let supports_prompts = !snapshot.prompts.is_empty();
        let supports_resources = !snapshot.resources.is_empty() || !snapshot.resource_templates.is_empty();

        Ok(CapabilitySync {
            server_id: server_id.to_string(),
            server_name: server_name.to_string(),
            supports_tools,
            supports_prompts,
            supports_resources,
            snapshot,
        })
    }

    /// Sync capabilities for multiple servers in parallel
    pub async fn sync_multiple_servers(
        &self,
        servers: Vec<(String, String, SyncStrategy)>, // (server_id, server_name, strategy)
    ) -> Result<Vec<CapabilitySync>> {
        tracing::info!("Starting capability sync for {} servers (concurrent)", servers.len());

        // Process servers sequentially to collect results
        // Note: This could be optimized with concurrent processing if needed
        let mut successes = Vec::new();

        for (server_id, server_name, strategy) in servers {
            match self.sync_server_capabilities(&server_id, &server_name, strategy).await {
                Ok(sync) => {
                    tracing::debug!("Successfully synced capabilities for server '{}'", server_name);
                    successes.push(sync);
                }
                Err(e) => {
                    tracing::warn!("Failed to sync capabilities for server '{}': {}", server_name, e);
                }
            }
        }

        tracing::info!(
            "Completed capability sync: {}/{} successful",
            successes.len(),
            successes.len()
        );
        Ok(successes)
    }

    /// Sync all servers from startup (all servers from database)
    pub async fn sync_connected_servers(&self) -> Result<Vec<CapabilitySync>> {
        // Get all servers from database instead of relying on connection pool state
        let all_servers = crate::config::server::get_all_servers(&self.db_pool).await?;

        let mut servers_with_strategy = Vec::new();

        // Sync capabilities for each server using auto-strategy selection
        for server in all_servers {
            if let Some(server_id) = server.id {
                // Use auto strategy: try connection first, fallback to config
                let strategy = {
                    let pool = self.connection_pool.lock().await;
                    if pool
                        .connections
                        .get(&server.name)
                        .is_some_and(|instances| instances.values().any(|conn| conn.is_connected()))
                    {
                        SyncStrategy::FromConnection
                    } else {
                        let config = crate::core::models::MCPServerConfig {
                            kind: server.server_type,
                            command: server.command,
                            url: server.url,
                            args: None,
                            env: None,
                            transport_type: server.transport_type,
                        };
                        SyncStrategy::FromConfig(config, server.server_type)
                    }
                };

                servers_with_strategy.push((server_id, server.name, strategy));
            }
        }

        self.sync_multiple_servers(servers_with_strategy).await
    }

    /// Sync servers from import configuration
    pub async fn sync_import_servers(
        &self,
        servers: Vec<(String, String, crate::core::models::MCPServerConfig, ServerType)>, // (server_id, server_name, config, server_type)
    ) -> Result<Vec<CapabilitySync>> {
        let servers_with_strategy = servers
            .into_iter()
            .map(|(server_id, server_name, config, server_type)| {
                (server_id, server_name, SyncStrategy::FromConfig(config, server_type))
            })
            .collect();

        self.sync_multiple_servers(servers_with_strategy).await
    }

    /// Helper: discover from existing connection
    async fn discover_from_existing_connection(
        &self,
        server_name: &str,
    ) -> Result<CapabilitySnapshot> {
        let mut pool = self.connection_pool.lock().await;
        let session_id = "capability_sync";
        let conn = pool
            .get_or_create_validation_instance(server_name, session_id, tokio::time::Duration::from_secs(30))
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to get connection for server '{}'", server_name))?;

        let snapshot = discover_from_connection(conn).await?;

        // Cleanup validation instance
        if let Err(e) = pool.destroy_validation_instance(server_name, session_id).await {
            tracing::trace!(server_name = %server_name, error = %e, "Failed to destroy validation instance");
        }

        Ok(snapshot)
    }

    /// Convenience function: Sync single server by name with auto-strategy selection
    pub async fn auto_sync_server(
        &self,
        server_name: &str,
    ) -> Result<CapabilitySync> {
        // Get server from database
        let server_row = crate::config::models::Server::find_by_name(&self.db_pool, server_name)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Server '{}' not found", server_name))?;

        let server_id = server_row
            .id
            .ok_or_else(|| anyhow::anyhow!("Server '{}' has no ID", server_name))?;

        // Try connection first, fallback to config
        let strategy = {
            let pool = self.connection_pool.lock().await;
            if pool
                .connections
                .get(server_name)
                .is_some_and(|instances| instances.values().any(|conn| conn.is_connected()))
            {
                SyncStrategy::FromConnection
            } else {
                let config = crate::core::models::MCPServerConfig {
                    kind: server_row.server_type,
                    command: server_row.command,
                    url: server_row.url,
                    args: None,
                    env: None,
                    transport_type: server_row.transport_type,
                };
                SyncStrategy::FromConfig(config, server_row.server_type)
            }
        };

        self.sync_server_capabilities(&server_id, server_name, strategy).await
    }

    /// Sync capabilities for a single server that just connected successfully
    /// This method is optimized for event-driven capability sync triggered by connection events
    pub async fn sync_single_server(
        &self,
        server_name: &str,
    ) -> Result<CapabilitySync> {
        // Get server from database
        let server_row = crate::config::models::Server::find_by_name(&self.db_pool, server_name)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Server '{}' not found", server_name))?;

        let server_id = server_row
            .id
            .ok_or_else(|| anyhow::anyhow!("Server '{}' has no ID", server_name))?;

        // Use FromConnection strategy since we know the server just connected successfully
        let strategy = SyncStrategy::FromConnection;

        tracing::debug!(
            "Syncing capabilities for newly connected server '{}' using FromConnection strategy",
            server_name
        );

        self.sync_server_capabilities(&server_id, server_name, strategy).await
    }
}
