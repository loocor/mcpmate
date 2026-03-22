use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    embed_auth_config_from_env_file();

    // Allow cfg gate in sources and enable a compile-time cfg for a special diagnostic build of the market proxy.
    println!("cargo:rustc-check-cfg=cfg(market_diag_default)");
    // Set MCPMATE_TAURI_MARKET_DIAG_DEFAULT=1 in the environment to turn this on.
    match env::var("MCPMATE_TAURI_MARKET_DIAG_DEFAULT") {
        Ok(v) if matches!(v.as_str(), "1" | "true" | "TRUE" | "True") => {
            println!("cargo:rustc-cfg=market_diag_default");
            println!("cargo:warning=Building with market diagnostic logging enabled by default");
        }
        _ => {}
    }

    // Pass through select environment variables as compile-time env for runtime access.
    if let Ok(v) = env::var("MCPMATE_TAURI_PREVIEW_EXPIRY_DATE") {
        println!("cargo:rustc-env=MCPMATE_TAURI_PREVIEW_EXPIRY_DATE={}", v);
    }
    if let Ok(v) = env::var("MCPMATE_TAURI_ENABLE_INSPECTOR") {
        println!("cargo:rustc-env=MCPMATE_TAURI_ENABLE_INSPECTOR={}", v);
    }

    tauri_build::build();
}

/// Load `embed.env` next to Cargo.toml and emit `cargo:rustc-env` for account/auth settings.
/// Environment variables override file values when set (same names without AUTH_ prefix for override keys).
fn embed_auth_config_from_env_file() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let path = Path::new(&manifest_dir).join("embed.env");
    println!("cargo:rerun-if-changed={}", path.display());

    let mut from_file: HashMap<String, String> = HashMap::new();
    if path.is_file() {
        let raw =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        for line in raw.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let key = k.trim().to_string();
            let value = v.trim().trim_matches('"').trim_matches('\'').to_string();
            if !key.is_empty() && !value.is_empty() {
                from_file.insert(key, value);
            }
        }
    } else {
        panic!(
            "Missing {} — create it (see comments inside) or copy from repo defaults.",
            path.display()
        );
    }

    let auth_base = env::var("MCPMATE_AUTH_WORKER_BASE")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| from_file.get("AUTH_WORKER_BASE").cloned())
        .unwrap_or_else(|| {
            panic!(
                "AUTH_WORKER_BASE missing in {} and MCPMATE_AUTH_WORKER_BASE not set",
                path.display()
            )
        });

    let keychain_service = env::var("MCPMATE_KEYCHAIN_SERVICE")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| from_file.get("KEYCHAIN_SERVICE").cloned())
        .unwrap_or_else(|| {
            panic!(
                "KEYCHAIN_SERVICE missing in {} and MCPMATE_KEYCHAIN_SERVICE not set",
                path.display()
            )
        });

    // Must stay in sync with `identifier` in tauri.conf.json unless you intentionally migrate keychain entries.
    println!("cargo:rustc-env=MCPMATE_AUTH_WORKER_BASE={auth_base}");
    println!("cargo:rustc-env=MCPMATE_KEYCHAIN_SERVICE={keychain_service}");
}
