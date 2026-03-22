use std::{sync::Arc, time::Duration};

use anyhow::{Context, Error, Result};
use mcpmate::system::config::init_port_config;
use mcpmate::{
    clients::ClientConfigService,
    common::{MCPMatePaths, global_paths, set_global_paths},
    core::{
        foundation::monitor,
        proxy::{
            Args, ProxyServer,
            init::{setup_database, setup_logging, setup_proxy_server_with_params},
            startup::{start_api_server, start_background_connections, start_proxy_server},
        },
    },
};
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
mod runtime_ports;
mod shell;
use shell::{ShellPreferences, ShellState};
use tauri_plugin_deep_link::DeepLinkExt;
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_updater::Builder as UpdaterPluginBuilder;
use tauri_plugin_updater::UpdaterExt;
use tokio::{sync::Mutex as AsyncMutex, task::JoinHandle, time::timeout};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

const SHUTDOWN_TIMEOUT_SECS: u64 = 5;
const MENU_CHECK_UPDATES_ID: &str = "menu.help.check_for_updates";
const MENU_ABOUT_ID: &str = "menu.help.about";

#[derive(Clone, Default)]
struct BackendState {
    inner: Arc<AsyncMutex<Option<BackendRuntime>>>,
}

impl BackendState {
    async fn set(&self, runtime: BackendRuntime) {
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

impl From<ShellPreferences> for ShellPreferencesView {
    fn from(value: ShellPreferences) -> Self {
        Self {
            menu_bar_icon_mode: value.menu_bar_icon_mode,
            show_dock_icon: value.show_dock_icon,
        }
    }
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
                Ok(Ok(Err(err))) => {
                    warn!(error = %err, "MCP server reported error while shutting down")
                }
                Ok(Err(err)) => warn!(error = %err, "MCP server task join error"),
                Err(_) => warn!("Timed out waiting for MCP server shutdown"),
            }
        }

        self.api_cancel.cancel();
        match timeout(
            Duration::from_secs(SHUTDOWN_TIMEOUT_SECS),
            &mut self.api_task,
        )
        .await
        {
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
            show_about_dialog(app_handle);
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

            let open_main_item =
                MenuItem::with_id(app, shell::MENU_OPEN_MAIN, "Open MCPMate", true, None::<&str>)?;
            let initial_toggle_text = if tauri::async_runtime::block_on(shell_state.is_backend_running()) {
                "Stop Service"
            } else {
                "Start Service"
            };
            let toggle_service_item = MenuItem::with_id(
                app,
                shell::MENU_TOGGLE_SERVICE,
                initial_toggle_text,
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
                .item(&toggle_service_item)
                .separator()
                .item(&settings_item)
                .item(&about_item)
                .separator()
                .item(&quit_item)
                .build()?;

            let backend_state_for_tray = backend_state.clone();
            let shell_state_for_tray = shell_state.clone();

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
                                if let Err(err) = handle.emit(shell::EVENT_OPEN_MAIN, json!({})) {
                                    warn!(error = %err, "Failed to emit open-main event to frontend");
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
                            show_about_dialog(app_handle);
                        }
                        shell::MENU_QUIT => {
                            app_handle.exit(0);
                        }
                        shell::MENU_TOGGLE_SERVICE => {
                            let handle = app_handle.clone();
                            let backend_state = backend_state_for_tray.clone();
                            let shell_state = shell_state_for_tray.clone();
                            tauri::async_runtime::spawn(async move {
                                if shell_state.is_backend_running().await {
                                    if let Some(runtime) = backend_state.take().await {
                                        runtime.shutdown().await;
                                    }
                                    if let Err(err) = shell_state.update_backend_running(false).await {
                                        warn!(error = %err, "Failed to update shell state after stopping backend");
                                    }
                                } else {
                                    let args = resolve_args();
                                    if let Err(err) = args.validate() {
                                        warn!(error = %err, "Backend arguments invalid; refusing to start service");
                                        return;
                                    }
                                    launch_backend(backend_state.clone(), shell_state.clone(), args).await;
                                    if let Err(err) = shell::ensure_window_visibility(&handle) {
                                        warn!(error = %err, "Failed to reveal main window after starting service");
                                    }
                                    if let Err(err) = handle.emit(shell::EVENT_OPEN_MAIN, json!({})) {
                                        warn!(error = %err, "Failed to emit open-main after starting service");
                                    }
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
            tauri::async_runtime::block_on(shell_state.register_tray(tray_icon, toggle_service_item.clone()))?;

            bootstrap_backend(backend_state.clone(), shell_state.clone())?;
            spawn_main_window(app)?;

            {
                let handle = app.handle().clone();
                let _ = app.deep_link().on_open_url(move |event| {
                    for url in event.urls() {
                        if let Err(err) = account::handle_oauth_url(&handle, url.as_str()) {
                            warn!(error = %err, "Failed to handle OAuth deep link");
                        }
                    }
                });
                if let Ok(Some(urls)) = app.deep_link().get_current() {
                    let handle = app.handle().clone();
                    for url in urls {
                        if let Err(err) = account::handle_oauth_url(&handle, url.as_str()) {
                            warn!(error = %err, "Failed to handle startup OAuth deep link");
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
        mcp_shell_read_runtime_ports,
        mcp_shell_restart_backend_with_ports,
        mcp_account_start_github_login,
        mcp_account_get_status,
        mcp_account_logout
    ]);

    builder
        .build(tauri::generate_context!())
        .map_err(Error::new)?
        .run(move |app_handle, event| {
            if let RunEvent::Exit = event {
                if let Some(state) = app_handle.try_state::<BackendState>()
                    && let Some(runtime) = tauri::async_runtime::block_on(state.take())
                {
                    tauri::async_runtime::block_on(runtime.shutdown());
                }
                if let Some(shell_state) = app_handle.try_state::<ShellState>()
                    && let Err(err) =
                        tauri::async_runtime::block_on(shell_state.update_backend_running(false))
                {
                    warn!(error = %err, "Failed to mark backend stopped during exit");
                }
            }
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

fn configure_tauri_environment() {
    const SKIP_BOARD_STATIC: &str = "MCPMATE_SKIP_BOARD_STATIC";

    if std::env::var_os(SKIP_BOARD_STATIC).is_none() {
        unsafe {
            std::env::set_var(SKIP_BOARD_STATIC, "1");
        }
    }

    // macOS packaged apps often inherit a minimal PATH when launched from Finder,
    // which breaks spawning of developer runtimes (bunx/npx/python) from the backend.
    // To make debug builds work out of the box, we: (1) ensure absolute shims for
    // common commands under ~/.mcpmate/bin, (2) prepend that bin plus typical Homebrew
    // locations to PATH.
    #[cfg(target_os = "macos")]
    {
        use std::{fs, io::Write, os::unix::fs::PermissionsExt, path::PathBuf};

        // Resolve MCPMate base dir the same way backend does by default
        let base_dir = match MCPMatePaths::new() {
            Ok(p) => p.base_dir().to_path_buf(),
            Err(_) => {
                let home = std::env::var("HOME").unwrap_or_else(|_| String::from("/"));
                PathBuf::from(home).join(".mcpmate")
            }
        };

        // Prepare ~/.mcpmate/bin and ~/.mcpmate/runtimes/* helper dirs
        let bin_dir = base_dir.join("bin");
        let _ = fs::create_dir_all(&bin_dir);

        let bunx_path = base_dir.join("runtimes").join("bun").join("bunx");

        // Write a tiny shim for `npx` that prefers our managed bunx, then falls back to system npx
        let npx_shim = bin_dir.join("npx");
        if !npx_shim.exists() {
            let mut f = fs::File::create(&npx_shim).ok();
            if let Some(mut file) = f.take() {
                let script = format!(
                    "#!/bin/sh\nset -e\nBUNX=\"{}\"\nif [ -x \"$BUNX\" ]; then exec \"$BUNX\" \"$@\"; fi\n# fallback to system npx if present\nif command -v npx >/dev/null 2>&1; then exec \"$(command -v npx)\" \"$@\"; fi\necho 'npx is unavailable (no bunx in ~/.mcpmate/runtimes and npx not found in PATH)' 1>&2\nexit 127\n",
                    bunx_path.display()
                );
                let _ = file.write_all(script.as_bytes());
                let _ = fs::set_permissions(&npx_shim, fs::Permissions::from_mode(0o755));
            }
        }

        // Provide a conservative Python shim that prefers the system interpreter
        let python3_candidates = [
            "/usr/bin/python3",
            "/opt/homebrew/bin/python3",
            "/usr/local/bin/python3",
        ];
        let py_shim_paths = [bin_dir.join("python3"), bin_dir.join("python")];
        for shim in &py_shim_paths {
            if !shim.exists()
                && let Ok(mut file) = fs::File::create(shim)
            {
                let mut found = None;
                for c in &python3_candidates {
                    if std::path::Path::new(c).exists() {
                        found = Some(*c);
                        break;
                    }
                }
                let body = if let Some(p) = found {
                    format!("#!/bin/sh\nexec \"{}\" \"$@\"\n", p)
                } else {
                    "#!/bin/sh\nexec /usr/bin/env python3 \"$@\"\n".to_string()
                };
                let _ = file.write_all(body.as_bytes());
                let _ = fs::set_permissions(shim, fs::Permissions::from_mode(0o755));
            }
        }

        // Compose PATH: ~/.mcpmate/bin + runtimes + common Homebrew prefixes + existing PATH
        let mut extra_paths: Vec<String> = vec![
            bin_dir.display().to_string(),
            base_dir.join("runtimes").join("bun").display().to_string(),
            base_dir.join("runtimes").join("uv").display().to_string(),
            "/opt/homebrew/bin".into(),
            "/usr/local/bin".into(),
        ];
        if let Ok(current) = std::env::var("PATH") {
            extra_paths.push(current);
        }
        let new_path = extra_paths.join(":");
        unsafe {
            std::env::set_var("PATH", new_path);
        }
    }
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

fn show_about_dialog(app_handle: &tauri::AppHandle) {
    let pkg = app_handle.package_info();
    let version = pkg.version.to_string();
    let tauri_version = tauri::VERSION;
    let message = format!(
        "MCPMate Desktop Beta\n\nVersion: {}\nTauri: {}\n\nAuto-update will activate once CDN hosting & signing pipeline are live.",
        version, tauri_version
    );

    app_handle
        .dialog()
        .message(message)
        .title("About MCPMate")
        .buttons(MessageDialogButtons::Ok)
        .show(|_| {});
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

    builder.build()?;

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

fn bootstrap_backend(state: BackendState, shell_state: ShellState) -> Result<()> {
    let args = resolve_args();
    args.validate().map_err(Error::msg)?;

    tauri::async_runtime::spawn(launch_backend(state, shell_state, args));

    Ok(())
}

async fn launch_backend(state: BackendState, shell_state: ShellState, args: Args) {
    match start_backend(args).await {
        Ok(runtime) => {
            info!("MCPMate backend started successfully for Tauri shell");
            state.set(runtime).await;
            if let Err(err) = shell_state.update_backend_running(true).await {
                warn!(error = %err, "Failed to mark backend as running in shell state");
            }
        }
        Err(err) => {
            error!(error = %err, "Failed to start MCPMate backend inside Tauri");
            if let Err(e) = shell_state.update_backend_running(false).await {
                warn!(error = %e, "Failed to update shell state after backend start failure");
            }
        }
    }
}

async fn start_backend(args: Args) -> Result<BackendRuntime> {
    // Optional OpenAPI password lock for local experiments (open-source builds stay unlocked by default).
    for (from, to) in [
        ("MCPMATE_TAURI_OPENAPI_PASSWORD", "MCPMATE_OPENAPI_PASSWORD"),
        ("MCPMATE_TAURI_OPENAPI_ENABLED", "MCPMATE_OPENAPI_ENABLED"),
    ] {
        if let Ok(v) = std::env::var(from)
            && !v.trim().is_empty()
        {
            unsafe { std::env::set_var(to, v) };
        }
    }

    // Inspector native-mode toggle passthrough: allow Tauri env to control backend env.
    if let Ok(v) = std::env::var("MCPMATE_TAURI_ENABLE_INSPECTOR") {
        let enable = matches_ignore_case(&v, ["1", "true", "yes"]);
        unsafe { std::env::set_var("MCPMATE_INSPECTOR_NATIVE", if enable { "1" } else { "0" }) };
    }

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
    use mcpmate::common::global_paths;

    let env_api = read_port_env("MCPMATE_TAURI_API_PORT");
    let env_mcp = read_port_env("MCPMATE_TAURI_MCP_PORT");
    let file = runtime_ports::PersistedRuntimePorts::load(global_paths());

    let api_port = env_api
        .or_else(|| file.as_ref().map(|p| p.api_port))
        .unwrap_or(ports::API_PORT);
    let mcp_port = env_mcp
        .or_else(|| file.as_ref().map(|p| p.mcp_port))
        .unwrap_or(ports::MCP_PORT);

    let log_level = clean_env_string("MCPMATE_TAURI_LOG").unwrap_or_else(|| "info".to_string());

    let transport =
        clean_env_string("MCPMATE_TAURI_TRANSPORT").unwrap_or_else(|| "uni".to_string());

    let profile = clean_env_string("MCPMATE_TAURI_PROFILE").and_then(|raw| {
        let profiles: Vec<String> = raw
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if profiles.is_empty() {
            None
        } else {
            Some(profiles)
        }
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

#[derive(serde::Serialize)]
struct RuntimePorts {
    api_port: u16,
    mcp_port: u16,
    api_url: String,
    mcp_http_url: String,
    mcp_sse_url: String,
}

async fn reapply_hosted_clients_if_mcp_port_changed(
    runtime: &BackendRuntime,
    previous_mcp_port: u16,
    mcp_port: u16,
) {
    if mcp_port == previous_mcp_port {
        return;
    }

    let Some(db) = runtime.proxy.database.as_ref() else {
        warn!("No database on proxy; skipping hosted client reapply after MCP port change");
        return;
    };

    let pool = Arc::new(db.pool.clone());
    let service = match ClientConfigService::bootstrap(pool).await {
        Ok(s) => s,
        Err(err) => {
            warn!(
                error = %err,
                "Could not bootstrap client service for hosted reapply after MCP port change"
            );
            return;
        }
    };

    match service
        .reapply_hosted_managed_clients_after_mcp_port_change()
        .await
    {
        Ok(summary) => {
            info!(
                attempted = summary.attempted,
                applied = summary.applied,
                scheduled = summary.scheduled,
                failed = summary.failures.len(),
                previous_mcp_port,
                mcp_port,
                "Reapplied hosted client configs after MCP port change"
            );
            for (client_id, err) in &summary.failures {
                warn!(
                    client = %client_id,
                    error = %err,
                    "Hosted client config reapply failed after MCP port change"
                );
            }
        }
        Err(err) => {
            warn!(
                error = %err,
                "Hosted client batch reapply failed after MCP port change"
            );
        }
    }
}

#[tauri::command]
async fn mcp_shell_read_runtime_ports() -> Result<RuntimePorts, String> {
    let cfg = mcpmate::system::config::get_runtime_port_config();
    Ok(RuntimePorts {
        api_port: cfg.api_port,
        mcp_port: cfg.mcp_port,
        api_url: cfg.api_url(),
        mcp_http_url: cfg.mcp_http_url(),
        mcp_sse_url: cfg.mcp_sse_url(),
    })
}

#[tauri::command]
async fn mcp_shell_restart_backend_with_ports(
    app: tauri::AppHandle,
    shell_state: tauri::State<'_, ShellState>,
    backend_state: tauri::State<'_, BackendState>,
    api_port: u16,
    mcp_port: u16,
) -> Result<(), String> {
    if api_port == 0 || mcp_port == 0 || api_port == mcp_port {
        return Err("invalid port values".into());
    }

    let previous_mcp_port = mcpmate::system::config::get_runtime_port_config().mcp_port;

    // Stop current backend if running
    if let Some(runtime) = backend_state.take().await {
        runtime.shutdown().await;
        if let Err(err) = shell_state.update_backend_running(false).await {
            warn!(error = %err, "Failed to mark backend stopped before restart");
        }
    }

    // Update environment for consistency
    unsafe {
        std::env::set_var("MCPMATE_TAURI_API_PORT", api_port.to_string());
        std::env::set_var("MCPMATE_TAURI_MCP_PORT", mcp_port.to_string());
        // Keep generic envs in sync so backend CLI parsing also sees the same values if used elsewhere
        std::env::set_var("MCPMATE_API_PORT", api_port.to_string());
        std::env::set_var("MCPMATE_MCP_PORT", mcp_port.to_string());
    }

    // Start backend with explicit ports
    let args = Args {
        mcp_port,
        api_port,
        log_level: clean_env_string("MCPMATE_TAURI_LOG").unwrap_or_else(|| "info".into()),
        transport: clean_env_string("MCPMATE_TAURI_TRANSPORT").unwrap_or_else(|| "uni".into()),
        profile: clean_env_string("MCPMATE_TAURI_PROFILE").map(|raw| {
            raw.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
        }),
        minimal: std::env::var("MCPMATE_TAURI_MINIMAL")
            .map(|v| matches_ignore_case(&v, ["1", "true", "yes"]))
            .unwrap_or(false),
    };

    match start_backend(args).await {
        Ok(runtime) => {
            reapply_hosted_clients_if_mcp_port_changed(&runtime, previous_mcp_port, mcp_port).await;

            backend_state.set(runtime).await;
            if let Err(err) = shell_state.update_backend_running(true).await {
                warn!(error = %err, "Failed to mark backend running after restart");
            }
            let persisted = runtime_ports::PersistedRuntimePorts { api_port, mcp_port };
            if let Err(err) = runtime_ports::PersistedRuntimePorts::save(
                mcpmate::common::global_paths(),
                &persisted,
            ) {
                warn!(error = %err, "Failed to persist runtime ports for next launch");
            }
            // Notify frontend that ports changed
            let payload = serde_json::json!({
                "api_port": api_port,
                "mcp_port": mcp_port,
            });
            let _ = app.emit("mcpmate://backend/portsChanged", payload);
            Ok(())
        }
        Err(err) => Err(err.to_string()),
    }
}

fn matches_ignore_case(value: &str, accepted: [&str; 3]) -> bool {
    accepted
        .iter()
        .any(|candidate| value.eq_ignore_ascii_case(candidate))
}

fn clean_env_string(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}
