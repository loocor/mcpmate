use std::{sync::Arc, time::Duration};

use anyhow::{Context, Error, Result};
use mcpmate::{
    common::{MCPMatePaths, global_paths, set_global_paths},
    core::{
        foundation::monitor,
        proxy::{
            Args, ProxyServer,
            init::{setup_database, setup_logging, setup_proxy_server_with_params},
            startup::{start_api_server, start_background_connections, start_proxy_server},
        },
    },
    system::config::init_port_config,
};
use tauri::{
    Manager, RunEvent,
    utils::config::WindowConfig,
    webview::{NewWindowResponse, WebviewWindowBuilder},
};
use tauri_plugin_shell::ShellExt;
use tokio::{sync::Mutex as AsyncMutex, task::JoinHandle, time::timeout};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

const SHUTDOWN_TIMEOUT_SECS: u64 = 5;

#[derive(Clone, Default)]
struct BackendState {
    inner: Arc<AsyncMutex<Option<BackendRuntime>>>,
}

impl BackendState {
    async fn set(
        &self,
        runtime: BackendRuntime,
    ) {
        let mut guard = self.inner.lock().await;
        *guard = Some(runtime);
    }

    async fn take(&self) -> Option<BackendRuntime> {
        let mut guard = self.inner.lock().await;
        guard.take()
    }
}

struct BackendRuntime {
    proxy: ProxyServer,
    api_task: JoinHandle<()>,
    api_cancel: CancellationToken,
    mcp_handle: Option<JoinHandle<Result<(), anyhow::Error>>>,
}

impl BackendRuntime {
    async fn shutdown(mut self) {
        info!("Shutting down MCPMate backend tasks from Tauri");

        if let Err(err) = self.proxy.initiate_shutdown().await {
            warn!(error = %err, "Failed to initiate proxy shutdown");
        }

        if let Some(handle) = self.mcp_handle.take() {
            match timeout(Duration::from_secs(SHUTDOWN_TIMEOUT_SECS), handle).await {
                Ok(Ok(Ok(()))) => info!("MCP server shutdown completed"),
                Ok(Ok(Err(err))) => warn!(error = %err, "MCP server reported error while shutting down"),
                Ok(Err(err)) => warn!(error = %err, "MCP server task join error"),
                Err(_) => warn!("Timed out waiting for MCP server shutdown"),
            }
        }

        self.api_cancel.cancel();
        match timeout(Duration::from_secs(SHUTDOWN_TIMEOUT_SECS), &mut self.api_task).await {
            Ok(Ok(())) => info!("API server shutdown completed"),
            Ok(Err(err)) => warn!(error = %err, "API server task join error"),
            Err(_) => warn!("Timed out waiting for API server shutdown"),
        }

        if let Err(err) = self.proxy.complete_shutdown().await {
            warn!(error = %err, "Failed to complete proxy shutdown");
        }

        info!("Backend shutdown sequence finished");
    }
}

pub fn run() -> Result<()> {
    let backend_state = BackendState::default();

    let mut builder = tauri::Builder::default();

    builder = builder.manage(backend_state.clone());

    builder
        .plugin(tauri_plugin_shell::init())
        .setup(move |app| {
            initialize_paths(app)?;
            configure_tauri_environment();
            bootstrap_backend(app, backend_state.clone())?;
            spawn_main_window(app)?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .map_err(Error::new)?
        .run(move |app_handle, event| {
            if let RunEvent::Exit = event {
                if let Some(state) = app_handle.try_state::<BackendState>() {
                    if let Some(runtime) = tauri::async_runtime::block_on(state.take()) {
                        tauri::async_runtime::block_on(runtime.shutdown());
                    }
                }
            }
        });

    Ok(())
}

fn configure_tauri_environment() {
    const SKIP_BOARD_STATIC: &str = "MCPMATE_SKIP_BOARD_STATIC";

    if std::env::var_os(SKIP_BOARD_STATIC).is_none() {
        unsafe {
            std::env::set_var(SKIP_BOARD_STATIC, "1");
        }
    }
}

fn spawn_main_window(app: &mut tauri::App) -> Result<()> {
    if app.get_webview_window("main").is_some() {
        return Ok(());
    }

    let window_config = app
        .config()
        .app
        .windows
        .iter()
        .find(|cfg| cfg.label == "main")
        .cloned()
        .unwrap_or_else(default_main_window_config);

    let app_handle = app.handle().clone();

    let builder = WebviewWindowBuilder::from_config(app, &window_config)?.on_new_window(move |url, _features| {
        let scheme = url.scheme();
        if matches!(scheme, "http" | "https") {
            if let Err(err) = app_handle.shell().open(url.as_str().to_string(), None) {
                warn!(
                    error = %err,
                    target_url = %url,
                    "Failed to open external link from webview"
                );
            }
        } else {
            warn!(target_url = %url, "Blocked unsupported window.open URL scheme");
        }

        NewWindowResponse::Deny
    });

    builder.build()?;

    Ok(())
}

fn default_main_window_config() -> WindowConfig {
    let mut conf = WindowConfig::default();
    conf.label = "main".into();
    conf.title = "MCPMate".into();
    conf.width = 1280.0;
    conf.height = 800.0;
    conf.resizable = true;
    conf.create = false;
    conf
}

fn initialize_paths(app: &mut tauri::App) -> Result<()> {
    let app_handle = app.handle();

    let selected_paths = match try_use_default_paths() {
        Ok(paths) => paths,
        Err(err) => {
            warn!(error = %err, "Falling back to Tauri app data directory for MCPMate storage");
            use_app_data_paths(&app_handle)?
        }
    };

    unsafe {
        std::env::set_var("MCPMATE_DATA_DIR", selected_paths.base_dir());
    }

    if let Err(err) = set_global_paths(selected_paths.clone()) {
        let existing = global_paths();
        if existing.base_dir() != selected_paths.base_dir() {
            return Err(err.context("global MCPMate paths already initialized with a different base directory"));
        }
    }

    selected_paths.ensure_directories()?;

    info!("Using MCPMate data directory: {}", selected_paths.base_dir().display());

    Ok(())
}

fn try_use_default_paths() -> Result<MCPMatePaths> {
    let paths = MCPMatePaths::new()?;
    if let Err(err) = paths.ensure_directories() {
        Err(err.context("failed to prepare default MCPMate directories"))
    } else {
        Ok(paths)
    }
}

fn use_app_data_paths(app_handle: &tauri::AppHandle) -> Result<MCPMatePaths> {
    let data_dir = app_handle
        .path()
        .app_data_dir()
        .context("failed to determine Tauri app data directory")?;

    std::fs::create_dir_all(&data_dir)
        .with_context(|| format!("failed to create app data directory at {}", data_dir.display()))?;

    let paths = MCPMatePaths::from_base_dir(&data_dir)?;
    paths
        .ensure_directories()
        .with_context(|| format!("failed to initialize MCPMate directories under {}", data_dir.display()))?;
    Ok(paths)
}

fn bootstrap_backend(
    _app: &mut tauri::App,
    state: BackendState,
) -> Result<()> {
    let args = resolve_args();
    args.validate().map_err(Error::msg)?;

    tauri::async_runtime::spawn(async move {
        match start_backend(args).await {
            Ok(runtime) => {
                info!("MCPMate backend started successfully for Tauri shell");
                state.set(runtime).await;
            }
            Err(err) => {
                error!(error = %err, "Failed to start MCPMate backend inside Tauri");
            }
        }
    });

    Ok(())
}

async fn start_backend(args: Args) -> Result<BackendRuntime> {
    init_port_config(args.api_port, args.mcp_port);
    setup_logging(&args)?;
    monitor::initialize_metrics_reporting();

    let startup_mode = args.get_startup_mode();
    let db = setup_database().await?;
    let (proxy_arc, proxy_arc_for_api) = setup_proxy_server_with_params(db, &startup_mode).await?;

    start_background_connections(&proxy_arc, proxy_arc_for_api.clone()).await?;

    let mut proxy_clone = (*proxy_arc).clone();
    let mcp_handle = start_proxy_server(&mut proxy_clone, &args).await?;
    let (api_task, api_cancel) = start_api_server(proxy_arc_for_api.clone(), &args).await?;

    Ok(BackendRuntime {
        proxy: proxy_clone,
        api_task,
        api_cancel,
        mcp_handle,
    })
}

fn resolve_args() -> Args {
    use mcpmate::common::constants::ports;

    let api_port = read_port_env("MCPMATE_TAURI_API_PORT").unwrap_or(ports::API_PORT);
    let mcp_port = read_port_env("MCPMATE_TAURI_MCP_PORT").unwrap_or(ports::MCP_PORT);

    let log_level = clean_env_string("MCPMATE_TAURI_LOG").unwrap_or_else(|| "info".to_string());

    let transport = clean_env_string("MCPMATE_TAURI_TRANSPORT").unwrap_or_else(|| "uni".to_string());

    let profile = clean_env_string("MCPMATE_TAURI_PROFILE").and_then(|raw| {
        let profiles: Vec<String> = raw
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if profiles.is_empty() { None } else { Some(profiles) }
    });

    let minimal = std::env::var("MCPMATE_TAURI_MINIMAL")
        .map(|v| matches_ignore_case(&v, ["1", "true", "yes"]))
        .unwrap_or(false);

    Args {
        mcp_port,
        api_port,
        log_level,
        transport,
        profile,
        minimal,
    }
}

fn read_port_env(name: &str) -> Option<u16> {
    std::env::var(name)
        .ok()
        .and_then(|raw| raw.trim().parse::<u16>().ok())
        .filter(|port| *port != 0)
}

fn matches_ignore_case(
    value: &str,
    accepted: [&str; 3],
) -> bool {
    accepted.iter().any(|candidate| value.eq_ignore_ascii_case(candidate))
}

fn clean_env_string(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}
