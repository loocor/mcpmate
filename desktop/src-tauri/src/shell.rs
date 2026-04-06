use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
};

use anyhow::{Context, Result};
use mcpmate::common::MCPMatePaths;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Wry, image::Image, menu::MenuItem, tray::TrayIcon};
use tokio::sync::Mutex as AsyncMutex;

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
pub const MENU_SERVICE_STATUS: &str = "mcpmate.tray.service_status";
pub const MENU_START_SERVICE: &str = "mcpmate.tray.start_service";
pub const MENU_RESTART_SERVICE: &str = "mcpmate.tray.restart_service";
pub const MENU_STOP_SERVICE: &str = "mcpmate.tray.stop_service";
pub const MENU_OPEN_SETTINGS: &str = "mcpmate.tray.open_settings";
pub const MENU_SHOW_ABOUT: &str = "mcpmate.tray.show_about";
pub const MENU_QUIT: &str = "mcpmate.tray.quit";

pub const EVENT_OPEN_SETTINGS: &str = "mcpmate://open-settings";
pub const EVENT_CORE_STATE_CHANGED: &str = "mcpmate://core/status-changed";

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
}

impl Default for ShellPreferences {
    fn default() -> Self {
        Self {
            menu_bar_icon_mode: MenuBarIconMode::Runtime,
            show_dock_icon: true,
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

pub fn ensure_window_visibility<M>(manager: &M) -> Result<()>
where
    M: Manager<Wry>,
{
    if let Some(window) = manager.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }

    super::spawn_main_window(manager)?;

    if let Some(window) = manager.get_webview_window("main") {
        let _ = manager.app_handle().show();
        let _ = window.show();
        let _ = window.set_focus();
    }

    Ok(())
}
