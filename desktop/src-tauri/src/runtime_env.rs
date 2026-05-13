use std::{collections::BTreeMap, env, fs, io::Write, path::PathBuf};

use anyhow::Result;
use mcpmate::common::MCPMatePaths;

pub fn configure_process_environment() -> Result<()> {
    const SKIP_BOARD_STATIC: &str = "MCPMATE_SKIP_BOARD_STATIC";

    if env::var_os(SKIP_BOARD_STATIC).is_none() {
        // Safe here because desktop startup sets process env before the Tauri runtime spawns worker threads.
        unsafe {
            env::set_var(SKIP_BOARD_STATIC, "1");
        }
    }

    for (key, value) in desktop_runtime_environment()? {
        // Safe here because desktop startup sets process env before the Tauri runtime spawns worker threads.
        unsafe {
            env::set_var(key, value);
        }
    }

    Ok(())
}

pub fn service_environment_entries() -> Result<Vec<(String, String)>> {
    Ok(desktop_runtime_environment()?.into_iter().collect())
}

pub fn merge_service_environment(base: Vec<(String, String)>) -> Result<Vec<(String, String)>> {
    let mut merged: BTreeMap<String, String> = base.into_iter().collect();

    for (key, value) in service_environment_entries()? {
        merged.insert(key, value);
    }

    Ok(merged.into_iter().collect())
}

fn desktop_runtime_environment() -> Result<BTreeMap<String, String>> {
    #[cfg(target_os = "macos")]
    {
        let mut env_entries = BTreeMap::new();
        let path = ensure_desktop_runtime_path()?;
        env_entries.insert("PATH".to_string(), path);

        if let Ok(home) = env::var("HOME")
            && !home.trim().is_empty()
        {
            env_entries.insert("HOME".to_string(), home);
        }

        Ok(env_entries)
    }

    #[cfg(not(target_os = "macos"))]
    {
        Ok(BTreeMap::new())
    }
}

#[cfg(target_os = "macos")]
fn ensure_desktop_runtime_path() -> Result<String> {
    let base_dir = resolve_base_dir()?;
    let bin_dir = base_dir.join("bin");
    fs::create_dir_all(&bin_dir)?;

    let bun_runtime_dir = base_dir.join("runtimes").join("bun");
    let bunx_path = bun_runtime_dir.join("bunx");

    let npx_shim = bin_dir.join("npx");
    ensure_executable_shim(
        &npx_shim,
        &npx_shim_body(&bin_dir, &bunx_path, &bun_runtime_dir),
    )?;

    let python3_candidates = [
        "/usr/bin/python3",
        "/opt/homebrew/bin/python3",
        "/usr/local/bin/python3",
    ];
    let py_shim_paths = [bin_dir.join("python3"), bin_dir.join("python")];
    for shim in &py_shim_paths {
        let found = python3_candidates
            .iter()
            .find(|candidate| std::path::Path::new(candidate).exists());
        let body = if let Some(path) = found {
            format!("#!/bin/sh\nexec \"{}\" \"$@\"\n", path)
        } else {
            "#!/bin/sh\nexec /usr/bin/env python3 \"$@\"\n".to_string()
        };
        ensure_executable_shim(shim, &body)?;
    }

    let mut extra_paths: Vec<String> = vec![
        bin_dir.display().to_string(),
        base_dir.join("runtimes").join("bun").display().to_string(),
        base_dir.join("runtimes").join("uv").display().to_string(),
        "/opt/homebrew/bin".into(),
        "/usr/local/bin".into(),
    ];
    if let Ok(current) = env::var("PATH") {
        extra_paths.push(current);
    }

    Ok(extra_paths.join(":"))
}

#[cfg(target_os = "macos")]
fn npx_shim_body(
    bin_dir: &std::path::Path,
    bunx_path: &std::path::Path,
    bun_runtime_dir: &std::path::Path,
) -> String {
    format!(
        "#!/bin/sh\nset -e\nBUNX=\"{}\"\nSELF_DIR=\"{}\"\nif [ -x \"$BUNX\" ]; then exec \"$BUNX\" \"$@\"; fi\nSEARCH_PATH=\"\"\nOLD_IFS=\"$IFS\"\nIFS=:; for entry in $PATH; do\n  if [ \"$entry\" = \"$SELF_DIR\" ]; then\n    continue\n  fi\n  if [ -z \"$SEARCH_PATH\" ]; then\n    SEARCH_PATH=\"$entry\"\n  else\n    SEARCH_PATH=\"$SEARCH_PATH:$entry\"\n  fi\ndone\nIFS=\"$OLD_IFS\"\nif [ -n \"$SEARCH_PATH\" ] && PATH=\"$SEARCH_PATH\" command -v npx >/dev/null 2>&1; then\n  exec env PATH=\"$SEARCH_PATH\" \"$(PATH=\"$SEARCH_PATH\" command -v npx)\" \"$@\"\nfi\necho 'npx is unavailable (no bunx in {} and no system npx outside {})' 1>&2\nexit 127\n",
        bunx_path.display(),
        bin_dir.display(),
        bun_runtime_dir.display(),
        bin_dir.display(),
    )
}

#[cfg(target_os = "macos")]
fn ensure_executable_shim(path: &std::path::Path, body: &str) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let needs_write = match fs::read_to_string(path) {
        Ok(existing) => existing != body,
        Err(_) => true,
    };

    if needs_write {
        let mut file = fs::File::create(path)?;
        file.write_all(body.as_bytes())?;
    }

    fs::set_permissions(path, fs::Permissions::from_mode(0o755))?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn resolve_base_dir() -> Result<PathBuf> {
    Ok(MCPMatePaths::new()?.base_dir().to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_service_environment_preserves_base_entries() {
        let env = merge_service_environment(vec![
            ("MCPMATE_DATA_DIR".to_string(), "/tmp/mcpmate".to_string()),
            ("MCPMATE_API_PORT".to_string(), "8080".to_string()),
        ])
        .expect("merge service environment");

        assert!(
            env.iter()
                .any(|(key, value)| key == "MCPMATE_DATA_DIR" && value == "/tmp/mcpmate")
        );
        assert!(
            env.iter()
                .any(|(key, value)| key == "MCPMATE_API_PORT" && value == "8080")
        );
    }

    #[test]
    fn merge_service_environment_deduplicates_base_keys() {
        let env = merge_service_environment(vec![
            ("MCPMATE_DATA_DIR".to_string(), "/tmp/mcpmate".to_string()),
            ("PATH".to_string(), "/tmp/bin".to_string()),
        ])
        .expect("merge service environment");

        assert_eq!(env.iter().filter(|(key, _)| key == "PATH").count(), 1);
        assert!(
            env.iter()
                .any(|(key, value)| key == "MCPMATE_DATA_DIR" && value == "/tmp/mcpmate")
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn service_environment_includes_path_on_macos() {
        let env = service_environment_entries().expect("service environment entries");
        let path = env
            .iter()
            .find(|(key, _)| key == "PATH")
            .map(|(_, value)| value.clone())
            .expect("PATH entry");

        assert!(path.contains(".mcpmate/bin"));
        assert!(path.contains("/opt/homebrew/bin") || path.contains("/usr/local/bin"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn npx_shim_body_removes_self_dir_from_path_lookup() {
        let body = npx_shim_body(
            std::path::Path::new("/tmp/mcpmate/bin"),
            std::path::Path::new("/tmp/mcpmate/runtimes/bun/bunx"),
            std::path::Path::new("/tmp/mcpmate/runtimes/bun"),
        );

        assert!(body.contains("SELF_DIR=\"/tmp/mcpmate/bin\""));
        assert!(body.contains("PATH=\"$SEARCH_PATH\" command -v npx"));
        assert!(body.contains("no system npx outside /tmp/mcpmate/bin"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn ensure_executable_shim_rewrites_existing_body() {
        let unique = format!(
            "mcpmate-runtime-env-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        );
        let base = std::env::temp_dir().join(unique);
        fs::create_dir_all(&base).expect("create temp dir");
        let path = base.join("shim");

        fs::write(&path, "old body").expect("write old body");
        ensure_executable_shim(&path, "new body").expect("rewrite shim");

        let body = fs::read_to_string(&path).expect("read rewritten shim");
        assert_eq!(body, "new body");

        fs::remove_dir_all(base).expect("remove temp dir");
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn service_environment_is_empty_off_macos() {
        assert!(
            service_environment_entries()
                .expect("service environment entries")
                .is_empty()
        );
    }
}
