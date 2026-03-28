//! GitHub OAuth session storage and deep-link handling (macOS v1).
//!
//! Sync/upload to the auth worker is deferred; this module only stores JWT + stable device id.

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_opener::OpenerExt;

/// Public auth worker base URL, from `src-tauri/embed.env` at compile time (or env override).
///
/// The GitHub OAuth App used by the worker must register redirect URI
/// `{AUTH_WORKER_BASE}/auth/github/callback` (no stray slash mismatch).
///
/// If the app shows `invalid_state` after GitHub, the worker usually failed to persist OAuth
/// state in KV before redirecting (see `auth/src/index.ts` — `await` the KV put). Cookie-based
/// flows would need `SameSite=Lax`, not Strict, on the state cookie.
pub const AUTH_WORKER_BASE: &str = env!("MCPMATE_AUTH_WORKER_BASE");

/// macOS Keychain service name; must match `identifier` in `tauri.conf.json` unless migrating credentials.
const KEYCHAIN_SERVICE: &str = env!("MCPMATE_KEYCHAIN_SERVICE");
const KEYCHAIN_JWT_USER: &str = "oauth_jwt";
const DEVICE_ID_FILE: &str = "device_id";

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountStatus {
    pub logged_in: bool,
    pub device_id: String,
    pub device_name: String,
}

#[cfg(target_os = "macos")]
fn keychain_entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_JWT_USER).map_err(|e| e.to_string())
}

#[cfg(target_os = "macos")]
pub fn read_jwt() -> Option<String> {
    keychain_entry()
        .ok()
        .and_then(|e| e.get_password().ok())
        .filter(|s| !s.is_empty())
}

#[cfg(target_os = "macos")]
fn store_jwt(token: &str) -> Result<(), String> {
    let entry = keychain_entry()?;
    entry.set_password(token).map_err(|e| e.to_string())
}

#[cfg(target_os = "macos")]
fn delete_jwt() -> Result<(), String> {
    let entry = keychain_entry()?;
    let _ = entry.delete_credential();
    Ok(())
}

fn device_id_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(dir.join(DEVICE_ID_FILE))
}

pub fn ensure_device_id(app: &AppHandle) -> Result<String, String> {
    let path = device_id_path(app)?;
    if let Ok(existing) = std::fs::read_to_string(&path) {
        let trimmed = existing.trim().to_string();
        if !trimmed.is_empty() {
            return Ok(trimmed);
        }
    }
    let id = uuid::Uuid::new_v4().to_string();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, &id).map_err(|e| e.to_string())?;
    Ok(id)
}

fn device_display_name() -> String {
    hostname::get()
        .map(|h| h.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "Unknown Device".to_string())
}

/// Open the system browser at the Worker OAuth entrypoint.
#[cfg(target_os = "macos")]
pub fn start_github_login(app: &AppHandle) -> Result<(), String> {
    let _ = ensure_device_id(app)?;
    let url = format!("{AUTH_WORKER_BASE}/auth/github");
    app.opener()
        .open_url(url, None::<String>)
        .map_err(|e| e.to_string())
}

#[cfg(not(target_os = "macos"))]
pub fn start_github_login(_app: &AppHandle) -> Result<(), String> {
    Err("Account linking is only available on macOS.".into())
}

#[cfg(target_os = "macos")]
pub fn get_status(app: &AppHandle) -> Result<AccountStatus, String> {
    Ok(AccountStatus {
        logged_in: read_jwt().is_some(),
        device_id: ensure_device_id(app)?,
        device_name: device_display_name(),
    })
}

#[cfg(not(target_os = "macos"))]
pub fn get_status(_app: &AppHandle) -> Result<AccountStatus, String> {
    Err("Account status is only available on macOS.".into())
}

#[cfg(target_os = "macos")]
pub fn logout() -> Result<(), String> {
    delete_jwt()
}

#[cfg(not(target_os = "macos"))]
pub fn logout() -> Result<(), String> {
    Err("Account logout is only available on macOS.".into())
}

/// Handle `mcpmate://auth?token=...` or `?error=...` from the Worker redirect.
pub fn handle_oauth_url(app: &AppHandle, url_str: &str) -> Result<(), String> {
    let parsed = url::Url::parse(url_str).map_err(|e| e.to_string())?;
    if parsed.scheme() != "mcpmate" {
        return Ok(());
    }
    if parsed.host_str() != Some("auth") {
        return Ok(());
    }

    let mut token: Option<String> = None;
    let mut oauth_error: Option<String> = None;
    for (k, v) in parsed.query_pairs() {
        match k.as_ref() {
            "token" => token = Some(v.into_owned()),
            "error" => oauth_error = Some(v.into_owned()),
            _ => {}
        }
    }

    if let Some(e) = oauth_error {
        let _ = app.emit(
            "mcp-account/oauth-finished",
            serde_json::json!({ "ok": false, "error": e }),
        );
        return Ok(());
    }

    if let Some(token) = token {
        #[cfg(target_os = "macos")]
        {
            store_jwt(&token)?;
        }
        #[cfg(not(target_os = "macos"))]
        {
            drop(token);
        }
        let _ = app.emit(
            "mcp-account/oauth-finished",
            serde_json::json!({ "ok": true }),
        );
    }

    Ok(())
}
