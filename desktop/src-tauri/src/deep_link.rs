//! Routes registered `mcpmate://` URLs (OAuth, extension-driven server import).

use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

const IMPORT_SERVER_MAX_DECODED_BYTES: usize = 65_536;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportServerDeepLinkPayload {
    pub text: String,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
}

/// Dispatch `mcpmate://auth`, `mcpmate://import/server`, etc.
pub fn route_mcpmate_deep_link(app: &AppHandle, url_str: &str) -> Result<(), String> {
    let parsed = url::Url::parse(url_str).map_err(|e| e.to_string())?;
    if parsed.scheme() != "mcpmate" {
        return Ok(());
    }

    match parsed.host_str() {
        Some("auth") => crate::account::handle_oauth_url(app, url_str),
        Some("import") => handle_import_path(app, &parsed),
        _ => Ok(()),
    }
}

fn handle_import_path(app: &AppHandle, parsed: &url::Url) -> Result<(), String> {
    let path = parsed.path().trim_end_matches('/');
    if path != "/server" {
        return Ok(());
    }

    let encoded = parsed
        .query_pairs()
        .find(|(k, _)| k == "p")
        .map(|(_, v)| v.into_owned())
        .ok_or_else(|| "import/server deep link missing \"p\" query".to_string())?;

    let payload = decode_import_server_payload(&encoded)?;

    // Persist the payload so frontend can pull it during cold start even if the
    // first event dispatch happens before React listeners are mounted.
    if let Some(state) = app.try_state::<crate::DeepLinkState>() {
        tauri::async_runtime::block_on(state.set_pending_server_import(payload.clone()));
    }

    // Ensure a visible, focused main window before dispatching the import event.
    // This allows extension-triggered deep links to reliably wake MCPMate from
    // hidden/closed-window states on desktop shells.
    crate::shell::ensure_window_visibility(app).map_err(|e| e.to_string())?;

    app.emit("mcp-import/server", payload)
        .map_err(|e| e.to_string())?;

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
