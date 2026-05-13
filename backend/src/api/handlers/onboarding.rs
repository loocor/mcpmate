use std::{collections::HashMap, process::Output, sync::Arc, time::Duration};

use axum::{Json, extract::State};
use schemars::JsonSchema;
use serde::Serialize;
use tokio::time::Instant;

use super::ApiError;
use super::client::parse_rule_from_api_data;
use crate::api::models::onboarding::{
    OnboardingCompleteReq, OnboardingServerCandidate, OnboardingServerScanData, OnboardingServerScanError,
    OnboardingServerScanReq, OnboardingServerScanResp, OnboardingStatusData, OnboardingStatusResp, RuntimeCheckData,
    RuntimeCheckResp, RuntimeEntry,
};
use crate::api::routes::AppState;
use crate::common::constants::database::tables;
use crate::common::server::ServerType;
use crate::config::server::import::build_import_plan_from_entries;
use crate::macros::resp::api_resp;

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Onboarding action result")]
pub struct OnboardingActionData {
    #[schemars(description = "Whether the action succeeded")]
    pub ok: bool,
}

api_resp!(OnboardingActionResp, OnboardingActionData, "Onboarding action response");

async fn set_onboarding_completed(
    state: Arc<AppState>,
    completed: bool,
) -> Result<Json<OnboardingActionResp>, ApiError> {
    let db = state
        .database
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Database not available".into()))?;

    let mut settings = crate::system::settings::get_settings(&db.pool)
        .await
        .map_err(|err| ApiError::InternalError(err.to_string()))?;

    let previous = settings.clone();
    settings.onboarding_completed = completed;

    crate::system::settings::apply_settings_with_effects(&db.pool, &previous, &settings, state.client_service.clone())
        .await
        .map_err(|err| ApiError::InternalError(err.to_string()))?;

    Ok(Json(OnboardingActionResp::success(OnboardingActionData { ok: true })))
}

fn candidate_fingerprint(config: &crate::api::models::server::ServersImportConfig) -> String {
    let Ok(st) = ServerType::from_client_format(config.kind.trim()) else {
        return String::new();
    };
    match st {
        ServerType::Stdio => crate::config::server::fingerprint::fingerprint_for_stdio(
            config.command.as_deref().unwrap_or_default(),
            config.args.as_deref().unwrap_or(&[]),
        ),
        ServerType::Sse | ServerType::StreamableHttp => {
            let base = crate::config::server::fingerprint::url_signature(config.url.as_deref().unwrap_or_default())
                .fingerprint;
            format!("{}|{}", base, st.client_format())
        }
    }
}

fn push_source(
    candidate: &mut OnboardingServerCandidate,
    client_id: &str,
    client_name: &str,
) {
    if !candidate.source_client_ids.iter().any(|value| value == client_id) {
        candidate.source_client_ids.push(client_id.to_string());
        candidate.source_clients.push(client_name.to_string());
    }
}

/// GET /api/onboarding/status
pub async fn get_status(State(state): State<Arc<AppState>>) -> Result<Json<OnboardingStatusResp>, ApiError> {
    let db = state
        .database
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Database not available".into()))?;

    let settings = crate::system::settings::get_settings(&db.pool)
        .await
        .map_err(|err| ApiError::InternalError(err.to_string()))?;

    let servers_count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {}", tables::SERVER_CONFIG))
        .fetch_one(&db.pool)
        .await
        .map_err(|err| ApiError::InternalError(err.to_string()))?;

    let clients_count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {}", tables::CLIENT))
        .fetch_one(&db.pool)
        .await
        .map_err(|err| ApiError::InternalError(err.to_string()))?;

    Ok(Json(OnboardingStatusResp::success(OnboardingStatusData {
        completed: settings.onboarding_completed,
        servers_count: servers_count as usize,
        clients_count: clients_count as usize,
    })))
}

/// POST /api/onboarding/complete
pub async fn complete(
    State(state): State<Arc<AppState>>,
    Json(request): Json<OnboardingCompleteReq>,
) -> Result<Json<OnboardingActionResp>, ApiError> {
    set_onboarding_completed(state, request.completed).await
}

/// POST /api/onboarding/reset
pub async fn reset(State(state): State<Arc<AppState>>) -> Result<Json<OnboardingActionResp>, ApiError> {
    set_onboarding_completed(state, false).await
}

/// POST /api/onboarding/server-scan
pub async fn server_scan(
    State(state): State<Arc<AppState>>,
    Json(request): Json<OnboardingServerScanReq>,
) -> Result<Json<OnboardingServerScanResp>, ApiError> {
    let service = state
        .client_service
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Client service unavailable".into()))?;
    let descriptors = service
        .list_clients(true, false)
        .await
        .map_err(|err| ApiError::InternalError(err.to_string()))?;
    let mut descriptor_by_id = descriptors
        .into_iter()
        .map(|descriptor| (descriptor.state.identifier().to_string(), descriptor))
        .collect::<HashMap<_, _>>();
    let mut candidates = HashMap::<String, OnboardingServerCandidate>::new();
    let mut by_name = HashMap::<String, String>::new();
    let mut by_fingerprint = HashMap::<String, String>::new();
    let mut errors = Vec::new();

    for client in request.clients {
        let Some(descriptor) = descriptor_by_id.remove(&client.identifier) else {
            errors.push(OnboardingServerScanError {
                client_name: client.identifier.clone(),
                message: "Client is not currently detected or registered".to_string(),
            });
            continue;
        };
        let client_name = descriptor.state.display_name().to_string();
        let scan_result = async {
            let request_config_path = client.config_path.trim();
            let config_path = if request_config_path.is_empty() {
                descriptor.config_path.as_deref()
            } else {
                Some(request_config_path)
            }
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| "Client has no detected local config path".to_string())?;
            let parse_rule_owned = client.config_file_parse.as_ref().map(parse_rule_from_api_data);
            let inspected = service
                .inspect_config_path_for_import(&descriptor.state, config_path, parse_rule_owned.as_ref())
                .await
                .map_err(|e| e.to_string())?;
            Ok(build_import_plan_from_entries(inspected.inspection.entries))
        }
        .await;

        let plan = match scan_result {
            Ok(plan) => plan,
            Err(message) => {
                errors.push(OnboardingServerScanError { client_name, message });
                continue;
            }
        };

        for (name, config) in plan.items {
            let normalized_name = name.trim().to_ascii_lowercase();
            if normalized_name.is_empty() {
                continue;
            }
            let name_key = format!("name:{normalized_name}");
            let fingerprint = candidate_fingerprint(&config);
            let fingerprint_key = if fingerprint.is_empty() {
                None
            } else {
                Some(format!("fingerprint:{fingerprint}"))
            };
            let existing_key = by_name.get(&name_key).cloned().or_else(|| {
                fingerprint_key
                    .as_ref()
                    .and_then(|fingerprint_key| by_fingerprint.get(fingerprint_key).cloned())
            });

            if let Some(existing_key) = existing_key {
                if let Some(candidate) = candidates.get_mut(&existing_key) {
                    push_source(candidate, &client.identifier, &client_name);
                }
                continue;
            }

            let key = fingerprint_key.clone().unwrap_or_else(|| name_key.clone());
            let kind = match ServerType::from_client_format(config.kind.trim()) {
                Ok(st) => st.client_format().to_string(),
                Err(_) => continue,
            };
            let candidate = OnboardingServerCandidate {
                key: key.clone(),
                name,
                kind,
                command: config.command,
                args: config.args.unwrap_or_default(),
                env: config.env.unwrap_or_default(),
                url: config.url,
                source_clients: vec![client_name.clone()],
                source_client_ids: vec![client.identifier.clone()],
            };
            by_name.insert(name_key, key.clone());
            if let Some(fingerprint_key) = fingerprint_key {
                by_fingerprint.insert(fingerprint_key, key.clone());
            }
            candidates.insert(key, candidate);
        }
    }

    let mut candidates = candidates.into_values().collect::<Vec<_>>();
    candidates.sort_by_key(|candidate| candidate.name.to_ascii_lowercase());

    Ok(Json(OnboardingServerScanResp::success(OnboardingServerScanData {
        candidates,
        errors,
    })))
}

fn runtime_locator_command() -> &'static str {
    if cfg!(windows) { "where" } else { "which" }
}

#[derive(Clone, Copy)]
struct RuntimeCheckProbe<'a> {
    program: &'a str,
    args: &'a [&'a str],
}

const NODE_RUNTIME_PROBES: &[RuntimeCheckProbe<'_>] = &[RuntimeCheckProbe {
    program: "node",
    args: &["--version"],
}];
const NPX_RUNTIME_PROBES: &[RuntimeCheckProbe<'_>] = &[RuntimeCheckProbe {
    program: "npx",
    args: &["--version"],
}];
const BUN_RUNTIME_PROBES: &[RuntimeCheckProbe<'_>] = &[RuntimeCheckProbe {
    program: "bun",
    args: &["--version"],
}];
const BUNX_RUNTIME_PROBES: &[RuntimeCheckProbe<'_>] = &[RuntimeCheckProbe {
    program: "bunx",
    args: &["--version"],
}];
#[cfg(windows)]
const PYTHON_RUNTIME_PROBES: &[RuntimeCheckProbe<'_>] = &[
    RuntimeCheckProbe {
        program: "python",
        args: &["--version"],
    },
    RuntimeCheckProbe {
        program: "py",
        args: &["-3", "--version"],
    },
];
#[cfg(not(windows))]
const PYTHON_RUNTIME_PROBES: &[RuntimeCheckProbe<'_>] = &[RuntimeCheckProbe {
    program: "python3",
    args: &["--version"],
}];
const UV_RUNTIME_PROBES: &[RuntimeCheckProbe<'_>] = &[RuntimeCheckProbe {
    program: "uv",
    args: &["--version"],
}];
const UVX_RUNTIME_PROBES: &[RuntimeCheckProbe<'_>] = &[RuntimeCheckProbe {
    program: "uvx",
    args: &["--version"],
}];

const RUNTIME_CHECK_COMMAND_TIMEOUT: Duration = Duration::from_secs(5);
const RUNTIME_CHECK_TOTAL_TIMEOUT: Duration = Duration::from_secs(20);

fn remaining_runtime_check_budget(started_at: Instant) -> Option<Duration> {
    RUNTIME_CHECK_TOTAL_TIMEOUT.checked_sub(started_at.elapsed())
}

async fn run_command_with_timeout(
    program: &str,
    args: &[&str],
    timeout: Duration,
) -> Option<Output> {
    if timeout.is_zero() {
        return None;
    }

    let mut command = tokio::process::Command::new(program);
    command.kill_on_drop(true).args(args);

    tokio::time::timeout(timeout, command.output()).await.ok()?.ok()
}

fn resolve_runtime_path(stdout: &[u8]) -> Option<String> {
    String::from_utf8_lossy(stdout)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_string)
}

async fn run_runtime_check_command(
    program: &str,
    args: &[&str],
    started_at: Instant,
) -> Option<Output> {
    let remaining = remaining_runtime_check_budget(started_at)?;
    run_command_with_timeout(program, args, remaining.min(RUNTIME_CHECK_COMMAND_TIMEOUT)).await
}

fn normalize_runtime_version(
    stdout: &[u8],
    stderr: &[u8],
) -> Option<String> {
    let stdout = String::from_utf8_lossy(stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(stderr).trim().to_string();
    let raw = if stdout.is_empty() { stderr } else { stdout };
    let trimmed = raw
        .split_once('(')
        .map(|(head, _)| head.trim().to_string())
        .unwrap_or(raw);
    if trimmed.is_empty() { None } else { Some(trimmed) }
}

/// GET /api/onboarding/runtime-check
/// Detects available runtimes (node, bun, python3, uv, etc.) on the host system.
pub async fn runtime_check(State(_state): State<Arc<AppState>>) -> Result<Json<RuntimeCheckResp>, ApiError> {
    let checks: &[(&str, &[RuntimeCheckProbe<'_>])] = &[
        ("node", NODE_RUNTIME_PROBES),
        ("npx", NPX_RUNTIME_PROBES),
        ("bun", BUN_RUNTIME_PROBES),
        ("bunx", BUNX_RUNTIME_PROBES),
        ("python3", PYTHON_RUNTIME_PROBES),
        ("uv", UV_RUNTIME_PROBES),
        ("uvx", UVX_RUNTIME_PROBES),
    ];

    let mut runtimes = Vec::with_capacity(checks.len());
    let mut has_js = false;
    let mut has_python = false;
    let started_at = Instant::now();
    let locator = runtime_locator_command();

    for &(name, probes) in checks {
        let mut available = false;
        let mut version = None;
        let mut resolved_program = None;

        for probe in probes {
            let result = run_runtime_check_command(probe.program, probe.args, started_at).await;
            if let Some(output) = result.filter(|output| output.status.success()) {
                available = true;
                version = normalize_runtime_version(&output.stdout, &output.stderr);
                resolved_program = Some(probe.program);
                break;
            }
        }

        let path = if let Some(program) = resolved_program {
            run_runtime_check_command(locator, &[program], started_at)
                .await
                .filter(|output| output.status.success())
                .and_then(|output| resolve_runtime_path(&output.stdout))
        } else {
            None
        };

        if available {
            match name {
                "node" | "bun" | "npx" | "bunx" => has_js = true,
                "python3" | "uv" | "uvx" => has_python = true,
                _ => {}
            }
        }

        runtimes.push(RuntimeEntry {
            name: name.to_string(),
            available,
            version,
            path,
        });
    }

    Ok(Json(RuntimeCheckResp::success(RuntimeCheckData {
        runtimes,
        has_js_runtime: has_js,
        has_python_runtime: has_python,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::models::client::{ClientConfigFileParseData, ClientConfigType};
    use crate::api::models::onboarding::{OnboardingCompleteReq, OnboardingServerScanClient, OnboardingServerScanReq};
    use crate::api::routes::AppState;
    use crate::clients::{
        ClientConfigService,
        service::settings::ActiveClientSettingsUpdate,
        source::{ClientConfigSource, DbTemplateSource, FileTemplateSource, TemplateRoot},
    };
    use crate::common::constants::database::tables;
    use crate::config::{
        client::init::{initialize_client_table, initialize_system_settings},
        database::Database,
        profile::init::initialize_profile_tables,
        server::init::initialize_server_tables,
    };
    use crate::core::{
        cache::{RedbCacheManager, manager::CacheConfig},
        models::Config,
        pool::UpstreamConnectionPool,
        profile::ConfigApplicationStateManager,
    };
    use crate::inspector::{calls::InspectorCallRegistry, sessions::InspectorSessionManager};
    use crate::system::metrics::MetricsCollector;
    use sqlx::sqlite::SqlitePoolOptions;
    use std::{collections::HashSet, path::PathBuf, sync::Arc, time::Duration};
    use tempfile::TempDir;
    use tokio::sync::Mutex;

    struct TestContext {
        _temp_dir: TempDir,
        state: Arc<AppState>,
        pool: sqlx::SqlitePool,
    }

    async fn create_test_context() -> TestContext {
        let temp_dir = TempDir::new().expect("temp dir");
        let db_pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("sqlite pool");

        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&db_pool)
            .await
            .expect("enable foreign keys");

        initialize_server_tables(&db_pool).await.expect("init server tables");
        initialize_profile_tables(&db_pool).await.expect("init profile tables");
        initialize_client_table(&db_pool).await.expect("init client table");
        initialize_system_settings(&db_pool)
            .await
            .expect("init system settings");

        let database = Arc::new(Database {
            pool: db_pool.clone(),
            path: PathBuf::from(":memory:"),
        });

        let template_root = TemplateRoot::new(temp_dir.path().join("client-templates"));
        let template_source = Arc::new(
            FileTemplateSource::bootstrap(template_root)
                .await
                .expect("template source"),
        );
        ClientConfigService::seed_runtime_template_snapshots(&db_pool, template_source.as_ref())
            .await
            .expect("seed runtime templates");
        ClientConfigService::seed_client_runtime_rows(&db_pool, template_source.as_ref())
            .await
            .expect("seed runtime rows");

        let runtime_source: Arc<dyn ClientConfigSource> =
            Arc::new(DbTemplateSource::new(Arc::new(db_pool.clone())).expect("runtime source"));
        let client_service = Arc::new(
            ClientConfigService::with_source(Arc::new(db_pool.clone()), runtime_source)
                .await
                .expect("client service"),
        );

        let cache_path = temp_dir.path().join("capability.redb");
        let redb_cache = Arc::new(RedbCacheManager::new(cache_path, CacheConfig::default()).expect("cache manager"));

        let state = Arc::new(AppState {
            connection_pool: Arc::new(Mutex::new(UpstreamConnectionPool::new(
                Arc::new(Config::default()),
                Some(database.clone()),
            ))),
            metrics_collector: Arc::new(MetricsCollector::new(Duration::from_secs(5))),
            http_proxy: None,
            profile_merge_service: None,
            database: Some(database),
            audit_database: None,
            audit_service: None,
            config_application_state: Arc::new(ConfigApplicationStateManager::new()),
            redb_cache,
            unified_query: None,
            client_service: Some(client_service),
            inspector_calls: Arc::new(InspectorCallRegistry::new()),
            inspector_sessions: Arc::new(InspectorSessionManager::new()),
            oauth_manager: Some(Arc::new(crate::core::oauth::OAuthManager::new(db_pool.clone()))),
        });

        TestContext {
            _temp_dir: temp_dir,
            state,
            pool: db_pool,
        }
    }

    #[tokio::test]
    async fn status_complete_and_reset_update_onboarding_state() {
        let context = create_test_context().await;

        sqlx::query(&format!(
            "INSERT INTO {} (id, identifier, name, display_name) VALUES (?, ?, ?, ?)",
            tables::CLIENT
        ))
        .bind("client-1")
        .bind("client-1")
        .bind("Client One")
        .bind("Client One")
        .execute(&context.pool)
        .await
        .expect("insert client");

        sqlx::query(&format!(
            "INSERT INTO {} (id, name, server_type, command) VALUES (?, ?, ?, ?)",
            tables::SERVER_CONFIG
        ))
        .bind("server-1")
        .bind("Server One")
        .bind("stdio")
        .bind("echo")
        .execute(&context.pool)
        .await
        .expect("insert server");

        let Json(initial_status) = get_status(State(context.state.clone()))
            .await
            .expect("get initial status");
        assert!(initial_status.success);
        let initial_data = initial_status.data.expect("initial status data");
        assert!(!initial_data.completed);
        assert_eq!(initial_data.clients_count, 1);
        assert_eq!(initial_data.servers_count, 1);

        let Json(complete_resp) = complete(
            State(context.state.clone()),
            Json(OnboardingCompleteReq { completed: true }),
        )
        .await
        .expect("complete onboarding");
        assert!(complete_resp.success);
        assert!(complete_resp.data.expect("complete data").ok);

        let Json(completed_status) = get_status(State(context.state.clone()))
            .await
            .expect("get completed status");
        assert!(completed_status.data.expect("completed status data").completed);

        let Json(reset_resp) = reset(State(context.state.clone())).await.expect("reset onboarding");
        assert!(reset_resp.success);
        assert!(reset_resp.data.expect("reset data").ok);

        let Json(reset_status) = get_status(State(context.state)).await.expect("get reset status");
        assert!(!reset_status.data.expect("reset status data").completed);
    }

    #[tokio::test]
    async fn server_scan_returns_error_for_unknown_client_identifier() {
        let context = create_test_context().await;

        let Json(response) = server_scan(
            State(context.state),
            Json(OnboardingServerScanReq {
                clients: vec![OnboardingServerScanClient {
                    identifier: "missing-client".to_string(),
                    display_name: Some("Missing Client".to_string()),
                    config_path: "/tmp/missing.json".to_string(),
                    config_file_parse: None,
                }],
            }),
        )
        .await
        .expect("server scan response");

        assert!(response.success);
        let data = response.data.expect("server scan data");
        assert!(data.candidates.is_empty());
        assert_eq!(data.errors.len(), 1);
        assert_eq!(data.errors[0].client_name, "missing-client");
    }

    #[tokio::test]
    async fn server_scan_uses_requested_parse_rule_for_selected_client() {
        let context = create_test_context().await;
        let config_path = context._temp_dir.path().join("custom-client.json");
        tokio::fs::write(
            &config_path,
            r#"{"context_servers":{"Server A":{"command":"node","args":["server.js"],"env":{"A":"B"}}}}"#,
        )
        .await
        .expect("write client config");

        let service = context.state.client_service.as_ref().expect("client service");
        service
            .set_active_client_settings(
                "custom.client",
                ActiveClientSettingsUpdate {
                    display_name: Some("Custom Client".to_string()),
                    connection_mode: Some("local_config_detected".to_string()),
                    config_path: Some(config_path.to_string_lossy().to_string()),
                    clear_config_file_parse: true,
                    ..ActiveClientSettingsUpdate::default()
                },
            )
            .await
            .expect("create custom client");

        let Json(response) = server_scan(
            State(context.state),
            Json(OnboardingServerScanReq {
                clients: vec![OnboardingServerScanClient {
                    identifier: "custom.client".to_string(),
                    display_name: Some("Custom Client".to_string()),
                    config_path: config_path.to_string_lossy().to_string(),
                    config_file_parse: Some(ClientConfigFileParseData {
                        format: "json".to_string(),
                        container_type: ClientConfigType::Standard,
                        container_keys: vec!["context_servers".to_string()],
                    }),
                }],
            }),
        )
        .await
        .expect("server scan response");

        let data = response.data.expect("server scan data");
        assert!(data.errors.is_empty());
        assert_eq!(data.candidates.len(), 1);
        assert_eq!(data.candidates[0].name, "Server A");
        assert_eq!(data.candidates[0].command.as_deref(), Some("node"));
    }

    #[tokio::test]
    async fn server_scan_uses_wildcard_parse_rule_for_project_scoped_servers() {
        let context = create_test_context().await;
        let config_path = context._temp_dir.path().join("claude-code.json");
        tokio::fs::write(
            &config_path,
            r#"{"projects":{"/Volumes/External/GitHub/MCPMate":{"mcpServers":{"Project Server":{"command":"node","args":["server.js"],"env":{"A":"B"}}}}}}"#,
        )
        .await
        .expect("write client config");

        let service = context.state.client_service.as_ref().expect("client service");
        service
            .set_active_client_settings(
                "custom.client",
                ActiveClientSettingsUpdate {
                    display_name: Some("Custom Client".to_string()),
                    connection_mode: Some("local_config_detected".to_string()),
                    config_path: Some(config_path.to_string_lossy().to_string()),
                    clear_config_file_parse: true,
                    ..ActiveClientSettingsUpdate::default()
                },
            )
            .await
            .expect("create custom client");

        let Json(response) = server_scan(
            State(context.state),
            Json(OnboardingServerScanReq {
                clients: vec![OnboardingServerScanClient {
                    identifier: "custom.client".to_string(),
                    display_name: Some("Custom Client".to_string()),
                    config_path: config_path.to_string_lossy().to_string(),
                    config_file_parse: Some(ClientConfigFileParseData {
                        format: "json".to_string(),
                        container_type: ClientConfigType::Standard,
                        container_keys: vec!["projects.*.mcpServers".to_string()],
                    }),
                }],
            }),
        )
        .await
        .expect("server scan response");

        let data = response.data.expect("server scan data");
        assert!(data.errors.is_empty());
        assert_eq!(data.candidates.len(), 1);
        assert_eq!(data.candidates[0].name, "Project Server");
        assert_eq!(data.candidates[0].command.as_deref(), Some("node"));
    }

    #[tokio::test]
    async fn runtime_check_returns_expected_runtime_matrix_shape() {
        let context = create_test_context().await;

        let Json(response) = runtime_check(State(context.state))
            .await
            .expect("runtime check response");

        assert!(response.success);
        let data = response.data.expect("runtime check data");

        let names = data
            .runtimes
            .iter()
            .map(|entry| entry.name.as_str())
            .collect::<HashSet<_>>();
        let expected = HashSet::from(["node", "npx", "bun", "bunx", "python3", "uv", "uvx"]);
        assert_eq!(names, expected);

        let has_js_from_rows = data
            .runtimes
            .iter()
            .any(|entry| entry.available && matches!(entry.name.as_str(), "node" | "npx" | "bun" | "bunx"));
        let has_python_from_rows = data
            .runtimes
            .iter()
            .any(|entry| entry.available && matches!(entry.name.as_str(), "python3" | "uv" | "uvx"));

        assert_eq!(data.has_js_runtime, has_js_from_rows);
        assert_eq!(data.has_python_runtime, has_python_from_rows);
    }

    #[test]
    fn resolve_runtime_path_uses_first_non_empty_line() {
        assert_eq!(
            resolve_runtime_path(b"\nC:/Python312/python.exe\r\nC:/Windows/py.exe\r\n").as_deref(),
            Some("C:/Python312/python.exe")
        );
    }

    #[test]
    fn resolve_runtime_path_returns_none_for_blank_output() {
        assert!(resolve_runtime_path(b"\n  \r\n").is_none());
    }

    #[cfg(windows)]
    #[test]
    fn python_runtime_probes_cover_windows_launchers() {
        let programs = PYTHON_RUNTIME_PROBES
            .iter()
            .map(|probe| probe.program)
            .collect::<HashSet<_>>();

        assert!(programs.contains("python"));
        assert!(programs.contains("py"));
    }

    #[cfg(not(windows))]
    #[test]
    fn python_runtime_probes_use_python3_off_windows() {
        assert_eq!(PYTHON_RUNTIME_PROBES.len(), 1);
        assert_eq!(PYTHON_RUNTIME_PROBES[0].program, "python3");
        assert_eq!(PYTHON_RUNTIME_PROBES[0].args, ["--version"]);
    }

    #[tokio::test]
    async fn runtime_check_command_times_out_for_long_running_process() {
        #[cfg(unix)]
        let result = run_command_with_timeout("sh", &["-c", "sleep 1"], Duration::from_millis(50)).await;

        #[cfg(windows)]
        let result =
            run_command_with_timeout("cmd", &["/C", "ping -n 2 127.0.0.1 >NUL"], Duration::from_millis(50)).await;

        assert!(result.is_none());
    }

    #[test]
    fn normalize_runtime_version_extracts_stdout_first() {
        assert_eq!(
            normalize_runtime_version(b"v22.12.0\n", b"").as_deref(),
            Some("v22.12.0")
        );
    }

    #[test]
    fn normalize_runtime_version_falls_back_to_stderr() {
        assert_eq!(
            normalize_runtime_version(b"", b"v22.12.0\n").as_deref(),
            Some("v22.12.0")
        );
    }

    #[test]
    fn normalize_runtime_version_strips_trailing_explanation() {
        assert_eq!(
            normalize_runtime_version(b"v22.12.0 (Some extra info)\n", b"").as_deref(),
            Some("v22.12.0")
        );
    }

    #[test]
    fn normalize_runtime_version_returns_none_for_blank_output() {
        assert!(normalize_runtime_version(b"\n", b"  \n").is_none());
    }
}
