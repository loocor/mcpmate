use std::collections::HashMap;
use std::sync::Arc;

use mcpmate_secrets::SecretResolver;

use super::shared::*;
use crate::api::models::server::{
    ServerCapabilityMeta, ServerPreviewData, ServerPreviewItemData, ServerPreviewItemReq, ServerPreviewReq,
    ServerPreviewResp, ServerPromptsData, ServerResourceTemplatesData, ServerResourcesData, ServerToolsData,
};
use crate::core::capability::naming::{NamingKind, plan_external_identifiers};
use crate::core::models::MCPServerConfig;
use crate::core::secrets::resolve_runtime_server_config_with_optional_resolver;
use crate::core::secrets::store::LocalSecretStore;

/// Preview capabilities for arbitrary server configs.
///
/// Saved-server previews may refresh stored OAuth tokens while resolving effective headers.
pub async fn preview_servers(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ServerPreviewReq>,
) -> Result<Json<ServerPreviewResp>, ApiError> {
    let timeout = req.timeout_ms.map(std::time::Duration::from_millis);
    let include_details = req.include_details.unwrap_or(true);
    let db_pool = state.database.as_ref().map(|db| db.pool.clone());
    let secret_store = state.secret_store.read().await.clone();

    // Process sequentially to avoid uncontrolled concurrency; can add a small semaphore later
    let mut items_out: Vec<ServerPreviewItemData> = Vec::with_capacity(req.servers.len());
    for item in req.servers {
        items_out.push(preview_one(item, timeout, include_details, db_pool.as_ref(), secret_store.clone()).await);
    }

    Ok(Json(ServerPreviewResp::success(ServerPreviewData { items: items_out })))
}

async fn preview_one(
    item: ServerPreviewItemReq,
    timeout: Option<std::time::Duration>,
    include_details: bool,
    db_pool: Option<&sqlx::SqlitePool>,
    secret_store: Option<Arc<LocalSecretStore>>,
) -> ServerPreviewItemData {
    // Map kind -> ServerType
    let kind = match crate::common::server::ServerType::from_client_format(item.kind.as_str()) {
        Ok(k) => k,
        Err(_) => {
            return empty_with_error(item.name, format!("Invalid server kind: {}", item.kind));
        }
    };

    let effective_headers = match resolve_preview_headers(
        item.headers.clone(),
        item.server_id.as_deref(),
        db_pool,
        secret_store.clone(),
    )
    .await
    {
        Ok(headers) => headers,
        Err(e) => return empty_with_error(item.name, e.to_string()),
    };

    let raw_cfg = MCPServerConfig {
        kind,
        command: item.command.clone(),
        url: item.url.clone(),
        args: item.args.clone(),
        env: item.env.clone(),
        headers: effective_headers,
    };

    let secret_resolver = secret_store.as_deref().map(|store| store as &dyn SecretResolver);
    let cfg = match resolve_runtime_server_config_with_optional_resolver(&raw_cfg, secret_resolver) {
        Ok(resolved) => resolved,
        Err(err) => return empty_with_error(item.name, err.to_string()),
    };

    let mut client: Option<reqwest::Client> = None;
    if kind.is_http_transport() {
        if let Some(headers) = cfg.headers.as_ref() {
            let mut header_map = reqwest::header::HeaderMap::new();
            for (k, v) in headers.iter() {
                if let Ok(name) = reqwest::header::HeaderName::from_bytes(k.as_bytes()) {
                    if let Ok(value) = reqwest::header::HeaderValue::from_str(v) {
                        header_map.insert(name, value);
                    }
                }
            }
            let builder = reqwest::Client::builder().default_headers(header_map);
            if let Ok(built) = builder.build() {
                client = Some(built);
            }
        }
    }

    // The shared Inspector timeout is a fresh deadline for each MCP operation.
    let snap =
        crate::config::server::capabilities::discover_from_config_preview(&item.name, &cfg, kind, client, timeout)
            .await;

    match snap {
        Ok(s) => {
            if let (Some(pool), Some(server_id)) = (db_pool, item.server_id.as_deref()) {
                if let Err(error) =
                    crate::config::server::capabilities::persist_snapshot_server_info(pool, server_id, &s).await
                {
                    return empty_with_error(item.name, error.to_string());
                }
            }
            build_item(item.name.clone(), s, include_details)
                .unwrap_or_else(|error| empty_with_error(item.name, error.to_string()))
        }
        Err(e) => empty_with_error(item.name, e.to_string()),
    }
}

async fn resolve_preview_headers(
    item_headers: Option<HashMap<String, String>>,
    server_id: Option<&str>,
    db_pool: Option<&sqlx::SqlitePool>,
    secret_store: Option<Arc<LocalSecretStore>>,
) -> anyhow::Result<Option<HashMap<String, String>>> {
    if let (Some(pool), Some(server_id)) = (db_pool, server_id) {
        let manager = crate::core::oauth::OAuthManager::new_optional_store(pool.clone(), secret_store);
        return manager.get_effective_server_headers(server_id, item_headers).await;
    }

    Ok(item_headers)
}

fn build_item(
    name: String,
    mut snap: crate::config::server::capabilities::CapabilitySnapshot,
    include_details: bool,
) -> anyhow::Result<ServerPreviewItemData> {
    let tool_names = snap.tools.iter().map(|tool| tool.name.clone()).collect::<Vec<_>>();
    let tool_plan = plan_external_identifiers(NamingKind::Tool, &name, &tool_names)?;
    for tool in &mut snap.tools {
        tool.unique_name = tool_plan.get(&tool.name).cloned();
    }
    let prompt_plan = plan_external_identifiers(
        NamingKind::Prompt,
        &name,
        &snap
            .prompts
            .iter()
            .map(|prompt| prompt.name.clone())
            .collect::<Vec<_>>(),
    )?;
    let resource_plan = plan_external_identifiers(
        NamingKind::Resource,
        &name,
        &snap
            .resources
            .iter()
            .map(|resource| resource.uri.clone())
            .collect::<Vec<_>>(),
    )?;
    let template_plan = plan_external_identifiers(
        NamingKind::ResourceTemplate,
        &name,
        &snap
            .resource_templates
            .iter()
            .map(|template| template.uri_template.clone())
            .collect::<Vec<_>>(),
    )?;

    // tools
    let tool_items: Vec<serde_json::Value> = if include_details {
        snap.tools
            .iter()
            .map(super::capability::tool_json_from_cached)
            .collect()
    } else {
        Vec::new()
    };

    // resources
    let resource_items: Vec<serde_json::Value> = if include_details {
        snap.resources
            .iter()
            .map(|r| {
                serde_json::json!({
                    "uri": r.uri,
                    "resource_uri": r.uri,
                    "unique_uri": resource_plan.get(&r.uri),
                    "name": r.name,
                    "description": r.description,
                    "mime_type": r.mime_type,
                    "enabled": r.enabled,
                    "cached_at": r.cached_at.to_rfc3339(),
                })
            })
            .collect()
    } else {
        Vec::new()
    };

    let template_items: Vec<serde_json::Value> = if include_details {
        snap.resource_templates
            .iter()
            .map(|t| {
                serde_json::json!({
                    "uri_template": t.uri_template,
                    "unique_uri_template": template_plan.get(&t.uri_template),
                    "name": t.name,
                    "description": t.description,
                    "mime_type": t.mime_type,
                    "enabled": t.enabled,
                    "cached_at": t.cached_at.to_rfc3339(),
                })
            })
            .collect()
    } else {
        Vec::new()
    };

    let prompt_items: Vec<serde_json::Value> = if include_details {
        snap.prompts
            .iter()
            .map(|p| {
                serde_json::json!({
                    "name": p.name,
                    "prompt_name": p.name,
                    "unique_name": prompt_plan.get(&p.name),
                    "description": p.description,
                    "arguments": p.arguments.iter().map(|a| serde_json::json!({
                        "name": a.name,
                        "description": a.description,
                        "required": a.required,
                    })).collect::<Vec<_>>()
                })
            })
            .collect()
    } else {
        Vec::new()
    };

    let meta = ServerCapabilityMeta {
        cache_hit: false,
        strategy: "preview".to_string(),
        source: "live".to_string(),
    };

    Ok(ServerPreviewItemData {
        name,
        ok: true,
        error: None,
        tools: ServerToolsData {
            items: tool_items,
            state: "ok".to_string(),
            meta: meta.clone(),
        },
        resources: ServerResourcesData {
            items: resource_items,
            state: "ok".to_string(),
            meta: meta.clone(),
        },
        resource_templates: ServerResourceTemplatesData {
            items: template_items,
            state: "ok".to_string(),
            meta: meta.clone(),
        },
        prompts: ServerPromptsData {
            items: prompt_items,
            state: "ok".to_string(),
            meta,
        },
    })
}

fn empty_with_error(
    name: String,
    err: String,
) -> ServerPreviewItemData {
    let meta = ServerCapabilityMeta {
        cache_hit: false,
        strategy: "preview".to_string(),
        source: "none".to_string(),
    };
    ServerPreviewItemData {
        name,
        ok: false,
        error: Some(err),
        tools: ServerToolsData {
            items: Vec::new(),
            state: "error".to_string(),
            meta: meta.clone(),
        },
        resources: ServerResourcesData {
            items: Vec::new(),
            state: "error".to_string(),
            meta: meta.clone(),
        },
        resource_templates: ServerResourceTemplatesData {
            items: Vec::new(),
            state: "error".to_string(),
            meta: meta.clone(),
        },
        prompts: ServerPromptsData {
            items: Vec::new(),
            state: "error".to_string(),
            meta,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        models::{ServerOAuthConfig, ServerOAuthToken},
        server::{init::initialize_server_tables, upsert_server_oauth_config, upsert_server_oauth_token},
    };
    use crate::core::cache::{CachedPromptInfo, CachedResourceInfo, CachedResourceTemplateInfo, CachedToolInfo};
    use crate::core::secrets::store::{SecretCreateInput, SecretKindInput};
    use crate::test_helpers::oauth_secret_origin;
    use chrono::{Duration, Utc};
    use tempfile::TempDir;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{body_string_contains, method, path},
    };

    async fn setup_pool() -> sqlx::SqlitePool {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("connect in-memory sqlite");
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .expect("enable foreign keys");
        initialize_server_tables(&pool).await.expect("init server tables");
        pool
    }

    #[test]
    fn preview_projects_all_capability_kinds_through_the_shared_naming_plan() {
        let now = Utc::now();
        let snapshot = crate::config::server::capabilities::CapabilitySnapshot {
            tools: vec![CachedToolInfo {
                name: "get_searxng_status".to_string(),
                description: None,
                input_schema_json: r#"{"type":"object"}"#.to_string(),
                output_schema_json: None,
                unique_name: None,
                icons: None,
                enabled: true,
                cached_at: now,
            }],
            prompts: vec![CachedPromptInfo {
                name: "searxng_summary".to_string(),
                description: None,
                arguments: Vec::new(),
                icons: None,
                enabled: true,
                cached_at: now,
            }],
            resources: vec![CachedResourceInfo {
                uri: "file:///status".to_string(),
                name: Some("Status".to_string()),
                description: None,
                mime_type: None,
                icons: None,
                enabled: true,
                cached_at: now,
            }],
            resource_templates: vec![CachedResourceTemplateInfo {
                uri_template: "file:///{path}".to_string(),
                name: Some("File".to_string()),
                description: None,
                mime_type: None,
                enabled: true,
                cached_at: now,
            }],
            protocol_version: None,
            ..Default::default()
        };

        let preview = build_item("searxng".to_string(), snapshot, true).expect("build preview");

        assert_eq!(preview.tools.items[0]["name"], "get_searxng_status");
        assert_eq!(preview.tools.items[0]["unique_name"], "searxng_get_status");
        assert_eq!(preview.prompts.items[0]["prompt_name"], "searxng_summary");
        assert_eq!(preview.prompts.items[0]["unique_name"], "searxng_summary");
        assert_eq!(preview.resources.items[0]["resource_uri"], "file:///status");
        assert_eq!(preview.resources.items[0]["unique_uri"], "searxng:file:///status");
        assert_eq!(preview.resource_templates.items[0]["uri_template"], "file:///{path}");
        assert_eq!(
            preview.resource_templates.items[0]["unique_uri_template"],
            "searxng_file:///{path}"
        );
    }

    async fn setup_secret_store(pool: sqlx::SqlitePool) -> (Arc<LocalSecretStore>, TempDir) {
        let temp_dir = TempDir::new().expect("temp dir");
        let store = LocalSecretStore::initialize_with_development_root_key(
            pool,
            temp_dir.path().join("secrets").join("local-root.key"),
        )
        .await
        .expect("initialize secret store");
        (Arc::new(store), temp_dir)
    }

    async fn insert_http_server(
        pool: &sqlx::SqlitePool,
        server_id: &str,
    ) {
        sqlx::query(
            r#"
            INSERT INTO server_config (id, name, server_type, url, enabled)
            VALUES (?, ?, 'streamable_http', 'https://example.com/mcp', 1)
            "#,
        )
        .bind(server_id)
        .bind(format!("server-{server_id}"))
        .execute(pool)
        .await
        .expect("insert http server");
    }

    async fn store_expired_oauth_token(
        pool: &sqlx::SqlitePool,
        secret_store: &LocalSecretStore,
        server_id: &str,
        token_endpoint: String,
    ) {
        upsert_server_oauth_config(
            pool,
            &ServerOAuthConfig {
                id: None,
                server_id: server_id.to_string(),
                authorization_endpoint: "https://issuer.example.com/authorize".to_string(),
                token_endpoint,
                client_id: "client-1".to_string(),
                client_secret: None,
                scopes: Some("read write".to_string()),
                redirect_uri: "http://localhost:5173/oauth/callback".to_string(),
                created_at: None,
                updated_at: None,
            },
        )
        .await
        .expect("save oauth config");
        let access_token = secret_store
            .create_secret(SecretCreateInput {
                alias: format!("oauth/{server_id}/access-token"),
                kind: SecretKindInput::OAuthAccessToken,
                value: "access-old".to_string(),
                label: Some(format!("OAuth access token for server-{server_id}")),
                origin: Some(oauth_secret_origin(
                    server_id,
                    &format!("server-{server_id}"),
                    "access-token",
                )),
            })
            .await
            .expect("store access token")
            .placeholder;
        let refresh_token = secret_store
            .create_secret(SecretCreateInput {
                alias: format!("oauth/{server_id}/refresh-token"),
                kind: SecretKindInput::OAuthRefreshToken,
                value: "refresh-123".to_string(),
                label: Some(format!("OAuth refresh token for server-{server_id}")),
                origin: Some(oauth_secret_origin(
                    server_id,
                    &format!("server-{server_id}"),
                    "refresh-token",
                )),
            })
            .await
            .expect("store refresh token")
            .placeholder;
        upsert_server_oauth_token(
            pool,
            &ServerOAuthToken {
                id: None,
                server_id: server_id.to_string(),
                access_token,
                refresh_token: Some(refresh_token),
                token_type: "bearer".to_string(),
                expires_at: Some((Utc::now() - Duration::minutes(1)).to_rfc3339()),
                scope: Some("read write".to_string()),
                created_at: None,
                updated_at: None,
            },
        )
        .await
        .expect("store expired token");
    }

    #[tokio::test]
    async fn resolve_preview_headers_refreshes_expired_oauth_token() {
        let pool = setup_pool().await;
        let (secret_store, _temp_dir) = setup_secret_store(pool.clone()).await;
        insert_http_server(&pool, "serv_preview_refresh").await;
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/token"))
            .and(body_string_contains("grant_type=refresh_token"))
            .and(body_string_contains("refresh_token=refresh-123"))
            .and(body_string_contains("resource=https%3A%2F%2Fexample.com%2Fmcp"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "access-new",
                "token_type": "bearer",
                "expires_in": 3600
            })))
            .mount(&mock_server)
            .await;

        store_expired_oauth_token(
            &pool,
            secret_store.as_ref(),
            "serv_preview_refresh",
            format!("{}/token", mock_server.uri()),
        )
        .await;

        let headers = resolve_preview_headers(None, Some("serv_preview_refresh"), Some(&pool), Some(secret_store))
            .await
            .expect("resolve headers")
            .expect("headers");

        assert_eq!(
            headers.get("authorization").map(String::as_str),
            Some("Bearer access-new")
        );
    }

    #[test]
    fn resolve_runtime_server_config_replaces_http_url_and_header_placeholders() {
        use mcpmate_secrets::testing::InMemorySecretResolver;

        let resolver = InMemorySecretResolver::from_pairs([
            ("mcp_id", "67db41067bb48c3e0fe32177"),
            ("http_token", "runtime-bearer-token"),
        ]);
        let raw = MCPServerConfig {
            kind: crate::common::server::ServerType::StreamableHttp,
            command: None,
            args: None,
            url: Some("https://mcpstore.co/mcp/[[secret:mcp_id]]".to_string()),
            env: None,
            headers: Some(HashMap::from([(
                "Authorization".to_string(),
                "Bearer [[secret:http_token]]".to_string(),
            )])),
        };

        let resolved =
            crate::core::secrets::resolve_runtime_server_config_with_optional_resolver(&raw, Some(&resolver))
                .expect("resolve preview config");

        assert_eq!(
            resolved.url.as_deref(),
            Some("https://mcpstore.co/mcp/67db41067bb48c3e0fe32177")
        );
        let headers = resolved.headers.expect("resolved headers");
        assert_eq!(
            headers.get("Authorization").map(String::as_str),
            Some("Bearer runtime-bearer-token")
        );
    }

    #[tokio::test]
    async fn preview_reports_oauth_header_resolution_errors() {
        let pool = setup_pool().await;
        let (secret_store, _temp_dir) = setup_secret_store(pool.clone()).await;
        insert_http_server(&pool, "serv_preview_error").await;

        store_expired_oauth_token(
            &pool,
            secret_store.as_ref(),
            "serv_preview_error",
            "http://not-loopback.example.com/token".to_string(),
        )
        .await;

        let item = ServerPreviewItemReq {
            name: "Preview Error".to_string(),
            server_id: Some("serv_preview_error".to_string()),
            kind: "streamable_http".to_string(),
            command: None,
            url: Some("https://example.com/mcp".to_string()),
            args: None,
            env: None,
            headers: None,
        };

        let preview = preview_one(
            item,
            Some(std::time::Duration::from_millis(100)),
            false,
            Some(&pool),
            Some(secret_store),
        )
        .await;

        assert!(!preview.ok);
        assert!(
            preview
                .error
                .as_deref()
                .is_some_and(|error| error.contains("OAuth token endpoint must use HTTPS"))
        );
    }
}
