use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
    time::{Duration, Instant},
};

use crate::operator_window;
use anyhow::{Context, Result};
use mcpmate::common::MCPMatePaths;
use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, Manager, PhysicalPosition, Rect, WebviewUrl, Window, Wry,
    image::Image,
    menu::{HELP_SUBMENU_ID, Menu, MenuItem, MenuItemKind, PredefinedMenuItem, Submenu},
    tray::TrayIcon,
    utils::config::{Color, WindowConfig},
    webview::{NewWindowResponse, WebviewWindow, WebviewWindowBuilder},
};
use tauri_plugin_opener::OpenerExt;
use tokio::sync::Mutex as AsyncMutex;
use tracing::warn;

/// tray-icon resets `NSImage.template` to off on every [`TrayIcon::set_icon`] on macOS; re-enable
/// so the menu bar follows effective appearance (wallpaper tint, light/dark, etc.).
fn set_tray_icon_with_template(tray: &TrayIcon<Wry>, icon: Image<'static>) -> Result<()> {
    tray.set_icon(Some(icon))
        .map_err(|e| anyhow::anyhow!(e))
        .context("failed to set tray icon")?;
    #[cfg(target_os = "macos")]
    {
        tray.set_icon_as_template(true)
            .map_err(|e| anyhow::anyhow!(e))
            .context("failed to set tray icon as template")?;
    }
    Ok(())
}

/// Monochrome glyph on transparency (typically black on alpha). On macOS, pair with
/// `TrayIconBuilder::icon_as_template(true)` so the system tints it for light/dark menu bar.
const TRAY_TEMPLATE_ICON_BYTES: &[u8] = include_bytes!("../icons/icon_tray.png");

static TRAY_TEMPLATE_ICON: OnceLock<Image<'static>> = OnceLock::new();

pub fn tray_template_icon() -> Image<'static> {
    TRAY_TEMPLATE_ICON
        .get_or_init(|| {
            Image::from_bytes(TRAY_TEMPLATE_ICON_BYTES).unwrap_or_else(|e| {
                panic!("failed to decode icons/icon_tray.png for tray: {e}");
            })
        })
        .clone()
}

pub const TRAY_ID: &str = "mcpmate.tray.main";
pub const MENU_OPEN_MAIN: &str = "mcpmate.tray.open_main";
pub const MENU_OPEN_OPERATOR: &str = "mcpmate.tray.open_operator";
pub const MENU_SERVICE_STATUS: &str = "mcpmate.tray.service_status";
pub const MENU_START_SERVICE: &str = "mcpmate.tray.start_service";
pub const MENU_RESTART_SERVICE: &str = "mcpmate.tray.restart_service";
pub const MENU_STOP_SERVICE: &str = "mcpmate.tray.stop_service";
pub const MENU_OPEN_SETTINGS: &str = "mcpmate.tray.open_settings";
pub const MENU_SHOW_ABOUT: &str = "mcpmate.tray.show_about";
pub const MENU_QUIT: &str = "mcpmate.tray.quit";
pub const APP_MENU_CHECK_UPDATES: &str = "menu.help.check_for_updates";
pub const APP_MENU_ABOUT: &str = "menu.help.about";

pub const EVENT_OPEN_SETTINGS: &str = "mcpmate://open-settings";
pub const EVENT_CORE_STATE_CHANGED: &str = "mcpmate://core/status-changed";
pub const EVENT_OPEN_FULL_BOARD_PATH: &str = "mcpmate://open-full-board-path";

const OPERATOR_WINDOW_LABEL: &str = "operator";
const OPERATOR_PANEL_WIDTH: f64 = 420.0;
const OPERATOR_PANEL_DEFAULT_HEIGHT: f64 = 640.0;
const OPERATOR_PANEL_MIN_HEIGHT: f64 = 420.0;
const OPERATOR_PANEL_MAX_HEIGHT: f64 = 1200.0;
const OPERATOR_PANEL_TRAY_FOCUS_LOSS_GRACE: Duration = Duration::from_millis(500);
const OPERATOR_PANEL_TRAY_GAP: f64 = 8.0;
const OPERATOR_PANEL_TRANSPARENT_BACKGROUND: Color = Color(0, 0, 0, 0);
const OPERATOR_WINDOW_INIT_SCRIPT: &str = r#"
(function () {
  function isOperatorDragRegion(path) {
    for (const node of path) {
      if (!(node instanceof HTMLElement)) continue;
      if (node.dataset.operatorNoDrag === "true") return false;
      if (node.dataset.operatorDragRegion === "true") return true;
    }
    return false;
  }

  const CLICKABLE_TAGS = new Set([
    "A",
    "BUTTON",
    "INPUT",
    "SELECT",
    "TEXTAREA",
    "LABEL",
    "SUMMARY",
  ]);

  function isClickableElement(element) {
    return (
      CLICKABLE_TAGS.has(element.tagName)
      || (element.hasAttribute("contenteditable")
        && element.getAttribute("contenteditable") !== "false")
      || (element.hasAttribute("tabindex") && element.getAttribute("tabindex") !== "-1")
    );
  }

  function blockOperatorHeaderDoubleClick(event) {
    if (event.button !== 0 || event.detail < 2) return;
    if (!isOperatorDragRegion(event.composedPath())) return;
    event.preventDefault();
    event.stopImmediatePropagation();
  }

  function startOperatorHeaderDrag(event) {
    if (event.button !== 0 || event.detail !== 1) return;
    if (!isOperatorDragRegion(event.composedPath())) return;
    const target = event.target;
    if (target instanceof Element && isClickableElement(target)) return;
    event.preventDefault();
    const internals = window.__TAURI_INTERNALS__;
    if (!internals) return;
    internals.invoke("plugin:window|start_dragging");
  }

  document.addEventListener("mousedown", blockOperatorHeaderDoubleClick, true);
  document.addEventListener("mouseup", blockOperatorHeaderDoubleClick, true);
  document.addEventListener("mousedown", startOperatorHeaderDrag, true);
})();
"#;
static OPERATOR_PANEL_FOCUS_LOSS_DISMISSAL_AT: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();
static LAST_TRAY_ICON_RECT: OnceLock<Mutex<Option<Rect>>> = OnceLock::new();

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MenuBarIconMode {
    /// Show the menu bar icon while the backend is running.
    #[default]
    Runtime,
    /// Hide the menu bar icon completely (unless forced by other settings).
    Hidden,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellPreferences {
    pub menu_bar_icon_mode: MenuBarIconMode,
    pub show_dock_icon: bool,
    #[serde(default)]
    pub operator_intro_shown: bool,
}

impl Default for ShellPreferences {
    fn default() -> Self {
        Self {
            menu_bar_icon_mode: MenuBarIconMode::Runtime,
            show_dock_icon: true,
            operator_intro_shown: false,
        }
    }
}

impl ShellPreferences {
    const FILE_NAME: &'static str = "desktop-shell.json";

    pub fn load(paths: &MCPMatePaths) -> Result<Self> {
        let path = Self::path(paths);
        let prefs = if path.exists() {
            let data =
                fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?;
            let mut parsed: ShellPreferences = serde_json::from_slice(&data)
                .with_context(|| format!("failed to parse {}", path.display()))?;
            parsed.apply_constraints();
            parsed
        } else {
            ShellPreferences::default()
        };
        Ok(prefs)
    }

    pub fn save_to_path(path: &Path, prefs: &ShellPreferences) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create parent directory {}", parent.display())
            })?;
        }
        let mut copy = prefs.clone();
        copy.apply_constraints();
        let payload = serde_json::to_vec_pretty(&copy)
            .context("failed to encode desktop shell preferences")?;
        fs::write(path, payload).with_context(|| format!("failed to write {}", path.display()))?;
        Ok(())
    }

    pub fn path(paths: &MCPMatePaths) -> PathBuf {
        paths.base_dir().join("config").join(Self::FILE_NAME)
    }

    pub fn apply_constraints(&mut self) {
        if !self.show_dock_icon {
            self.menu_bar_icon_mode = MenuBarIconMode::Runtime;
        }
    }
}

#[derive(Default)]
struct ShellRuntimeState {
    preferences: ShellPreferences,
    prefs_path: PathBuf,
    pending_full_board_path: Option<String>,
    tray: Option<TrayIcon<Wry>>,
    status_item: Option<MenuItem<Wry>>,
    start_item: Option<MenuItem<Wry>>,
    restart_item: Option<MenuItem<Wry>>,
    stop_item: Option<MenuItem<Wry>>,
    service_running: bool,
}

#[derive(Clone)]
pub struct ShellState {
    inner: Arc<AsyncMutex<ShellRuntimeState>>,
}

impl ShellState {
    pub fn new(preferences: ShellPreferences, prefs_path: PathBuf) -> Self {
        let mut state = ShellRuntimeState {
            preferences,
            prefs_path,
            ..ShellRuntimeState::default()
        };
        state.preferences.apply_constraints();
        Self {
            inner: Arc::new(AsyncMutex::new(state)),
        }
    }

    pub async fn current_preferences(&self) -> ShellPreferences {
        self.inner.lock().await.preferences.clone()
    }

    pub async fn set_pending_full_board_path(&self, path: String) {
        self.inner.lock().await.pending_full_board_path = Some(path);
    }

    pub async fn clear_pending_full_board_path(&self) {
        self.inner.lock().await.pending_full_board_path = None;
    }

    pub async fn take_pending_full_board_path(&self) -> Option<String> {
        self.inner.lock().await.pending_full_board_path.take()
    }

    pub async fn operator_intro_shown(&self) -> bool {
        self.inner.lock().await.preferences.operator_intro_shown
    }

    pub async fn mark_operator_intro_shown(&self) -> Result<()> {
        let mut guard = self.inner.lock().await;
        guard.preferences.operator_intro_shown = true;
        ShellPreferences::save_to_path(&guard.prefs_path, &guard.preferences)
    }

    pub async fn register_tray(
        &self,
        tray: TrayIcon<Wry>,
        status_item: MenuItem<Wry>,
        start_item: MenuItem<Wry>,
        restart_item: MenuItem<Wry>,
        stop_item: MenuItem<Wry>,
    ) -> Result<()> {
        {
            let mut guard = self.inner.lock().await;
            guard.tray = Some(tray.clone());
            guard.status_item = Some(status_item.clone());
            guard.start_item = Some(start_item.clone());
            guard.restart_item = Some(restart_item.clone());
            guard.stop_item = Some(stop_item.clone());
            Self::update_service_menu_items(
                &status_item,
                &start_item,
                &restart_item,
                &stop_item,
                guard.service_running,
                "Unknown",
            )?;
            let prefs = guard.preferences.clone();
            drop(guard);

            let visible = Self::should_show_icon(&prefs);
            if visible {
                let icon = tray_template_icon();
                set_tray_icon_with_template(&tray, icon)
                    .context("failed to set tray icon during registration")?;
                tray.set_visible(true)
                    .context("failed to show tray icon during registration")?;
            } else {
                tray.set_icon(None)
                    .context("failed to clear tray icon during registration")?;
                tray.set_visible(false)
                    .context("failed to hide tray icon during registration")?;
            }
        }
        Ok(())
    }

    pub async fn apply_preferences(
        &self,
        app_handle: &AppHandle<Wry>,
        mut prefs: ShellPreferences,
    ) -> Result<()> {
        prefs.apply_constraints();

        let (path, tray, prev_show_dock_icon) = {
            let mut guard = self.inner.lock().await;
            let prev_show_dock_icon = guard.preferences.show_dock_icon;
            guard.preferences = prefs.clone();
            (
                guard.prefs_path.clone(),
                guard.tray.clone(),
                prev_show_dock_icon,
            )
        };

        if prev_show_dock_icon != prefs.show_dock_icon {
            apply_activation_policy(app_handle, &prefs)?;
        }

        if let Some(tray) = tray {
            Self::apply_tray_visibility(&tray, &prefs)?;
        }

        ShellPreferences::save_to_path(&path, &prefs)?;
        Ok(())
    }

    pub async fn update_service_status(&self, running: bool, label: &str) -> Result<()> {
        let (tray, status_item, start_item, restart_item, stop_item, prefs) = {
            let mut guard = self.inner.lock().await;
            guard.service_running = running;
            (
                guard.tray.clone(),
                guard.status_item.clone(),
                guard.start_item.clone(),
                guard.restart_item.clone(),
                guard.stop_item.clone(),
                guard.preferences.clone(),
            )
        };

        if let (Some(status_item), Some(start_item), Some(restart_item), Some(stop_item)) =
            (status_item, start_item, restart_item, stop_item)
        {
            Self::update_service_menu_items(
                &status_item,
                &start_item,
                &restart_item,
                &stop_item,
                running,
                label,
            )?;
        }

        if let Some(tray) = tray {
            Self::apply_tray_visibility(&tray, &prefs)?;
        }

        Ok(())
    }

    fn should_show_icon(prefs: &ShellPreferences) -> bool {
        if !prefs.show_dock_icon {
            return true;
        }

        match prefs.menu_bar_icon_mode {
            MenuBarIconMode::Runtime => true,
            MenuBarIconMode::Hidden => false,
        }
    }

    fn apply_tray_visibility(tray: &TrayIcon<Wry>, prefs: &ShellPreferences) -> Result<()> {
        let visible = Self::should_show_icon(prefs);
        if visible {
            let icon = tray_template_icon();
            set_tray_icon_with_template(tray, icon)
                .context("failed to set tray icon when enabling menu bar icon")?;
            tray.set_visible(true).context("failed to show tray icon")?;
        } else {
            tray.set_icon(None)
                .context("failed to clear tray icon when hiding menu bar icon")?;
            tray.set_visible(false)
                .context("failed to hide tray icon")?;
        }
        Ok(())
    }

    fn update_service_menu_items(
        status_item: &MenuItem<Wry>,
        start_item: &MenuItem<Wry>,
        restart_item: &MenuItem<Wry>,
        stop_item: &MenuItem<Wry>,
        running: bool,
        label: &str,
    ) -> Result<()> {
        status_item
            .set_text(format!("Local Core: {label}"))
            .context("failed to update tray service status label")?;
        start_item
            .set_enabled(!running)
            .context("failed to update tray start item state")?;
        restart_item
            .set_enabled(running)
            .context("failed to update tray restart item state")?;
        stop_item
            .set_enabled(running)
            .context("failed to update tray stop item state")?;
        Ok(())
    }
}

pub fn validate_full_board_path(path: &str) -> Result<String> {
    if path.is_empty() {
        anyhow::bail!("full board path must not be empty");
    }
    if path != path.trim() {
        anyhow::bail!("full board path must not contain surrounding whitespace");
    }
    if !path.starts_with('/') {
        anyhow::bail!("full board path must start with /");
    }
    if path.starts_with("//") {
        anyhow::bail!("full board path must be an app route, not a protocol-relative URL");
    }

    let route_path = path
        .split(['?', '#'])
        .next()
        .unwrap_or(path)
        .trim_end_matches('/');
    if route_path == "/operator" || route_path.starts_with("/operator/") {
        anyhow::bail!("full board path must not target the operator window route");
    }

    Ok(path.to_string())
}

pub fn remember_tray_icon_rect(rect: Rect) {
    *LAST_TRAY_ICON_RECT
        .get_or_init(|| Mutex::new(None))
        .lock()
        .expect("tray icon rect state poisoned") = Some(rect);
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct MonitorBounds {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}

fn physical_point(position: &tauri::Position) -> (f64, f64) {
    match position {
        tauri::Position::Physical(point) => (f64::from(point.x), f64::from(point.y)),
        tauri::Position::Logical(point) => (point.x, point.y),
    }
}

fn physical_dimensions(size: &tauri::Size) -> (f64, f64) {
    match size {
        tauri::Size::Physical(size) => (f64::from(size.width), f64::from(size.height)),
        tauri::Size::Logical(size) => (size.width, size.height),
    }
}

fn monitor_bounds_from_physical_parts(
    position: &PhysicalPosition<i32>,
    size: &tauri::PhysicalSize<u32>,
) -> MonitorBounds {
    MonitorBounds {
        left: f64::from(position.x),
        top: f64::from(position.y),
        right: f64::from(position.x) + f64::from(size.width),
        bottom: f64::from(position.y) + f64::from(size.height),
    }
}

fn monitor_bounds_containing_point(
    monitors: &[MonitorBounds],
    x: f64,
    y: f64,
) -> Option<MonitorBounds> {
    monitors.iter().copied().find(|monitor| {
        x >= monitor.left && x < monitor.right && y >= monitor.top && y < monitor.bottom
    })
}

fn compute_operator_window_position(
    tray_rect: Rect,
    panel_width: f64,
    panel_height: f64,
    monitor: MonitorBounds,
) -> PhysicalPosition<i32> {
    let (tray_x, tray_y) = physical_point(&tray_rect.position);
    let (tray_w, tray_h) = physical_dimensions(&tray_rect.size);
    let tray_center_x = tray_x + tray_w / 2.0;

    let mut x = tray_center_x - panel_width / 2.0;
    let mut y = tray_y + tray_h + OPERATOR_PANEL_TRAY_GAP;

    if y + panel_height > monitor.bottom {
        y = tray_y - panel_height - OPERATOR_PANEL_TRAY_GAP;
    }

    x = x.clamp(
        monitor.left,
        (monitor.right - panel_width).max(monitor.left),
    );
    y = y.clamp(
        monitor.top,
        (monitor.bottom - panel_height).max(monitor.top),
    );

    PhysicalPosition::new(x.round() as i32, y.round() as i32)
}

fn resolve_tray_icon_rect<M>(manager: &M) -> Option<Rect>
where
    M: Manager<Wry>,
{
    if let Some(rect) = LAST_TRAY_ICON_RECT
        .get()
        .and_then(|state| state.lock().ok())
        .and_then(|guard| *guard)
    {
        return Some(rect);
    }

    manager
        .app_handle()
        .tray_by_id(TRAY_ID)
        .and_then(|tray| tray.rect().ok().flatten())
}

fn resolve_operator_monitor_bounds(
    window: &WebviewWindow<Wry>,
    tray_rect: Rect,
) -> Result<MonitorBounds> {
    let (tray_x, tray_y) = physical_point(&tray_rect.position);
    let (tray_w, tray_h) = physical_dimensions(&tray_rect.size);
    let tray_center_x = tray_x + tray_w / 2.0;
    let tray_center_y = tray_y + tray_h / 2.0;
    let available_monitor_bounds = window
        .available_monitors()?
        .iter()
        .map(|monitor| monitor_bounds_from_physical_parts(monitor.position(), monitor.size()))
        .collect::<Vec<_>>();

    if let Some(bounds) =
        monitor_bounds_containing_point(&available_monitor_bounds, tray_center_x, tray_center_y)
    {
        return Ok(bounds);
    }

    let monitor = window
        .current_monitor()?
        .or_else(|| window.primary_monitor().ok().flatten())
        .context("failed to resolve monitor for operator panel placement")?;

    Ok(monitor_bounds_from_physical_parts(
        monitor.position(),
        monitor.size(),
    ))
}

fn position_operator_window_near_tray<M>(manager: &M, window: &WebviewWindow<Wry>) -> Result<()>
where
    M: Manager<Wry>,
{
    let Some(tray_rect) = resolve_tray_icon_rect(manager) else {
        return Ok(());
    };

    let monitor_bounds = resolve_operator_monitor_bounds(window, tray_rect)?;
    let outer_size = window.outer_size()?;
    let position = compute_operator_window_position(
        tray_rect,
        f64::from(outer_size.width),
        f64::from(outer_size.height),
        monitor_bounds,
    );

    window
        .set_position(position)
        .context("failed to position operator panel near tray")?;
    Ok(())
}

pub fn apply_activation_policy(
    app_handle: &AppHandle<Wry>,
    prefs: &ShellPreferences,
) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        use tauri::ActivationPolicy;
        let policy = if prefs.show_dock_icon {
            ActivationPolicy::Regular
        } else {
            ActivationPolicy::Accessory
        };
        app_handle
            .set_activation_policy(policy)
            .context("failed to update macOS activation policy")?;
        if prefs.show_dock_icon {
            let _ = app_handle.show();
        } else {
            let _ = app_handle.hide();
            if let Some(window) = app_handle.get_webview_window("main") {
                let _ = window.show();
            }
        }
    }
    Ok(())
}

pub fn initialize_app_menu(app: &mut tauri::App) -> Result<()> {
    let app_handle = app.handle();

    let menu = Menu::default(app_handle)?;

    let about_item = MenuItem::with_id(app, APP_MENU_ABOUT, "About MCPMate", true, None::<&str>)?;
    let check_updates_item = MenuItem::with_id(
        app,
        APP_MENU_CHECK_UPDATES,
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

    #[cfg(debug_assertions)]
    let init_script = String::from(
        r#"window.__MCPMATE_IS_TAURI__ = true;
        "#,
    );

    #[cfg(not(debug_assertions))]
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

    #[cfg(any(debug_assertions, feature = "devtools"))]
    window.open_devtools();

    #[cfg(target_os = "macos")]
    {
        let _ = manager.app_handle().show();
    }
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

pub fn ensure_window_visibility<M>(manager: &M) -> Result<()>
where
    M: Manager<Wry>,
{
    if let Some(window) = manager.get_webview_window("main") {
        window.show().context("failed to show main window")?;
        window.set_focus().context("failed to focus main window")?;
        return Ok(());
    }

    spawn_main_window(manager)?;

    if let Some(window) = manager.get_webview_window("main") {
        #[cfg(target_os = "macos")]
        {
            let _ = manager.app_handle().show();
        }
        window
            .show()
            .context("failed to show spawned main window")?;
        window
            .set_focus()
            .context("failed to focus spawned main window")?;
    }

    Ok(())
}

pub fn spawn_operator_window<M>(manager: &M) -> Result<()>
where
    M: Manager<Wry>,
{
    if manager.get_webview_window(OPERATOR_WINDOW_LABEL).is_some() {
        return Ok(());
    }

    let app_handle = manager.app_handle().clone();
    let mut builder = WebviewWindowBuilder::new(
        manager,
        OPERATOR_WINDOW_LABEL,
        WebviewUrl::App("/operator".into()),
    )
    .title("MCPMate Operator")
    .inner_size(OPERATOR_PANEL_WIDTH, OPERATOR_PANEL_DEFAULT_HEIGHT)
    .min_inner_size(OPERATOR_PANEL_WIDTH, OPERATOR_PANEL_MIN_HEIGHT)
    .max_inner_size(OPERATOR_PANEL_WIDTH, OPERATOR_PANEL_MAX_HEIGHT)
    .resizable(true)
    .maximizable(false)
    .decorations(false)
    .transparent(true)
    .visible(false)
    .skip_taskbar(true)
    .disable_drag_drop_handler()
    .background_color(OPERATOR_PANEL_TRANSPARENT_BACKGROUND);

    #[cfg(debug_assertions)]
    let init_script =
        format!("window.__MCPMATE_IS_TAURI__ = true;\n{OPERATOR_WINDOW_INIT_SCRIPT}",);

    #[cfg(not(debug_assertions))]
    let init_script = format!(
        r#"window.addEventListener('contextmenu', (event) => {{
            if (event.metaKey || event.ctrlKey) {{
                return;
            }}
            event.preventDefault();
        }});
        window.__MCPMATE_IS_TAURI__ = true;
        {OPERATOR_WINDOW_INIT_SCRIPT}"#,
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
                        "Failed to open external link from operator webview"
                    );
                }
                NewWindowResponse::Deny
            }
            "tauri" | "app" | "about" | "mcpmate" | "" => NewWindowResponse::Allow,
            other => {
                warn!(target_url = %url, scheme = other, "Blocked unsupported operator window.open URL scheme");
                NewWindowResponse::Deny
            }
        }
    });

    let window = builder.build()?;

    window
        .set_maximizable(false)
        .context("failed to disable operator panel maximize")?;

    window
        .set_background_color(Some(OPERATOR_PANEL_TRANSPARENT_BACKGROUND))
        .context("failed to set operator panel transparent background")?;

    operator_window::apply_operator_window_chrome(&window)
        .context("failed to apply operator panel window chrome")?;

    #[cfg(any(debug_assertions, feature = "devtools"))]
    window.open_devtools();

    Ok(())
}

pub fn ensure_operator_window_visibility<M>(manager: &M) -> Result<()>
where
    M: Manager<Wry>,
{
    show_operator_window_from_tray(manager)
}

pub async fn show_operator_intro_once<M>(manager: &M, shell_state: &ShellState) -> Result<bool>
where
    M: Manager<Wry>,
{
    if shell_state.operator_intro_shown().await {
        return Ok(false);
    }

    ensure_operator_window_visibility(manager)?;
    shell_state.mark_operator_intro_shown().await?;
    Ok(true)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OperatorPanelHideRequest {
    Attached,
}

fn should_hide_operator_panel(pinned: bool, request: OperatorPanelHideRequest) -> bool {
    match request {
        OperatorPanelHideRequest::Attached => !pinned,
    }
}

fn should_auto_hide_operator_panel_on_focus_change(
    window_label: &str,
    focused: bool,
    pinned: bool,
) -> bool {
    window_label == OPERATOR_WINDOW_LABEL
        && !focused
        && should_hide_operator_panel(pinned, OperatorPanelHideRequest::Attached)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OperatorPanelToggleAction {
    Show,
    Hide,
    KeepHidden,
}

fn operator_tray_toggle_action(
    is_visible: bool,
    pinned: bool,
    hidden_by_recent_focus_loss: bool,
) -> OperatorPanelToggleAction {
    if is_visible && should_hide_operator_panel(pinned, OperatorPanelHideRequest::Attached) {
        OperatorPanelToggleAction::Hide
    } else if !is_visible && hidden_by_recent_focus_loss {
        OperatorPanelToggleAction::KeepHidden
    } else {
        OperatorPanelToggleAction::Show
    }
}

fn show_operator_window_from_tray<M>(manager: &M) -> Result<()>
where
    M: Manager<Wry>,
{
    spawn_operator_window(manager)?;
    if let Some(window) = manager.get_webview_window(OPERATOR_WINDOW_LABEL) {
        position_operator_window_near_tray(manager, &window)?;
        clear_operator_focus_loss_dismissal();
        operator_window::apply_operator_window_chrome(&window)
            .context("failed to refresh operator panel window chrome")?;
        window.show().context("failed to show operator panel")?;
        window
            .set_focus()
            .context("failed to focus operator panel")?;
    }
    Ok(())
}

fn operator_focus_loss_dismissal_at() -> &'static Mutex<Option<Instant>> {
    OPERATOR_PANEL_FOCUS_LOSS_DISMISSAL_AT.get_or_init(|| Mutex::new(None))
}

fn mark_operator_focus_loss_dismissal(now: Instant) {
    *operator_focus_loss_dismissal_at()
        .lock()
        .expect("operator focus-loss dismissal state poisoned") = Some(now);
}

fn clear_operator_focus_loss_dismissal() {
    *operator_focus_loss_dismissal_at()
        .lock()
        .expect("operator focus-loss dismissal state poisoned") = None;
}

fn take_recent_operator_focus_loss_dismissal(now: Instant) -> bool {
    let hidden_at = operator_focus_loss_dismissal_at()
        .lock()
        .expect("operator focus-loss dismissal state poisoned")
        .take();

    hidden_at
        .and_then(|hidden_at| now.checked_duration_since(hidden_at))
        .is_some_and(|elapsed| elapsed <= OPERATOR_PANEL_TRAY_FOCUS_LOSS_GRACE)
}

pub fn toggle_operator_window_visibility<M>(manager: &M) -> Result<()>
where
    M: Manager<Wry>,
{
    if let Some(window) = manager.get_webview_window(OPERATOR_WINDOW_LABEL) {
        let is_visible = window
            .is_visible()
            .context("failed to read operator panel visibility")?;
        let is_pinned = window
            .is_always_on_top()
            .context("failed to read operator panel pin state")?;

        let hidden_by_recent_focus_loss =
            !is_visible && take_recent_operator_focus_loss_dismissal(Instant::now());
        if is_visible {
            clear_operator_focus_loss_dismissal();
        }
        match operator_tray_toggle_action(is_visible, is_pinned, hidden_by_recent_focus_loss) {
            OperatorPanelToggleAction::Hide => {
                window.hide().context("failed to hide operator panel")?;
                return Ok(());
            }
            OperatorPanelToggleAction::KeepHidden => return Ok(()),
            OperatorPanelToggleAction::Show => {
                return show_operator_window_from_tray(manager);
            }
        }
    }

    show_operator_window_from_tray(manager)
}

pub fn hide_operator_window<M>(manager: &M) -> Result<()>
where
    M: Manager<Wry>,
{
    if let Some(window) = manager.get_webview_window(OPERATOR_WINDOW_LABEL) {
        clear_operator_focus_loss_dismissal();
        window.hide().context("failed to hide operator panel")?;
    }
    Ok(())
}

pub fn hide_operator_window_for_attached_close(window: &Window<Wry>) -> Result<bool> {
    if window.label() != OPERATOR_WINDOW_LABEL {
        return Ok(false);
    }

    let is_pinned = window
        .is_always_on_top()
        .context("failed to read operator panel pin state")?;
    if should_hide_operator_panel(is_pinned, OperatorPanelHideRequest::Attached) {
        clear_operator_focus_loss_dismissal();
        window.hide().context("failed to hide operator panel")?;
    }

    Ok(true)
}

pub fn hide_operator_window_on_focus_change(window: &Window<Wry>, focused: bool) -> Result<()> {
    if window.label() != OPERATOR_WINDOW_LABEL || focused {
        return Ok(());
    }

    let is_pinned = window
        .is_always_on_top()
        .context("failed to read operator panel pin state")?;
    if should_auto_hide_operator_panel_on_focus_change(window.label(), focused, is_pinned) {
        window.hide().context("failed to hide operator panel")?;
        mark_operator_focus_loss_dismissal(Instant::now());
    }

    Ok(())
}

pub fn set_operator_window_pinned<M>(manager: &M, pinned: bool) -> Result<()>
where
    M: Manager<Wry>,
{
    spawn_operator_window(manager)?;
    if let Some(window) = manager.get_webview_window(OPERATOR_WINDOW_LABEL) {
        window
            .set_always_on_top(pinned)
            .context("failed to update operator panel pin state")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri::{PhysicalPosition, PhysicalSize, Position, Size};

    #[test]
    fn operator_attached_close_keeps_pinned_panel_visible() {
        assert!(!should_hide_operator_panel(
            true,
            OperatorPanelHideRequest::Attached
        ));
    }

    #[test]
    fn operator_focus_loss_auto_hides_unpinned_operator_panel() {
        assert!(should_auto_hide_operator_panel_on_focus_change(
            OPERATOR_WINDOW_LABEL,
            false,
            false
        ));
    }

    #[test]
    fn operator_focus_loss_keeps_pinned_operator_panel_visible() {
        assert!(!should_auto_hide_operator_panel_on_focus_change(
            OPERATOR_WINDOW_LABEL,
            false,
            true
        ));
    }

    #[test]
    fn operator_focus_gain_does_not_auto_hide() {
        assert!(!should_auto_hide_operator_panel_on_focus_change(
            OPERATOR_WINDOW_LABEL,
            true,
            false
        ));
    }

    #[test]
    fn main_window_focus_loss_does_not_auto_hide_operator_panel() {
        assert!(!should_auto_hide_operator_panel_on_focus_change(
            "main", false, false
        ));
    }

    #[test]
    fn operator_tray_toggle_keeps_hidden_after_focus_loss_dismissal() {
        assert_eq!(
            operator_tray_toggle_action(false, false, true),
            OperatorPanelToggleAction::KeepHidden
        );
    }

    #[test]
    fn operator_tray_toggle_shows_hidden_without_focus_loss_dismissal() {
        assert_eq!(
            operator_tray_toggle_action(false, false, false),
            OperatorPanelToggleAction::Show
        );
    }

    #[test]
    fn operator_tray_toggle_keeps_pinned_panel_visible() {
        assert_eq!(
            operator_tray_toggle_action(true, true, false),
            OperatorPanelToggleAction::Show
        );
    }

    #[test]
    fn full_board_path_rejects_relative_paths() {
        assert!(validate_full_board_path("clients").is_err());
    }

    #[test]
    fn full_board_path_rejects_operator_route() {
        assert!(validate_full_board_path("/operator").is_err());
    }

    #[test]
    fn operator_window_positions_below_tray_icon() {
        let tray_rect = Rect {
            position: Position::Physical(PhysicalPosition::new(900, 10)),
            size: Size::Physical(PhysicalSize::new(24, 24)),
        };
        let position = compute_operator_window_position(
            tray_rect,
            420.0,
            640.0,
            MonitorBounds {
                left: 0.0,
                top: 0.0,
                right: 1440.0,
                bottom: 900.0,
            },
        );

        assert_eq!(position.x, 702);
        assert_eq!(position.y, 42);
    }

    #[test]
    fn operator_window_flips_above_tray_when_bottom_overflows() {
        let tray_rect = Rect {
            position: Position::Physical(PhysicalPosition::new(900, 860)),
            size: Size::Physical(PhysicalSize::new(24, 24)),
        };
        let position = compute_operator_window_position(
            tray_rect,
            420.0,
            640.0,
            MonitorBounds {
                left: 0.0,
                top: 0.0,
                right: 1440.0,
                bottom: 900.0,
            },
        );

        assert_eq!(position.x, 702);
        assert_eq!(position.y, 212);
    }

    #[test]
    fn operator_monitor_bounds_uses_monitor_containing_tray_icon() {
        let monitors = [
            MonitorBounds {
                left: 0.0,
                top: 0.0,
                right: 1440.0,
                bottom: 900.0,
            },
            MonitorBounds {
                left: 1440.0,
                top: 0.0,
                right: 2880.0,
                bottom: 900.0,
            },
        ];

        assert_eq!(
            monitor_bounds_containing_point(&monitors, 1800.0, 20.0),
            Some(monitors[1])
        );
    }

    #[tokio::test]
    async fn pending_full_board_path_round_trips_once() {
        let state = ShellState::new(ShellPreferences::default(), PathBuf::from("prefs.json"));

        state
            .set_pending_full_board_path("/clients".to_string())
            .await;

        assert_eq!(
            state.take_pending_full_board_path().await,
            Some("/clients".to_string())
        );
        assert_eq!(state.take_pending_full_board_path().await, None);
    }
}
