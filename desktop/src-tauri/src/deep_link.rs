//! Routes registered `mcpmate://` URLs (OAuth, extension-driven server import).

use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use tracing::{info, warn};

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
    if let Some(state) = app.try_state::<crate::DeepLinkState>() {
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
