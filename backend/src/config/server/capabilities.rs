// Server capabilities persistence helpers (shadow tables + REDB dual-write)
// Centralizes insert/update logic so API handlers and migration can reuse.

use anyhow::{Context, Result};
use once_cell::sync::OnceCell;
use sqlx::{Pool, Sqlite, Transaction};

use crate::core::capability::naming::{
    NamingKind, begin_naming_transaction, reconcile_external_identifier_additions, reconcile_external_identifiers,
};
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

const PACKAGE_RUNNER_PREVIEW_STARTUP_TIMEOUT: Duration = Duration::from_secs(5 * 60);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PreviewStdioTimeouts {
    startup: Duration,
    tools: Duration,
    package_runner: bool,
}

fn is_preview_package_runner(command: &str) -> bool {
    let executable = command
        .trim()
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let executable = executable
        .strip_suffix(".exe")
        .or_else(|| executable.strip_suffix(".cmd"))
        .or_else(|| executable.strip_suffix(".bat"))
        .unwrap_or(&executable);

    matches!(executable, "bunx" | "npx" | "uvx")
}

fn preview_stdio_timeouts(
    command: &str,
    operation_timeout: Option<Duration>,
) -> PreviewStdioTimeouts {
    let package_runner = is_preview_package_runner(command);
    let startup = if package_runner {
        PACKAGE_RUNNER_PREVIEW_STARTUP_TIMEOUT
    } else {
        operation_timeout.unwrap_or_else(|| crate::core::foundation::utils::get_connection_timeout(command))
    };

    PreviewStdioTimeouts {
        startup,
        tools: operation_timeout.unwrap_or_else(|| crate::core::foundation::utils::get_tools_timeout(command)),
        package_runner,
    }
}

async fn run_preview_operation<T, F>(
    operation: &str,
    operation_timeout: Option<Duration>,
    future: F,
) -> Result<T>
where
    F: std::future::Future<Output = Result<T>>,
{
    match operation_timeout {
        Some(duration) => timeout(duration, future)
            .await
            .map_err(|_| anyhow::anyhow!("Timed out during {operation} after {}ms", duration.as_millis()))?,
        None => future.await,
    }
}

/// Internal helpers to deduplicate discovery and application steps
mod discovery_helpers {
    use super::*;

    pub async fn collect_all_prompts(service: &crate::core::transport::ClientService) -> Result<Vec<CachedPromptInfo>> {
        let mut out = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let result = service
                .list_prompts(
                    cursor
                        .clone()
                        .map(|c| rmcp::model::PaginatedRequestParams::default().with_cursor(Some(c))),
                )
                .await
                .context("prompts/list failed during capability discovery")?;
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
                    icons: p.icons,
                    enabled: true,
                    cached_at: chrono::Utc::now(),
                });
            }
            cursor = result.next_cursor;
            if cursor.is_none() {
                break;
            }
        }
        Ok(out)
    }

    pub async fn collect_all_resources(
        service: &crate::core::transport::ClientService
    ) -> Result<Vec<CachedResourceInfo>> {
        let mut out = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let result = service
                .list_resources(
                    cursor
                        .clone()
                        .map(|c| rmcp::model::PaginatedRequestParams::default().with_cursor(Some(c))),
                )
                .await
                .context("resources/list failed during capability discovery")?;
            for r in result.resources {
                out.push(CachedResourceInfo {
                    uri: r.uri.clone(),
                    name: Some(r.name.clone()),
                    description: r.description.clone(),
                    mime_type: r.mime_type.clone(),
                    icons: r.icons.clone(),
                    enabled: true,
                    cached_at: chrono::Utc::now(),
                });
            }
            cursor = result.next_cursor;
            if cursor.is_none() {
                break;
            }
        }
        Ok(out)
    }

    pub async fn collect_all_resource_templates(
        service: &crate::core::transport::ClientService
    ) -> Result<Vec<CachedResourceTemplateInfo>> {
        let mut out = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let result = service
                .list_resource_templates(Some(
                    rmcp::model::PaginatedRequestParams::default().with_cursor(cursor.clone()),
                ))
                .await
                .context("resources/templates/list failed during capability discovery")?;
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
        Ok(out)
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
            snapshot.protocol_version.clone(),
        )
        .await?;

        super::persist_snapshot_server_info(db_pool, server_id, snapshot).await?;

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

pub(crate) async fn apply_discovered_snapshot(
    db_pool: &Pool<Sqlite>,
    redb: &RedbCacheManager,
    server_id: &str,
    server_name: &str,
    snapshot: &CapabilitySnapshot,
    seed_profiles: bool,
) -> Result<()> {
    discovery_helpers::apply_snapshot(db_pool, redb, server_id, server_name, snapshot, seed_profiles).await
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
    pub protocol_version: Option<String>,
    pub upstream_name: Option<String>,
    pub upstream_title: Option<String>,
    pub server_version: Option<String>,
}

pub async fn persist_snapshot_server_info(
    pool: &Pool<Sqlite>,
    server_id: &str,
    snapshot: &CapabilitySnapshot,
) -> Result<()> {
    let (Some(upstream_name), Some(protocol_version)) =
        (snapshot.upstream_name.clone(), snapshot.protocol_version.clone())
    else {
        return Ok(());
    };
    crate::config::server::meta::update_server_info(
        pool,
        server_id,
        upstream_name,
        snapshot.upstream_title.clone(),
        snapshot.server_version.clone(),
        protocol_version,
    )
    .await
}

/// Discover capabilities from an existing upstream connection (API temporary instance)
pub async fn discover_from_connection(conn: &crate::core::pool::UpstreamConnection) -> Result<CapabilitySnapshot> {
    let peer_info = conn.service.as_ref().and_then(|service| service.peer_info());
    let mut snap = CapabilitySnapshot {
        protocol_version: peer_info.map(|info| info.protocol_version.to_string()),
        upstream_name: peer_info.map(|info| info.server_info.name.clone()),
        upstream_title: peer_info.and_then(|info| info.server_info.title.clone()),
        server_version: peer_info.map(|info| info.server_info.version.clone()),
        ..Default::default()
    };

    // Tools
    for t in &conn.tools {
        let schema = t.schema_as_json_value();
        let input_schema_json = serde_json::to_string(&schema).unwrap_or_else(|_| "{}".to_string());
        snap.tools.push(CachedToolInfo {
            name: t.name.to_string(),
            description: t.description.clone().map(|d| d.into_owned()),
            input_schema_json,
            output_schema_json: t.output_schema.as_ref().map(|s| {
                serde_json::to_string(&serde_json::Value::Object((**s).clone())).unwrap_or_else(|_| "{}".to_string())
            }),
            unique_name: None,
            icons: t.icons.clone(),
            enabled: true,
            cached_at: chrono::Utc::now(),
        });
    }

    // Prompts (paginate defensively)
    if conn.supports_prompts() {
        if let Some(service) = &conn.service {
            let items = discovery_helpers::collect_all_prompts(service).await?;
            snap.prompts.extend(items);
        }
    }

    // Resources and templates (paginate fully)
    if conn.supports_resources() {
        if let Some(service) = &conn.service {
            let resources = discovery_helpers::collect_all_resources(service).await?;
            let templates = discovery_helpers::collect_all_resource_templates(service).await?;
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
        crate::common::server::ServerType::Sse | crate::common::server::ServerType::StreamableHttp => {
            connect_http_server(server_name, server_config, TransportType::StreamableHttp)
                .await
                .map(|(s, t, c)| (s, t, c, None))?
        }
    };

    let peer_info = service.peer_info();
    let mut snap = CapabilitySnapshot {
        protocol_version: peer_info.map(|info| info.protocol_version.to_string()),
        upstream_name: peer_info.map(|info| info.server_info.name.clone()),
        upstream_title: peer_info.and_then(|info| info.server_info.title.clone()),
        server_version: peer_info.map(|info| info.server_info.version.clone()),
        ..Default::default()
    };

    // Tools
    for t in &tools {
        let schema = t.schema_as_json_value();
        let input_schema_json = serde_json::to_string(&schema).unwrap_or_else(|_| "{}".to_string());
        snap.tools.push(CachedToolInfo {
            name: t.name.to_string(),
            description: t.description.clone().map(|d| d.into_owned()),
            input_schema_json,
            output_schema_json: t.output_schema.as_ref().map(|s| {
                serde_json::to_string(&serde_json::Value::Object((**s).clone())).unwrap_or_else(|_| "{}".to_string())
            }),
            unique_name: None,
            icons: t.icons.clone(),
            enabled: true,
            cached_at: chrono::Utc::now(),
        });
    }

    // Prompts (paginate defensively)
    if capabilities.as_ref().and_then(|c| c.prompts.as_ref()).is_some() {
        let items = discovery_helpers::collect_all_prompts(&service).await?;
        snap.prompts.extend(items);
    }

    // Resources & templates (paginate fully)
    if capabilities.as_ref().and_then(|c| c.resources.as_ref()).is_some() {
        let resources = discovery_helpers::collect_all_resources(&service).await?;
        let templates = discovery_helpers::collect_all_resource_templates(&service).await?;
        snap.resources.extend(resources);
        snap.resource_templates.extend(templates);
    }

    Ok(snap)
}

/// Discover capabilities with optional custom HTTP client and timeouts (used by preview)
pub async fn discover_from_config_preview(
    server_name: &str,
    server_config: &crate::core::models::MCPServerConfig,
    server_type: crate::common::server::ServerType,
    http_client: Option<reqwest::Client>,
    operation_timeout: Option<std::time::Duration>,
) -> Result<CapabilitySnapshot> {
    use crate::core::transport::{
        TransportType, connect_http_server, connect_http_server_with_client, connect_http_server_with_client_timeouts,
        stdio::connect_stdio_server_with_timeouts,
    };
    use tokio_util::sync::CancellationToken;

    let (service, tools, capabilities, _pid) = match server_type {
        crate::common::server::ServerType::Stdio => {
            let command = server_config.command.as_deref().unwrap_or_default();
            let timeouts = preview_stdio_timeouts(command, operation_timeout);
            let result = connect_stdio_server_with_timeouts(
                server_name,
                server_config,
                CancellationToken::new(),
                None,
                timeouts.startup,
                timeouts.tools,
            )
            .await;

            if timeouts.package_runner {
                result.with_context(|| {
                    format!(
                        "Package runner preview startup failed for '{server_name}' after allowing up to {}s",
                        timeouts.startup.as_secs()
                    )
                })?
            } else {
                result?
            }
        }
        crate::common::server::ServerType::Sse | crate::common::server::ServerType::StreamableHttp => {
            if let Some(timeout) = operation_timeout {
                let client = http_client.unwrap_or_default();
                let (service, tools, capabilities) = connect_http_server_with_client_timeouts(
                    server_name,
                    server_config,
                    client,
                    TransportType::StreamableHttp,
                    timeout,
                    timeout,
                )
                .await?;
                (service, tools, capabilities, None)
            } else if let Some(client) = http_client {
                let (service, tools, capabilities) =
                    connect_http_server_with_client(server_name, server_config, client, TransportType::StreamableHttp)
                        .await?;
                (service, tools, capabilities, None)
            } else {
                let (s, t, c) = connect_http_server(server_name, server_config, TransportType::StreamableHttp).await?;
                (s, t, c, None)
            }
        }
    };

    let peer_info = service.peer_info();
    let mut snap = CapabilitySnapshot {
        protocol_version: peer_info.map(|info| info.protocol_version.to_string()),
        upstream_name: peer_info.map(|info| info.server_info.name.clone()),
        upstream_title: peer_info.and_then(|info| info.server_info.title.clone()),
        server_version: peer_info.map(|info| info.server_info.version.clone()),
        ..Default::default()
    };
    for t in &tools {
        let schema = t.schema_as_json_value();
        let input_schema_json = serde_json::to_string(&schema).unwrap_or_else(|_| "{}".to_string());
        snap.tools.push(CachedToolInfo {
            name: t.name.to_string(),
            description: t.description.clone().map(|d| d.into_owned()),
            input_schema_json,
            output_schema_json: t.output_schema.as_ref().map(|s| {
                serde_json::to_string(&serde_json::Value::Object((**s).clone())).unwrap_or_else(|_| "{}".to_string())
            }),
            unique_name: None,
            icons: t.icons.clone(),
            enabled: true,
            cached_at: chrono::Utc::now(),
        });
    }
    if capabilities.as_ref().and_then(|c| c.prompts.as_ref()).is_some() {
        let items = run_preview_operation(
            "prompts/list",
            operation_timeout,
            discovery_helpers::collect_all_prompts(&service),
        )
        .await?;
        snap.prompts.extend(items);
    }
    if capabilities.as_ref().and_then(|c| c.resources.as_ref()).is_some() {
        let resources = run_preview_operation(
            "resources/list",
            operation_timeout,
            discovery_helpers::collect_all_resources(&service),
        )
        .await?;
        let templates = run_preview_operation(
            "resources/templates/list",
            operation_timeout,
            discovery_helpers::collect_all_resource_templates(&service),
        )
        .await?;
        snap.resources.extend(resources);
        snap.resource_templates.extend(templates);
    }
    Ok(snap)
}

/// Upsert an exact upstream prompt and its external identifier.
async fn upsert_shadow_prompt_row(
    tx: &mut Transaction<'_, Sqlite>,
    server_id: &str,
    server_name: &str,
    prompt_name: &str,
    unique_name: &str,
    description: Option<&str>,
) -> Result<()> {
    let id = crate::generate_id!("sprm");
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
    .bind(unique_name)
    .bind(description)
    .execute(&mut **tx)
    .await
    .context("Failed to upsert shadow prompt")?;
    Ok(())
}

pub async fn upsert_shadow_prompt(
    pool: &Pool<Sqlite>,
    server_id: &str,
    server_name: &str,
    prompt_name: &str,
    description: Option<&str>,
) -> Result<String> {
    let mut tx = begin_naming_transaction(pool)
        .await
        .context("Failed to begin shadow prompt update")?;
    let reconciliation = reconcile_external_identifier_additions(
        &mut tx,
        NamingKind::Prompt,
        server_id,
        server_name,
        &[prompt_name.to_string()],
    )
    .await?;
    let unique_name = reconciliation.identifier_for(prompt_name)?.to_string();
    upsert_shadow_prompt_row(&mut tx, server_id, server_name, prompt_name, &unique_name, description).await?;
    let catalog_changed = reconciliation.catalog_changed();
    tx.commit().await.context("Failed to commit shadow prompt update")?;
    if catalog_changed {
        crate::core::events::EventBus::global().publish(crate::core::events::Event::CapabilityCatalogChanged {
            server_id: server_id.to_string(),
            server_name: server_name.to_string(),
        });
    }
    Ok(unique_name)
}

/// Upsert an exact upstream resource URI and its external identifier.
async fn upsert_shadow_resource_row(
    tx: &mut Transaction<'_, Sqlite>,
    server_id: &str,
    server_name: &str,
    uri: &str,
    unique_uri: &str,
    name: Option<&str>,
    description: Option<&str>,
    mime_type: Option<&str>,
) -> Result<()> {
    let id = crate::generate_id!("sres");
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
    .bind(unique_uri)
    .bind(name)
    .bind(description)
    .bind(mime_type)
    .execute(&mut **tx)
    .await
    .context("Failed to upsert shadow resource")?;
    Ok(())
}

pub async fn upsert_shadow_resource(
    pool: &Pool<Sqlite>,
    server_id: &str,
    server_name: &str,
    uri: &str,
    name: Option<&str>,
    description: Option<&str>,
    mime_type: Option<&str>,
) -> Result<String> {
    let mut tx = begin_naming_transaction(pool)
        .await
        .context("Failed to begin shadow resource update")?;
    let reconciliation = reconcile_external_identifier_additions(
        &mut tx,
        NamingKind::Resource,
        server_id,
        server_name,
        &[uri.to_string()],
    )
    .await?;
    let unique_uri = reconciliation.identifier_for(uri)?.to_string();
    upsert_shadow_resource_row(
        &mut tx,
        server_id,
        server_name,
        uri,
        &unique_uri,
        name,
        description,
        mime_type,
    )
    .await?;
    let catalog_changed = reconciliation.catalog_changed();
    tx.commit().await.context("Failed to commit shadow resource update")?;
    if catalog_changed {
        crate::core::events::EventBus::global().publish(crate::core::events::Event::CapabilityCatalogChanged {
            server_id: server_id.to_string(),
            server_name: server_name.to_string(),
        });
    }
    Ok(unique_uri)
}

/// Upsert an exact upstream resource template and its external identifier.
async fn upsert_shadow_resource_template_row(
    tx: &mut Transaction<'_, Sqlite>,
    server_id: &str,
    server_name: &str,
    uri_template: &str,
    unique_name: &str,
    name: Option<&str>,
    description: Option<&str>,
) -> Result<()> {
    let id = crate::generate_id!("srst");
    let route_uri = crate::core::capability::resource_uri::template_match_key(unique_name)?;
    sqlx::query(
        r#"
        INSERT INTO server_resource_templates (id, server_id, server_name, uri_template, unique_name, route_uri, name, description)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(server_id, uri_template) DO UPDATE SET
            server_name = excluded.server_name,
            unique_name = excluded.unique_name,
            route_uri = excluded.route_uri,
            name = excluded.name,
            description = excluded.description,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(&id)
    .bind(server_id)
    .bind(server_name)
    .bind(uri_template)
    .bind(unique_name)
    .bind(route_uri)
    .bind(name)
    .bind(description)
    .execute(&mut **tx)
    .await
    .context("Failed to upsert shadow resource template")?;
    Ok(())
}

pub async fn upsert_shadow_resource_template(
    pool: &Pool<Sqlite>,
    server_id: &str,
    server_name: &str,
    uri_template: &str,
    name: Option<&str>,
    description: Option<&str>,
) -> Result<String> {
    let mut tx = begin_naming_transaction(pool)
        .await
        .context("Failed to begin shadow resource template update")?;
    let reconciliation = reconcile_external_identifier_additions(
        &mut tx,
        NamingKind::ResourceTemplate,
        server_id,
        server_name,
        &[uri_template.to_string()],
    )
    .await?;
    let unique_name = reconciliation.identifier_for(uri_template)?.to_string();
    upsert_shadow_resource_template_row(
        &mut tx,
        server_id,
        server_name,
        uri_template,
        &unique_name,
        name,
        description,
    )
    .await?;
    let catalog_changed = reconciliation.catalog_changed();
    tx.commit()
        .await
        .context("Failed to commit shadow resource template update")?;
    if catalog_changed {
        crate::core::events::EventBus::global().publish(crate::core::events::Event::CapabilityCatalogChanged {
            server_id: server_id.to_string(),
            server_name: server_name.to_string(),
        });
    }
    Ok(unique_name)
}

pub(crate) async fn upsert_shadow_prompts_batch(
    pool: &Pool<Sqlite>,
    server_id: &str,
    server_name: &str,
    prompts: &[CachedPromptInfo],
) -> Result<bool> {
    let mut tx = begin_naming_transaction(pool)
        .await
        .context("Failed to begin shadow prompt batch")?;
    let catalog_changed = upsert_shadow_prompts_batch_in_transaction(&mut tx, server_id, server_name, prompts).await?;
    tx.commit().await.context("Failed to commit shadow prompt batch")?;
    Ok(catalog_changed)
}

pub(crate) async fn upsert_shadow_prompts_batch_in_transaction(
    tx: &mut Transaction<'_, Sqlite>,
    server_id: &str,
    server_name: &str,
    prompts: &[CachedPromptInfo],
) -> Result<bool> {
    let inventory = prompts.iter().map(|prompt| prompt.name.clone()).collect::<Vec<_>>();
    let reconciliation =
        reconcile_external_identifiers(tx, NamingKind::Prompt, server_id, server_name, &inventory).await?;
    for prompt in prompts {
        upsert_shadow_prompt_row(
            tx,
            server_id,
            server_name,
            &prompt.name,
            reconciliation.identifier_for(&prompt.name)?,
            prompt.description.as_deref(),
        )
        .await?;
    }
    let catalog_changed = reconciliation.catalog_changed();
    Ok(catalog_changed)
}

pub(crate) async fn upsert_shadow_resources_batch(
    pool: &Pool<Sqlite>,
    server_id: &str,
    server_name: &str,
    resources: &[CachedResourceInfo],
) -> Result<bool> {
    let mut tx = begin_naming_transaction(pool)
        .await
        .context("Failed to begin shadow resource batch")?;
    let catalog_changed =
        upsert_shadow_resources_batch_in_transaction(&mut tx, server_id, server_name, resources).await?;
    tx.commit().await.context("Failed to commit shadow resource batch")?;
    Ok(catalog_changed)
}

pub(crate) async fn upsert_shadow_resources_batch_in_transaction(
    tx: &mut Transaction<'_, Sqlite>,
    server_id: &str,
    server_name: &str,
    resources: &[CachedResourceInfo],
) -> Result<bool> {
    let inventory = resources
        .iter()
        .map(|resource| resource.uri.clone())
        .collect::<Vec<_>>();
    let reconciliation =
        reconcile_external_identifiers(tx, NamingKind::Resource, server_id, server_name, &inventory).await?;
    for resource in resources {
        upsert_shadow_resource_row(
            tx,
            server_id,
            server_name,
            &resource.uri,
            reconciliation.identifier_for(&resource.uri)?,
            resource.name.as_deref(),
            resource.description.as_deref(),
            resource.mime_type.as_deref(),
        )
        .await?;
    }
    let catalog_changed = reconciliation.catalog_changed();
    Ok(catalog_changed)
}

pub(crate) async fn upsert_shadow_resource_templates_batch(
    pool: &Pool<Sqlite>,
    server_id: &str,
    server_name: &str,
    templates: &[CachedResourceTemplateInfo],
) -> Result<bool> {
    let mut tx = begin_naming_transaction(pool)
        .await
        .context("Failed to begin shadow resource template batch")?;
    let catalog_changed =
        upsert_shadow_resource_templates_batch_in_transaction(&mut tx, server_id, server_name, templates).await?;
    tx.commit()
        .await
        .context("Failed to commit shadow resource template batch")?;
    Ok(catalog_changed)
}

pub(crate) async fn upsert_shadow_resource_templates_batch_in_transaction(
    tx: &mut Transaction<'_, Sqlite>,
    server_id: &str,
    server_name: &str,
    templates: &[CachedResourceTemplateInfo],
) -> Result<bool> {
    let mut projectable_templates = Vec::new();
    for template in templates {
        if crate::core::capability::resource_uri::resource_template_is_projectable(server_name, &template.uri_template)?
        {
            projectable_templates.push(template);
        } else {
            tracing::warn!(
                server_id,
                server_name,
                upstream_template = %template.uri_template,
                "Resource Template remains available upstream but cannot enter the canonical address space"
            );
        }
    }
    let inventory = projectable_templates
        .iter()
        .map(|template| template.uri_template.clone())
        .collect::<Vec<_>>();
    let reconciliation =
        reconcile_external_identifiers(tx, NamingKind::ResourceTemplate, server_id, server_name, &inventory).await?;
    for template in projectable_templates {
        upsert_shadow_resource_template_row(
            tx,
            server_id,
            server_name,
            &template.uri_template,
            reconciliation.identifier_for(&template.uri_template)?,
            template.name.as_deref(),
            template.description.as_deref(),
        )
        .await?;
    }
    let catalog_changed = reconciliation.catalog_changed();
    Ok(catalog_changed)
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
    protocol_version: Option<&str>,
) -> Result<()> {
    let protocol_version = protocol_version.unwrap_or("unknown").to_string();
    let server_data = CachedServerData {
        server_id: server_id.to_string(),
        server_name: server_name.to_string(),
        server_version: None,
        protocol_version,
        tools,
        resources,
        prompts,
        resource_templates,
        cached_at: chrono::Utc::now(),
        fingerprint: format!("store:{}:{}", server_id, chrono::Utc::now().timestamp()),
        scope: crate::core::cache::CacheScope::shared_raw(),
    };
    redb.store_server_data(&server_data)
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))
}

/// Atomically replace the complete REDB capability snapshot for a server.
pub async fn replace_redb_snapshot(
    redb: &RedbCacheManager,
    server_id: &str,
    server_name: &str,
    tools: Vec<CachedToolInfo>,
    resources: Vec<CachedResourceInfo>,
    prompts: Vec<CachedPromptInfo>,
    resource_templates: Vec<CachedResourceTemplateInfo>,
    protocol_version: Option<&str>,
) -> Result<()> {
    let protocol_version = protocol_version.unwrap_or("unknown").to_string();
    let server_data = CachedServerData {
        server_id: server_id.to_string(),
        server_name: server_name.to_string(),
        server_version: None,
        protocol_version,
        tools,
        resources,
        prompts,
        resource_templates,
        cached_at: chrono::Utc::now(),
        fingerprint: format!("replace:{}:{}", server_id, chrono::Utc::now().timestamp()),
        scope: crate::core::cache::CacheScope::shared_raw(),
    };
    redb.replace_server_data(&server_data)
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))
}

/// Store snapshot in REDB with a specific cache scope (for client-filtered entries).
pub async fn store_redb_snapshot_with_scope(
    redb: &RedbCacheManager,
    server_id: &str,
    server_name: &str,
    tools: Vec<CachedToolInfo>,
    resources: Vec<CachedResourceInfo>,
    prompts: Vec<CachedPromptInfo>,
    resource_templates: Vec<CachedResourceTemplateInfo>,
    protocol_version: Option<&str>,
    scope: crate::core::cache::CacheScope,
) -> Result<()> {
    let protocol_version = protocol_version.unwrap_or("unknown").to_string();
    let server_data = CachedServerData {
        server_id: server_id.to_string(),
        server_name: server_name.to_string(),
        server_version: None,
        protocol_version,
        tools,
        resources,
        prompts,
        resource_templates,
        cached_at: chrono::Utc::now(),
        fingerprint: format!("filtered:{}:{}", server_id, chrono::Utc::now().timestamp()),
        scope,
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
    tools: Vec<CachedToolInfo>,
    resources: Vec<CachedResourceInfo>,
    prompts: Vec<CachedPromptInfo>,
    templates: Vec<CachedResourceTemplateInfo>,
    protocol_version: Option<String>,
) -> Result<()> {
    store_dual_write_for_kinds(
        pool,
        redb,
        server_id,
        server_name,
        tools,
        resources,
        prompts,
        templates,
        protocol_version,
        crate::core::pool::CapSyncFlags::ALL,
    )
    .await
}

pub(crate) async fn apply_snapshot_catalog_in_transaction(
    tx: &mut Transaction<'_, Sqlite>,
    server_id: &str,
    server_name: &str,
    snapshot: &mut CapabilitySnapshot,
) -> Result<bool> {
    let mut catalog_changed = crate::config::server::tools::assign_unique_names_to_cached_tools_in_transaction(
        tx,
        server_id,
        server_name,
        &mut snapshot.tools,
    )
    .await?;
    catalog_changed |=
        upsert_shadow_prompts_batch_in_transaction(tx, server_id, server_name, &snapshot.prompts).await?;
    catalog_changed |=
        upsert_shadow_resources_batch_in_transaction(tx, server_id, server_name, &snapshot.resources).await?;
    catalog_changed |=
        upsert_shadow_resource_templates_batch_in_transaction(tx, server_id, server_name, &snapshot.resource_templates)
            .await?;
    Ok(catalog_changed)
}

fn client_visible_inventory_changed<T: serde::Serialize>(
    previous: &[T],
    current: &[T],
) -> Result<bool> {
    fn projection<T: serde::Serialize>(items: &[T]) -> Result<serde_json::Value> {
        let mut value =
            serde_json::to_value(items).context("Failed to serialize client-visible capability inventory")?;
        if let Some(items) = value.as_array_mut() {
            for item in items {
                if let Some(fields) = item.as_object_mut() {
                    fields.remove("cached_at");
                }
            }
        }
        Ok(value)
    }

    Ok(projection(previous)? != projection(current)?)
}

#[derive(Debug, thiserror::Error)]
#[error("Capability catalog committed for server '{server_id}', but cache convergence is pending: {reason}")]
pub struct CapabilityCacheConvergencePending {
    pub server_id: String,
    pub reason: String,
}

async fn finish_committed_snapshot(
    redb: &RedbCacheManager,
    server_id: &str,
    server_name: &str,
    catalog_changed: bool,
    invalidation_result: std::result::Result<usize, crate::core::cache::CacheError>,
) -> Result<()> {
    redb.clear_refreshing(server_id).await;
    if catalog_changed {
        crate::core::events::EventBus::global().publish(crate::core::events::Event::CapabilityCatalogChanged {
            server_id: server_id.to_string(),
            server_name: server_name.to_string(),
        });
    }

    invalidation_result.map(|_| ()).map_err(|error| {
        CapabilityCacheConvergencePending {
            server_id: server_id.to_string(),
            reason: error.to_string(),
        }
        .into()
    })
}

/// Persist authoritative inventories for the selected capability kinds while
/// preserving untouched kinds in the shared raw cache.
pub async fn store_dual_write_for_kinds(
    pool: &Pool<Sqlite>,
    redb: &RedbCacheManager,
    server_id: &str,
    server_name: &str,
    mut tools: Vec<CachedToolInfo>,
    resources: Vec<CachedResourceInfo>,
    prompts: Vec<CachedPromptInfo>,
    templates: Vec<CachedResourceTemplateInfo>,
    protocol_version: Option<String>,
    kinds: crate::core::pool::CapSyncFlags,
) -> Result<()> {
    let is_full_snapshot = kinds == crate::core::pool::CapSyncFlags::ALL;
    let existing_result = redb
        .get_server_data(&crate::core::cache::CacheQuery {
            server_id: server_id.to_string(),
            freshness_level: crate::core::cache::FreshnessLevel::Cached,
            include_disabled: true,
            scope: crate::core::cache::CacheScope::shared_raw(),
        })
        .await;
    let existing = if is_full_snapshot {
        existing_result.ok().and_then(|result| result.data)
    } else {
        Some(
            existing_result
            .map_err(|error| anyhow::anyhow!(error.to_string()))?
            .data
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Cannot apply partial capability refresh for server '{server_id}' without an existing shared raw baseline"
                )
            })?,
        )
    };

    let mut tx = begin_naming_transaction(pool)
        .await
        .context("Failed to begin authoritative capability snapshot update")?;
    let mut catalog_changed = false;
    if kinds.contains(crate::core::pool::CapSyncFlags::TOOLS) {
        catalog_changed |= crate::config::server::tools::assign_unique_names_to_cached_tools_in_transaction(
            &mut tx,
            server_id,
            server_name,
            &mut tools,
        )
        .await?;
    }

    if kinds.contains(crate::core::pool::CapSyncFlags::PROMPTS) {
        catalog_changed |=
            upsert_shadow_prompts_batch_in_transaction(&mut tx, server_id, server_name, &prompts).await?;
    }
    if kinds.contains(crate::core::pool::CapSyncFlags::RESOURCES) {
        catalog_changed |=
            upsert_shadow_resources_batch_in_transaction(&mut tx, server_id, server_name, &resources).await?;
    }
    if kinds.contains(crate::core::pool::CapSyncFlags::RESOURCE_TEMPLATES) {
        catalog_changed |=
            upsert_shadow_resource_templates_batch_in_transaction(&mut tx, server_id, server_name, &templates).await?;
    }

    let existing_protocol_version = existing.as_ref().map(|snapshot| snapshot.protocol_version.clone());
    let merged_tools = if kinds.contains(crate::core::pool::CapSyncFlags::TOOLS) {
        tools
    } else {
        existing
            .as_ref()
            .map(|snapshot| snapshot.tools.clone())
            .unwrap_or_default()
    };
    let merged_resources = if kinds.contains(crate::core::pool::CapSyncFlags::RESOURCES) {
        resources
    } else {
        existing
            .as_ref()
            .map(|snapshot| snapshot.resources.clone())
            .unwrap_or_default()
    };
    let merged_prompts = if kinds.contains(crate::core::pool::CapSyncFlags::PROMPTS) {
        prompts
    } else {
        existing
            .as_ref()
            .map(|snapshot| snapshot.prompts.clone())
            .unwrap_or_default()
    };
    let merged_templates = if kinds.contains(crate::core::pool::CapSyncFlags::RESOURCE_TEMPLATES) {
        templates
    } else {
        existing
            .as_ref()
            .map(|snapshot| snapshot.resource_templates.clone())
            .unwrap_or_default()
    };
    let protocol_version = protocol_version.or(existing_protocol_version);

    if let Some(previous) = existing.as_ref() {
        if kinds.contains(crate::core::pool::CapSyncFlags::TOOLS) {
            catalog_changed |= client_visible_inventory_changed(&previous.tools, &merged_tools)?;
        }
        if kinds.contains(crate::core::pool::CapSyncFlags::RESOURCES) {
            catalog_changed |= client_visible_inventory_changed(&previous.resources, &merged_resources)?;
        }
        if kinds.contains(crate::core::pool::CapSyncFlags::PROMPTS) {
            catalog_changed |= client_visible_inventory_changed(&previous.prompts, &merged_prompts)?;
        }
        if kinds.contains(crate::core::pool::CapSyncFlags::RESOURCE_TEMPLATES) {
            catalog_changed |= client_visible_inventory_changed(&previous.resource_templates, &merged_templates)?;
        }
    }

    let cache_write_result = if is_full_snapshot {
        replace_redb_snapshot(
            redb,
            server_id,
            server_name,
            merged_tools,
            merged_resources,
            merged_prompts,
            merged_templates,
            protocol_version.as_deref(),
        )
        .await
    } else {
        store_redb_snapshot(
            redb,
            server_id,
            server_name,
            merged_tools,
            merged_resources,
            merged_prompts,
            merged_templates,
            protocol_version.as_deref(),
        )
        .await
    };
    cache_write_result.context("Failed to write authoritative capability cache")?;
    if let Err(error) = tx.commit().await {
        return match redb.remove_server_data(server_id).await {
            Ok(()) => Err(error).context("Failed to commit authoritative capability catalog"),
            Err(cleanup_error) => Err(anyhow::anyhow!(
                "Failed to commit authoritative capability catalog: {error}; additionally failed to remove capability cache: {cleanup_error}"
            )),
        }
        .map_err(|error| error.context(format!("server_id={server_id}")));
    }
    let invalidation_result = redb.invalidate_client_filtered_by_server(server_id).await;
    finish_committed_snapshot(redb, server_id, server_name, catalog_changed, invalidation_result).await
}

async fn profile_has_seed_tool(
    pool: &Pool<Sqlite>,
    profile_id: &str,
    server_id: &str,
    upstream_tool_name: &str,
) -> bool {
    sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1
            FROM profile_tool pt
            JOIN server_tools st ON st.id = pt.server_tool_id
			WHERE pt.profile_id = ?
			  AND st.server_id = ?
			  AND st.tool_name = ?
        )
        "#,
    )
    .bind(profile_id)
    .bind(server_id)
    .bind(upstream_tool_name)
    .fetch_one(pool)
    .await
    .unwrap_or(false)
}

async fn profile_has_seed_capability(
    pool: &Pool<Sqlite>,
    profile_id: &str,
    server_id: &str,
    table: &str,
    value_column: &str,
    value: &str,
) -> bool {
    let query =
        format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE profile_id = ? AND server_id = ? AND {value_column} = ?)");
    sqlx::query_scalar::<_, bool>(&query)
        .bind(profile_id)
        .bind(server_id)
        .bind(value)
        .fetch_one(pool)
        .await
        .unwrap_or(false)
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

        // Tools: only seed missing rows; never override existing user toggles.
        for t in &snapshot.tools {
            if !profile_has_seed_tool(pool, profile_id, server_id, &t.name).await {
                let _ = crate::config::profile::add_tool_to_profile(pool, profile_id, server_id, &t.name, true).await;
            }
        }
        // Resources: only seed missing rows; never override existing user toggles.
        for r in &snapshot.resources {
            if !profile_has_seed_capability(pool, profile_id, server_id, "profile_resource", "resource_uri", &r.uri)
                .await
            {
                let _ =
                    crate::config::profile::add_resource_to_profile(pool, profile_id, server_id, &r.uri, true).await;
            }
        }
        // Prompts: only seed missing rows; never override existing user toggles.
        for p in &snapshot.prompts {
            if !profile_has_seed_capability(pool, profile_id, server_id, "profile_prompt", "prompt_name", &p.name).await
            {
                let _ = crate::config::profile::add_prompt_to_profile(pool, profile_id, server_id, &p.name, true).await;
            }
        }

        // Resource templates: only seed missing rows; never override existing user toggles.
        for t in &snapshot.resource_templates {
            if !profile_has_seed_capability(
                pool,
                profile_id,
                server_id,
                "profile_resource_template",
                "uri_template",
                &t.uri_template,
            )
            .await
            {
                let _ = crate::config::profile::add_resource_template_to_profile(
                    pool,
                    profile_id,
                    server_id,
                    &t.uri_template,
                    true,
                )
                .await;
            }
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
        .get_or_create_validation_instance(server_id, "api", Duration::from_secs(5 * 60))
        .await
    {
        Ok(Some(c)) => c,
        Ok(None) => anyhow::bail!(
            "No validation instance is available for capability sync of server '{}'",
            server_name
        ),
        Err(error) => {
            return Err(error).with_context(|| {
                format!("Failed to create a validation instance for capability sync of server '{server_name}'")
            });
        }
    };

    // Discover and apply (now fully paginated)
    let sync_result = async {
        let snap = discover_from_connection(conn).await?;
        discovery_helpers::apply_snapshot(db_pool, redb, server_id, server_name, &snap, true).await
    }
    .await;

    // Cleanup
    if let Err(e) = pool.destroy_validation_instance(server_id, "api").await {
        tracing::trace!(server_name = %server_name, error = %e, "Failed to destroy validation instance (api)");
    }

    if let Err(error) = sync_result {
        if let Some(collision) =
            crate::config::server::namespace_repair::record_capability_collision_from_error(db_pool, &error).await?
        {
            pool.block_server_after_capability_collision(&collision.server_id).await;
            pool.sync_servers_from_active_profile().await.with_context(|| {
                format!(
                    "Failed to block server '{}' after external capability collision",
                    collision.server_id
                )
            })?;
            tracing::warn!(
                server_id = %collision.server_id,
                conflicting_server_id = %collision.conflicting_server_id,
                external_identifier = %collision.external_identifier,
                "Blocked server after external capability collision; namespace remediation is required"
            );
        }
        return Err(error);
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
            SyncStrategy::FromConnection => self.discover_from_existing_connection(server_id, server_name).await?,
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
                    headers: None,
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
                            headers: None,
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
        server_id: &str,
        server_name: &str,
    ) -> Result<CapabilitySnapshot> {
        let mut pool = self.connection_pool.lock().await;
        let session_id = "capability_sync";
        let conn = pool
            .get_or_create_validation_instance(server_id, session_id, tokio::time::Duration::from_secs(30))
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to get connection for server '{}'", server_name))?;

        let snapshot = discover_from_connection(conn).await?;

        // Cleanup validation instance
        if let Err(e) = pool.destroy_validation_instance(server_id, session_id).await {
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
                    headers: None,
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
/// Resolve the default lock timeout (seconds) to use when acquiring the upstream
/// connection pool. Allow overriding via `MCPMATE_CAPABILITY_POOL_LOCK_TIMEOUT_SECS`
/// to accommodate environments where upstream servers have heavy cold-start costs.
pub fn default_pool_lock_timeout_secs() -> u64 {
    static TIMEOUT: OnceCell<u64> = OnceCell::new();
    *TIMEOUT.get_or_init(|| {
        std::env::var("MCPMATE_CAPABILITY_POOL_LOCK_TIMEOUT_SECS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(60)
    })
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use sqlx::sqlite::SqlitePoolOptions;
    use std::collections::HashMap;
    use std::path::Path;
    use tempfile::TempDir;

    use super::*;
    use crate::core::cache::fingerprint::{
        CapabilityFingerprint, CodeFingerprint, ConfigFingerprint, DependencyFingerprint, MCPServerFingerprint,
    };
    use crate::core::cache::operations::CacheOperations;
    use crate::core::cache::{CacheQuery, CacheScope, CachedServerData, FreshnessLevel, SERVERS_TABLE};

    async fn capability_store_pool() -> Pool<Sqlite> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("connect in-memory database");
        crate::config::server::init::initialize_server_tables(&pool)
            .await
            .expect("initialize server tables");
        crate::config::profile::init::initialize_profile_tables(&pool)
            .await
            .expect("initialize profile tables");
        crate::config::client::init::initialize_client_table(&pool)
            .await
            .expect("initialize client table");
        sqlx::query("INSERT INTO server_config (id, name, server_type) VALUES ('server-a', 'docs', 'stdio')")
            .execute(&pool)
            .await
            .expect("insert server");
        pool
    }

    #[tokio::test]
    async fn raw_snapshot_keeps_templates_that_cannot_enter_external_projection() {
        let pool = capability_store_pool().await;
        let cache_dir = TempDir::new().expect("create cache directory");
        let redb = RedbCacheManager::new(
            cache_dir.path().join("resource-template-projection.redb"),
            crate::core::cache::manager::CacheConfig::default(),
        )
        .expect("create cache manager");

        store_dual_write(
            &pool,
            &redb,
            "server-a",
            "docs",
            Vec::new(),
            Vec::new(),
            Vec::new(),
            vec![cached_template("file:///{path}"), cached_template("file:///{+path}")],
            Some("2025-11-25".to_string()),
        )
        .await
        .expect("store raw templates independently from external projection");

        let cached = redb
            .get_server_data(&CacheQuery {
                server_id: "server-a".to_string(),
                freshness_level: FreshnessLevel::Cached,
                include_disabled: true,
                scope: CacheScope::shared_raw(),
            })
            .await
            .expect("read raw snapshot")
            .data
            .expect("raw snapshot exists");
        assert_eq!(cached.resource_templates.len(), 2);

        let projected = sqlx::query_scalar::<_, String>(
            "SELECT uri_template FROM server_resource_templates WHERE server_id = ? ORDER BY uri_template",
        )
        .bind("server-a")
        .fetch_all(&pool)
        .await
        .expect("load projected templates");
        assert_eq!(projected, vec!["file:///{path}".to_string()]);
    }

    fn cached_tool(name: &str) -> CachedToolInfo {
        CachedToolInfo {
            name: name.to_string(),
            description: None,
            input_schema_json: r#"{"type":"object"}"#.to_string(),
            output_schema_json: Some(r#"{"type":"object"}"#.to_string()),
            unique_name: None,
            icons: None,
            enabled: true,
            cached_at: Utc::now(),
        }
    }

    fn cached_resource(uri: &str) -> CachedResourceInfo {
        CachedResourceInfo {
            uri: uri.to_string(),
            name: Some(uri.to_string()),
            description: None,
            mime_type: None,
            icons: None,
            enabled: true,
            cached_at: Utc::now(),
        }
    }

    fn cached_prompt(name: &str) -> CachedPromptInfo {
        CachedPromptInfo {
            name: name.to_string(),
            description: None,
            arguments: Vec::new(),
            icons: None,
            enabled: true,
            cached_at: Utc::now(),
        }
    }

    fn cached_template(uri_template: &str) -> CachedResourceTemplateInfo {
        CachedResourceTemplateInfo {
            uri_template: uri_template.to_string(),
            name: Some(uri_template.to_string()),
            description: None,
            mime_type: None,
            enabled: true,
            cached_at: Utc::now(),
        }
    }

    fn test_fingerprint() -> MCPServerFingerprint {
        MCPServerFingerprint {
            code_fingerprint: CodeFingerprint {
                file_hashes: HashMap::new(),
                total_files: 1,
                total_size: 1,
                last_modified: Utc::now(),
            },
            dependency_fingerprint: DependencyFingerprint {
                package_lock_hash: None,
                manifest_hash: "manifest".to_string(),
                resolved_versions: HashMap::new(),
            },
            capability_fingerprint: CapabilityFingerprint {
                tools_hash: "tools".to_string(),
                resources_hash: "resources".to_string(),
                prompts_hash: "prompts".to_string(),
                server_info_hash: "server".to_string(),
            },
            config_fingerprint: ConfigFingerprint {
                server_config_hash: "config".to_string(),
                environment_hash: "environment".to_string(),
                arguments_hash: "arguments".to_string(),
            },
            combined_hash: "combined".to_string(),
            generated_at: Utc::now(),
        }
    }

    fn create_corrupt_shared_raw_cache(path: &Path) {
        let database = redb::Database::create(path).expect("create corrupt cache fixture");
        let old_snapshot = CachedServerData {
            server_id: "server-a".to_string(),
            server_name: "docs".to_string(),
            server_version: None,
            protocol_version: "2024-11-05".to_string(),
            tools: vec![cached_tool("old_tool")],
            resources: vec![cached_resource("file:///old")],
            prompts: vec![cached_prompt("old_prompt")],
            resource_templates: vec![cached_template("file:///{old}")],
            cached_at: Utc::now(),
            fingerprint: "old-snapshot".to_string(),
            scope: CacheScope::shared_raw(),
        };
        CacheOperations::new(&database)
            .store_server_data(&old_snapshot)
            .expect("store old per-kind cache entries");

        let write_txn = database.begin_write().expect("begin corrupting cache entry");
        {
            let mut servers = write_txn.open_table(SERVERS_TABLE).expect("open server cache table");
            servers
                .insert("server-a#production#raw", &[0xff, 0x00][..])
                .expect("replace server snapshot with invalid bincode");
        }
        write_txn.commit().expect("commit corrupt cache fixture");
    }

    #[tokio::test]
    async fn full_snapshot_replaces_corrupt_server_data_and_stale_per_kind_entries() {
        let pool = capability_store_pool().await;
        let cache_dir = TempDir::new().expect("create cache directory");
        let cache_path = cache_dir.path().join("corrupt-full.redb");
        create_corrupt_shared_raw_cache(&cache_path);
        let redb = RedbCacheManager::new(cache_path, crate::core::cache::manager::CacheConfig::default())
            .expect("open corrupt cache fixture");
        let fingerprint = test_fingerprint();
        redb.store_fingerprint("server-a", &fingerprint)
            .await
            .expect("store server fingerprint");

        store_dual_write_for_kinds(
            &pool,
            &redb,
            "server-a",
            "docs",
            vec![cached_tool("new_tool")],
            vec![cached_resource("file:///new")],
            vec![cached_prompt("new_prompt")],
            vec![cached_template("file:///{new}")],
            Some("2025-11-25".to_string()),
            crate::core::pool::CapSyncFlags::ALL,
        )
        .await
        .expect("full snapshot should replace corrupt cache data");

        let cached = redb
            .get_server_data(&CacheQuery {
                server_id: "server-a".to_string(),
                freshness_level: FreshnessLevel::Cached,
                include_disabled: true,
                scope: CacheScope::shared_raw(),
            })
            .await
            .expect("read rebuilt full snapshot")
            .data
            .expect("rebuilt full snapshot exists");
        assert_eq!(cached.protocol_version, "2025-11-25");
        assert_eq!(
            cached.tools.iter().map(|item| item.name.as_str()).collect::<Vec<_>>(),
            ["new_tool"]
        );
        assert_eq!(
            cached
                .resources
                .iter()
                .map(|item| item.uri.as_str())
                .collect::<Vec<_>>(),
            ["file:///new"]
        );
        assert_eq!(
            cached.prompts.iter().map(|item| item.name.as_str()).collect::<Vec<_>>(),
            ["new_prompt"]
        );
        assert_eq!(
            cached
                .resource_templates
                .iter()
                .map(|item| item.uri_template.as_str())
                .collect::<Vec<_>>(),
            ["file:///{new}"]
        );

        assert_eq!(
            redb.get_server_tools("server-a", true)
                .await
                .expect("read rebuilt tools")
                .iter()
                .map(|item| item.name.as_str())
                .collect::<Vec<_>>(),
            ["new_tool"]
        );
        assert_eq!(
            redb.get_server_resources("server-a", true)
                .await
                .expect("read rebuilt resources")
                .iter()
                .map(|item| item.uri.as_str())
                .collect::<Vec<_>>(),
            ["file:///new"]
        );
        assert_eq!(
            redb.get_server_prompts("server-a", true)
                .await
                .expect("read rebuilt prompts")
                .iter()
                .map(|item| item.name.as_str())
                .collect::<Vec<_>>(),
            ["new_prompt"]
        );
        assert_eq!(
            redb.get_server_resource_templates("server-a", true)
                .await
                .expect("read rebuilt resource templates")
                .iter()
                .map(|item| item.uri_template.as_str())
                .collect::<Vec<_>>(),
            ["file:///{new}"]
        );
        assert_eq!(
            redb.get_fingerprint("server-a").await.expect("read server fingerprint"),
            Some(fingerprint)
        );
    }

    #[tokio::test]
    async fn partial_snapshot_rejects_corrupt_server_data_without_clearing_other_kinds() {
        let pool = capability_store_pool().await;
        let cache_dir = TempDir::new().expect("create cache directory");
        let cache_path = cache_dir.path().join("corrupt-partial.redb");
        create_corrupt_shared_raw_cache(&cache_path);
        let redb = RedbCacheManager::new(cache_path, crate::core::cache::manager::CacheConfig::default())
            .expect("open corrupt cache fixture");
        crate::config::server::tools::upsert_server_tool(&pool, "server-a", "docs", "old_tool", None)
            .await
            .expect("store old tool catalog row");
        upsert_shadow_prompt(&pool, "server-a", "docs", "old_prompt", None)
            .await
            .expect("store old prompt catalog row");
        upsert_shadow_resource(&pool, "server-a", "docs", "file:///old", None, None, None)
            .await
            .expect("store old resource catalog row");
        upsert_shadow_resource_template(&pool, "server-a", "docs", "file:///{old}", Some("Old"), None)
            .await
            .expect("store old resource template catalog row");

        let error = store_dual_write_for_kinds(
            &pool,
            &redb,
            "server-a",
            "docs",
            vec![cached_tool("new_tool")],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Some("2025-11-25".to_string()),
            crate::core::pool::CapSyncFlags::TOOLS,
        )
        .await
        .expect_err("partial snapshot must not replace a corrupt full snapshot");

        assert!(error.to_string().contains("Serialization error"));
        assert_eq!(
            redb.get_server_tools("server-a", true)
                .await
                .expect("old tool entries remain")
                .iter()
                .map(|item| item.name.as_str())
                .collect::<Vec<_>>(),
            ["old_tool"]
        );
        assert_eq!(
            redb.get_server_resources("server-a", true)
                .await
                .expect("old resource entries remain")
                .iter()
                .map(|item| item.uri.as_str())
                .collect::<Vec<_>>(),
            ["file:///old"]
        );
        assert_eq!(
            redb.get_server_prompts("server-a", true)
                .await
                .expect("old prompt entries remain")
                .iter()
                .map(|item| item.name.as_str())
                .collect::<Vec<_>>(),
            ["old_prompt"]
        );
        assert_eq!(
            redb.get_server_resource_templates("server-a", true)
                .await
                .expect("old resource template entries remain")
                .iter()
                .map(|item| item.uri_template.as_str())
                .collect::<Vec<_>>(),
            ["file:///{old}"]
        );

        for (table, column, expected) in [
            ("server_tools", "tool_name", "old_tool"),
            ("server_prompts", "prompt_name", "old_prompt"),
            ("server_resources", "resource_uri", "file:///old"),
            ("server_resource_templates", "uri_template", "file:///{old}"),
        ] {
            let query = format!("SELECT {column} FROM {table} WHERE server_id = 'server-a'");
            let values: Vec<String> = sqlx::query_scalar(&query)
                .fetch_all(&pool)
                .await
                .unwrap_or_else(|error| panic!("load {table} catalog rows: {error}"));
            assert_eq!(values, [expected], "{table} catalog changed after partial failure");
        }
    }

    #[tokio::test]
    async fn partial_snapshot_requires_an_existing_shared_raw_baseline() {
        let pool = capability_store_pool().await;
        let cache_dir = TempDir::new().expect("create cache directory");
        let redb = RedbCacheManager::new(
            cache_dir.path().join("missing-partial-baseline.redb"),
            crate::core::cache::manager::CacheConfig::default(),
        )
        .expect("create cache manager");

        let error = store_dual_write_for_kinds(
            &pool,
            &redb,
            "server-a",
            "docs",
            vec![cached_tool("new_tool")],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Some("2025-11-25".to_string()),
            crate::core::pool::CapSyncFlags::TOOLS,
        )
        .await
        .expect_err("partial refresh without a shared raw baseline must fail");

        assert!(error.to_string().contains("shared raw baseline"));
        let stored_tools = sqlx::query_scalar::<_, String>(
            "SELECT tool_name FROM server_tools WHERE server_id = 'server-a' ORDER BY tool_name",
        )
        .fetch_all(&pool)
        .await
        .expect("load tool catalog");
        assert!(stored_tools.is_empty());
    }

    #[tokio::test]
    async fn client_visible_tool_metadata_change_emits_catalog_changed() {
        let pool = capability_store_pool().await;
        let cache_dir = TempDir::new().expect("create cache directory");
        let redb = RedbCacheManager::new(
            cache_dir.path().join("tool-metadata-change.redb"),
            crate::core::cache::manager::CacheConfig::default(),
        )
        .expect("create cache manager");
        let mut initial = cached_tool("read");
        initial.description = Some("Initial description".to_string());
        store_dual_write(
            &pool,
            &redb,
            "server-a",
            "docs",
            vec![initial],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Some("2025-11-25".to_string()),
        )
        .await
        .expect("store initial snapshot");

        let mut events = crate::core::events::EventBus::global().subscribe_async();
        let mut updated = cached_tool("read");
        updated.description = Some("Updated description".to_string());
        store_dual_write(
            &pool,
            &redb,
            "server-a",
            "docs",
            vec![updated],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Some("2025-11-25".to_string()),
        )
        .await
        .expect("store metadata-only snapshot change");

        let event = tokio::time::timeout(std::time::Duration::from_secs(1), async move {
            loop {
                match events.recv().await {
                    Ok(crate::core::events::Event::CapabilityCatalogChanged { server_id, .. })
                        if server_id == "server-a" =>
                    {
                        break;
                    }
                    Ok(_) => continue,
                    Err(error) => panic!("event receiver failed: {error}"),
                }
            }
        })
        .await;
        assert!(event.is_ok(), "metadata-only change must notify downstream clients");
    }

    #[tokio::test]
    async fn post_commit_cache_failure_clears_refreshing_and_still_notifies_catalog_change() {
        let cache_dir = TempDir::new().expect("create cache directory");
        let redb = RedbCacheManager::new(
            cache_dir.path().join("post-commit-cache-failure.redb"),
            crate::core::cache::manager::CacheConfig::default(),
        )
        .expect("create cache manager");
        redb.set_refreshing("server-a", std::time::Duration::from_secs(60))
            .await;
        let mut events = crate::core::events::EventBus::global().subscribe_async();

        let error = finish_committed_snapshot(
            &redb,
            "server-a",
            "docs",
            true,
            Err(crate::core::cache::CacheError::ConcurrentAccess),
        )
        .await
        .expect_err("post-commit invalidation failure must remain visible");

        assert!(error.to_string().contains("catalog committed"));
        assert!(error.to_string().contains("cache convergence is pending"));
        assert!(!redb.is_refreshing("server-a").await);
        let event = tokio::time::timeout(std::time::Duration::from_secs(1), async move {
            loop {
                match events.recv().await {
                    Ok(crate::core::events::Event::CapabilityCatalogChanged { server_id, .. })
                        if server_id == "server-a" =>
                    {
                        break;
                    }
                    Ok(_) => continue,
                    Err(error) => panic!("event receiver failed: {error}"),
                }
            }
        })
        .await;
        assert!(event.is_ok(), "committed catalog change must still be announced");
    }

    #[tokio::test]
    async fn preview_timeout_is_applied_to_each_operation_independently() {
        let timeout = Duration::from_millis(40);

        let first = run_preview_operation("prompts/list", Some(timeout), async {
            tokio::time::sleep(Duration::from_millis(30)).await;
            Ok::<_, anyhow::Error>("prompts")
        })
        .await
        .expect("first operation should finish within its own deadline");
        let second = run_preview_operation("resources/list", Some(timeout), async {
            tokio::time::sleep(Duration::from_millis(30)).await;
            Ok::<_, anyhow::Error>("resources")
        })
        .await
        .expect("second operation should receive a fresh deadline");

        assert_eq!(first, "prompts");
        assert_eq!(second, "resources");
    }

    #[tokio::test]
    async fn preview_timeout_identifies_the_failed_operation() {
        let error = run_preview_operation("resources/templates/list", Some(Duration::from_millis(5)), async {
            tokio::time::sleep(Duration::from_millis(30)).await;
            Ok::<_, anyhow::Error>(())
        })
        .await
        .expect_err("slow operation must time out");

        assert!(error.to_string().contains("resources/templates/list"));
    }

    #[test]
    fn package_runner_preview_uses_independent_startup_timeout() {
        let operation_timeout = Duration::from_secs(17);

        for command in ["uvx", "/managed/bin/bunx", r"C:\runtime\npx.exe"] {
            let timeouts = preview_stdio_timeouts(command, Some(operation_timeout));

            assert_eq!(timeouts.startup, Duration::from_secs(5 * 60));
            assert_eq!(timeouts.tools, operation_timeout);
            assert!(timeouts.package_runner);
        }
    }

    #[test]
    fn direct_binary_preview_keeps_operation_timeout_for_startup() {
        let operation_timeout = Duration::from_secs(17);
        let timeouts = preview_stdio_timeouts("paddleocr_mcp", Some(operation_timeout));

        assert_eq!(timeouts.startup, operation_timeout);
        assert_eq!(timeouts.tools, operation_timeout);
        assert!(!timeouts.package_runner);
    }

    #[tokio::test]
    async fn snapshot_identity_is_persisted_without_overwriting_namespace() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("connect in-memory database");
        crate::config::server::init::initialize_server_tables(&pool)
            .await
            .expect("initialize server tables");
        sqlx::query("INSERT INTO server_config (id, name, server_type) VALUES ('server-a', 'everything', 'stdio')")
            .execute(&pool)
            .await
            .expect("insert server");
        let snapshot = CapabilitySnapshot {
            upstream_name: Some("io.modelcontextprotocol/everything".to_string()),
            upstream_title: Some("Everything Reference Server".to_string()),
            server_version: Some("1.2.3".to_string()),
            protocol_version: Some("2025-11-25".to_string()),
            ..Default::default()
        };

        persist_snapshot_server_info(&pool, "server-a", &snapshot)
            .await
            .expect("persist snapshot identity");

        let namespace: String = sqlx::query_scalar("SELECT name FROM server_config WHERE id = 'server-a'")
            .fetch_one(&pool)
            .await
            .expect("load namespace");
        let upstream_name: String =
            sqlx::query_scalar("SELECT upstream_name FROM server_meta WHERE server_id = 'server-a'")
                .fetch_one(&pool)
                .await
                .expect("load upstream name");
        assert_eq!(namespace, "everything");
        assert_eq!(upstream_name, "io.modelcontextprotocol/everything");
    }

    #[tokio::test]
    async fn empty_authoritative_snapshot_clears_capability_flags() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("connect in-memory database");
        crate::config::server::init::initialize_server_tables(&pool)
            .await
            .expect("initialize server tables");
        sqlx::query(
            "INSERT INTO server_config (id, name, server_type, capabilities) VALUES ('server-a', 'docs', 'stdio', 'tools,prompts,resources')",
        )
        .execute(&pool)
        .await
        .expect("insert server");

        overwrite_capabilities(&pool, "server-a", false, false, false)
            .await
            .expect("persist empty authoritative support flags");

        let capabilities: Option<String> =
            sqlx::query_scalar("SELECT capabilities FROM server_config WHERE id = 'server-a'")
                .fetch_one(&pool)
                .await
                .expect("load capability flags");
        assert_eq!(capabilities, None);
    }

    #[tokio::test]
    async fn raw_snapshot_preserves_exact_values_for_every_capability_kind() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("connect in-memory database");
        crate::config::server::init::initialize_server_tables(&pool)
            .await
            .expect("initialize server tables");
        crate::config::profile::init::initialize_profile_tables(&pool)
            .await
            .expect("initialize profile tables");
        crate::config::client::init::initialize_client_table(&pool)
            .await
            .expect("initialize client table");
        sqlx::query("INSERT INTO server_config (id, name, server_type) VALUES ('server-a', 'searxng', 'stdio')")
            .execute(&pool)
            .await
            .expect("insert server");

        let cache_dir = TempDir::new().expect("create cache directory");
        let redb = RedbCacheManager::new(
            cache_dir.path().join("capability.redb"),
            crate::core::cache::manager::CacheConfig::default(),
        )
        .expect("create cache manager");
        let now = Utc::now();
        store_dual_write(
            &pool,
            &redb,
            "server-a",
            "searxng",
            vec![CachedToolInfo {
                name: "get_searxng_status".to_string(),
                description: None,
                input_schema_json: r#"{"type":"object"}"#.to_string(),
                output_schema_json: None,
                unique_name: None,
                icons: None,
                enabled: true,
                cached_at: now,
            }],
            vec![CachedResourceInfo {
                uri: "file:///searxng/status".to_string(),
                name: Some("Status".to_string()),
                description: None,
                mime_type: None,
                icons: None,
                enabled: true,
                cached_at: now,
            }],
            vec![CachedPromptInfo {
                name: "get_searxng_help".to_string(),
                description: None,
                arguments: Vec::new(),
                icons: None,
                enabled: true,
                cached_at: now,
            }],
            vec![CachedResourceTemplateInfo {
                uri_template: "searxng://status/{id}".to_string(),
                name: Some("Status template".to_string()),
                description: None,
                mime_type: None,
                enabled: true,
                cached_at: now,
            }],
            Some("2025-11-25".to_string()),
        )
        .await
        .expect("store raw snapshot");

        let cached = redb
            .get_server_data(&CacheQuery {
                server_id: "server-a".to_string(),
                freshness_level: FreshnessLevel::Cached,
                include_disabled: true,
                scope: CacheScope::shared_raw(),
            })
            .await
            .expect("read raw snapshot")
            .data
            .expect("raw snapshot exists");
        assert_eq!(cached.tools[0].name, "get_searxng_status");
        assert_eq!(cached.tools[0].unique_name.as_deref(), Some("searxng_get_status"));
        assert_eq!(cached.prompts[0].name, "get_searxng_help");
        assert_eq!(cached.resources[0].uri, "file:///searxng/status");
        assert_eq!(cached.resource_templates[0].uri_template, "searxng://status/{id}");
        let (unique_name, route_uri): (String, String) =
            sqlx::query_as("SELECT unique_name, route_uri FROM server_resource_templates WHERE server_id = 'server-a'")
                .fetch_one(&pool)
                .await
                .expect("load persisted template route");
        assert_eq!(unique_name, "mcpmate://resources/template/searxng/searxng/status/{id}");
        assert_eq!(route_uri, "mcpmate://resources/template/searxng/searxng/status/{}");
    }

    #[tokio::test]
    async fn empty_snapshot_removes_stale_rows_from_every_capability_catalog() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("connect in-memory database");
        crate::config::server::init::initialize_server_tables(&pool)
            .await
            .expect("initialize server tables");
        crate::config::profile::init::initialize_profile_tables(&pool)
            .await
            .expect("initialize profile tables");
        crate::config::client::init::initialize_client_table(&pool)
            .await
            .expect("initialize client table");
        sqlx::query("INSERT INTO server_config (id, name, server_type) VALUES ('server-a', 'docs', 'stdio')")
            .execute(&pool)
            .await
            .expect("insert server");
        crate::config::server::tools::upsert_server_tool(&pool, "server-a", "docs", "read", None)
            .await
            .expect("insert tool");
        upsert_shadow_prompt(&pool, "server-a", "docs", "help", None)
            .await
            .expect("insert prompt");
        upsert_shadow_resource(&pool, "server-a", "docs", "file:///guide", None, None, None)
            .await
            .expect("insert resource");
        upsert_shadow_resource_template(&pool, "server-a", "docs", "file:///{path}", Some("Files"), None)
            .await
            .expect("insert template");
        sqlx::query(
            "INSERT INTO server_issued_resources (id, server_id, server_name, resource_uri, unique_uri) VALUES ('issued-1', 'server-a', 'docs', 'file:///generated', 'mcpmate://resources/docs/file/generated')",
        )
        .execute(&pool)
        .await
        .expect("insert issued resource route");
        let cache_dir = TempDir::new().expect("create cache directory");
        let redb = RedbCacheManager::new(
            cache_dir.path().join("empty-capability.redb"),
            crate::core::cache::manager::CacheConfig::default(),
        )
        .expect("create cache manager");

        store_dual_write(
            &pool,
            &redb,
            "server-a",
            "docs",
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Some("2025-11-25".to_string()),
        )
        .await
        .expect("store empty authoritative snapshot");

        for table in [
            "server_tools",
            "server_prompts",
            "server_resources",
            "server_resource_templates",
        ] {
            let query = format!("SELECT COUNT(*) FROM {table} WHERE server_id = 'server-a'");
            let count: i64 = sqlx::query_scalar(&query)
                .fetch_one(&pool)
                .await
                .expect("count catalog rows");
            assert_eq!(count, 0, "{table} retained stale capability rows");
        }
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM server_issued_resources WHERE server_id = 'server-a'",)
                .fetch_one(&pool)
                .await
                .expect("count issued resource routes"),
            1,
            "authoritative resources/list reconciliation must not delete issued routes"
        );
    }

    #[tokio::test]
    async fn issued_resource_routes_survive_inventory_churn_without_route_churn() {
        let pool = capability_store_pool().await;
        let issued_upstream = "demo://resource/generated";
        let issued_uri = crate::core::capability::resource_registry::issue_resource_route(
            &pool,
            "server-a",
            "docs",
            issued_upstream,
        )
        .await
        .expect("issue dynamic resource route");
        assert_eq!(issued_uri, "mcpmate://resources/docs/demo/generated");

        let listed_upstream = "demo://resources/generated";
        upsert_shadow_resources_batch(&pool, "server-a", "docs", &[cached_resource(listed_upstream)])
            .await
            .expect("persist colliding listed resource");

        let listed_uri: String = sqlx::query_scalar("SELECT unique_uri FROM server_resources WHERE resource_uri = ?")
            .bind(listed_upstream)
            .fetch_one(&pool)
            .await
            .expect("load listed resource URI");
        assert_ne!(listed_uri, issued_uri);
        assert_eq!(
            crate::core::capability::resource_registry::resolve_resource_route(&pool, &issued_uri)
                .await
                .expect("resolve preserved issued route")
                .upstream_uri,
            issued_upstream
        );

        upsert_shadow_resources_batch(
            &pool,
            "server-a",
            "docs",
            &[cached_resource(issued_upstream), cached_resource(listed_upstream)],
        )
        .await
        .expect("promote issued resource into listed inventory");

        assert_eq!(
            sqlx::query_scalar::<_, String>("SELECT unique_uri FROM server_resources WHERE resource_uri = ?")
                .bind(issued_upstream)
                .fetch_one(&pool)
                .await
                .expect("load promoted listed resource"),
            issued_uri
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM server_issued_resources WHERE server_id = 'server-a' AND resource_uri = ?",
            )
            .bind(issued_upstream)
            .fetch_one(&pool)
            .await
            .expect("count retained issued routes"),
            1,
            "listing an issued upstream URI must retain its issued route"
        );
        assert_eq!(
            crate::core::capability::resource_registry::resolve_resource_route(&pool, &issued_uri)
                .await
                .expect("resolve listed route before inventory removal")
                .source,
            crate::core::capability::resource_registry::ResourceRouteSource::Listed
        );

        upsert_shadow_resources_batch(&pool, "server-a", "docs", &[cached_resource(listed_upstream)])
            .await
            .expect("remove issued resource from authoritative inventory");
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM server_resources WHERE resource_uri = ?")
                .bind(issued_upstream)
                .fetch_one(&pool)
                .await
                .expect("count removed listed resource"),
            0
        );
        assert_eq!(
            crate::core::capability::resource_registry::resolve_resource_route(&pool, &issued_uri)
                .await
                .expect("resolve preserved issued route after inventory removal")
                .source,
            crate::core::capability::resource_registry::ResourceRouteSource::Issued
        );

        upsert_shadow_resources_batch(
            &pool,
            "server-a",
            "docs",
            &[cached_resource(issued_upstream), cached_resource(listed_upstream)],
        )
        .await
        .expect("relist issued resource");
        assert_eq!(
            sqlx::query_scalar::<_, String>("SELECT unique_uri FROM server_resources WHERE resource_uri = ?")
                .bind(issued_upstream)
                .fetch_one(&pool)
                .await
                .expect("load relisted resource URI"),
            issued_uri,
            "relisting must reuse the original canonical URI"
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM server_issued_resources WHERE server_id = 'server-a' AND resource_uri = ?",
            )
            .bind(issued_upstream)
            .fetch_one(&pool)
            .await
            .expect("count issued routes after relisting"),
            1
        );
    }

    #[tokio::test]
    async fn invalid_full_snapshot_does_not_commit_an_earlier_capability_kind() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("connect in-memory database");
        crate::config::server::init::initialize_server_tables(&pool)
            .await
            .expect("initialize server tables");
        crate::config::profile::init::initialize_profile_tables(&pool)
            .await
            .expect("initialize profile tables");
        crate::config::client::init::initialize_client_table(&pool)
            .await
            .expect("initialize client table");
        sqlx::query("INSERT INTO server_config (id, name, server_type) VALUES ('server-a', 'docs', 'stdio')")
            .execute(&pool)
            .await
            .expect("insert server");
        let cache_dir = TempDir::new().expect("create cache directory");
        let redb = RedbCacheManager::new(
            cache_dir.path().join("atomic-capability.redb"),
            crate::core::cache::manager::CacheConfig::default(),
        )
        .expect("create cache manager");
        let now = Utc::now();

        store_dual_write(
            &pool,
            &redb,
            "server-a",
            "docs",
            vec![CachedToolInfo {
                name: "old_tool".to_string(),
                description: None,
                input_schema_json: r#"{"type":"object"}"#.to_string(),
                output_schema_json: None,
                unique_name: None,
                icons: None,
                enabled: true,
                cached_at: now,
            }],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Some("2025-11-25".to_string()),
        )
        .await
        .expect("store initial snapshot");

        let duplicate_prompt = CachedPromptInfo {
            name: "duplicate".to_string(),
            description: None,
            arguments: Vec::new(),
            icons: None,
            enabled: true,
            cached_at: now,
        };
        let error = store_dual_write(
            &pool,
            &redb,
            "server-a",
            "docs",
            vec![CachedToolInfo {
                name: "new_tool".to_string(),
                description: None,
                input_schema_json: r#"{"type":"object"}"#.to_string(),
                output_schema_json: None,
                unique_name: None,
                icons: None,
                enabled: true,
                cached_at: now,
            }],
            Vec::new(),
            vec![duplicate_prompt.clone(), duplicate_prompt],
            Vec::new(),
            Some("2025-11-25".to_string()),
        )
        .await
        .expect_err("invalid later inventory must roll back the full catalog update");

        assert!(error.to_string().contains("duplicate upstream"));
        let tools = sqlx::query_scalar::<_, String>(
            "SELECT tool_name FROM server_tools WHERE server_id = 'server-a' ORDER BY tool_name",
        )
        .fetch_all(&pool)
        .await
        .expect("load committed tools");
        assert_eq!(tools, vec!["old_tool"]);
    }

    #[tokio::test]
    async fn scoped_snapshot_replaces_only_the_requested_capability_kind() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("connect in-memory database");
        crate::config::server::init::initialize_server_tables(&pool)
            .await
            .expect("initialize server tables");
        crate::config::profile::init::initialize_profile_tables(&pool)
            .await
            .expect("initialize profile tables");
        crate::config::client::init::initialize_client_table(&pool)
            .await
            .expect("initialize client table");
        sqlx::query("INSERT INTO server_config (id, name, server_type) VALUES ('server-a', 'docs', 'stdio')")
            .execute(&pool)
            .await
            .expect("insert server");
        let cache_dir = TempDir::new().expect("create cache directory");
        let redb = RedbCacheManager::new(
            cache_dir.path().join("scoped-capability.redb"),
            crate::core::cache::manager::CacheConfig::default(),
        )
        .expect("create cache manager");
        let now = Utc::now();
        store_dual_write(
            &pool,
            &redb,
            "server-a",
            "docs",
            vec![CachedToolInfo {
                name: "read".to_string(),
                description: None,
                input_schema_json: r#"{"type":"object"}"#.to_string(),
                output_schema_json: None,
                unique_name: None,
                icons: None,
                enabled: true,
                cached_at: now,
            }],
            vec![CachedResourceInfo {
                uri: "file:///guide".to_string(),
                name: Some("Guide".to_string()),
                description: None,
                mime_type: None,
                icons: None,
                enabled: true,
                cached_at: now,
            }],
            vec![CachedPromptInfo {
                name: "help".to_string(),
                description: None,
                arguments: Vec::new(),
                icons: None,
                enabled: true,
                cached_at: now,
            }],
            vec![CachedResourceTemplateInfo {
                uri_template: "file:///{path}".to_string(),
                name: Some("Files".to_string()),
                description: None,
                mime_type: None,
                enabled: true,
                cached_at: now,
            }],
            Some("2025-11-25".to_string()),
        )
        .await
        .expect("store initial full snapshot");

        store_dual_write_for_kinds(
            &pool,
            &redb,
            "server-a",
            "docs",
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Some("2025-11-25".to_string()),
            crate::core::pool::CapSyncFlags::TOOLS,
        )
        .await
        .expect("replace only the tool inventory");

        let cached = redb
            .get_server_data(&CacheQuery {
                server_id: "server-a".to_string(),
                freshness_level: FreshnessLevel::Cached,
                include_disabled: true,
                scope: CacheScope::shared_raw(),
            })
            .await
            .expect("read scoped snapshot")
            .data
            .expect("scoped snapshot exists");
        assert!(cached.tools.is_empty());
        assert_eq!(cached.prompts.len(), 1);
        assert_eq!(cached.resources.len(), 1);
        assert_eq!(cached.resource_templates.len(), 1);
    }
}
