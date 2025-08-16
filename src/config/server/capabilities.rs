// Server capabilities persistence helpers (shadow tables + REDB dual-write)
// Centralizes insert/update logic so API handlers and migration can reuse.

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};

use crate::common::capability::CapabilityToken;
use crate::core::cache::{
    CachedPromptInfo, CachedResourceInfo, CachedResourceTemplateInfo, CachedServerData, CachedToolInfo,
    RedbCacheManager,
};
use crate::core::pool::UpstreamConnectionPool;
use tokio::time::{Duration, timeout};

/// Unified capability snapshot container
#[derive(Debug, Clone, Default)]
pub struct CapabilitySnapshot {
    pub tools: Vec<CachedToolInfo>,
    pub resources: Vec<CachedResourceInfo>,
    pub prompts: Vec<CachedPromptInfo>,
    pub resource_templates: Vec<CachedResourceTemplateInfo>,
}

/// Discover capabilities from an existing upstream connection (API temporary instance)
pub async fn discover_from_connection(
    conn: &crate::core::connection::UpstreamConnection
) -> Result<CapabilitySnapshot> {
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

    // Prompts
    if conn.supports_prompts() {
        if let Some(service) = &conn.service {
            if let Ok(list_result) = service.list_prompts(None).await {
                for p in list_result.prompts {
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
                    snap.prompts.push(CachedPromptInfo {
                        name: p.name,
                        description: p.description,
                        arguments: converted_args,
                        enabled: true,
                        cached_at: chrono::Utc::now(),
                    });
                }
            }
        }
    }

    // Resources and templates
    if conn.supports_resources() {
        if let Some(service) = &conn.service {
            if let Ok(list_result) = service.list_resources(None).await {
                for r in list_result.resources {
                    snap.resources.push(CachedResourceInfo {
                        uri: r.uri.clone(),
                        name: Some(r.name.clone()),
                        description: r.description.clone(),
                        mime_type: r.mime_type.clone(),
                        enabled: true,
                        cached_at: chrono::Utc::now(),
                    });
                }
            }

            let mut cursor = None;
            while let Ok(result) = service
                .list_resource_templates(Some(rmcp::model::PaginatedRequestParam { cursor }))
                .await
            {
                for t in result.resource_templates {
                    snap.resource_templates.push(CachedResourceTemplateInfo {
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

    // Prompts
    if capabilities.as_ref().and_then(|c| c.prompts.as_ref()).is_some() {
        if let Ok(list_result) = service.list_prompts(None).await {
            for p in list_result.prompts {
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
                snap.prompts.push(CachedPromptInfo {
                    name: p.name,
                    description: p.description,
                    arguments: converted_args,
                    enabled: true,
                    cached_at: chrono::Utc::now(),
                });
            }
        }
    }

    // Resources & templates
    if capabilities.as_ref().and_then(|c| c.resources.as_ref()).is_some() {
        if let Ok(list_result) = service.list_resources(None).await {
            for r in list_result.resources {
                snap.resources.push(CachedResourceInfo {
                    uri: r.uri.clone(),
                    name: Some(r.name.clone()),
                    description: r.description.clone(),
                    mime_type: r.mime_type.clone(),
                    enabled: true,
                    cached_at: chrono::Utc::now(),
                });
            }
        }

        let mut cursor = None;
        while let Ok(result) = service
            .list_resource_templates(Some(rmcp::model::PaginatedRequestParam { cursor }))
            .await
        {
            for t in result.resource_templates {
                snap.resource_templates.push(CachedResourceTemplateInfo {
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
    let unique_name = prompt_name.to_string();
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
    let unique_name = uri.to_string();
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
    let unique_name = uri_template.to_string();
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
    tools: Vec<CachedToolInfo>,
    resources: Vec<CachedResourceInfo>,
    prompts: Vec<CachedPromptInfo>,
    templates: Vec<CachedResourceTemplateInfo>,
) -> Result<()> {
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

    // SQLite: tools via existing helper
    if !tools.is_empty() {
        let items: Vec<(String, Option<String>)> =
            tools.iter().map(|t| (t.name.clone(), t.description.clone())).collect();
        let server_name_owned = server_name.to_string();
        let _ =
            crate::config::server::tools::batch_upsert_server_tools(pool, server_id, &server_name_owned, &items).await;
    }

    // SQLite: prompts/resources/templates
    for p in &prompts {
        let _ = upsert_shadow_prompt(pool, server_id, server_name, &p.name, p.description.as_deref()).await;
    }
    for r in &resources {
        let _ = upsert_shadow_resource(
            pool,
            server_id,
            server_name,
            &r.uri,
            r.name.as_deref(),
            r.description.as_deref(),
            r.mime_type.as_deref(),
        )
        .await;
    }
    for t in &templates {
        let _ = upsert_shadow_resource_template(
            pool,
            server_id,
            server_name,
            &t.uri_template,
            t.name.as_deref(),
            t.description.as_deref(),
        )
        .await;
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
    supports_resource_templates: bool,
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
    if supports_resource_templates {
        caps.push(CapabilityToken::ResourceTemplates.as_str());
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
        _ => return Ok(()),
    };

    // Discover and store
    let snap = discover_from_connection(conn).await?;
    // Clone for store and keep original for capability flags
    let tools_clone = snap.tools.clone();
    let resources_clone = snap.resources.clone();
    let prompts_clone = snap.prompts.clone();
    let templates_clone = snap.resource_templates.clone();
    store_dual_write(
        db_pool,
        redb,
        server_id,
        server_name,
        tools_clone,
        resources_clone,
        prompts_clone,
        templates_clone,
    )
    .await?;

    // Full overwrite of capabilities using protocol support flags from this connection
    let supports_tools = !snap.tools.is_empty();
    let supports_prompts = !snap.prompts.is_empty();
    let supports_resources = !snap.resources.is_empty();
    let supports_resource_templates = !snap.resource_templates.is_empty();
    overwrite_capabilities(
        db_pool,
        server_id,
        supports_tools,
        supports_prompts,
        supports_resources,
        supports_resource_templates,
    )
    .await?;

    // Cleanup
    let _ = pool.destroy_validation_instance(server_name, "api").await;
    Ok(())
}
