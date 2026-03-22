use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result};
use mcpmate::common::MCPMatePaths;
use serde::{Deserialize, Serialize};
use tauri::{App, AppHandle, Manager, Wry, image::Image, menu::MenuItem, tray::TrayIcon};
use tokio::sync::Mutex as AsyncMutex;

pub const TRAY_ID: &str = "mcpmate.tray.main";
pub const MENU_OPEN_MAIN: &str = "mcpmate.tray.open_main";
pub const MENU_TOGGLE_SERVICE: &str = "mcpmate.tray.toggle_service";
pub const MENU_OPEN_SETTINGS: &str = "mcpmate.tray.open_settings";
pub const MENU_SHOW_ABOUT: &str = "mcpmate.tray.show_about";
pub const MENU_QUIT: &str = "mcpmate.tray.quit";

pub const EVENT_OPEN_MAIN: &str = "mcpmate://open-main";
pub const EVENT_OPEN_SETTINGS: &str = "mcpmate://open-settings";

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
    toggle_item: Option<MenuItem<Wry>>,
    backend_running: bool,
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

    pub async fn is_backend_running(&self) -> bool {
        self.inner.lock().await.backend_running
    }

    pub async fn register_tray(
        &self,
        tray: TrayIcon<Wry>,
        toggle_item: MenuItem<Wry>,
    ) -> Result<()> {
        {
            let mut guard = self.inner.lock().await;
            guard.tray = Some(tray.clone());
            guard.toggle_item = Some(toggle_item.clone());
            Self::update_toggle_item_label(&toggle_item, guard.backend_running)?;
            let prefs = guard.preferences.clone();
            let backend_running = guard.backend_running;
            drop(guard);

            let visible = Self::should_show_icon(&prefs, backend_running);
            if visible {
                if let Some(icon) = tray.app_handle().default_window_icon().cloned() {
                    tray.set_icon(Some(icon))
                        .context("failed to set tray icon during registration")?;
                }
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

        let (path, tray, backend_running) = {
            let mut guard = self.inner.lock().await;
            guard.preferences = prefs.clone();
            (
                guard.prefs_path.clone(),
                guard.tray.clone(),
                guard.backend_running,
            )
        };

        apply_activation_policy(app_handle, &prefs)?;

        if let Some(tray) = tray {
            Self::apply_tray_visibility(&tray, app_handle, &prefs, backend_running)?;
        }

        ShellPreferences::save_to_path(&path, &prefs)?;
        Ok(())
    }

    pub async fn update_backend_running(&self, running: bool) -> Result<()> {
        let (tray, toggle_item, prefs) = {
            let mut guard = self.inner.lock().await;
            guard.backend_running = running;
            (
                guard.tray.clone(),
                guard.toggle_item.clone(),
                guard.preferences.clone(),
            )
        };

        if let Some(item) = toggle_item {
            Self::update_toggle_item_label(&item, running)?;
        }

        if let Some(tray) = tray {
            Self::apply_tray_visibility(&tray, tray.app_handle(), &prefs, running)?;
        }

        Ok(())
    }

    fn should_show_icon(prefs: &ShellPreferences, _backend_running: bool) -> bool {
        if !prefs.show_dock_icon {
            return true;
        }

        match prefs.menu_bar_icon_mode {
            MenuBarIconMode::Runtime => true,
            MenuBarIconMode::Hidden => false,
        }
    }

    fn apply_tray_visibility(
        tray: &TrayIcon<Wry>,
        app_handle: &AppHandle<Wry>,
        prefs: &ShellPreferences,
        backend_running: bool,
    ) -> Result<()> {
        let visible = Self::should_show_icon(prefs, backend_running);
        if visible {
            if let Some(icon) = app_handle.default_window_icon().cloned() {
                tray.set_icon(Some(icon))
                    .context("failed to set tray icon when enabling menu bar icon")?;
            }
            tray.set_visible(true).context("failed to show tray icon")?;
        } else {
            tray.set_icon(None)
                .context("failed to clear tray icon when hiding menu bar icon")?;
            tray.set_visible(false)
                .context("failed to hide tray icon")?;
        }
        Ok(())
    }

    fn update_toggle_item_label(item: &MenuItem<Wry>, running: bool) -> Result<()> {
        let text = if running {
            "Stop Service"
        } else {
            "Start Service"
        };
        item.set_text(text)
            .context("failed to update tray toggle item text")
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

    super::spawn_main_window(manager)
}

pub fn tray_icon_image(app: &App) -> Result<Image<'static>> {
    if let Some(icon) = app.default_window_icon() {
        Ok(icon.clone().to_owned())
    } else {
        Err(anyhow::anyhow!(
            "desktop bundle missing default window icon"
        ))
    }
}
