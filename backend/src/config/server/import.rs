// Unified server import core for MCPMate
// Provides a single entrypoint used by: server API import, client config import, and first-run config import.

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::api::models::server::{ServerMetaPayload, ServersImportConfig};
use crate::clients::analyzer::{ConfigImportSkipReason, InspectedServerEntry};
use crate::clients::models::ClientConfigFileParse;
use crate::clients::service::ClientConfigService;
use crate::common::constants::profile_keys;
use crate::common::server::ServerType;
use crate::common::types::{ServerSource, ServerSourceType};
use crate::config::models::{Server, ServerMeta};
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

impl SkipReason {
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::DuplicateName => "duplicate_name",
            Self::DuplicateFingerprint => "duplicate_fingerprint",
            Self::ConfigInvalidEntry => "config_invalid_entry",
            Self::ConfigMissingCommand => "config_missing_command",
            Self::ConfigMissingUrl => "config_missing_url",
            Self::ConfigUnrecognized => "config_unrecognized",
            Self::UrlQueryMismatch { .. } => "url_query_mismatch",
        }
    }

    pub(crate) fn is_duplicate_fingerprint(&self) -> bool {
        matches!(self, Self::DuplicateFingerprint)
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

pub(crate) struct ImportCandidate {
    server_type: ServerType,
    persisted_kind: &'static str,
    pub(crate) fingerprint: String,
    pub(crate) url_signature: Option<fingerprint::UrlSignature>,
}

pub(crate) fn prepare_import_candidate_from_parts(
    kind: &str,
    command: Option<&str>,
    url: Option<&str>,
    args: &[String],
) -> Result<ImportCandidate> {
    let lc = kind.trim().to_ascii_lowercase();
    let server_type =
        ServerType::from_client_format(&lc).map_err(|_| anyhow::anyhow!(format!("Invalid server type '{}'", kind)))?;
    let persisted_kind = server_type.client_format();
    let command_for_validation = command.map(str::to_string);
    let url_for_validation = url.map(str::to_string);
    validate_server_config(persisted_kind, &command_for_validation, &url_for_validation)
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    let mut url_signature: Option<fingerprint::UrlSignature> = None;
    let fp = match server_type {
        ServerType::Stdio => fingerprint::fingerprint_for_stdio(command.unwrap_or_default(), args),
        ServerType::Sse | ServerType::StreamableHttp => {
            let sig = fingerprint::url_signature(url.unwrap_or_default());
            let key = format!("{}|{}", sig.fingerprint, persisted_kind);
            url_signature = Some(sig);
            key
        }
    };

    Ok(ImportCandidate {
        server_type,
        persisted_kind,
        fingerprint: fp,
        url_signature,
    })
}

fn prepare_import_candidate(cfg: &ServersImportConfig) -> Result<ImportCandidate> {
    prepare_import_candidate_from_parts(
        &cfg.kind,
        cfg.command.as_deref(),
        cfg.url.as_deref(),
        cfg.args.as_deref().unwrap_or(&[]),
    )
}

pub(crate) trait ImportConflictIndex {
    fn names(&self) -> &HashSet<String>;
    fn fingerprints(&self) -> &HashSet<String>;
    fn url_bases(&self) -> &HashSet<String>;
    fn url_signatures(&self) -> &HashMap<String, fingerprint::UrlSignature>;
}

pub(crate) fn import_conflict_reason(
    existing: &impl ImportConflictIndex,
    name: &str,
    candidate: &ImportCandidate,
    opts: &ImportOptions,
) -> Option<SkipReason> {
    if opts.by_fingerprint
        && !candidate.fingerprint.is_empty()
        && existing.fingerprints().contains(&candidate.fingerprint)
    {
        return Some(SkipReason::DuplicateFingerprint);
    }

    if opts.by_fingerprint {
        if let Some(sig) = candidate.url_signature.as_ref() {
            if existing.url_bases().contains(&sig.base) {
                let existing_sig = existing.url_signatures().get(&sig.base);
                return Some(SkipReason::UrlQueryMismatch {
                    existing_query: existing_sig.and_then(|s| s.display_query()),
                    incoming_query: sig.display_query(),
                });
            }
        }
    }

    if opts.by_name && existing.names().contains(name) {
        return Some(SkipReason::DuplicateName);
    }

    None
}

pub(crate) async fn find_import_conflicts(
    db_pool: &Pool<Sqlite>,
    items: &HashMap<String, ServersImportConfig>,
    opts: &ImportOptions,
) -> Result<HashMap<String, SkipReason>> {
    let existing = ExistingIndex::build(db_pool).await?;
    let mut conflicts = HashMap::new();

    for (name, cfg) in items {
        let candidate = prepare_import_candidate(cfg)?;
        if let Some(reason) = import_conflict_reason(&existing, name, &candidate, opts) {
            conflicts.insert(name.clone(), reason);
        }
    }

    Ok(conflicts)
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
    entries: impl IntoIterator<Item = InspectedServerEntry>,
    client_identifier: &str,
) -> ClientImportPlan {
    let mut items = HashMap::new();
    let mut skipped_servers = Vec::new();
    for entry in entries {
        if is_mcpmate_import_entry(&entry) {
            continue;
        }

        match import_config_from_inspected_entry(entry, client_identifier) {
            Ok((name, config)) => {
                items.insert(name, config);
            }
            Err(skipped) => skipped_servers.push(skipped),
        }
    }

    ClientImportPlan { items, skipped_servers }
}

fn import_config_from_inspected_entry(
    entry: InspectedServerEntry,
    client_identifier: &str,
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
            source: Some(ServerSource::new(
                ServerSourceType::Local,
                Some(client_identifier.to_string()),
            )),
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

    Ok(build_import_plan_from_entries(entries, identifier))
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
        let candidate = prepare_import_candidate(&cfg)?;
        if let Some(reason) = import_conflict_reason(&existing, &name, &candidate, &opts) {
            if record_conflict(&mut outcome, &name, reason, opts.conflict_policy) {
                continue;
            }
        }

        // Normalize args/env once for both preview and apply.
        let (args_norm, env_norm) = normalize_args_env(
            cfg.args.clone().unwrap_or_default(),
            cfg.env.clone().unwrap_or_default(),
        );

        // Preview: report would-be imported without DB side-effects
        if opts.preview {
            outcome.imported.push(build_imported_server(
                name,
                &cfg,
                args_norm,
                env_norm,
                candidate.persisted_kind,
            ));
            continue;
        }

        // Apply: upsert server, args, env, headers
        let mut server = match candidate.server_type {
            ServerType::Stdio => Server::new_stdio(name.clone(), cfg.command.clone()),
            ServerType::Sse => Server::new_sse(name.clone(), cfg.url.clone()),
            ServerType::StreamableHttp => Server::new_streamable_http(name.clone(), cfg.url.clone()),
        };
        server.source = cfg.source.clone();
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
            candidate.persisted_kind,
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

impl ImportConflictIndex for ExistingIndex {
    fn names(&self) -> &HashSet<String> {
        &self.names
    }

    fn fingerprints(&self) -> &HashSet<String> {
        &self.fingerprints
    }

    fn url_bases(&self) -> &HashSet<String> {
        &self.url_bases
    }

    fn url_signatures(&self) -> &HashMap<String, fingerprint::UrlSignature> {
        &self.url_signatures
    }
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
        let plan = build_import_plan_from_entries(
            [
                server_entry("MCPMate", "stdio", Some("mcpmate-bridge"), None, None),
                server_entry(
                    "context7",
                    "streamable_http",
                    None,
                    Some("http://127.0.0.1:8123/mcp"),
                    None,
                ),
                server_entry("shadcn-mcp-server", "unclassified", None, None, None),
            ],
            "test-client",
        );

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
        let plan = build_import_plan_from_entries(
            [
                server_entry("broken", "unclassified", None, None, Some("config_invalid_entry")),
                server_entry("valid", "stdio", Some("uvx"), None, None),
            ],
            "test-client",
        );

        assert!(plan.items.contains_key("valid"));
        assert_eq!(plan.skipped_servers.len(), 1);
        assert_eq!(plan.skipped_servers[0].name, "broken");
        assert!(matches!(plan.skipped_servers[0].reason, SkipReason::ConfigInvalidEntry));
    }
}
