// Unified server import core for MCPMate
// Provides a single entrypoint used by: server API import, client config import, and first-run config import.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::api::models::server::{RegistryRepositoryInfo, ServerIcon, ServerMetaPayload, ServersImportConfig};
use crate::clients::analyzer::{ConfigImportSkipReason, InspectedServerEntry};
use crate::clients::models::ClientConfigFileParse;
use crate::clients::service::ClientConfigService;
use crate::common::constants::profile_keys;
use crate::common::server::ServerType;
use crate::config::models::{Server, ServerMeta};
use crate::config::registry::RegistryCacheService;
use crate::config::registry::cache::RegistryCacheEntry;
use crate::config::server as server_ops;
use crate::config::server::{args, env, fingerprint, get_all_servers, upsert_server};

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

impl ImportOptions {
    /// Default options for dashboard and first-run imports (skip on conflict; optional preview).
    pub fn dashboard_import(
        preview: bool,
        target_profile: Option<String>,
    ) -> Self {
        Self {
            by_name: true,
            by_fingerprint: true,
            conflict_policy: ConflictPolicy::Skip,
            preview,
            target_profile,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct ImportOutcome {
    pub imported: Vec<ImportedServer>,
    pub skipped: Vec<SkippedServer>,
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
    pub url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SkippedServer {
    pub name: String,
    pub reason: SkipReason,
}

#[derive(Debug, Clone)]
pub enum SkipReason {
    DuplicateName,
    DuplicateFingerprint,
    ConfigInvalidEntry,
    ConfigMissingCommand,
    ConfigMissingUrl,
    ConfigUnrecognized,
    UrlQueryMismatch {
        existing_query: Option<String>,
        incoming_query: Option<String>,
    },
}

impl From<ConfigImportSkipReason> for SkipReason {
    fn from(reason: ConfigImportSkipReason) -> Self {
        match reason {
            ConfigImportSkipReason::InvalidEntry => Self::ConfigInvalidEntry,
            ConfigImportSkipReason::MissingCommand => Self::ConfigMissingCommand,
            ConfigImportSkipReason::MissingUrl => Self::ConfigMissingUrl,
            ConfigImportSkipReason::Unrecognized => Self::ConfigUnrecognized,
        }
    }
}

pub struct ClientImportPlan {
    pub items: HashMap<String, ServersImportConfig>,
    pub skipped_servers: Vec<SkippedServer>,
}

fn record_conflict(
    outcome: &mut ImportOutcome,
    name: &str,
    reason: SkipReason,
    policy: ConflictPolicy,
) -> bool {
    match policy {
        ConflictPolicy::Skip => {
            outcome.skipped.push(SkippedServer {
                name: name.to_string(),
                reason,
            });
            true
        }
        ConflictPolicy::Error => {
            outcome.failed.insert(name.to_string(), "duplicate".to_string());
            true
        }
        ConflictPolicy::Update => false,
    }
}

fn build_imported_server(
    name: String,
    cfg: &ServersImportConfig,
    args: Vec<String>,
    env: HashMap<String, String>,
    server_type: &str,
) -> ImportedServer {
    ImportedServer {
        name,
        command: cfg.command.clone(),
        args,
        env,
        server_type: server_type.to_string(),
        url: cfg.url.clone(),
    }
}

fn is_mcpmate_import_entry(entry: &InspectedServerEntry) -> bool {
    entry.name.eq_ignore_ascii_case(profile_keys::MCPMATE)
}

pub(crate) fn build_import_plan_from_entries(
    entries: impl IntoIterator<Item = InspectedServerEntry>
) -> ClientImportPlan {
    let mut items = HashMap::new();
    let mut skipped_servers = Vec::new();
    for entry in entries {
        if is_mcpmate_import_entry(&entry) {
            continue;
        }

        match import_config_from_inspected_entry(entry) {
            Ok((name, config)) => {
                items.insert(name, config);
            }
            Err(skipped) => skipped_servers.push(skipped),
        }
    }

    ClientImportPlan { items, skipped_servers }
}

fn import_config_from_inspected_entry(
    entry: InspectedServerEntry
) -> std::result::Result<(String, ServersImportConfig), SkippedServer> {
    let (kind, command, url) = match entry.resolved_import_transport() {
        Ok(target) => (
            target.kind.to_string(),
            target.command.map(str::to_string),
            target.url.map(str::to_string),
        ),
        Err(reason) => {
            return Err(SkippedServer {
                name: entry.name,
                reason: reason.into(),
            });
        }
    };

    let InspectedServerEntry {
        name,
        args,
        env,
        headers,
        ..
    } = entry;
    let headers = if headers.is_empty() { None } else { Some(headers) };

    Ok((
        name,
        ServersImportConfig {
            kind,
            command,
            args: Some(args),
            url,
            env: Some(env),
            headers,
            registry_server_id: None,
            meta: None,
        },
    ))
}

pub async fn plan_import_from_client_inspection(
    service: &ClientConfigService,
    identifier: &str,
    config_path_override: Option<&str>,
    parse_rule: Option<&ClientConfigFileParse>,
    selected_server_names: &[String],
) -> Result<ClientImportPlan> {
    let trimmed_override = config_path_override.map(str::trim).filter(|path| !path.is_empty());
    let inspected = if let Some(path) = trimmed_override {
        let state = service
            .fetch_state(identifier)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Client '{}' not found", identifier))?;
        service
            .inspect_config_path_for_import(&state, path, parse_rule)
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?
    } else {
        service
            .inspect_current_config_for_import(identifier)
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?
    };

    let selected: HashSet<String> = selected_server_names
        .iter()
        .map(|name| name.trim().to_ascii_lowercase())
        .filter(|name| !name.is_empty())
        .collect();

    let entries: Vec<InspectedServerEntry> = inspected
        .inspection
        .entries
        .into_iter()
        .filter(|entry| selected.is_empty() || selected.contains(&entry.name.trim().to_ascii_lowercase()))
        .collect();

    Ok(build_import_plan_from_entries(entries))
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

    for (name, cfg) in items.into_iter() {
        let lc = cfg.kind.trim().to_ascii_lowercase();
        let server_type = ServerType::from_client_format(&lc)
            .map_err(|_| anyhow::anyhow!(format!("Invalid server type '{}'", cfg.kind)))?;
        let persisted_kind = server_type.client_format();
        validate_server_config(persisted_kind, &cfg.command, &cfg.url).map_err(|e| anyhow::anyhow!(e.to_string()))?;

        // Compute fingerprint
        let mut url_signature: Option<fingerprint::UrlSignature> = None;
        let fp = match server_type {
            ServerType::Stdio => fingerprint::fingerprint_for_stdio(
                cfg.command.as_deref().unwrap_or_default(),
                cfg.args.as_deref().unwrap_or(&[]),
            ),
            ServerType::Sse | ServerType::StreamableHttp => {
                let sig = fingerprint::url_signature(cfg.url.as_deref().unwrap_or_default());
                let key = format!("{}|{}", sig.fingerprint, persisted_kind);
                url_signature = Some(sig);
                key
            }
        };

        // Dedup
        let by_name_dup = opts.by_name && existing.names.contains(&name);
        let by_fp_dup = opts.by_fingerprint && !fp.is_empty() && existing.fingerprints.contains(&fp);

        if by_fp_dup
            && record_conflict(
                &mut outcome,
                &name,
                SkipReason::DuplicateFingerprint,
                opts.conflict_policy,
            )
        {
            continue;
        }

        if opts.by_fingerprint && !by_fp_dup {
            if let Some(sig) = url_signature.as_ref() {
                if existing.url_bases.contains(&sig.base) {
                    let existing_sig = existing.url_signatures.get(&sig.base);
                    if record_conflict(
                        &mut outcome,
                        &name,
                        SkipReason::UrlQueryMismatch {
                            existing_query: existing_sig.and_then(|s| s.display_query()),
                            incoming_query: sig.display_query(),
                        },
                        opts.conflict_policy,
                    ) {
                        continue;
                    }
                }
            }
        }

        if by_name_dup && record_conflict(&mut outcome, &name, SkipReason::DuplicateName, opts.conflict_policy) {
            continue;
        }

        // Normalize args/env once for both preview and apply.
        let (args_norm, env_norm) = normalize_args_env(
            cfg.args.clone().unwrap_or_default(),
            cfg.env.clone().unwrap_or_default(),
        );

        // Preview: report would-be imported without DB side-effects
        if opts.preview {
            outcome
                .imported
                .push(build_imported_server(name, &cfg, args_norm, env_norm, persisted_kind));
            continue;
        }

        // Apply: upsert server, args, env, headers
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

        if let Some(headers) = cfg.headers.as_ref() {
            if !headers.is_empty() {
                let _ = crate::config::server::upsert_server_headers(db_pool, &server_id, headers).await;
            }
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

        // Associate to target profiles only when explicitly requested
        if let Some(pid) = opts.target_profile.clone() {
            if let Err(err) = crate::config::profile::add_server_to_profile(db_pool, &pid, &server_id, true).await {
                tracing::error!(
                    target: "mcpmate::config::server::import",
                    server_id = %server_id,
                    profile_id = %pid,
                    error = %err,
                    "Failed to associate imported server with target profile"
                );
            }
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

        outcome.imported.push(build_imported_server(
            name,
            &cfg,
            args_norm,
            env_norm,
            server_type.client_format(),
        ));
    }

    Ok(outcome)
}

pub(crate) async fn upsert_import_meta(
    db_pool: &Pool<Sqlite>,
    server_id: &str,
    payload: &ServerMetaPayload,
) -> Result<()> {
    let meta = server_meta_from_payload(server_id, payload)?;

    server_ops::upsert_server_meta(db_pool, &meta)
        .await
        .context("Failed to persist server metadata during import")?;

    Ok(())
}

pub(crate) fn server_meta_from_payload(
    server_id: &str,
    payload: &ServerMetaPayload,
) -> Result<ServerMeta> {
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
    meta.icons_json = payload
        .icons
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .context("Failed to serialize server icons for import")?;

    Ok(meta)
}

pub(crate) fn normalize_args_env(
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
    url_bases: HashSet<String>,
    url_signatures: HashMap<String, fingerprint::UrlSignature>,
}

impl ExistingIndex {
    async fn build(db: &Pool<Sqlite>) -> Result<Self> {
        let mut names = HashSet::new();
        let mut fps = HashSet::new();
        let mut url_bases = HashSet::new();
        let mut url_sigs = HashMap::new();
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
                fps.insert(fingerprint::fingerprint_for_stdio(cmd, &args_list));
            }
            if let Some(url) = s.url.as_ref() {
                let sig = fingerprint::url_signature(url);
                let key = format!("{}|{}", sig.fingerprint, s.server_type.client_format());
                fps.insert(key);
                url_bases.insert(sig.base.clone());
                url_sigs.entry(sig.base.clone()).or_insert(sig);
            }
        }
        Ok(Self {
            names,
            fingerprints: fps,
            url_bases,
            url_signatures: url_sigs,
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
        "sse" | "streamable_http" if url.is_none() => Err("URL is required for HTTP-based servers"),
        "stdio" | "sse" | "streamable_http" => Ok(()),
        _ => Err("Invalid server type"),
    }
}

// Fingerprint helpers for stdio servers live in fingerprint.rs

// ========================= Registry Import =========================

/// Package information extracted from registry cache
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RegistryPackage {
    name: Option<String>,
    version: Option<String>,
}

/// Remote information extracted from registry cache
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RegistryRemote {
    url: Option<String>,
    r#type: Option<String>,
}

/// Result of converting a registry package to import config
#[derive(Debug, Clone)]
struct PackageImportConfig {
    kind: String,
    command: Option<String>,
    args: Option<Vec<String>>,
    url: Option<String>,
}

/// Convert npm package to import configuration
/// npm packages use: npx -y <identifier>@<version>
fn npm_package_to_import_config(
    identifier: &str,
    version: Option<&str>,
) -> PackageImportConfig {
    let full_identifier = match version {
        Some(v) => format!("{}@{}", identifier, v),
        None => identifier.to_string(),
    };
    PackageImportConfig {
        kind: "stdio".to_string(),
        command: Some("npx".to_string()),
        args: Some(vec!["-y".to_string(), full_identifier]),
        url: None,
    }
}

/// Convert remote URL to import configuration
/// remotes use streamable_http transport
fn remote_to_import_config(url: &str) -> PackageImportConfig {
    PackageImportConfig {
        kind: "streamable_http".to_string(),
        command: None,
        args: None,
        url: Some(url.to_string()),
    }
}

/// Parse packages_json from registry cache entry
fn parse_packages(packages_json: Option<&str>) -> Result<Vec<RegistryPackage>> {
    match packages_json {
        Some(json) if !json.is_empty() => serde_json::from_str(json).context("Failed to parse packages JSON"),
        _ => Ok(Vec::new()),
    }
}

/// Parse remotes_json from registry cache entry
fn parse_remotes(remotes_json: Option<&str>) -> Result<Vec<RegistryRemote>> {
    match remotes_json {
        Some(json) if !json.is_empty() => serde_json::from_str(json).context("Failed to parse remotes JSON"),
        _ => Ok(Vec::new()),
    }
}

/// Convert registry cache entry to import configuration
/// Priority: remotes > packages (remotes are preferred for HTTP-based servers)
fn registry_entry_to_import_config(
    entry: &RegistryCacheEntry,
    preferred_version: Option<&str>,
) -> Result<Option<PackageImportConfig>> {
    // First, check for remotes (HTTP-based servers)
    let remotes = parse_remotes(entry.remotes_json.as_deref())?;
    if let Some(remote) = remotes.first() {
        if let Some(url) = &remote.url {
            return Ok(Some(remote_to_import_config(url)));
        }
    }

    // Then, check for packages (stdio-based servers)
    let packages = parse_packages(entry.packages_json.as_deref())?;
    if let Some(package) = packages.first() {
        let name = package
            .name
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Package name is required for stdio server"))?;

        // Use preferred version if provided, otherwise use package version
        let version = preferred_version.or(package.version.as_deref());

        return Ok(Some(npm_package_to_import_config(name, version)));
    }

    // No packages or remotes found
    Ok(None)
}

fn parse_registry_icons(raw: Option<&str>) -> Option<Vec<ServerIcon>> {
    #[derive(Deserialize)]
    struct RegistryCacheIcon {
        #[serde(default)]
        url: Option<String>,
    }

    let raw = raw?;
    let icons = serde_json::from_str::<Vec<RegistryCacheIcon>>(raw).ok()?;
    let icons: Vec<ServerIcon> = icons
        .into_iter()
        .filter_map(|icon| {
            let src = icon.url?.trim().to_string();
            if src.is_empty() {
                None
            } else {
                Some(ServerIcon {
                    src,
                    mime_type: None,
                    sizes: None,
                })
            }
        })
        .collect();

    if icons.is_empty() { None } else { Some(icons) }
}

fn parse_repository(raw: Option<&str>) -> Option<RegistryRepositoryInfo> {
    raw.and_then(|source| serde_json::from_str(source).ok())
}

fn build_registry_extras(entry: &RegistryCacheEntry) -> Option<serde_json::Value> {
    let mut object = serde_json::Map::new();

    if let Some(title) = entry.title.as_ref().filter(|value| !value.trim().is_empty()) {
        object.insert("title".to_string(), serde_json::Value::String(title.clone()));
    }

    if let Some(packages_json) = entry.packages_json.as_ref().filter(|value| !value.trim().is_empty()) {
        if let Ok(packages) = serde_json::from_str::<serde_json::Value>(packages_json) {
            object.insert("packages".to_string(), packages);
        }
    }

    if let Some(remotes_json) = entry.remotes_json.as_ref().filter(|value| !value.trim().is_empty()) {
        if let Ok(remotes) = serde_json::from_str::<serde_json::Value>(remotes_json) {
            object.insert("remotes".to_string(), remotes);
        }
    }

    if !entry.status.trim().is_empty() {
        object.insert("status".to_string(), serde_json::Value::String(entry.status.clone()));
    }

    if let Some(published_at) = entry.published_at {
        object.insert(
            "publishedAt".to_string(),
            serde_json::Value::String(published_at.to_rfc3339()),
        );
    }

    if let Some(updated_at) = entry.updated_at {
        object.insert(
            "updatedAt".to_string(),
            serde_json::Value::String(updated_at.to_rfc3339()),
        );
    }

    if object.is_empty() {
        None
    } else {
        Some(serde_json::Value::Object(object))
    }
}

/// Build ServerMetaPayload from registry cache entry
pub(crate) fn build_meta_from_entry(entry: &RegistryCacheEntry) -> ServerMetaPayload {
    ServerMetaPayload {
        description: entry.description.clone(),
        version: Some(entry.version.clone()),
        website_url: entry.website_url.clone(),
        repository: parse_repository(entry.repository_json.as_deref()),
        meta: entry.meta_json.as_ref().and_then(|m| serde_json::from_str(m).ok()),
        extras: build_registry_extras(entry),
        icons: parse_registry_icons(entry.icons_json.as_deref()),
    }
}

/// Import a server from registry cache
///
/// # Arguments
/// * `db_pool` - Database pool for persistence
/// * `connection_pool` - Connection pool for capability sync
/// * `redb_cache` - Cache manager for capabilities
/// * `cache_service` - Registry cache service
/// * `name` - Server name in registry
/// * `version` - Optional version (defaults to latest cached version)
/// * `opts` - Import options
///
/// # Returns
/// * `Ok(ImportOutcome)` - Import result
/// * `Err` - If server not found or import failed
pub async fn import_from_registry(
    db_pool: &Pool<Sqlite>,
    connection_pool: &Arc<tokio::sync::Mutex<UpstreamConnectionPool>>,
    redb_cache: &Arc<RedbCacheManager>,
    cache_service: &RegistryCacheService,
    name: &str,
    version: Option<&str>,
    opts: ImportOptions,
) -> Result<ImportOutcome> {
    tracing::info!(
        target: "mcpmate::config::server::import",
        name = %name,
        version = ?version,
        "Importing server from registry"
    );

    // Fetch from cache
    let entry = cache_service
        .get_by_name(name)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Server '{}' not found in registry cache", name))?;

    // Check if server is active
    if entry.status != "active" {
        return Err(anyhow::anyhow!(
            "Server '{}' is not active (status: {})",
            name,
            entry.status
        ));
    }

    // Convert to import config
    let import_config = registry_entry_to_import_config(&entry, version)?
        .ok_or_else(|| anyhow::anyhow!("Server '{}' has no valid packages or remotes configuration", name))?;

    // Build ServersImportConfig
    let config = ServersImportConfig {
        kind: import_config.kind,
        command: import_config.command,
        args: import_config.args,
        url: import_config.url,
        env: None,
        headers: None,
        registry_server_id: Some(name.to_string()),
        meta: Some(build_meta_from_entry(&entry)),
    };

    // Build items map
    let mut items = HashMap::new();
    items.insert(name.to_string(), config);

    // Call import_batch
    import_batch(db_pool, connection_pool, redb_cache, items, opts).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server_entry(
        name: &str,
        transport: &str,
        command: Option<&str>,
        url: Option<&str>,
        issue: Option<&str>,
    ) -> InspectedServerEntry {
        InspectedServerEntry {
            name: name.to_string(),
            transport: transport.to_string(),
            command: command.map(str::to_string),
            args: Vec::new(),
            env: HashMap::new(),
            headers: HashMap::new(),
            url: url.map(str::to_string),
            issue: issue.map(str::to_string),
        }
    }

    #[test]
    fn client_config_import_plan_filters_out_mcpmate_self_entry() {
        let plan = build_import_plan_from_entries([
            server_entry("MCPMate", "stdio", Some("mcpmate-bridge"), None, None),
            server_entry(
                "context7",
                "streamable_http",
                None,
                Some("http://127.0.0.1:8123/mcp"),
                None,
            ),
            server_entry("shadcn-mcp-server", "unclassified", None, None, None),
        ]);

        assert!(!plan.items.contains_key("MCPMate"));
        let context7 = plan.items.get("context7").expect("context7 server entry");
        assert_eq!(context7.kind, "streamable_http");
        assert_eq!(context7.url.as_deref(), Some("http://127.0.0.1:8123/mcp"));
        assert_eq!(plan.skipped_servers.len(), 1);
        assert_eq!(plan.skipped_servers[0].name, "shadcn-mcp-server");
        assert!(matches!(plan.skipped_servers[0].reason, SkipReason::ConfigUnrecognized));
    }

    #[test]
    fn client_config_import_plan_reports_invalid_entries() {
        let plan = build_import_plan_from_entries([
            server_entry("broken", "unclassified", None, None, Some("config_invalid_entry")),
            server_entry("valid", "stdio", Some("uvx"), None, None),
        ]);

        assert!(plan.items.contains_key("valid"));
        assert_eq!(plan.skipped_servers.len(), 1);
        assert_eq!(plan.skipped_servers[0].name, "broken");
        assert!(matches!(plan.skipped_servers[0].reason, SkipReason::ConfigInvalidEntry));
    }

    #[test]
    fn test_npm_package_to_import_config_with_version() {
        let config = npm_package_to_import_config("@modelcontextprotocol/server-filesystem", Some("1.0.0"));
        assert_eq!(config.kind, "stdio");
        assert_eq!(config.command, Some("npx".to_string()));
        assert_eq!(
            config.args,
            Some(vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-filesystem@1.0.0".to_string()
            ])
        );
        assert!(config.url.is_none());
    }

    #[test]
    fn test_npm_package_to_import_config_without_version() {
        let config = npm_package_to_import_config("@modelcontextprotocol/server-filesystem", None);
        assert_eq!(config.kind, "stdio");
        assert_eq!(config.command, Some("npx".to_string()));
        assert_eq!(
            config.args,
            Some(vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-filesystem".to_string()
            ])
        );
    }

    #[test]
    fn test_remote_to_import_config() {
        let config = remote_to_import_config("https://api.example.com/mcp");
        assert_eq!(config.kind, "streamable_http");
        assert!(config.command.is_none());
        assert!(config.args.is_none());
        assert_eq!(config.url, Some("https://api.example.com/mcp".to_string()));
    }

    #[test]
    fn test_parse_packages_valid_json() {
        let json = r#"[{"name": "@scope/package", "version": "1.0.0"}]"#;
        let packages = parse_packages(Some(json)).unwrap();
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].name, Some("@scope/package".to_string()));
        assert_eq!(packages[0].version, Some("1.0.0".to_string()));
    }

    #[test]
    fn test_parse_packages_empty_json() {
        let packages = parse_packages(Some("[]")).unwrap();
        assert!(packages.is_empty());
    }

    #[test]
    fn test_parse_packages_none() {
        let packages = parse_packages(None).unwrap();
        assert!(packages.is_empty());
    }

    #[test]
    fn test_parse_remotes_valid_json() {
        let json = r#"[{"url": "https://api.example.com/mcp", "type": "http"}]"#;
        let remotes = parse_remotes(Some(json)).unwrap();
        assert_eq!(remotes.len(), 1);
        assert_eq!(remotes[0].url, Some("https://api.example.com/mcp".to_string()));
    }

    #[test]
    fn test_registry_entry_to_import_config_with_remote() {
        let entry = RegistryCacheEntry {
            server_name: "test-server".to_string(),
            version: "1.0.0".to_string(),
            schema_url: None,
            title: None,
            description: None,
            packages_json: Some(r#"[{"name": "test-pkg"}]"#.to_string()),
            remotes_json: Some(r#"[{"url": "https://api.example.com/mcp"}]"#.to_string()),
            icons_json: None,
            meta_json: None,
            website_url: None,
            repository_json: None,
            status: "active".to_string(),
            published_at: None,
            updated_at: None,
            synced_at: chrono::Utc::now(),
        };

        let config = registry_entry_to_import_config(&entry, None).unwrap().unwrap();
        // Remotes take priority
        assert_eq!(config.kind, "streamable_http");
        assert_eq!(config.url, Some("https://api.example.com/mcp".to_string()));
    }

    #[test]
    fn test_registry_entry_to_import_config_with_npm_package() {
        let entry = RegistryCacheEntry {
            server_name: "test-server".to_string(),
            version: "1.0.0".to_string(),
            schema_url: None,
            title: None,
            description: None,
            packages_json: Some(r#"[{"name": "@scope/package", "version": "1.0.0"}]"#.to_string()),
            remotes_json: None,
            icons_json: None,
            meta_json: None,
            website_url: None,
            repository_json: None,
            status: "active".to_string(),
            published_at: None,
            updated_at: None,
            synced_at: chrono::Utc::now(),
        };

        let config = registry_entry_to_import_config(&entry, None).unwrap().unwrap();
        assert_eq!(config.kind, "stdio");
        assert_eq!(config.command, Some("npx".to_string()));
        assert_eq!(
            config.args,
            Some(vec!["-y".to_string(), "@scope/package@1.0.0".to_string()])
        );
    }

    #[test]
    fn test_registry_entry_to_import_config_with_preferred_version() {
        let entry = RegistryCacheEntry {
            server_name: "test-server".to_string(),
            version: "1.0.0".to_string(),
            schema_url: None,
            title: None,
            description: None,
            packages_json: Some(r#"[{"name": "@scope/package", "version": "1.0.0"}]"#.to_string()),
            remotes_json: None,
            icons_json: None,
            meta_json: None,
            website_url: None,
            repository_json: None,
            status: "active".to_string(),
            published_at: None,
            updated_at: None,
            synced_at: chrono::Utc::now(),
        };

        let config = registry_entry_to_import_config(&entry, Some("2.0.0")).unwrap().unwrap();
        assert_eq!(
            config.args,
            Some(vec!["-y".to_string(), "@scope/package@2.0.0".to_string()])
        );
    }

    #[test]
    fn test_registry_entry_to_import_config_no_packages_or_remotes() {
        let entry = RegistryCacheEntry {
            server_name: "test-server".to_string(),
            version: "1.0.0".to_string(),
            schema_url: None,
            title: None,
            description: None,
            packages_json: None,
            remotes_json: None,
            icons_json: None,
            meta_json: None,
            website_url: None,
            repository_json: None,
            status: "active".to_string(),
            published_at: None,
            updated_at: None,
            synced_at: chrono::Utc::now(),
        };

        let config = registry_entry_to_import_config(&entry, None).unwrap();
        assert!(config.is_none());
    }

    #[test]
    fn test_build_meta_from_entry() {
        let entry = RegistryCacheEntry {
            server_name: "test-server".to_string(),
            version: "1.0.0".to_string(),
            schema_url: Some("https://modelcontextprotocol.io/schema/server.schema.json".to_string()),
            title: Some("Test Server".to_string()),
            description: Some("A test server".to_string()),
            packages_json: Some(r#"[{"name":"@scope/package","version":"1.0.0"}]"#.to_string()),
            remotes_json: Some(r#"[{"url":"https://example.com/mcp","type":"streamable_http"}]"#.to_string()),
            icons_json: Some(r#"[{"url":"https://example.com/icon.png"}]"#.to_string()),
            meta_json: Some(r#"{"io.modelcontextprotocol.registry/official": {"status": "published"}}"#.to_string()),
            website_url: Some("https://example.com/server".to_string()),
            repository_json: Some(r#"{"url":"https://github.com/example/test-server","source":"github"}"#.to_string()),
            status: "active".to_string(),
            published_at: Some(chrono::Utc::now()),
            updated_at: Some(chrono::Utc::now()),
            synced_at: chrono::Utc::now(),
        };

        let meta = build_meta_from_entry(&entry);
        assert_eq!(meta.description, Some("A test server".to_string()));
        assert_eq!(meta.version, Some("1.0.0".to_string()));
        assert_eq!(meta.website_url.as_deref(), Some("https://example.com/server"));
        assert_eq!(
            meta.repository.as_ref().and_then(|repo| repo.source.as_deref()),
            Some("github")
        );
        assert!(meta.meta.is_some());
        assert_eq!(meta.icons.as_ref().map(Vec::len), Some(1));
        assert!(meta.extras.is_some());
    }
}
