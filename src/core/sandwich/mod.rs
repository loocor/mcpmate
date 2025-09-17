//! Sandwich - a minimal, unified capability pipeline (REDB-first → runtime → async sync)
//!
//! Goals:
//! - Keep capability flow light and centralized: single trunk + small branches per kind
//! - Inputs: route kind (API|MCP), capability kind (Tools|Prompts|Resources), server_id, refresh/timeout
//! - Outputs: for API → JSON; for MCP → domain items (here as JSON of rmcp models, proxy maps to MCP objects)

use std::{borrow::Cow, sync::Arc, time::Duration};

use anyhow::Result;
use futures::future::BoxFuture;
use rmcp::model::{JsonObject, PaginatedRequestParam};
use rmcp::service::{Peer, RoleClient};
use serde::Serialize;
use sqlx::Row;
use tokio::sync::Mutex;

use crate::core::{
    cache::{CacheQuery, CachedToolInfo, FreshnessLevel, RedbCacheManager},
    capability::internal::{collect_capability_from_instance_peer, is_method_not_supported},
    pool::{CapSyncFlags, UpstreamConnectionPool},
};

#[derive(Clone, Copy, Debug)]
pub enum RouteKind {
    Api,
    Mcp,
}

#[derive(Clone, Copy, Debug)]
pub enum CapabilityKind {
    Tools,
    Prompts,
    Resources,
    ResourceTemplates,
}

#[derive(Clone, Copy, Debug)]
pub enum RefreshStrategy {
    CacheFirst,
    Force,
}

#[derive(Clone, Debug)]
pub struct ListCtx {
    pub route: RouteKind,
    pub capability: CapabilityKind,
    pub server_id: String,
    pub refresh: Option<RefreshStrategy>,
    pub timeout: Option<Duration>,
    pub validation_session: Option<String>,
}

#[derive(Clone, Debug)]
pub struct CallCtx {
    pub server_id: String,
    pub tool_name: String, // upstream original name preferred in this layer
    pub timeout: Option<Duration>,
    pub arguments: Option<JsonObject>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Meta {
    pub cache_hit: bool,
    pub source: String, // "cache" | "runtime" | "temporary"
    pub duration_ms: u64,
    pub had_peer: bool, // whether a runtime peer was used (helps decide temp instance creation)
}

#[derive(Clone, Debug, Serialize)]
pub struct ListResult {
    pub items: Vec<serde_json::Value>,
    pub meta: Meta,
}

#[derive(Clone, Copy, Debug)]
pub enum InstanceSource {
    Runtime,
    Temporary,
}

impl InstanceSource {
    fn as_str(self) -> &'static str {
        match self {
            InstanceSource::Runtime => "runtime",
            InstanceSource::Temporary => "temporary",
        }
    }
}

#[derive(Clone)]
pub struct InstanceHandle {
    pub server_id: String,
    pub server_name: String,
    pub instance_id: String,
    pub peer: Peer<RoleClient>,
    pub source: InstanceSource,
}

pub struct Sandwich;

impl Sandwich {
    pub async fn list(
        ctx: &ListCtx,
        redb: &Arc<RedbCacheManager>,
        pool: &Arc<Mutex<UpstreamConnectionPool>>,
        database: &Arc<crate::config::database::Database>,
    ) -> Result<ListResult> {
        let start = std::time::Instant::now();
        let timeout = ctx.timeout.unwrap_or_else(|| Duration::from_secs(10));

        // 1) REDB-first (skip when Force)
        if !matches!(ctx.refresh, Some(RefreshStrategy::Force)) {
            let cache_query = CacheQuery {
                server_id: ctx.server_id.clone(),
                freshness_level: FreshnessLevel::Cached,
                include_disabled: false,
            };
            if let Ok(result) = redb.get_server_data(&cache_query).await {
                if result.cache_hit {
                    if let Some(data) = result.data {
                        let items: Vec<serde_json::Value> = match ctx.capability {
                            CapabilityKind::Tools => data
                                .tools
                                .into_iter()
                                .filter_map(|tool| Self::convert_cached_tool(&data.server_name, tool))
                                .map(|tool| serde_json::to_value(tool).unwrap_or(serde_json::Value::Null))
                                .collect(),
                            CapabilityKind::Prompts => data
                                .prompts
                                .into_iter()
                                .map(|p| serde_json::to_value(p).unwrap_or(serde_json::Value::Null))
                                .collect(),
                            CapabilityKind::Resources => data
                                .resources
                                .into_iter()
                                .map(|r| serde_json::to_value(r).unwrap_or(serde_json::Value::Null))
                                .collect(),
                            CapabilityKind::ResourceTemplates => data
                                .resource_templates
                                .into_iter()
                                .map(|t| serde_json::to_value(t).unwrap_or(serde_json::Value::Null))
                                .collect(),
                        };
                        if !items.is_empty() {
                            return Ok(ListResult {
                                items,
                                meta: Meta {
                                    cache_hit: true,
                                    source: "cache".to_string(),
                                    duration_ms: start.elapsed().as_millis() as u64,
                                    had_peer: false,
                                },
                            });
                        }

                        // Cache returned empty list. If the server declared support for this capability,
                        // treat this as a soft MISS and continue to runtime fetch.
                        // ResourceTemplates inherits the declaration from Resources.
                        let token = match ctx.capability {
                            CapabilityKind::Tools => Some("tools"),
                            CapabilityKind::Prompts => Some("prompts"),
                            CapabilityKind::Resources => Some("resources"),
                            CapabilityKind::ResourceTemplates => Some("resources"),
                        };
                        let mut declared_has = false;
                        if let Some(tok) = token {
                            if let Ok(row_opt) = sqlx::query("SELECT capabilities FROM server_config WHERE id = ?")
                                .bind(&ctx.server_id)
                                .fetch_optional(&database.pool)
                                .await
                            {
                                if let Some(row) = row_opt {
                                    let caps: Option<String> = row.try_get("capabilities").ok();
                                    if let Some(caps) = caps {
                                        declared_has = caps.split(',').any(|c| c.trim().eq_ignore_ascii_case(tok));
                                    }
                                }
                            }
                        }
                        if !declared_has {
                            return Ok(ListResult {
                                items: Vec::new(),
                                meta: Meta {
                                    cache_hit: true,
                                    source: "cache".to_string(),
                                    duration_ms: start.elapsed().as_millis() as u64,
                                    had_peer: false,
                                },
                            });
                        }
                    }
                }
            }
        }

        // 2) Runtime via peer (lock pool shortly, then fetch outside lock)
        let (peer_opt, instance_id_opt, server_name, instance_source) = {
            let pool_guard = pool.lock().await;
            let name = crate::core::capability::global_server_mapping_manager()
                .get_name_by_id(&ctx.server_id)
                .await
                .unwrap_or_else(|| ctx.server_id.clone());
            let snap = pool_guard.get_snapshot();
            let mut peer_opt = None;
            let mut instance_id_opt = None;
            let mut instance_source = InstanceSource::Runtime;
            if let Some(instances) = snap.get(&ctx.server_id) {
                if let Some((iid, _status, _res, _prm, peer)) = instances.iter().find(|(_, st, _, _, p)| {
                    matches!(st, crate::core::foundation::types::ConnectionStatus::Ready) && p.is_some()
                }) {
                    peer_opt = peer.clone();
                    instance_id_opt = Some(iid.clone());
                }
            }

            if peer_opt.is_none() {
                if let Some(session_id) = ctx.validation_session.as_ref() {
                    if let Some(session_servers) = pool_guard.validation_sessions.get(session_id) {
                        if let Some(conn) = session_servers.get(&name) {
                            if let Some(service) = conn.service.as_ref() {
                                peer_opt = Some(service.peer().clone());
                                instance_id_opt = Some(conn.id.clone());
                                instance_source = InstanceSource::Temporary;
                            }
                        }
                    }
                }
            }

            (peer_opt, instance_id_opt, name, instance_source)
        };

        let peer = match peer_opt {
            Some(p) => p,
            None => {
                return Ok(ListResult {
                    items: Vec::new(),
                    meta: Meta {
                        cache_hit: false,
                        source: InstanceSource::Runtime.as_str().to_string(),
                        duration_ms: start.elapsed().as_millis() as u64,
                        had_peer: false,
                    },
                });
            }
        };

        let instance_id = instance_id_opt.unwrap_or_else(|| "default".to_string());
        let instance = InstanceHandle {
            server_id: ctx.server_id.clone(),
            server_name,
            instance_id,
            peer: peer.clone(),
            source: instance_source,
        };

        let mut result = Self::list_with_instance(ctx, instance, timeout, database.clone()).await?;
        result.meta.duration_ms = start.elapsed().as_millis() as u64;
        Ok(result)
    }

    pub async fn call_tool(
        ctx: &CallCtx,
        pool: &Arc<Mutex<UpstreamConnectionPool>>,
    ) -> Result<rmcp::model::CallToolResult> {
        let timeout = ctx.timeout.unwrap_or_else(|| Duration::from_secs(30));

        // Select peer from light snapshot
        let (peer_opt, _server_name) = {
            let pool_guard = pool.lock().await;
            let name = crate::core::capability::global_server_mapping_manager()
                .get_name_by_id(&ctx.server_id)
                .await
                .unwrap_or_else(|| ctx.server_id.clone());
            let snap = pool_guard.get_snapshot();
            if let Some(instances) = snap.get(&ctx.server_id) {
                if let Some((_, _status, _res, _prm, peer)) = instances.iter().find(|(_, st, _, _, p)| {
                    matches!(st, crate::core::foundation::types::ConnectionStatus::Ready) && p.is_some()
                }) {
                    (peer.clone(), name)
                } else {
                    (None, name)
                }
            } else {
                (None, name)
            }
        };

        let Some(peer) = peer_opt else {
            return Err(anyhow::anyhow!("No ready instance for server {}", ctx.server_id));
        };

        // Call with timeout (upstream original name is expected here)
        let tool_name = ctx.tool_name.clone();
        let arguments = ctx.arguments.clone();
        let fut = async move {
            peer.call_tool(rmcp::model::CallToolRequestParam {
                name: tool_name.into(),
                arguments,
            })
            .await
        };
        match tokio::time::timeout(timeout, fut).await {
            Ok(Ok(res)) => Ok(res),
            Ok(Err(e)) => Err(anyhow::anyhow!("Tool call failed: {}", e)),
            Err(_) => Err(anyhow::anyhow!("Tool call timeout for server {}", ctx.server_id)),
        }
    }

    async fn list_with_instance(
        ctx: &ListCtx,
        instance: InstanceHandle,
        timeout: Duration,
        database: Arc<crate::config::database::Database>,
    ) -> Result<ListResult> {
        use crate::core::capability::naming::{generate_unique_name, NamingKind};

        let server_id = instance.server_id.clone();
        let server_name = instance.server_name.clone();
        let peer = instance.peer.clone();

        let items: Vec<serde_json::Value> = match ctx.capability {
            CapabilityKind::Tools => {
                let fetch_page = move |p: Peer<RoleClient>,
                                       cursor: Option<String>|
                      -> BoxFuture<
                    'static,
                    anyhow::Result<(Vec<rmcp::model::Tool>, Option<String>)>,
                > {
                    Box::pin(async move {
                        let result = p.list_tools(Some(PaginatedRequestParam { cursor })).await?;
                        Ok((result.tools, result.next_cursor))
                    })
                };

                collect_capability_from_instance_peer(
                    peer.clone(),
                    timeout,
                    fetch_page,
                    |mut tool, srv_name, _srv_id, _inst_id| {
                        let unique_name = generate_unique_name(NamingKind::Tool, srv_name, &tool.name);
                        tool.name = unique_name.into();
                        serde_json::to_value(tool).unwrap_or(serde_json::Value::Null)
                    },
                    &server_id,
                    &server_name,
                    &instance.instance_id,
                    is_method_not_supported,
                )
                .await
            }
            CapabilityKind::Prompts => {
                let fetch_page = move |p: Peer<RoleClient>,
                                       cursor: Option<String>|
                      -> BoxFuture<
                    'static,
                    anyhow::Result<(Vec<rmcp::model::Prompt>, Option<String>)>,
                > {
                    Box::pin(async move {
                        let result = p.list_prompts(Some(PaginatedRequestParam { cursor })).await?;
                        Ok((result.prompts, result.next_cursor))
                    })
                };

                collect_capability_from_instance_peer(
                    peer.clone(),
                    timeout,
                    fetch_page,
                    |prompt, _srv_name, _srv_id, _inst_id| {
                        // TODO: handle prompt unique naming similar to tools/resources.
                        serde_json::to_value(prompt).unwrap_or(serde_json::Value::Null)
                    },
                    &server_id,
                    &server_name,
                    &instance.instance_id,
                    is_method_not_supported,
                )
                .await
            }
            CapabilityKind::Resources => {
                let fetch_page = move |p: Peer<RoleClient>,
                                       cursor: Option<String>|
                      -> BoxFuture<
                    'static,
                    anyhow::Result<(Vec<rmcp::model::Resource>, Option<String>)>,
                > {
                    Box::pin(async move {
                        let result = p.list_resources(Some(PaginatedRequestParam { cursor })).await?;
                        Ok((result.resources, result.next_cursor))
                    })
                };

                collect_capability_from_instance_peer(
                    peer.clone(),
                    timeout,
                    fetch_page,
                    |resource, _srv_name, _srv_id, _inst_id| {
                        // TODO: introduce naming module to distinguish duplicate resources across servers.
                        serde_json::to_value(resource).unwrap_or(serde_json::Value::Null)
                    },
                    &server_id,
                    &server_name,
                    &instance.instance_id,
                    is_method_not_supported,
                )
                .await
            }
            CapabilityKind::ResourceTemplates => {
                let fetch_page = move |p: Peer<RoleClient>,
                                       cursor: Option<String>|
                      -> BoxFuture<
                    'static,
                    anyhow::Result<(Vec<rmcp::model::ResourceTemplate>, Option<String>)>,
                > {
                    Box::pin(async move {
                        let result = p
                            .list_resource_templates(Some(PaginatedRequestParam { cursor }))
                            .await?;
                        Ok((result.resource_templates, result.next_cursor))
                    })
                };

                collect_capability_from_instance_peer(
                    peer.clone(),
                    timeout,
                    fetch_page,
                    |template, _srv_name, _srv_id, _inst_id| {
                        // TODO: introduce naming module to distinguish duplicate templates across servers.
                        serde_json::to_value(template).unwrap_or(serde_json::Value::Null)
                    },
                    &server_id,
                    &server_name,
                    &instance.instance_id,
                    is_method_not_supported,
                )
                .await
            }
        };

        // Async DB sync using pool helper (fire-and-forget)
        if !items.is_empty() {
            let db = database.clone();
            let server_id = instance.server_id.clone();
            let instance_id = instance.instance_id.clone();
            let peer = instance.peer.clone();
            let flags = match ctx.capability {
                CapabilityKind::Tools => CapSyncFlags::TOOLS,
                CapabilityKind::Prompts => CapSyncFlags::PROMPTS,
                CapabilityKind::Resources => CapSyncFlags::RESOURCES,
                CapabilityKind::ResourceTemplates => CapSyncFlags::RESOURCE_TEMPLATES,
            };
            tokio::spawn(async move {
                let _ =
                    UpstreamConnectionPool::sync_capabilities(&db, &server_id, &instance_id, &peer, flags, None).await;
            });
        }

        Ok(ListResult {
            items,
            meta: Meta {
                cache_hit: false,
                source: instance.source.as_str().to_string(),
                duration_ms: 0,
                had_peer: true,
            },
        })
    }
}

impl Sandwich {
    fn convert_cached_tool(
        server_name: &str,
        cached: CachedToolInfo,
    ) -> Option<rmcp::model::Tool> {
        use crate::core::capability::naming::{generate_unique_name, NamingKind};

        let schema_value: serde_json::Value = serde_json::from_str(&cached.input_schema_json).ok()?;
        let schema_object = schema_value.as_object()?.clone();

        let unique_name = cached
            .unique_name
            .clone()
            .unwrap_or_else(|| generate_unique_name(NamingKind::Tool, server_name, &cached.name));

        Some(rmcp::model::Tool {
            name: Cow::Owned(unique_name),
            title: None,
            description: cached.description.map(Cow::Owned),
            input_schema: Arc::new(schema_object),
            output_schema: None,
            annotations: None,
            icons: None,
        })
    }
}
