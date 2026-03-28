use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

struct BackendBuildContext {
    backend_dir: PathBuf,
    sidecar_dir: PathBuf,
    target: String,
    profile: String,
    exe_suffix: &'static str,
}

fn main() {
    embed_auth_config_from_env_file();
    ensure_local_core_sidecar();
    ensure_bridge_sidecar();

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

fn ensure_local_core_sidecar() {
    let context = backend_build_context();
    emit_backend_rerun_hints(&context.backend_dir, &["src/main.rs", "Cargo.toml"]);
    ensure_backend_sidecar(&context, "mcpmate", "mcpmate-core");
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

fn ensure_bridge_sidecar() {
    let context = backend_build_context();
    ensure_backend_sidecar(&context, "bridge", "bridge");
}

fn backend_build_context() -> BackendBuildContext {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.join("../..");
    let backend_dir = workspace_root.join("backend");
    let target = env::var("TARGET").expect("TARGET");
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    let exe_suffix = if target.contains("windows") {
        ".exe"
    } else {
        ""
    };

    BackendBuildContext {
        sidecar_dir: backend_dir.join("target/sidecars"),
        backend_dir,
        target,
        profile,
        exe_suffix,
    }
}

fn emit_backend_rerun_hints(backend_dir: &Path, paths: &[&str]) {
    for path in paths {
        println!(
            "cargo:rerun-if-changed={}",
            backend_dir.join(path).display()
        );
    }
}

fn ensure_backend_sidecar(context: &BackendBuildContext, binary_name: &str, sidecar_name: &str) {
    let sidecar_target = context.sidecar_dir.join(format!(
        "{sidecar_name}-{}{}",
        context.target, context.exe_suffix
    ));
    let sidecar_plain = context
        .sidecar_dir
        .join(format!("{sidecar_name}{}", context.exe_suffix));

    let mut cargo = Command::new("cargo");
    cargo
        .arg("build")
        .arg("--manifest-path")
        .arg(context.backend_dir.join("Cargo.toml"))
        .arg("-p")
        .arg("mcpmate")
        .arg("--bin")
        .arg(binary_name)
        .arg("--target")
        .arg(&context.target);

    if context.profile == "release" {
        cargo.arg("--release");
    }

    let status = cargo
        .status()
        .unwrap_or_else(|_| panic!("failed to invoke cargo build for {binary_name} sidecar"));
    assert!(status.success(), "failed to build {binary_name} sidecar");

    let built_binary = context
        .backend_dir
        .join("target")
        .join(&context.target)
        .join(&context.profile)
        .join(format!("{binary_name}{}", context.exe_suffix));

    assert!(
        built_binary.exists(),
        "missing built {binary_name} binary at {}",
        built_binary.display()
    );

    fs::create_dir_all(&context.sidecar_dir).expect("failed to create sidecar directory");
    fs::copy(&built_binary, &sidecar_target)
        .unwrap_or_else(|_| panic!("failed to copy {binary_name} sidecar target file"));
    fs::copy(&built_binary, &sidecar_plain)
        .unwrap_or_else(|_| panic!("failed to copy {binary_name} sidecar plain file"));
}
