use std::{env, fs};

fn main() {
    // Silence `unexpected_cfgs` warnings for our custom build-time cfg.
    println!("cargo:rustc-check-cfg=cfg(has_openapi_lock)");
    // Detect whether an embedded OpenAPI lock module exists and expose a cfg.
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let gen_path = format!("{}/gen/openapi_lock.rs", manifest_dir);
    println!("cargo:rerun-if-changed={}", gen_path);
    if fs::metadata(&gen_path).is_ok() {
        println!("cargo:rustc-cfg=has_openapi_lock");
    }

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
