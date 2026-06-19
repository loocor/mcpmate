//! Routes registered `mcpmate://` URLs (OAuth, extension-driven server import).

use std::sync::Arc;

#[cfg(target_os = "linux")]
use std::process::Command;

#[cfg(target_os = "linux")]
use anyhow::Context;
use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
#[cfg(target_os = "linux")]
use tauri_plugin_deep_link::DeepLinkExt;
use tokio::sync::Mutex as AsyncMutex;
use tracing::{info, warn};

const IMPORT_SERVER_MAX_DECODED_BYTES: usize = 65_536;

#[derive(Clone, Default)]
pub(crate) struct DeepLinkState {
    pending_server_import: Arc<AsyncMutex<Option<ImportServerDeepLinkPayload>>>,
}

impl DeepLinkState {
    pub(crate) async fn set_pending_server_import(&self, payload: ImportServerDeepLinkPayload) {
        let mut guard = self.pending_server_import.lock().await;
        *guard = Some(payload);
    }

    pub(crate) async fn take_pending_server_import(&self) -> Option<ImportServerDeepLinkPayload> {
        let mut guard = self.pending_server_import.lock().await;
        guard.take()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportServerDeepLinkPayload {
    pub text: String,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub source: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FrontendImportDiagnosticPayload {
    stage: String,
    handled: bool,
    has_payload: bool,
    has_text: bool,
    text_len: Option<usize>,
    has_format: bool,
    has_source: bool,
    current_path: Option<String>,
}

#[tauri::command]
pub(crate) async fn mcp_deep_link_take_pending_server_import(
    state: State<'_, DeepLinkState>,
) -> Result<Option<ImportServerDeepLinkPayload>, String> {
    let payload = state.take_pending_server_import().await;
    info!(
        has_payload = payload.is_some(),
        text_len = payload.as_ref().map(|payload| payload.text.len()),
        has_format = payload
            .as_ref()
            .is_some_and(|payload| payload.format.is_some()),
        has_source = payload
            .as_ref()
            .is_some_and(|payload| payload.source.is_some()),
        "Frontend requested pending import/server deep-link payload"
    );
    Ok(payload)
}

#[tauri::command]
pub(crate) fn mcp_deep_link_log_frontend_import(
    payload: FrontendImportDiagnosticPayload,
) -> Result<(), String> {
    info!(
        stage = %payload.stage,
        handled = payload.handled,
        has_payload = payload.has_payload,
        has_text = payload.has_text,
        text_len = payload.text_len,
        has_format = payload.has_format,
        has_source = payload.has_source,
        current_path = %payload.current_path.as_deref().unwrap_or("unknown"),
        "Frontend import/server deep-link diagnostic"
    );
    Ok(())
}

pub fn dispatch_mcpmate_deep_link(app: &AppHandle, url: &str, context: &'static str) {
    let handle = app.clone();
    let url = url.to_string();
    let sanitized_url = crate::utils::sanitize_url_for_logging(&url);
    info!(
        context,
        target_url = %sanitized_url,
        "Dispatching MCPMate deep link"
    );
    tauri::async_runtime::spawn(async move {
        if let Err(err) = route_mcpmate_deep_link(&handle, url.as_str()).await {
            let sanitized_url = crate::utils::sanitize_url_for_logging(&url);
            warn!(error = %err, target_url = %sanitized_url, context, "Failed to handle mcpmate deep link");
        }
    });
}

/// Dispatch `mcpmate://auth`, `mcpmate://import/server`, etc.
pub async fn route_mcpmate_deep_link(app: &AppHandle, url_str: &str) -> Result<(), String> {
    let sanitized_url = crate::utils::sanitize_url_for_logging(url_str);
    let parsed = url::Url::parse(url_str).map_err(|e| {
        warn!(error = %e, target_url = %sanitized_url, "Failed to parse desktop deep link");
        e.to_string()
    })?;
    let scheme = parsed.scheme();
    let host = parsed.host_str().unwrap_or("none");
    let path = parsed.path();
    info!(
        target_url = %sanitized_url,
        scheme,
        host,
        path,
        "Routing desktop deep link"
    );

    if parsed.scheme() != "mcpmate" {
        info!(scheme = parsed.scheme(), "Ignoring non-MCPMate deep link");
        return Ok(());
    }

    match parsed.host_str() {
        Some("auth") => crate::account::handle_oauth_url(app, url_str),
        Some("import") => handle_import_path(app, &parsed).await,
        other => {
            info!(
                host = other.unwrap_or("none"),
                "Ignoring unsupported MCPMate deep link host"
            );
            Ok(())
        }
    }
}

async fn handle_import_path(app: &AppHandle, parsed: &url::Url) -> Result<(), String> {
    let path = parsed.path().trim_end_matches('/');
    if path != "/server" {
        info!(path, "Ignoring unsupported MCPMate import deep link path");
        return Ok(());
    }

    let encoded = parsed
        .query_pairs()
        .find(|(k, _)| k == "p")
        .map(|(_, v)| v.into_owned())
        .ok_or_else(|| "import/server deep link missing \"p\" query".to_string())?;

    let payload = decode_import_server_payload(&encoded).map_err(|err| {
        warn!(
            error = %err,
            encoded_len = encoded.len(),
            "Failed to decode import/server deep link payload"
        );
        err
    })?;
    info!(
        encoded_len = encoded.len(),
        text_len = payload.text.len(),
        has_format = payload.format.is_some(),
        has_source = payload.source.is_some(),
        "Decoded import/server deep link payload"
    );

    // Persist the payload so frontend can pull it during cold start even if the
    // first event dispatch happens before React listeners are mounted.
    if let Some(state) = app.try_state::<DeepLinkState>() {
        state.set_pending_server_import(payload.clone()).await;
        info!("Stored pending import/server deep link payload");
    } else {
        warn!("DeepLinkState is unavailable while routing import/server deep link");
    }

    // Ensure a visible, focused main window before dispatching the import event.
    // This allows extension-triggered deep links to reliably wake MCPMate from
    // hidden/closed-window states on desktop shells.
    crate::shell::ensure_window_visibility(app).map_err(|e| {
        warn!(error = %e, "Failed to ensure window visibility for import/server deep link");
        e.to_string()
    })?;
    info!("Ensured window visibility for import/server deep link");

    app.emit("mcp-import/server", payload).map_err(|e| {
        warn!(error = %e, "Failed to emit mcp-import/server event");
        e.to_string()
    })?;
    info!("Emitted mcp-import/server event");

    Ok(())
}

fn decode_import_server_payload(encoded: &str) -> Result<ImportServerDeepLinkPayload, String> {
    let decoded = URL_SAFE_NO_PAD
        .decode(encoded.as_bytes())
        .or_else(|_| STANDARD.decode(encoded.as_bytes()))
        .map_err(|e| format!("invalid base64 in import/server deep link: {e}"))?;

    if decoded.len() > IMPORT_SERVER_MAX_DECODED_BYTES {
        return Err(format!(
            "import/server payload exceeds {IMPORT_SERVER_MAX_DECODED_BYTES} bytes"
        ));
    }

    let payload: ImportServerDeepLinkPayload =
        serde_json::from_slice(&decoded).map_err(|e| e.to_string())?;
    if payload.text.trim().is_empty() {
        return Err("import/server payload text is empty".into());
    }

    Ok(payload)
}

#[cfg(target_os = "linux")]
const LINUX_MCPMATE_SCHEME_HANDLER: &str = "x-scheme-handler/mcpmate";

#[cfg(target_os = "linux")]
const LINUX_MIMEAPPS_SCHEME_PREFIX: &str = "x-scheme-handler/mcpmate=";

#[cfg(target_os = "linux")]
const LINUX_PACKAGED_DESKTOP_FILE: &str = "/usr/share/applications/MCPMate.desktop";

#[cfg(target_os = "linux")]
pub fn reconcile_linux_deep_link_handlers(app: &tauri::App) {
    if std::env::var_os("APPIMAGE").is_some() {
        match app.deep_link().register_all() {
            Ok(()) => {
                info!("Registered Linux AppImage deep-link handlers");
            }
            Err(err) => {
                warn!(error = %err, "Failed to register Linux AppImage deep-link handlers");
            }
        }
        return;
    }

    if linux_packaged_handler_supports_deep_link_arg() {
        let app_handle = app.handle().clone();
        tauri::async_runtime::spawn_blocking(move || {
            cleanup_linux_runtime_deep_link_handler(&app_handle);
        });
        info!("Scheduled Linux runtime deep-link handler cleanup");
    } else {
        info!(
            "Skipping Linux runtime deep-link handler cleanup because packaged handler is not ready"
        );
    }
}

#[cfg(target_os = "linux")]
fn linux_packaged_handler_supports_deep_link_arg() -> bool {
    let Ok(content) = std::fs::read_to_string(LINUX_PACKAGED_DESKTOP_FILE) else {
        return false;
    };
    linux_desktop_entry_handles_mcpmate_scheme(&content)
        && linux_desktop_entry_exec_matches(&content, |value| value.contains("%u"))
}

#[cfg(target_os = "linux")]
fn cleanup_linux_runtime_deep_link_handler(app: &AppHandle) {
    let Ok(current_exe) = std::env::current_exe() else {
        warn!("Failed to resolve current executable for Linux deep-link handler cleanup");
        return;
    };
    let Some(binary_name) = current_exe.file_name().and_then(|name| name.to_str()) else {
        warn!("Failed to resolve current executable name for Linux deep-link handler cleanup");
        return;
    };
    let handler_file_name = format!("{binary_name}-handler.desktop");
    let Ok(applications_dir) = app.path().data_dir().map(|path| path.join("applications")) else {
        warn!("Failed to resolve Linux applications directory for deep-link handler cleanup");
        return;
    };
    let handler_path = applications_dir.join(&handler_file_name);
    let Ok(content) = std::fs::read_to_string(&handler_path) else {
        return;
    };
    if !is_tauri_generated_linux_deep_link_handler(&content, binary_name) {
        warn!(
            handler_path = %handler_path.display(),
            "Skipping Linux deep-link handler cleanup because file does not look generated by MCPMate"
        );
        return;
    }

    match std::fs::remove_file(&handler_path) {
        Ok(()) => {
            info!(
                handler_path = %handler_path.display(),
                "Removed legacy Linux runtime deep-link handler"
            );
        }
        Err(err) => {
            warn!(error = %err, handler_path = %handler_path.display(), "Failed to remove legacy Linux runtime deep-link handler");
            return;
        }
    }
    cleanup_linux_mimeapps_references(&handler_file_name);
    refresh_linux_applications_database(&applications_dir);
    set_linux_packaged_deep_link_default();
}

#[cfg(target_os = "linux")]
fn is_tauri_generated_linux_deep_link_handler(content: &str, binary_name: &str) -> bool {
    let has_scheme = linux_desktop_entry_handles_mcpmate_scheme(content);
    let has_exec = linux_desktop_entry_exec_matches(content, |value| {
        value.contains(binary_name) && value.contains("%u")
    });

    has_scheme
        && has_exec
        && linux_desktop_entry_has_line(content, "Type=Application")
        && linux_desktop_entry_has_line(content, "Name=MCPMate")
        && linux_desktop_entry_has_line(content, "NoDisplay=true")
}

#[cfg(target_os = "linux")]
fn linux_desktop_entry_handles_mcpmate_scheme(content: &str) -> bool {
    content.lines().any(|line| {
        linux_desktop_entry_value(line, "MimeType").is_some_and(|value| {
            value
                .split(';')
                .any(|entry| entry == LINUX_MCPMATE_SCHEME_HANDLER)
        })
    })
}

#[cfg(target_os = "linux")]
fn linux_desktop_entry_exec_matches(content: &str, matches_exec: impl Fn(&str) -> bool) -> bool {
    content.lines().any(|line| {
        linux_desktop_entry_value(line, "Exec").is_some_and(|value| matches_exec(value))
    })
}

#[cfg(target_os = "linux")]
fn linux_desktop_entry_value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    line.trim_start().strip_prefix(key)?.strip_prefix('=')
}

#[cfg(target_os = "linux")]
fn linux_desktop_entry_has_line(content: &str, expected: &str) -> bool {
    content.lines().any(|line| line.trim() == expected)
}

#[cfg(target_os = "linux")]
fn cleanup_linux_mimeapps_references(handler_file_name: &str) {
    let Some(home) = std::env::var_os("HOME") else {
        return;
    };
    let home = std::path::PathBuf::from(home);
    let mimeapps_paths = [
        home.join(".config/mimeapps.list"),
        home.join(".local/share/applications/mimeapps.list"),
    ];

    for path in mimeapps_paths {
        match remove_linux_mimeapps_handler_reference(&path, handler_file_name) {
            Ok(true) => {
                info!(
                    mimeapps_path = %path.display(),
                    handler_file_name,
                    "Removed legacy Linux deep-link handler reference"
                );
            }
            Ok(false) => {}
            Err(err) => {
                warn!(error = %err, mimeapps_path = %path.display(), "Failed to update Linux mimeapps list");
            }
        }
    }
}

#[cfg(target_os = "linux")]
fn refresh_linux_applications_database(applications_dir: &std::path::Path) {
    match Command::new("update-desktop-database")
        .arg(applications_dir)
        .status()
    {
        Ok(status) if status.success() => {
            info!(
                applications_dir = %applications_dir.display(),
                "Refreshed Linux desktop applications database"
            );
        }
        Ok(status) => {
            warn!(
                status = ?status.code(),
                applications_dir = %applications_dir.display(),
                "update-desktop-database exited unsuccessfully"
            );
        }
        Err(err) => {
            warn!(error = %err, "Failed to run update-desktop-database");
        }
    }
}

#[cfg(target_os = "linux")]
fn set_linux_packaged_deep_link_default() {
    match Command::new("xdg-mime")
        .args(["default", "MCPMate.desktop", LINUX_MCPMATE_SCHEME_HANDLER])
        .status()
    {
        Ok(status) if status.success() => {
            info!("Set packaged MCPMate desktop entry as Linux deep-link default");
        }
        Ok(status) => {
            warn!(
                status = ?status.code(),
                "xdg-mime exited unsuccessfully while setting MCPMate deep-link default"
            );
        }
        Err(err) => {
            warn!(error = %err, "Failed to run xdg-mime for MCPMate deep-link default");
        }
    }
}

#[cfg(target_os = "linux")]
fn remove_linux_mimeapps_handler_reference(
    path: &std::path::Path,
    handler_file_name: &str,
) -> anyhow::Result<bool> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return Ok(false);
    };
    let mut changed = false;
    let mut lines = Vec::new();

    for line in content.lines() {
        if let Some(value) = line.strip_prefix(LINUX_MIMEAPPS_SCHEME_PREFIX) {
            let entries = value
                .split(';')
                .filter(|entry| !entry.is_empty() && *entry != handler_file_name)
                .collect::<Vec<_>>();
            let original_entry_count = value.split(';').filter(|entry| !entry.is_empty()).count();

            if entries.len() != original_entry_count {
                changed = true;
                if !entries.is_empty() {
                    lines.push(format!(
                        "{}{};",
                        LINUX_MIMEAPPS_SCHEME_PREFIX,
                        entries.join(";")
                    ));
                }
                continue;
            }
        }
        lines.push(line.to_string());
    }

    if changed {
        let mut updated = lines.join("\n");
        updated.push('\n');
        std::fs::write(path, updated)
            .with_context(|| format!("failed to write {}", path.display()))?;
    }
    Ok(changed)
}

#[cfg(target_os = "linux")]
pub fn extract_linux_fallback_deep_links_from_argv(args: &[String]) -> Vec<String> {
    fn normalize_arg(arg: &str) -> &str {
        arg.trim().trim_matches('"').trim_matches('\'')
    }

    if args.len() == 2 && normalize_arg(&args[1]).starts_with("mcpmate://") {
        return Vec::new();
    }

    args.iter()
        .filter_map(|arg| {
            let trimmed = normalize_arg(arg);
            if trimmed.starts_with("mcpmate://") {
                Some(trimmed.to_string())
            } else {
                None
            }
        })
        .collect()
}
