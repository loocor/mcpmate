use super::*;
use crate::core::capability::naming::{NamingKind, resolve_unique_name};
use futures::StreamExt;
use rmcp::ErrorData as McpError;
use rmcp::model::{CallToolRequestParam, CallToolResult, PaginatedRequestParam};
use rmcp::service::RequestContext;

pub(super) async fn list_tools(
    server: &ProxyServer,
    _request: Option<PaginatedRequestParam>,
    _context: RequestContext<rmcp::RoleServer>,
) -> Result<rmcp::model::ListToolsResult, McpError> {
    let mut tools: Vec<rmcp::model::Tool> = Vec::new();

    if let Some(db) = &server.database {
        let enabled_servers: Vec<(String, String, Option<String>)> = sqlx::query_as(
            r#"
            SELECT sc.id, sc.name, sc.capabilities
            FROM server_config sc
            JOIN profile_server ps ON ps.server_id = sc.id AND ps.enabled = 1
            JOIN profile p ON p.id = ps.profile_id AND p.is_active = 1
            WHERE sc.enabled = 1
            GROUP BY sc.id, sc.name, sc.capabilities
            "#,
        )
        .fetch_all(&db.pool)
        .await
        .unwrap_or_default();

        let redb = &server.redb_cache;
        let pool = &server.connection_pool;

        let mut tasks = Vec::new();
        for (server_id, _server_name, capabilities) in enabled_servers {
            if !super::supports_capability(capabilities.as_deref(), crate::core::capability::CapabilityType::Tools) {
                continue;
            }
            let ctx = crate::core::capability::runtime::ListCtx {
                capability: crate::core::capability::CapabilityType::Tools,
                server_id: server_id.clone(),
                refresh: Some(crate::core::capability::runtime::RefreshStrategy::CacheFirst),
                timeout: Some(std::time::Duration::from_secs(10)),
                validation_session: None,
            };
            let redb = redb.clone();
            let pool = pool.clone();
            let db = db.clone();
            tasks.push(async move {
                match crate::core::capability::runtime::list(&ctx, &redb, &pool, &db).await {
                    Ok(result) => result.items.into_tools().unwrap_or_default(),
                    Err(_) => Vec::new(),
                }
            });
        }

        for mut v in futures::stream::iter(tasks)
            .buffer_unordered(crate::core::capability::facade::concurrency_limit())
            .collect::<Vec<_>>()
            .await
        {
            tools.append(&mut v);
        }
    }

    let builtin_tools = server.builtin_services.tools();
    tracing::debug!("Including {} builtin service tools", builtin_tools.len());
    tools.extend(builtin_tools);

    tracing::info!("Proxy listed {} total tools (including builtin services)", tools.len());

    Ok(rmcp::model::ListToolsResult {
        tools,
        next_cursor: None,
    })
}

pub(super) async fn call_tool(
    server: &ProxyServer,
    request: CallToolRequestParam,
    _context: RequestContext<rmcp::RoleServer>,
) -> Result<CallToolResult, McpError> {
    let call_id = crate::generate_id!("tcall");
    let started_at = std::time::Instant::now();

    tracing::debug!(
        call_id = %call_id,
        tool = %request.name,
        "ProxyServer::call_tool received request"
    );

    if let Some(result) = server.builtin_services.call_tool(&request).await {
        tracing::debug!(
            call_id = %call_id,
            tool = %request.name,
            "ProxyServer::call_tool handled by builtin service"
        );
        return match result {
            Ok(call_result) => Ok(call_result),
            Err(e) => {
                tracing::error!(
                    call_id = %call_id,
                    tool = %request.name,
                    error = %e,
                    "Builtin service tool failed"
                );
                Err(McpError::internal_error(e.to_string(), None))
            }
        };
    }

    if server.database.is_none() {
        tracing::error!("Database not available for tool calling");
        return Err(McpError::internal_error(
            "Database not available for tool calling".to_string(),
            None,
        ));
    }

    let (server_name, original_tool_name) =
        resolve_unique_name(NamingKind::Tool, &request.name)
            .await
            .map_err(|e| {
                tracing::error!(
                    call_id = %call_id,
                    tool = %request.name,
                    error = %e,
                    "ProxyServer::call_tool failed to resolve unique name"
                );
                McpError::internal_error(format!("Failed to resolve unique tool name: {}", e), None)
            })?;
    let server_id = crate::core::capability::resolver::to_id(&server_name)
        .await
        .ok()
        .flatten()
        .ok_or_else(|| {
            tracing::error!(
                call_id = %call_id,
                tool = %request.name,
                server_name = %server_name,
                "ProxyServer::call_tool missing server id for mapping"
            );
            McpError::internal_error("Server not found for tool mapping".to_string(), None)
        })?;

    tracing::debug!(
        call_id = %call_id,
        tool = %request.name,
        server_name = %server_name,
        server_id = %server_id,
        upstream_tool = %original_tool_name,
        "ProxyServer::call_tool resolved mapping"
    );

    // Resolve tool call timeout from env (fallback 30s)
    let call_timeout_secs: u64 = std::env::var("MCPMATE_TOOL_CALL_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(60);

    let ctx = crate::core::capability::runtime::CallCtx {
        call_id: call_id.clone(),
        server_id,
        tool_name: original_tool_name,
        timeout: Some(std::time::Duration::from_secs(call_timeout_secs)),
        arguments: request.arguments.clone(),
    };

    match crate::core::capability::runtime::call_tool(&ctx, &server.connection_pool).await {
        Ok(result) => {
            tracing::info!(
                call_id = %call_id,
                tool = %request.name,
                elapsed_ms = started_at.elapsed().as_millis() as u64,
                "ProxyServer::call_tool succeeded"
            );
            Ok(result)
        }
        Err(e) => {
            tracing::error!(
                call_id = %call_id,
                tool = %request.name,
                elapsed_ms = started_at.elapsed().as_millis() as u64,
                error = %e,
                "ProxyServer::call_tool upstream error"
            );
            Err(McpError::internal_error(e.to_string(), None))
        }
    }
}
