use std::{
    fs::OpenOptions,
    io::Write,
    process::{Child, Command, Stdio},
    sync::Arc,
};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use anyhow::{Context, Error, Result};
use mcpmate::common::{MCPMatePaths, global_paths, set_global_paths};
use serde_json::json;
use tauri::{
    Emitter, Manager, RunEvent, WindowEvent, Wry,
    menu::{
        HELP_SUBMENU_ID, Menu, MenuBuilder, MenuItem, MenuItemKind, PredefinedMenuItem, Submenu,
    },
    tray::TrayIconBuilder,
    utils::config::WindowConfig,
    webview::{NewWindowResponse, WebviewWindowBuilder},
};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};

mod account;
mod audit;
mod core_service;
mod deep_link;
mod oauth_callback_access;
mod runtime_env;
mod runtime_ports;
mod shell;
mod source_config;
use core_service::{
    LocalCoreServiceDiagnosticsView, LocalCoreServiceStatusView, install_local_service,
    read_local_service_status, resolve_local_core_binary, resolve_local_core_working_dir,
    restart_local_service, start_local_service, stop_local_service,
    sync_local_service_definition, uninstall_local_service,
};
use deep_link::ImportServerDeepLinkPayload;
use oauth_callback_access::OAuthCallbackAccessState;
use shell::{ShellPreferences, ShellState};
use source_config::{DesktopCoreSourceConfig, DesktopCoreSourceKind, LocalCoreRuntimeMode};
use tauri_plugin_deep_link::DeepLinkExt;
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_updater::Builder as UpdaterPluginBuilder;
use tauri_plugin_updater::UpdaterExt;
use tokio::sync::Mutex as AsyncMutex;
use tokio::time::{Duration, sleep};
use tracing::{info, warn};
use uuid::Uuid;

const MENU_CHECK_UPDATES_ID: &str = "menu.help.check_for_updates";
const MENU_ABOUT_ID: &str = "menu.help.about";

#[derive(Clone, Default)]
pub(crate) struct DeepLinkState {
    pending_server_import: Arc<AsyncMutex<Option<ImportServerDeepLinkPayload>>>,
}

impl DeepLinkState {
    async fn set_pending_server_import(&self, payload: ImportServerDeepLinkPayload) {
        let mut guard = self.pending_server_import.lock().await;
        *guard = Some(payload);
    }

    async fn take_pending_server_import(&self) -> Option<ImportServerDeepLinkPayload> {
        let mut guard = self.pending_server_import.lock().await;
        guard.take()
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ShellPreferencesPayload {
    #[serde(alias = "menu_bar_icon_mode")]
    menu_bar_icon_mode: shell::MenuBarIconMode,
    #[serde(alias = "show_dock_icon")]
    show_dock_icon: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ShellPreferencesView {
    menu_bar_icon_mode: shell::MenuBarIconMode,
    show_dock_icon: bool,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DesktopCoreSourcePayload {
    selected_source: DesktopCoreSourceKind,
    localhost_runtime_mode: LocalCoreRuntimeMode,
    localhost_api_port: u16,
    localhost_mcp_port: u16,
    remote_base_url: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopCoreSourceView {
    selected_source: DesktopCoreSourceKind,
    localhost_runtime_mode: LocalCoreRuntimeMode,
    localhost_api_port: u16,
    localhost_mcp_port: u16,
    remote_base_url: String,
    api_base_url: String,
    local_service: LocalCoreServiceStatusView,
    remote_available: bool,
}

struct DesktopManagedCoreProcess {
    child: Child,
    token: String,
}

#[derive(Clone, Default)]
struct DesktopManagedCoreState {
    inner: Arc<AsyncMutex<Option<DesktopManagedCoreProcess>>>,
}

impl DesktopManagedCoreState {
    async fn replace(&self, child: Child, token: String) {
        let mut guard = self.inner.lock().await;
        *guard = Some(DesktopManagedCoreProcess { child, token });
    }

    async fn take(&self, reason: &str) -> Option<Child> {
        let mut guard = self.inner.lock().await;
        if let Some(process) = guard.take() {
            append_desktop_managed_shell_log(&format!(
                "state:take reason={} token={}",
                reason, process.token
            ));
            Some(process.child)
        } else {
            append_desktop_managed_shell_log(&format!(
                "state:take reason={} token=missing",
                reason
            ));
            None
        }
    }

    async fn expected_token(&self) -> Option<String> {
        let mut guard = self.inner.lock().await;
        if let Some(process) = guard.as_mut() {
            match process.child.try_wait() {
                Ok(Some(status)) => {
                    let exit_info = status
                        .code()
                        .map(|code| format!("exit_code:{}", code))
                        .unwrap_or_else(|| "terminated_by_signal".to_string());
                    append_desktop_managed_shell_log(&format!(
                        "state:expected_token observed_exit status={} token={}",
                        exit_info, process.token
                    ));
                    *guard = None;
                    None
                }
                Ok(None) => Some(process.token.clone()),
                Err(_) => Some(process.token.clone()),
            }
        } else {
            None
        }
    }

    async fn is_spawned(&self) -> bool {
        let mut guard = self.inner.lock().await;
        if let Some(process) = guard.as_mut() {
            match process.child.try_wait() {
                Ok(Some(status)) => {
                    let exit_info = status
                        .code()
                        .map(|code| format!("exit_code:{}", code))
                        .unwrap_or_else(|| "terminated_by_signal".to_string());
                    append_desktop_managed_shell_log(&format!(
                        "state:is_spawned observed_exit status={} token={}",
                        exit_info, process.token
                    ));
                    *guard = None;
                    false
                }
                Ok(None) => true,
                Err(_) => true,
            }
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum LocalCoreServiceAction {
    Start,
    Restart,
    Stop,
    Status,
    Install,
    Uninstall,
}

impl From<ShellPreferences> for ShellPreferencesView {
    fn from(value: ShellPreferences) -> Self {
        Self {
            menu_bar_icon_mode: value.menu_bar_icon_mode,
            show_dock_icon: value.show_dock_icon,
        }
    }
}

impl DesktopCoreSourceView {
    fn from_config(
        config: &DesktopCoreSourceConfig,
        local_service: LocalCoreServiceStatusView,
    ) -> Self {
        let api_base_url = match config.selected_source {
            DesktopCoreSourceKind::Localhost => {
                format!("http://127.0.0.1:{}", config.localhost.api_port)
            }
            DesktopCoreSourceKind::Remote => config.remote.base_url.trim().to_string(),
        };

        Self {
            selected_source: config.selected_source,
            localhost_runtime_mode: config.localhost_runtime_mode,
            localhost_api_port: config.localhost.api_port,
            localhost_mcp_port: config.localhost.mcp_port,
            remote_base_url: config.remote.base_url.clone(),
            api_base_url,
            local_service,
            remote_available: false,
        }
    }
}

fn desktop_managed_startup_log_path(base_dir: &std::path::Path) -> std::path::PathBuf {
    base_dir.join("mcpmate-core-startup.log")
}

fn desktop_managed_shell_log_path() -> std::path::PathBuf {
    global_paths().logs_dir().join("desktop-managed-shell.log")
}

fn append_desktop_managed_shell_log(message: &str) {
    let log_path = desktop_managed_shell_log_path();
    if let Some(parent) = log_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_path) {
        let _ = writeln!(file, "{}", message);
    }
}

fn read_startup_log_tail(log_path: &std::path::Path) -> Option<String> {
    let content = std::fs::read_to_string(log_path).ok()?;
    let mut lines = content.lines().rev().take(60).collect::<Vec<_>>();
    lines.reverse();
    let tail = lines.join("\n").trim().to_string();
    (!tail.is_empty()).then_some(tail)
}

fn desktop_managed_diagnostics(
    base_dir: &std::path::Path,
    last_error: Option<String>,
) -> Option<LocalCoreServiceDiagnosticsView> {
    let log_path = desktop_managed_startup_log_path(base_dir);
    let startup_log_tail = read_startup_log_tail(&log_path);

    Some(LocalCoreServiceDiagnosticsView {
        startup_log_path: log_path.display().to_string(),
        startup_log_tail,
        last_error,
    })
}

async fn sync_shell_service_state(
    shell_state: &ShellState,
    status: &LocalCoreServiceStatusView,
) -> Result<()> {
    shell_state
        .update_service_status(status.is_active_for_menu(), &status.label)
        .await
}

fn emit_core_state_changed(app: &tauri::AppHandle, view: &DesktopCoreSourceView) {
    if let Err(err) = app.emit(shell::EVENT_CORE_STATE_CHANGED, view) {
        warn!(error = %err, "Failed to emit core-state-changed event");
    }
}

async fn read_core_state_view(
    config: &DesktopCoreSourceConfig,
    managed_state: &DesktopManagedCoreState,
) -> Result<DesktopCoreSourceView> {
    let local_service = match config.localhost_runtime_mode {
        LocalCoreRuntimeMode::Service => read_local_service_status(config).await?,
        LocalCoreRuntimeMode::DesktopManaged => {
            read_desktop_managed_status(managed_state, config).await?
        }
    };
    Ok(DesktopCoreSourceView::from_config(config, local_service))
}

async fn sync_and_emit_core_state(
    app: &tauri::AppHandle,
    shell_state: &ShellState,
    view: &DesktopCoreSourceView,
) -> Result<()> {
    sync_shell_service_state(shell_state, &view.local_service).await?;
    emit_core_state_changed(app, view);
    Ok(())
}

async fn stop_localhost_runtime(
    managed_state: &DesktopManagedCoreState,
    config: &DesktopCoreSourceConfig,
) {
    append_desktop_managed_shell_log(&format!(
        "stop:localhost_runtime requested mode={:?} api_port={} mcp_port={}",
        config.localhost_runtime_mode,
        config.localhost.api_port,
        config.localhost.mcp_port,
    ));
    match config.localhost_runtime_mode {
        LocalCoreRuntimeMode::DesktopManaged => {
            let _ = stop_desktop_managed_core(managed_state, config).await;
        }
        LocalCoreRuntimeMode::Service => {
            let _ = stop_local_service(config).await;
        }
    }
}

async fn wait_for_port_release(port: u16, label: &str, context: &str) {
    if let Err(err) = core_service::wait_for_port_available(port).await {
        warn!(error = %err, port, %label, %context, "Port did not become available");
    }
}

async fn wait_for_localhost_ports(config: &DesktopCoreSourceConfig, context: &str) {
    wait_for_port_release(config.localhost.api_port, "API", context).await;
    if config.localhost.mcp_port != config.localhost.api_port {
        wait_for_port_release(config.localhost.mcp_port, "MCP", context).await;
    }
}

async fn handle_localhost_source_transition(
    managed_state: &DesktopManagedCoreState,
    previous: &DesktopCoreSourceConfig,
    config: &DesktopCoreSourceConfig,
) {
    let runtime_mode_changed = previous.selected_source == DesktopCoreSourceKind::Localhost
        && config.selected_source == DesktopCoreSourceKind::Localhost
        && previous.localhost_runtime_mode != config.localhost_runtime_mode;

    if runtime_mode_changed {
        stop_localhost_runtime(managed_state, previous).await;
        wait_for_localhost_ports(config, "after stopping previous localhost core").await;
        return;
    }

    let leaving_desktop_managed = previous.selected_source == DesktopCoreSourceKind::Localhost
        && previous.localhost_runtime_mode == LocalCoreRuntimeMode::DesktopManaged
        && config.selected_source != DesktopCoreSourceKind::Localhost;

    if leaving_desktop_managed {
        let _ = stop_desktop_managed_core(managed_state, previous).await;
        wait_for_port_release(
            previous.localhost.api_port,
            "API",
            "after stopping desktop-managed core",
        )
        .await;
    }
}

pub fn run() -> Result<()> {
    let deep_link_state = DeepLinkState::default();
    let desktop_managed_core_state = DesktopManagedCoreState::default();

    let mut builder = tauri::Builder::default();

    builder = builder.manage(deep_link_state);
    builder = builder.manage(desktop_managed_core_state.clone());

    builder = builder.on_menu_event(|app_handle, event| {
        if event.id.as_ref() == MENU_CHECK_UPDATES_ID {
            let handle = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                let (title, message) = match handle.updater() {
                    Ok(updater) => match updater.check().await {
                        Ok(Some(update)) => (
                            "Update Available".to_string(),
                            format!(
                                "Version {} is ready. Auto-update will activate once CDN hosting is connected.",
                                update.version
                            ),
                        ),
                        Ok(None) => (
                            "Up To Date".to_string(),
                            "You are already running the latest MCPMate build.".to_string(),
                        ),
                        Err(err) => (
                            "Update Check Failed".to_string(),
                            format!("Unable to check for updates right now: {}", err),
                        ),
                    },
                    Err(err) => (
                        "Updater Unavailable".to_string(),
                        format!("The updater service is not ready yet. Infrastructure pending: {}", err),
                    ),
                };

                handle
                    .dialog()
                    .message(message)
                    .title(title)
                    .buttons(MessageDialogButtons::Ok)
                    .show(|_| {});
            });
        } else if event.id.as_ref() == MENU_ABOUT_ID {
            let handle = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(err) = shell::ensure_window_visibility(&handle) {
                    warn!(error = %err, "Failed to show main window for about navigation");
                }
                if let Err(err) = handle.emit(shell::EVENT_OPEN_SETTINGS, json!({ "tab": "about" })) {
                    warn!(error = %err, "Failed to emit open-settings event for about navigation");
                }
            });
        }
    });

    builder = builder.on_window_event(|window, event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            if let Err(err) = window.hide() {
                warn!(error = %err, "Failed to hide window on close request");
            }
        }
    });

    let updater_plugin = UpdaterPluginBuilder::new().build();

    let builder = builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(updater_plugin)
        .setup(move |app| {
            initialize_paths(app)?;
            configure_tauri_environment();
            initialize_menu(app)?;

            let data_paths = global_paths().clone();
            let shell_prefs = ShellPreferences::load(&data_paths)?;
            let prefs_path = ShellPreferences::path(&data_paths);
            let shell_state = ShellState::new(shell_prefs.clone(), prefs_path);
            shell::apply_activation_policy(app.handle(), &shell_prefs)?;
            app.manage(shell_state.clone());
            app.manage(OAuthCallbackAccessState::default());

            let open_main_item =
                MenuItem::with_id(app, shell::MENU_OPEN_MAIN, "Open MCPMate", true, None::<&str>)?;
            let service_status_item = MenuItem::with_id(
                app,
                shell::MENU_SERVICE_STATUS,
                "Local Core: Unknown",
                false,
                None::<&str>,
            )?;
            let start_service_item = MenuItem::with_id(
                app,
                shell::MENU_START_SERVICE,
                "Start Service",
                true,
                None::<&str>,
            )?;
            let restart_service_item = MenuItem::with_id(
                app,
                shell::MENU_RESTART_SERVICE,
                "Restart Service",
                true,
                None::<&str>,
            )?;
            let stop_service_item = MenuItem::with_id(
                app,
                shell::MENU_STOP_SERVICE,
                "Stop Service",
                true,
                None::<&str>,
            )?;
            let settings_item =
                MenuItem::with_id(app, shell::MENU_OPEN_SETTINGS, "Open Settings", true, None::<&str>)?;
            let about_item =
                MenuItem::with_id(app, shell::MENU_SHOW_ABOUT, "About MCPMate", true, None::<&str>)?;
            let quit_item =
                MenuItem::with_id(app, shell::MENU_QUIT, "Quit MCPMate", true, None::<&str>)?;

            let tray_menu = MenuBuilder::new(app)
                .item(&open_main_item)
                .item(&service_status_item)
                .item(&start_service_item)
                .item(&restart_service_item)
                .item(&stop_service_item)
                .separator()
                .item(&settings_item)
                .item(&about_item)
                .separator()
                .item(&quit_item)
                .build()?;

            let shell_state_for_tray = shell_state.clone();
            let managed_state_for_tray = desktop_managed_core_state.clone();

            let tray_icon_image = shell::tray_template_icon();

            let mut tray_builder = TrayIconBuilder::with_id(shell::TRAY_ID)
                .menu(&tray_menu)
                .icon(tray_icon_image)
                .on_menu_event(move |app_handle, event| {
                    let menu_id = event.id().as_ref();
                    match menu_id {
                        shell::MENU_OPEN_MAIN => {
                            let handle = app_handle.clone();
                            tauri::async_runtime::spawn(async move {
                                if let Err(err) = shell::ensure_window_visibility(&handle) {
                                    warn!(error = %err, "Failed to show main window from tray");
                                }
                            });
                        }
                        shell::MENU_OPEN_SETTINGS => {
                            let handle = app_handle.clone();
                            tauri::async_runtime::spawn(async move {
                                if let Err(err) = shell::ensure_window_visibility(&handle) {
                                    warn!(error = %err, "Failed to show main window for settings navigation");
                                }
                                if let Err(err) = handle.emit(shell::EVENT_OPEN_SETTINGS, json!({})) {
                                    warn!(error = %err, "Failed to emit open-settings event to frontend");
                                }
                            });
                        }
                        shell::MENU_SHOW_ABOUT => {
                            let handle = app_handle.clone();
                            tauri::async_runtime::spawn(async move {
                                if let Err(err) = shell::ensure_window_visibility(&handle) {
                                    warn!(error = %err, "Failed to show main window for about navigation");
                                }
                                if let Err(err) = handle.emit(shell::EVENT_OPEN_SETTINGS, json!({ "tab": "about" })) {
                                    warn!(error = %err, "Failed to emit open-settings event for about navigation");
                                }
                            });
                        }
                        shell::MENU_QUIT => {
                            app_handle.exit(0);
                        }
                        shell::MENU_START_SERVICE => {
                            let handle = app_handle.clone();
                            let shell_state = shell_state_for_tray.clone();
                            let managed_state = managed_state_for_tray.clone();
                            tauri::async_runtime::spawn(async move {
                                let config = match DesktopCoreSourceConfig::load(global_paths()) {
                                    Ok(config) => config,
                                    Err(err) => {
                                        warn!(error = %err, "Failed to load desktop core source config");
                                        return;
                                    }
                                };

                                match config.selected_source {
                                    DesktopCoreSourceKind::Localhost => {
									let result = match config.localhost_runtime_mode {
										LocalCoreRuntimeMode::Service => start_local_service(&handle, &config).await,
										LocalCoreRuntimeMode::DesktopManaged => start_desktop_managed_core(&handle, &managed_state, &config).await,
									};
									match result {
                                            Ok(status) => {
                                                if let Err(err) = sync_shell_service_state(&shell_state, &status).await {
                                                    warn!(error = %err, "Failed to sync tray state after starting service");
                                                }
										let view = DesktopCoreSourceView::from_config(&config, status.clone());
										emit_core_state_changed(&handle, &view);
                                                if let Err(err) = shell::ensure_window_visibility(&handle) {
                                                    warn!(error = %err, "Failed to reveal main window after starting service");
                                                }
                                            }
                                            Err(err) => {
                                                warn!(error = %err, "Failed to start localhost core service");
                                            }
                                        }
                                    }
                                    DesktopCoreSourceKind::Remote => {
                                        if let Err(err) = shell::ensure_window_visibility(&handle) {
                                            warn!(error = %err, "Failed to show settings for remote source");
                                        }
                                        if let Err(err) = handle.emit(shell::EVENT_OPEN_SETTINGS, json!({ "tab": "system" })) {
                                            warn!(error = %err, "Failed to emit open-settings for remote source");
                                        }
                                    }
                                }
                            });
                        }
                        shell::MENU_RESTART_SERVICE => {
                            let handle = app_handle.clone();
                            let shell_state = shell_state_for_tray.clone();
                            let managed_state = managed_state_for_tray.clone();
                            tauri::async_runtime::spawn(async move {
                                let config = match DesktopCoreSourceConfig::load(global_paths()) {
                                    Ok(config) => config,
                                    Err(err) => {
                                        warn!(error = %err, "Failed to load desktop core source config");
                                        return;
                                    }
                                };

                                match config.selected_source {
                                    DesktopCoreSourceKind::Localhost => {
									let result = match config.localhost_runtime_mode {
										LocalCoreRuntimeMode::Service => restart_local_service(&handle, &config).await,
										LocalCoreRuntimeMode::DesktopManaged => {
											let _ = stop_desktop_managed_core(&managed_state, &config).await;
											start_desktop_managed_core(&handle, &managed_state, &config).await
										}
									};
									match result {
                                            Ok(status) => {
                                                if let Err(err) = sync_shell_service_state(&shell_state, &status).await {
                                                    warn!(error = %err, "Failed to sync tray state after restarting service");
                                                }
										let view = DesktopCoreSourceView::from_config(&config, status.clone());
										emit_core_state_changed(&handle, &view);
                                            }
                                            Err(err) => {
                                                warn!(error = %err, "Failed to restart localhost core service");
                                            }
                                        }
                                    }
                                    DesktopCoreSourceKind::Remote => {
                                        if let Err(err) = shell::ensure_window_visibility(&handle) {
                                            warn!(error = %err, "Failed to show settings for remote source");
                                        }
                                        if let Err(err) = handle.emit(shell::EVENT_OPEN_SETTINGS, json!({ "tab": "system" })) {
                                            warn!(error = %err, "Failed to emit open-settings for remote source");
                                        }
                                    }
                                }
                            });
                        }
                        shell::MENU_STOP_SERVICE => {
                            let handle = app_handle.clone();
                            let shell_state = shell_state_for_tray.clone();
                            let managed_state = managed_state_for_tray.clone();
                            tauri::async_runtime::spawn(async move {
                                let config = match DesktopCoreSourceConfig::load(global_paths()) {
                                    Ok(config) => config,
                                    Err(err) => {
                                        warn!(error = %err, "Failed to load desktop core source config");
                                        return;
                                    }
                                };

                                match config.selected_source {
                                    DesktopCoreSourceKind::Localhost => {
									let result = match config.localhost_runtime_mode {
										LocalCoreRuntimeMode::Service => stop_local_service(&config).await,
										LocalCoreRuntimeMode::DesktopManaged => stop_desktop_managed_core(&managed_state, &config).await,
									};
									match result {
                                            Ok(status) => {
                                                if let Err(err) = sync_shell_service_state(&shell_state, &status).await {
                                                    warn!(error = %err, "Failed to sync tray state after stopping service");
                                                }
										let view = DesktopCoreSourceView::from_config(&config, status.clone());
										emit_core_state_changed(&handle, &view);
                                            }
                                            Err(err) => {
                                                warn!(error = %err, "Failed to stop localhost core service");
                                            }
                                        }
                                    }
                                    DesktopCoreSourceKind::Remote => {
                                        if let Err(err) = shell::ensure_window_visibility(&handle) {
                                            warn!(error = %err, "Failed to show settings for remote source");
                                        }
                                        if let Err(err) = handle.emit(shell::EVENT_OPEN_SETTINGS, json!({ "tab": "system" })) {
                                            warn!(error = %err, "Failed to emit open-settings for remote source");
                                        }
                                    }
                                }
                            });
                        }
                        shell::MENU_SERVICE_STATUS => {
                            let handle = app_handle.clone();
                            tauri::async_runtime::spawn(async move {
                                if let Err(err) = shell::ensure_window_visibility(&handle) {
                                    warn!(error = %err, "Failed to show main window for service status");
                                }
                                if let Err(err) = handle.emit(shell::EVENT_OPEN_SETTINGS, json!({ "tab": "system" })) {
                                    warn!(error = %err, "Failed to emit open-settings event to frontend");
                                }
                            });
                        }
                        _ => {}
                    }
                });

            #[cfg(target_os = "macos")]
            {
                tray_builder = tray_builder.icon_as_template(true);
            }

            let tray_icon = tray_builder.build(app)?;
            tauri::async_runtime::block_on(shell_state.register_tray(
                tray_icon,
                service_status_item.clone(),
                start_service_item.clone(),
                restart_service_item.clone(),
                stop_service_item.clone(),
            ))?;

            tauri::async_runtime::block_on(initialize_selected_core_source(
                app.handle().clone(),
                shell_state.clone(),
                desktop_managed_core_state.clone(),
            ))?;
            spawn_main_window(app)?;

            {
                let handle = app.handle().clone();
                let _ = app.deep_link().on_open_url(move |event| {
                    for url in event.urls() {
                        if let Err(err) = deep_link::route_mcpmate_deep_link(&handle, url.as_str()) {
                            warn!(error = %err, "Failed to handle mcpmate deep link");
                        }
                    }
                });
                if let Ok(Some(urls)) = app.deep_link().get_current() {
                    let handle = app.handle().clone();
                    for url in urls {
                        if let Err(err) = deep_link::route_mcpmate_deep_link(&handle, url.as_str()) {
                            warn!(error = %err, "Failed to handle startup mcpmate deep link");
                        }
                    }
                }
            }
            if !shell_prefs.show_dock_icon
                && let Some(window) = app.get_webview_window("main")
            {
                let _ = window.hide();
            }

            Ok(())
        });

    let builder = builder.invoke_handler(tauri::generate_handler![
        mcp_shell_apply_preferences,
        mcp_shell_read_preferences,
        mcp_shell_read_core_source,
        mcp_shell_apply_core_source,
        mcp_shell_manage_local_core_service,
        mcp_deep_link_take_pending_server_import,
        mcp_account_start_github_login,
        mcp_account_get_status,
        mcp_account_logout,
        mcp_oauth_prepare_callback_access,
        mcp_oauth_open_authorization_url
    ]);

    builder
        .build(tauri::generate_context!())
        .map_err(Error::new)?
        .run(move |app_handle, event| match event {
            #[cfg(target_os = "macos")]
            RunEvent::Reopen { .. } => {
                if let Err(err) = shell::ensure_window_visibility(app_handle) {
                    warn!(error = %err, "Failed to restore main window on app reopen");
                }
            }
            RunEvent::Exit => {
                if let Some(state) = app_handle.try_state::<DesktopManagedCoreState>()
                    && let Ok(config) = DesktopCoreSourceConfig::load(global_paths())
                    && config.localhost_runtime_mode == LocalCoreRuntimeMode::DesktopManaged
                {
                    let _ = tauri::async_runtime::block_on(stop_desktop_managed_core(
                        state.inner(),
                        &config,
                    ));
                }
            }
            _ => {}
        });

    Ok(())
}

#[tauri::command]
async fn mcp_shell_apply_preferences(
    app: tauri::AppHandle,
    state: tauri::State<'_, ShellState>,
    payload: ShellPreferencesPayload,
) -> Result<(), String> {
    let prev_show_dock_icon = state
        .inner()
        .clone()
        .current_preferences()
        .await
        .show_dock_icon;
    let prefs = ShellPreferences {
        menu_bar_icon_mode: payload.menu_bar_icon_mode,
        show_dock_icon: payload.show_dock_icon,
    };
    state
        .inner()
        .clone()
        .apply_preferences(&app, prefs.clone())
        .await
        .map_err(|err| err.to_string())?;

    // Only sync main window visibility when the Dock toggle actually changes. Otherwise every
    // settings sync (e.g. navigating to Settings) would hide the window while in accessory mode.
    if prev_show_dock_icon != prefs.show_dock_icon
        && let Some(window) = app.get_webview_window("main")
    {
        if prefs.show_dock_icon {
            let _ = window.show();
        } else {
            let _ = window.hide();
        }
    }

    Ok(())
}

#[tauri::command]
async fn mcp_shell_read_preferences(
    state: tauri::State<'_, ShellState>,
) -> Result<ShellPreferencesView, String> {
    Ok(ShellPreferencesView::from(
        state.inner().clone().current_preferences().await,
    ))
}

#[tauri::command]
async fn mcp_shell_read_core_source(
    managed_state: tauri::State<'_, DesktopManagedCoreState>,
) -> Result<DesktopCoreSourceView, String> {
    let config = DesktopCoreSourceConfig::load(global_paths()).map_err(|err| err.to_string())?;
    read_core_state_view(&config, managed_state.inner())
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn mcp_shell_apply_core_source(
    app: tauri::AppHandle,
    shell_state: tauri::State<'_, ShellState>,
    managed_state: tauri::State<'_, DesktopManagedCoreState>,
    payload: DesktopCoreSourcePayload,
) -> Result<DesktopCoreSourceView, String> {
    let previous = DesktopCoreSourceConfig::load(global_paths()).map_err(|err| err.to_string())?;
    let mut config = previous.clone();

    config.selected_source = payload.selected_source;
    config.localhost_runtime_mode = payload.localhost_runtime_mode;
    config.localhost.api_port = payload.localhost_api_port;
    config.localhost.mcp_port = payload.localhost_mcp_port;
    config.remote.base_url = payload.remote_base_url.trim().to_string();
    config.apply_constraints();

    DesktopCoreSourceConfig::save(global_paths(), &config).map_err(|err| err.to_string())?;
    persist_localhost_ports(&config);

    handle_localhost_source_transition(managed_state.inner(), &previous, &config).await;

    let view = match (config.selected_source, config.localhost_runtime_mode) {
        (DesktopCoreSourceKind::Localhost, LocalCoreRuntimeMode::Service) => {
            sync_local_service_definition(&app, &config)
                .await
                .map_err(|err| err.to_string())?;
            read_core_state_view(&config, managed_state.inner())
                .await
                .map_err(|err| err.to_string())?
        }
        _ => read_core_state_view(&config, managed_state.inner())
            .await
            .map_err(|err| err.to_string())?,
    };

    sync_and_emit_core_state(&app, shell_state.inner(), &view)
        .await
        .map_err(|err| err.to_string())?;

    if let Err(err) = audit::emit_desktop_audit_event(
        mcpmate::audit::AuditAction::CoreSourceApply,
        mcpmate::audit::AuditStatus::Success,
        Some(match config.selected_source {
            DesktopCoreSourceKind::Localhost => "localhost".to_string(),
            DesktopCoreSourceKind::Remote => "remote".to_string(),
        }),
        Some("Applied desktop core source configuration".to_string()),
        Some(json!({
            "selected_source": config.selected_source,
            "localhost_runtime_mode": config.localhost_runtime_mode,
            "localhost_api_port": config.localhost.api_port,
            "localhost_mcp_port": config.localhost.mcp_port,
            "remote_base_url": config.remote.base_url,
        })),
        None,
    )
    .await
    {
        warn!(error = %err, "Failed to emit desktop audit event for core source apply");
    }

    Ok(view)
}

#[tauri::command]
async fn mcp_shell_manage_local_core_service(
    app: tauri::AppHandle,
    shell_state: tauri::State<'_, ShellState>,
    managed_state: tauri::State<'_, DesktopManagedCoreState>,
    action: LocalCoreServiceAction,
) -> Result<DesktopCoreSourceView, String> {
    let config = DesktopCoreSourceConfig::load(global_paths()).map_err(|err| err.to_string())?;
    let action_name = format!("{:?}", action).to_lowercase();

    let audit_action = match (config.localhost_runtime_mode, action.clone()) {
        (LocalCoreRuntimeMode::Service, LocalCoreServiceAction::Start) => {
            Some(mcpmate::audit::AuditAction::LocalCoreServiceStart)
        }
        (LocalCoreRuntimeMode::Service, LocalCoreServiceAction::Restart) => {
            Some(mcpmate::audit::AuditAction::LocalCoreServiceRestart)
        }
        (LocalCoreRuntimeMode::Service, LocalCoreServiceAction::Stop) => {
            Some(mcpmate::audit::AuditAction::LocalCoreServiceStop)
        }
        (LocalCoreRuntimeMode::Service, LocalCoreServiceAction::Install) => {
            Some(mcpmate::audit::AuditAction::LocalCoreServiceInstall)
        }
        (LocalCoreRuntimeMode::Service, LocalCoreServiceAction::Uninstall) => {
            Some(mcpmate::audit::AuditAction::LocalCoreServiceUninstall)
        }
        (LocalCoreRuntimeMode::Service, LocalCoreServiceAction::Status) => None,
        (LocalCoreRuntimeMode::DesktopManaged, LocalCoreServiceAction::Start) => {
            Some(mcpmate::audit::AuditAction::DesktopManagedCoreStart)
        }
        (LocalCoreRuntimeMode::DesktopManaged, LocalCoreServiceAction::Restart) => {
            Some(mcpmate::audit::AuditAction::DesktopManagedCoreRestart)
        }
        (LocalCoreRuntimeMode::DesktopManaged, LocalCoreServiceAction::Stop) => {
            Some(mcpmate::audit::AuditAction::DesktopManagedCoreStop)
        }
        (LocalCoreRuntimeMode::DesktopManaged, LocalCoreServiceAction::Status) => None,
        (LocalCoreRuntimeMode::DesktopManaged, LocalCoreServiceAction::Install) => {
            Some(mcpmate::audit::AuditAction::LocalCoreServiceInstall)
        }
        (LocalCoreRuntimeMode::DesktopManaged, LocalCoreServiceAction::Uninstall) => {
            Some(mcpmate::audit::AuditAction::LocalCoreServiceUninstall)
        }
    };

    let view = match (config.localhost_runtime_mode, action.clone()) {
        (LocalCoreRuntimeMode::Service, LocalCoreServiceAction::Start) => {
            start_local_service(&app, &config)
                .await
                .map(|status| DesktopCoreSourceView::from_config(&config, status))
                .map_err(|err| err.to_string())?
        }
        (LocalCoreRuntimeMode::Service, LocalCoreServiceAction::Restart) => {
            restart_local_service(&app, &config)
                .await
                .map(|status| DesktopCoreSourceView::from_config(&config, status))
                .map_err(|err| err.to_string())?
        }
        (LocalCoreRuntimeMode::Service, LocalCoreServiceAction::Stop) => {
            stop_local_service(&config)
                .await
                .map(|status| DesktopCoreSourceView::from_config(&config, status))
                .map_err(|err| err.to_string())?
        }
        (LocalCoreRuntimeMode::Service, LocalCoreServiceAction::Status) => {
            read_core_state_view(&config, managed_state.inner())
                .await
                .map_err(|err| err.to_string())?
        }
        (LocalCoreRuntimeMode::Service, LocalCoreServiceAction::Install) => {
            install_local_service(&app, &config)
                .await
                .map(|status| DesktopCoreSourceView::from_config(&config, status))
                .map_err(|err| err.to_string())?
        }
        (LocalCoreRuntimeMode::Service, LocalCoreServiceAction::Uninstall) => {
            uninstall_local_service(&config)
                .map(|status| DesktopCoreSourceView::from_config(&config, status))
                .map_err(|err| err.to_string())?
        }
        (LocalCoreRuntimeMode::DesktopManaged, LocalCoreServiceAction::Start) => {
            start_desktop_managed_core(&app, managed_state.inner(), &config)
                .await
                .map(|status| DesktopCoreSourceView::from_config(&config, status))
                .map_err(|err| err.to_string())?
        }
        (LocalCoreRuntimeMode::DesktopManaged, LocalCoreServiceAction::Restart) => {
            let _ = stop_desktop_managed_core(managed_state.inner(), &config).await;
            start_desktop_managed_core(&app, managed_state.inner(), &config)
                .await
                .map(|status| DesktopCoreSourceView::from_config(&config, status))
                .map_err(|err| err.to_string())?
        }
        (LocalCoreRuntimeMode::DesktopManaged, LocalCoreServiceAction::Stop) => {
            stop_desktop_managed_core(managed_state.inner(), &config)
                .await
                .map(|status| DesktopCoreSourceView::from_config(&config, status))
                .map_err(|err| err.to_string())?
        }
        (LocalCoreRuntimeMode::DesktopManaged, LocalCoreServiceAction::Status) => {
            read_core_state_view(&config, managed_state.inner())
                .await
                .map_err(|err| err.to_string())?
        }
        (LocalCoreRuntimeMode::DesktopManaged, LocalCoreServiceAction::Install)
        | (LocalCoreRuntimeMode::DesktopManaged, LocalCoreServiceAction::Uninstall) => {
            let message = "install/uninstall are only available in service mode".to_string();
            if let Err(err) = audit::emit_desktop_audit_event(
                audit_action.expect("rejected desktop-managed action should have audit action"),
                mcpmate::audit::AuditStatus::Failed,
                Some(action_name.clone()),
                Some("Rejected local core service action".to_string()),
                Some(json!({
                    "localhost_runtime_mode": config.localhost_runtime_mode,
                    "action": action_name,
                })),
                Some(message.clone()),
            )
            .await
            {
                warn!(error = %err, "Failed to emit desktop audit event for rejected service action");
            }
            return Err(message);
        }
    };

    sync_and_emit_core_state(&app, shell_state.inner(), &view)
        .await
        .map_err(|err| err.to_string())?;

    if let Some(audit_action) = audit_action
        && let Err(err) = audit::emit_desktop_audit_event(
            audit_action,
            mcpmate::audit::AuditStatus::Success,
            Some(action_name.clone()),
            Some("Completed local core service action".to_string()),
            Some(json!({
                "localhost_runtime_mode": config.localhost_runtime_mode,
                "action": action_name,
                "status": view.local_service.status,
                "installed": view.local_service.installed,
                "running": view.local_service.running,
            })),
            None,
        )
        .await
    {
        warn!(error = %err, "Failed to emit desktop audit event for service action");
    }

    Ok(view)
}

#[tauri::command]
async fn mcp_deep_link_take_pending_server_import(
    state: tauri::State<'_, DeepLinkState>,
) -> Result<Option<ImportServerDeepLinkPayload>, String> {
    Ok(state.take_pending_server_import().await)
}

#[tauri::command]
fn mcp_account_start_github_login(app: tauri::AppHandle) -> Result<(), String> {
    account::start_github_login(&app)
}

#[tauri::command]
fn mcp_account_get_status(app: tauri::AppHandle) -> Result<account::AccountStatus, String> {
    account::get_status(&app)
}

#[tauri::command]
fn mcp_account_logout() -> Result<(), String> {
    account::logout()
}

#[tauri::command]
async fn mcp_oauth_prepare_callback_access(
    app: tauri::AppHandle,
    access_state: tauri::State<'_, OAuthCallbackAccessState>,
    server_id: String,
    api_base_url: String,
) -> Result<oauth_callback_access::OAuthCallbackAccessContract, String> {
    oauth_callback_access::prepare_callback_access(
        app,
        access_state.inner().clone(),
        server_id,
        api_base_url,
    )
    .await
}

#[tauri::command]
fn mcp_oauth_open_authorization_url(
    app: tauri::AppHandle,
    authorization_url: String,
) -> Result<(), String> {
    oauth_callback_access::open_authorization_url(&app, &authorization_url)
}

fn configure_tauri_environment() {
    runtime_env::configure_process_environment();
}

fn initialize_menu(app: &mut tauri::App) -> Result<()> {
    let app_handle = app.handle();

    let menu = Menu::default(app_handle)?;

    let about_item = MenuItem::with_id(app, MENU_ABOUT_ID, "About MCPMate", true, None::<&str>)?;
    let check_updates_item = MenuItem::with_id(
        app,
        MENU_CHECK_UPDATES_ID,
        "Check for Updates…",
        true,
        None::<&str>,
    )?;

    if let Some(MenuItemKind::Submenu(help_menu)) = menu.get(&HELP_SUBMENU_ID.to_string()) {
        let existing_items = help_menu.items()?.len();
        help_menu.insert(&check_updates_item, 0)?;
        help_menu.insert(&about_item, 0)?;
        if existing_items > 0 {
            let separator = PredefinedMenuItem::separator(app)?;
            help_menu.insert(&separator, 2)?;
        }
    } else {
        let help_menu = Submenu::with_id_and_items(
            app,
            HELP_SUBMENU_ID,
            "Help",
            true,
            &[&about_item, &check_updates_item],
        )?;
        menu.append(&help_menu)?;
    }

    app.set_menu(menu)?;

    Ok(())
}

pub(crate) fn spawn_main_window<M>(manager: &M) -> Result<()>
where
    M: Manager<Wry>,
{
    if manager.get_webview_window("main").is_some() {
        return Ok(());
    }

    let window_config = manager
        .app_handle()
        .config()
        .app
        .windows
        .iter()
        .find(|cfg| cfg.label == "main")
        .cloned()
        .unwrap_or_else(default_main_window_config);

    let app_handle = manager.app_handle().clone();

    let mut builder = WebviewWindowBuilder::from_config(manager, &window_config)?;

    #[cfg(target_os = "macos")]
    {
        builder = builder
            .title_bar_style(tauri::TitleBarStyle::Transparent)
            .hidden_title(true);
    }

    // Compose initialization script: disable context menu + expose native shell marker
    let init_script = String::from(
        r#"window.addEventListener('contextmenu', (event) => {
            if (event.metaKey || event.ctrlKey) {
                return;
            }
            event.preventDefault();
        });
        window.__MCPMATE_IS_TAURI__ = true;
        "#,
    );
    builder = builder.initialization_script(&init_script);

    let builder = builder.on_new_window(move |url, _features| {
        let scheme = url.scheme();
        match scheme {
            "http" | "https" => {
                if let Err(err) = app_handle.opener().open_url(url.as_str(), None::<String>) {
                    warn!(
                        error = %err,
                        target_url = %url,
                        "Failed to open external link from webview"
                    );
                }
                NewWindowResponse::Deny
            }
            "tauri" | "app" | "about" | "mcpmate" | "" => NewWindowResponse::Allow,
            other => {
                warn!(target_url = %url, scheme = other, "Blocked unsupported window.open URL scheme");
                NewWindowResponse::Deny
            }
        }
    });

    let window = builder.build()?;

    #[cfg(target_os = "macos")]
    let _ = manager.app_handle().show();
    let _ = window.show();
    let _ = window.set_focus();

    Ok(())
}

fn default_main_window_config() -> WindowConfig {
    WindowConfig {
        label: "main".into(),
        title: "MCPMate".into(),
        width: 1280.0,
        height: 800.0,
        resizable: true,
        create: false,
        ..Default::default()
    }
}

fn initialize_paths(app: &mut tauri::App) -> Result<()> {
    let app_handle = app.handle();

    let selected_paths = match try_use_default_paths() {
        Ok(paths) => paths,
        Err(err) => {
            warn!(error = %err, "Falling back to Tauri app data directory for MCPMate storage");
            use_app_data_paths(app_handle)?
        }
    };

    unsafe {
        std::env::set_var("MCPMATE_DATA_DIR", selected_paths.base_dir());
    }

    if let Err(err) = set_global_paths(selected_paths.clone()) {
        let existing = global_paths();
        if existing.base_dir() != selected_paths.base_dir() {
            return Err(err.context(
                "global MCPMate paths already initialized with a different base directory",
            ));
        }
    }

    selected_paths.ensure_directories()?;

    info!(
        "Using MCPMate data directory: {}",
        selected_paths.base_dir().display()
    );

    Ok(())
}

async fn initialize_selected_core_source(
    app: tauri::AppHandle,
    shell_state: ShellState,
    managed_state: DesktopManagedCoreState,
) -> Result<()> {
    let config = DesktopCoreSourceConfig::load(global_paths())?;
    let status = match config.localhost_runtime_mode {
        LocalCoreRuntimeMode::Service => read_local_service_status(&config).await?,
        LocalCoreRuntimeMode::DesktopManaged => {
            if config.selected_source == DesktopCoreSourceKind::Localhost {
                start_desktop_managed_core(&app, &managed_state, &config).await?
            } else {
                read_desktop_managed_status(&managed_state, &config).await?
            }
        }
    };
    let view = DesktopCoreSourceView::from_config(&config, status);
    sync_and_emit_core_state(&app, &shell_state, &view).await?;

    Ok(())
}

fn spawn_desktop_managed_core(
    app: &tauri::AppHandle,
    config: &DesktopCoreSourceConfig,
) -> Result<(Child, String)> {
    #[cfg(target_os = "windows")]
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let binary = resolve_local_core_binary(app)?;
    let base_dir = global_paths().base_dir().to_path_buf();
    let working_dir = resolve_local_core_working_dir(&binary, &base_dir);
    let desktop_managed_token = Uuid::new_v4().to_string();
    append_desktop_managed_shell_log(&format!(
        "spawn:start binary={} cwd={} data_dir={} api_port={} mcp_port={} token={} startup_log={}",
        binary.display(),
        working_dir.display(),
        base_dir.display(),
        config.localhost.api_port,
        config.localhost.mcp_port,
        desktop_managed_token,
        desktop_managed_startup_log_path(&base_dir).display(),
    ));
    let mut command = Command::new(binary.clone());
    command
        .arg("--api-port")
        .arg(config.localhost.api_port.to_string())
        .arg("--mcp-port")
        .arg(config.localhost.mcp_port.to_string())
        .arg("--log-level")
        .arg("info")
        .stdin(Stdio::null())
        .current_dir(&working_dir)
        .env("MCPMATE_DATA_DIR", &base_dir)
        .env("MCPMATE_DESKTOP_MANAGED_TOKEN", &desktop_managed_token)
        .env("MCPMATE_API_PORT", config.localhost.api_port.to_string())
        .env("MCPMATE_MCP_PORT", config.localhost.mcp_port.to_string());

    #[cfg(target_os = "windows")]
    command.creation_flags(CREATE_NO_WINDOW);

    configure_desktop_managed_stdio(&mut command, &base_dir);

    let mut child = command
        .spawn()
        .context("failed to spawn desktop-managed localhost core")?;

    // Validate child process state immediately after spawn to catch premature exits
    match child.try_wait() {
        Ok(Some(status)) => {
            let exit_info = if let Some(code) = status.code() {
                format!("exit code {}", code)
            } else {
                "terminated by signal".to_string()
            };
            let log_path = desktop_managed_startup_log_path(&base_dir);
            let log_tail = read_startup_log_tail(&log_path)
                .map(|tail| format!("\n\nLast startup log lines:\n{}", tail))
                .unwrap_or_default();
            append_desktop_managed_shell_log(&format!(
                "spawn:exited_immediately status={} startup_log={}",
                exit_info,
                log_path.display(),
            ));
            Err(anyhow::anyhow!(
                "mcpmate-core process exited immediately after spawn ({}). Check {} for error details.{}",
                exit_info,
                log_path.display(),
                log_tail,
            ))
        }
        Ok(None) => {
            // Process is still running, this is good
            append_desktop_managed_shell_log("spawn:child_running_after_spawn");
            Ok((child, desktop_managed_token))
        }
        Err(e) => {
            // try_wait() itself failed; attempt to kill the child and return error
            let _ = child.kill();
            append_desktop_managed_shell_log(&format!(
                "spawn:try_wait_failed error={}",
                e
            ));
            Err(e).context("failed to check child process state after spawn")
        }
    }
}

fn configure_desktop_managed_stdio(command: &mut Command, _base_dir: &std::path::Path) {
    #[cfg(debug_assertions)]
    {
        command.stdout(Stdio::inherit()).stderr(Stdio::inherit());
    }

    #[cfg(not(debug_assertions))]
    {
        command.stdout(Stdio::null());

        let log_path = _base_dir.join("mcpmate-core-startup.log");
        if std::fs::create_dir_all(_base_dir).is_ok()
            && let Ok(log_file) = OpenOptions::new().create(true).append(true).open(&log_path)
        {
            command.stderr(log_file);
        } else {
            command.stderr(Stdio::null());
        }
    }
}

async fn read_desktop_managed_status(
    state: &DesktopManagedCoreState,
    config: &DesktopCoreSourceConfig,
) -> Result<LocalCoreServiceStatusView> {
    read_desktop_managed_status_with_error(state, config, None).await
}

async fn read_desktop_managed_status_with_error(
    state: &DesktopManagedCoreState,
    config: &DesktopCoreSourceConfig,
    last_error: Option<String>,
) -> Result<LocalCoreServiceStatusView> {
    let base_dir = global_paths().base_dir().to_path_buf();
    let process_running = state.is_spawned().await;
    let expected_token = state.expected_token().await;
    let api_ready = if let Some(expected_token) = expected_token.as_deref() {
        core_service::probe_localhost_core(config.localhost.api_port, Some(expected_token)).await
    } else {
        false
    };
    let running = process_running || api_ready;
    let probe_summary = core_service::describe_localhost_core_probe(config.localhost.api_port).await;
    let diagnostics = if api_ready {
        None
    } else {
        desktop_managed_diagnostics(&base_dir, last_error)
    };

    append_desktop_managed_shell_log(&format!(
        "status:read process_running={} api_ready={} expected_token={} probe={} last_error={}",
        process_running,
        api_ready,
        expected_token.as_deref().unwrap_or("missing"),
        probe_summary,
        diagnostics
            .as_ref()
            .and_then(|item| item.last_error.as_deref())
            .unwrap_or("none"),
    ));

    let (status, label, detail) = if api_ready {
        (
            core_service::LocalCoreServiceStatusKind::Running,
            "Running".to_string(),
            "The localhost core is managed by MCPMate Desktop and is responding to health checks."
                .to_string(),
        )
    } else if process_running {
        (
            core_service::LocalCoreServiceStatusKind::RunningUnhealthy,
            "Running (Unhealthy)".to_string(),
            "The desktop-managed core process is still running, but the localhost health check has not succeeded. Review the startup diagnostics below before retrying."
                .to_string(),
        )
    } else if diagnostics.is_some() {
        (
            core_service::LocalCoreServiceStatusKind::Stopped,
            "Stopped".to_string(),
            "The localhost core is currently stopped. Review the startup diagnostics below before retrying."
                .to_string(),
        )
    } else {
        (
            core_service::LocalCoreServiceStatusKind::Stopped,
            "Stopped".to_string(),
            "The localhost core is currently stopped. Starting it will keep it alive while MCPMate Desktop is running."
                .to_string(),
        )
    };

    Ok(LocalCoreServiceStatusView {
        status,
        label,
        detail,
        level: "desktop".to_string(),
        installed: false,
        running,
        diagnostics,
    })
}

async fn start_desktop_managed_core(
    app: &tauri::AppHandle,
    state: &DesktopManagedCoreState,
    config: &DesktopCoreSourceConfig,
) -> Result<LocalCoreServiceStatusView> {
    append_desktop_managed_shell_log(&format!(
        "start:requested mode=desktop_managed api_port={} mcp_port={}",
        config.localhost.api_port,
        config.localhost.mcp_port,
    ));
    if core_service::probe_localhost_core(config.localhost.api_port, None).await {
        let probe_summary = core_service::describe_localhost_core_probe(config.localhost.api_port).await;
        append_desktop_managed_shell_log(&format!(
            "start:blocked_existing_core probe={}",
            probe_summary,
        ));
        return read_desktop_managed_status_with_error(
            state,
            config,
            Some(format!(
                "Port {} is already served by a core instance that was not started by this desktop session. Stop the external mcpmate-core process before starting Desktop mode.",
                config.localhost.api_port
            )),
        )
        .await;
    }
    let (child, token) = match spawn_desktop_managed_core(app, config) {
        Ok(result) => result,
        Err(err) => {
            append_desktop_managed_shell_log(&format!("start:spawn_failed error={}", err));
            return read_desktop_managed_status_with_error(state, config, Some(err.to_string())).await;
        }
    };
    append_desktop_managed_shell_log(&format!("start:spawn_succeeded token={}", token));
    state.replace(child, token).await;

    spawn_core_ready_notification(app.clone(), state.clone(), config.clone());

    read_desktop_managed_status(state, config).await
}

fn spawn_core_ready_notification(
    app: tauri::AppHandle,
    state: DesktopManagedCoreState,
    config: DesktopCoreSourceConfig,
) {
    tauri::async_runtime::spawn(async move {
        let expected_token = state.expected_token().await;
        append_desktop_managed_shell_log(&format!(
            "ready:wait_begin api_port={} expected_token={}",
            config.localhost.api_port,
            expected_token.as_deref().unwrap_or("missing"),
        ));

        let wait_result = core_service::wait_for_localhost_core(
            config.localhost.api_port,
            expected_token.as_deref(),
        )
            .await
            .err()
            .map(|err| err.to_string());

        if let Some(last_error) = wait_result {
            append_desktop_managed_shell_log(&format!(
                "ready:wait_failed error={}",
                last_error
            ));

            if let Ok(status) = read_desktop_managed_status_with_error(&state, &config, Some(last_error)).await {
                let view = DesktopCoreSourceView::from_config(&config, status);
                emit_core_state_changed(&app, &view);
            }
            return;
        }

        append_desktop_managed_shell_log("ready:wait_succeeded");

        if let Ok(status) = read_desktop_managed_status(&state, &config).await {
            let view = DesktopCoreSourceView::from_config(&config, status);
            emit_core_state_changed(&app, &view);
        }
    });
}

async fn stop_desktop_managed_core(
    state: &DesktopManagedCoreState,
    config: &DesktopCoreSourceConfig,
) -> Result<LocalCoreServiceStatusView> {
    append_desktop_managed_shell_log(&format!(
        "stop:desktop_managed requested api_port={} mcp_port={}",
        config.localhost.api_port,
        config.localhost.mcp_port,
    ));
    if let Some(mut child) = state.take("stop_desktop_managed_core").await {
        child
            .kill()
            .context("failed to kill desktop-managed localhost core process")?;
        append_desktop_managed_shell_log("stop:desktop_managed kill_sent");
        child
            .wait()
            .context("failed to wait for desktop-managed localhost core process exit")?;
        append_desktop_managed_shell_log("stop:desktop_managed wait_completed");
    }

    let _ = core_service::wait_for_localhost_core_stopped(config.localhost.api_port).await;

    for _ in 0..10 {
        let status = read_desktop_managed_status(state, config).await?;
        if !status.running {
            return Ok(status);
        }
        sleep(Duration::from_millis(300)).await;
    }

    read_desktop_managed_status(state, config).await
}

fn persist_localhost_ports(config: &DesktopCoreSourceConfig) {
    let persisted = runtime_ports::PersistedRuntimePorts {
        api_port: config.localhost.api_port,
        mcp_port: config.localhost.mcp_port,
    };

    if let Err(err) = runtime_ports::PersistedRuntimePorts::save(global_paths(), &persisted) {
        warn!(error = %err, "Failed to persist localhost core ports");
    }
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

    std::fs::create_dir_all(&data_dir).with_context(|| {
        format!(
            "failed to create app data directory at {}",
            data_dir.display()
        )
    })?;

    let paths = MCPMatePaths::from_base_dir(&data_dir)?;
    paths.ensure_directories().with_context(|| {
        format!(
            "failed to initialize MCPMate directories under {}",
            data_dir.display()
        )
    })?;
    Ok(paths)
}
