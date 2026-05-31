use anyhow::{Context, Result};
use tauri::{WebviewWindow, Wry};

/// Matches board operator panel `rounded-xl` (12px).
const OPERATOR_PANEL_SHELL_CORNER_RADIUS: f64 = 12.0;
/// Matches board operator frame `p-2` (8px).
const OPERATOR_PANEL_FRAME_INSET: f64 = 8.0;

pub fn apply_operator_window_chrome(window: &WebviewWindow<Wry>) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        window
            .with_webview(|webview| {
                apply_operator_window_chrome_macos(webview);
            })
            .context("failed to access operator webview for chrome styling")?;
    }

    Ok(())
}

#[cfg(target_os = "macos")]
fn apply_operator_window_chrome_macos(webview: tauri::webview::PlatformWebview) {
    unsafe {
        use objc2_app_kit::{NSColor, NSWindow};
        use objc2_foundation::NSObjectNSKeyValueCoding;
        use objc2_web_kit::WKWebView;

        let ns_window: &NSWindow = &*webview.ns_window().cast();
        ns_window.setOpaque(false);
        ns_window.setHasShadow(true);
        let clear = NSColor::clearColor();
        ns_window.setBackgroundColor(Some(&clear));

        let wk_webview: &WKWebView = &*webview.inner().cast();
        let no = objc2_foundation::NSNumber::numberWithBool(false);
        wk_webview.setValue_forKey(Some(&no), objc2_foundation::ns_string!("drawsBackground"));
        wk_webview.setUnderPageBackgroundColor(Some(&clear));
        let clear_cg_color = clear.CGColor();

        wk_webview.setWantsLayer(true);
        if let Some(layer) = wk_webview.layer() {
            layer.setBackgroundColor(Some(&clear_cg_color));
        }

        if let Some(parent) = wk_webview.superview() {
            parent.setWantsLayer(true);
            if let Some(layer) = parent.layer() {
                layer.setBackgroundColor(Some(&clear_cg_color));
            }
        }

        let Some(content_view) = ns_window.contentView() else {
            return;
        };
        content_view.setWantsLayer(true);
        let Some(layer) = content_view.layer() else {
            return;
        };
        layer.setBackgroundColor(Some(&clear_cg_color));
        layer.setCornerRadius(OPERATOR_PANEL_SHELL_CORNER_RADIUS + OPERATOR_PANEL_FRAME_INSET);
        // Keep shadow/filter paint in the transparent frame inset instead of clipping it.
        layer.setMasksToBounds(false);
    }
}
