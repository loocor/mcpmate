// Unified server import core for MCPMate
// Provides a single entrypoint used by: server API import, client config import, and first-run config import.

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use url::Url;

use crate::api::models::server::{ServerMetaPayload, ServersImportConfig};
use crate::common::server::ServerType;
use crate::config::models::{Server, ServerMeta};
use crate::config::server as server_ops;
use crate::config::server::{args, env, get_all_servers, upsert_server};

// Capability sync utilities (dual write to SQLite shadow + REDB)
use crate::config::server::capabilities::sync_via_connection_pool;
use crate::core::cache::RedbCacheManager;
use crate::core::pool::UpstreamConnectionPool;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictPolicy {
    Skip,
    Update,
    Error,
}

#[derive(Debug, Clone)]
pub struct ImportOptions {
    pub by_name: bool,
    pub by_fingerprint: bool,
    pub conflict_policy: ConflictPolicy,
    pub preview: bool,
    pub target_profile: Option<String>,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            by_name: true,
            by_fingerprint: true,
            conflict_policy: ConflictPolicy::Skip,
            preview: false,
            target_profile: None,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct ImportOutcome {
    pub imported: Vec<ImportedServer>,
    pub skipped: Vec<String>,
    pub failed: HashMap<String, String>,
    pub scheduled: bool,
}

#[derive(Debug, Clone)]
pub struct ImportedServer {
    pub name: String,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub server_type: String,
}

/// Import a batch of servers with consistent deduplication and capability sync.
/// - `items`: map of server name -> ServersImportConfig (kind/command/url/args/env)
pub async fn import_batch(
    db_pool: &Pool<Sqlite>,
    connection_pool: &Arc<tokio::sync::Mutex<UpstreamConnectionPool>>,
    redb_cache: &Arc<RedbCacheManager>,
    items: HashMap<String, ServersImportConfig>,
    opts: ImportOptions,
) -> Result<ImportOutcome> {
    let mut outcome = ImportOutcome::default();
    tracing::info!(
        target: "mcpmate::config::server::import",
        count = items.len(),
        preview = %opts.preview,
        target_profile = ?opts.target_profile,
        "Starting server import batch"
    );
    let existing = ExistingIndex::build(db_pool).await?;

    // We'll need profile association (lazily resolve default if not provided)
    let mut default_profile_cache: Option<String> = opts.target_profile.clone();

    for (name, cfg) in items.into_iter() {
        // Validate and normalize
        let server_type = ServerType::from_client_format(cfg.kind.as_str())
            .map_err(|_| anyhow::anyhow!(format!("Invalid server type '{}'", cfg.kind)))?;
        validate_server_config(&cfg.kind, &cfg.command, &cfg.url).map_err(|e| anyhow::anyhow!(e.to_string()))?;

        // Compute fingerprint
        let fp = match server_type {
            ServerType::Stdio => fingerprint_for_stdio(
                cfg.command.as_deref().unwrap_or_default(),
                cfg.args.as_deref().unwrap_or(&[]),
            ),
            ServerType::Sse | ServerType::StreamableHttp => {
                fingerprint_for_url(cfg.url.as_deref().unwrap_or_default()).unwrap_or_default()
            }
        };

        // Dedup
        let by_name_dup = opts.by_name && existing.names.contains(&name);
        let by_fp_dup = opts.by_fingerprint && !fp.is_empty() && existing.fingerprints.contains(&fp);
        if by_name_dup || by_fp_dup {
            match opts.conflict_policy {
                ConflictPolicy::Skip => {
                    outcome.skipped.push(name);
                    continue;
                }
                ConflictPolicy::Error => {
                    outcome.failed.insert(name, "duplicate".to_string());
                    continue;
                }
                ConflictPolicy::Update => {
                    // fall through to upsert below
                }
            }
        }

        // Preview: report would-be imported without DB side-effects
        if opts.preview {
            outcome.imported.push(ImportedServer {
                name,
                command: cfg.command.clone(),
                args: cfg.args.clone().unwrap_or_default(),
                env: cfg.env.clone().unwrap_or_default(),
                server_type: cfg.kind.clone(),
            });
            continue;
        }

        // Normalize args/env: move KEY=VALUE patterns from args into env for safety
        let (args_norm, env_norm) = normalize_args_env(
            cfg.args.clone().unwrap_or_default(),
            cfg.env.clone().unwrap_or_default(),
        );

        // Apply: upsert server, args, env
        let mut server = match server_type {
            ServerType::Stdio => Server::new_stdio(name.clone(), cfg.command.clone()),
            ServerType::Sse => Server::new_sse(name.clone(), cfg.url.clone()),
            ServerType::StreamableHttp => Server::new_streamable_http(name.clone(), cfg.url.clone()),
        };
        server.registry_server_id = cfg.registry_server_id.clone();
        // Persist transport_type consistent with server_type to aid validation/preview paths
        // (DB accepts lowercase client-format values per Type/Encode implementation)
        // Stdio/Sse/StreamableHttp map 1:1 here via Server::new_* constructors; keep as-is.

        let server_id = upsert_server(db_pool, &server)
            .await
            .with_context(|| format!("Failed to upsert server '{}'", name))?;

        if !args_norm.is_empty() {
            let _ = args::upsert_server_args(db_pool, &server_id, &args_norm).await;
        }
        if !env_norm.is_empty() {
            let _ = env::upsert_server_env(db_pool, &server_id, &env_norm).await;
        }

        if let Some(meta_payload) = cfg.meta.as_ref() {
            if let Err(err) = upsert_import_meta(db_pool, &server_id, meta_payload).await {
                tracing::warn!(
                    target: "mcpmate::config::server::import",
                    server_id = %server_id,
                    server_name = %name,
                    error = %err,
                    "Failed to persist metadata for imported server"
                );
            }
        }

        // Associate to a single target profile if provided, otherwise default profile
        let pid = if let Some(pid) = opts.target_profile.clone() {
            pid
        } else if let Some(pid) = default_profile_cache.clone() {
            pid
        } else {
            match crate::config::profile::ensure_default_anchor_profile_id(db_pool).await {
                Ok(id) => {
                    default_profile_cache = Some(id.clone());
                    id
                }
                Err(err) => {
                    tracing::error!(
                        target: "mcpmate::config::server::import",
                        error = %err,
                        "Failed to ensure default anchor profile while importing servers"
                    );
                    String::new()
                }
            }
        };
        if !pid.is_empty() {
            let _ = crate::config::profile::add_server_to_profile(db_pool, &pid, &server_id, true).await;
        }

        // Update resolver cache (id <-> name) so capability service can map server_id to server_name immediately
        crate::core::capability::resolver::upsert(&server_id, &name).await;

        // Capability discovery + dual write (schedule in background to avoid blocking import)
        {
            let cp = connection_pool.clone();
            let redb = redb_cache.clone();
            let dbp = db_pool.clone();
            let sid = server_id.clone();
            let sname = name.clone();
            tokio::spawn(async move {
                use tokio::time::{Duration, sleep};
                tracing::info!(
                    target: "mcpmate::config::server::import",
                    server_id = %sid,
                    server_name = %sname,
                    "Scheduling capability sync"
                );

                // Mark as refreshing for a short TTL
                let _ = redb.set_refreshing(&sid, Duration::from_secs(60)).await;

                let max_retries: u32 = std::env::var("MCPMATE_IMPORT_CAP_SYNC_RETRIES")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(2);
                let mut delay_ms: u64 = std::env::var("MCPMATE_IMPORT_CAP_SYNC_BACKOFF_MS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(2000);

                for attempt in 0..=max_retries {
                    match sync_via_connection_pool(
                        &cp,
                        &redb,
                        &dbp,
                        &sid,
                        &sname,
                        crate::config::server::capabilities::default_pool_lock_timeout_secs(),
                    )
                    .await
                    {
                        Ok(_) => {
                            tracing::info!(
                                target: "mcpmate::config::server::import",
                                server_id = %sid,
                                server_name = %sname,
                                attempt,
                                "Capability sync finished"
                            );
                            break;
                        }
                        Err(e) => {
                            if attempt >= max_retries {
                                tracing::warn!(
                                    target: "mcpmate::config::server::import",
                                    server_id = %sid,
                                    server_name = %sname,
                                    attempt,
                                    error = %e,
                                    "Capability sync failed after retries"
                                );
                                break;
                            }
                            tracing::warn!(
                                target: "mcpmate::config::server::import",
                                server_id = %sid,
                                server_name = %sname,
                                attempt,
                                backoff_ms = delay_ms,
                                error = %e,
                                "Capability sync failed, will retry"
                            );
                            sleep(Duration::from_millis(delay_ms)).await;
                            delay_ms = (delay_ms.saturating_mul(2)).min(30_000);
                        }
                    }
                }
            });
        }
        outcome.scheduled = true;

        outcome.imported.push(ImportedServer {
            name,
            command: cfg.command.clone(),
            args: args_norm,
            env: env_norm,
            server_type: server_type.client_format().to_string(),
        });
    }

    Ok(outcome)
}

async fn upsert_import_meta(
    db_pool: &Pool<Sqlite>,
    server_id: &str,
    payload: &ServerMetaPayload,
) -> Result<()> {
    let mut meta = ServerMeta::new(server_id.to_owned());
    meta.description = payload.description.clone();
    meta.website = payload.website_url.clone();
    meta.registry_version = payload.version.clone();
    meta.repository = payload
        .repository
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .context("Failed to serialize repository metadata for import")?;
    meta.registry_meta_json = payload
        .meta
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .context("Failed to serialize registry meta block for import")?;
    meta.extras_json = payload
        .extras
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .context("Failed to serialize extras metadata for import")?;

    server_ops::upsert_server_meta(db_pool, &meta)
        .await
        .context("Failed to persist server metadata during import")?;

    Ok(())
}

fn normalize_args_env(
    args: Vec<String>,
    env: std::collections::HashMap<String, String>,
) -> (Vec<String>, std::collections::HashMap<String, String>) {
    let mut env_map = env;
    let mut filtered_args = Vec::with_capacity(args.len());
    for a in args.into_iter() {
        if let Some((k, v)) = parse_env_assignment(&a).or_else(|| parse_env_assignment_fallback(&a)) {
            env_map.entry(k).or_insert(v);
        } else {
            filtered_args.push(a);
        }
    }
    (filtered_args, env_map)
}

// Less strict fallback for assignments like KEY="VALUE" with spaces trimmed
fn parse_env_assignment_fallback(s: &str) -> Option<(String, String)> {
    if s.starts_with('-') {
        return None;
    }
    let eq = s.find('=')?;
    let (k, v) = s.split_at(eq);
    if k.is_empty() {
        return None;
    }
    let mut value = v[1..].trim().to_string();
    if ((value.starts_with('"') && value.ends_with('"')) || (value.starts_with('\'') && value.ends_with('\'')))
        && value.len() >= 2
    {
        value = value[1..value.len() - 1].to_string();
    }
    Some((k.to_string(), value))
}

// Strict env assignment parser: KEY=VALUE with KEY matching [A-Za-z_][A-Za-z0-9_]* and not starting with '-'
fn parse_env_assignment(s: &str) -> Option<(String, String)> {
    if s.starts_with('-') {
        return None;
    }
    let eq = s.find('=')?;
    let (k, v) = s.split_at(eq);
    if k.is_empty() {
        return None;
    }
    let mut chars = k.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => (),
        _ => return None,
    };
    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    let mut value = v[1..].trim().to_string();
    if ((value.starts_with('"') && value.ends_with('"')) || (value.starts_with('\'') && value.ends_with('\'')))
        && value.len() >= 2
    {
        value = value[1..value.len() - 1].to_string();
    }
    Some((k.to_string(), value))
}

// ========================= Helpers =========================

#[derive(Debug)]
struct ExistingIndex {
    names: HashSet<String>,
    fingerprints: HashSet<String>,
}

impl ExistingIndex {
    async fn build(db: &Pool<Sqlite>) -> Result<Self> {
        let mut names = HashSet::new();
        let mut fps = HashSet::new();
        let servers = get_all_servers(db).await?;
        for s in servers {
            names.insert(s.name.clone());
            if let Some(cmd) = s.command.as_ref() {
                // load args
                let args_list = if let Some(id) = s.id.as_ref() {
                    args::get_server_args(db, id)
                        .await
                        .unwrap_or_default()
                        .into_iter()
                        .map(|a| a.arg_value)
                        .collect()
                } else {
                    Vec::new()
                };
                fps.insert(fingerprint_for_stdio(cmd, &args_list));
            }
            if let Some(url) = s.url.as_ref() {
                if let Some(fp) = fingerprint_for_url(url) {
                    fps.insert(fp);
                }
            }
        }
        Ok(Self {
            names,
            fingerprints: fps,
        })
    }
}

fn validate_server_config(
    kind: &str,
    command: &Option<String>,
    url: &Option<String>,
) -> Result<(), &'static str> {
    match kind {
        "stdio" if command.is_none() => Err("Command is required for stdio servers"),
        "sse" | "streamable_http" if url.is_none() => Err("URL is required for sse/streamable_http servers"),
        "stdio" | "sse" | "streamable_http" => Ok(()),
        _ => Err("Invalid server type"),
    }
}

fn fingerprint_for_stdio(
    command: &str,
    args: &[String],
) -> String {
    let cmd = command.trim().to_ascii_lowercase();
    let mut a: Vec<&str> = args.iter().map(|s| s.trim()).filter(|s| !s.is_empty()).collect();

    // Node runners: npx/bunx/pnpm dlx/yarn dlx => node-pkg:<pkg> ...
    if cmd == "npx" || cmd == "bunx" || cmd == "pnpm" || cmd == "yarn" {
        if (cmd == "pnpm" || cmd == "yarn") && a.first().map(|s| *s == "dlx").unwrap_or(false) && a.len() >= 2 {
            let pkg = a[1];
            return format!("node-pkg:{}:{}", pkg, a.get(2..).unwrap_or(&[]).join(" "));
        }
        if !a.is_empty() {
            let pkg = a[0];
            return format!("node-pkg:{}:{}", pkg, a.get(1..).unwrap_or(&[]).join(" "));
        }
    }

    // Python: uvx python -m <mod> / python -m <mod> / uvx <pkg>
    if cmd == "uvx" || cmd == "pipx" || cmd == "python" || cmd == "python3" || cmd == "py" {
        if cmd == "uvx" && a.first().map(|s| s.to_ascii_lowercase()) == Some("python".to_string()) {
            a.remove(0);
        }
        let lower: Vec<String> = a.iter().map(|s| s.to_ascii_lowercase()).collect();
        if lower.first().map(|s| s.as_str()) == Some("-m") {
            if let Some(module) = a.get(1) {
                return format!("python-module:{}:{}", module, a.get(2..).unwrap_or(&[]).join(" "));
            }
        }
        if cmd == "uvx" && !a.is_empty() {
            let pkg = a[0];
            return format!("python-pkg:{}:{}", pkg, a.get(1..).unwrap_or(&[]).join(" "));
        }
        if (cmd == "python" || cmd == "python3" || cmd == "py") && !a.is_empty() {
            let script = a[0].rsplit('/').next().unwrap_or(a[0]);
            return format!("python-file:{}:{}", script, a.get(1..).unwrap_or(&[]).join(" "));
        }
    }

    // Default: raw cmd + first 3 args
    let mut tail = String::new();
    for (i, v) in a.iter().take(3).enumerate() {
        if i > 0 {
            tail.push(' ');
        }
        tail.push_str(v);
    }
    format!("cmd:{}:{}", cmd, tail)
}

fn fingerprint_for_url(raw: &str) -> Option<String> {
    let url = Url::parse(raw).ok()?;
    let mut url = url;
    url.set_fragment(None);

    let scheme = url.scheme().to_ascii_lowercase();
    let host = url.host_str()?.to_ascii_lowercase();
    let port_opt = url.port();
    let default_port = match scheme.as_str() {
        "http" => Some(80),
        "https" => Some(443),
        _ => None,
    };
    let port_part = match (port_opt, default_port) {
        (Some(p), Some(d)) if p != d => format!(":{}", p),
        (Some(p), None) => format!(":{}", p),
        _ => String::new(),
    };
    let mut path = url.path().to_string();
    if path.ends_with('/') && path.len() > 1 {
        path.pop();
    }
    if path.is_empty() {
        path = "/".to_string();
    }
    let query_sorted = if let Some(q) = url.query() {
        let mut pairs: Vec<(String, String)> = url::form_urlencoded::parse(q.as_bytes()).into_owned().collect();
        pairs.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
        if pairs.is_empty() {
            String::new()
        } else {
            let serialized = pairs
                .into_iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("&");
            format!("?{}", serialized)
        }
    } else {
        String::new()
    };
    Some(format!("{}://{}{}{}{}", scheme, host, port_part, path, query_sorted))
}
