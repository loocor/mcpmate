use std::{collections::BTreeMap, env, fs, io::Write, path::PathBuf};

use anyhow::Result;
use mcpmate::common::MCPMatePaths;

pub fn configure_process_environment() {
    const SKIP_BOARD_STATIC: &str = "MCPMATE_SKIP_BOARD_STATIC";

    if env::var_os(SKIP_BOARD_STATIC).is_none() {
        unsafe {
            env::set_var(SKIP_BOARD_STATIC, "1");
        }
    }

    for (key, value) in desktop_runtime_environment() {
        unsafe {
            env::set_var(key, value);
        }
    }
}

pub fn service_environment_entries() -> Vec<(String, String)> {
    desktop_runtime_environment().into_iter().collect()
}

pub fn merge_service_environment(base: Vec<(String, String)>) -> Vec<(String, String)> {
    let mut merged: BTreeMap<String, String> = base.into_iter().collect();

    for (key, value) in service_environment_entries() {
        merged.insert(key, value);
    }

    merged.into_iter().collect()
}

fn desktop_runtime_environment() -> BTreeMap<String, String> {
    #[cfg(target_os = "macos")]
    {
        let mut env_entries = BTreeMap::new();
        if let Ok(path) = ensure_desktop_runtime_path() {
            env_entries.insert("PATH".to_string(), path);
        }

        if let Ok(home) = env::var("HOME")
            && !home.trim().is_empty()
        {
            env_entries.insert("HOME".to_string(), home);
        }

        env_entries
    }

    #[cfg(not(target_os = "macos"))]
    {
        BTreeMap::new()
    }
}

#[cfg(target_os = "macos")]
fn ensure_desktop_runtime_path() -> Result<String> {
    let base_dir = resolve_base_dir();
    let bin_dir = base_dir.join("bin");
    fs::create_dir_all(&bin_dir)?;

    let bun_runtime_dir = base_dir.join("runtimes").join("bun");
    let bunx_path = bun_runtime_dir.join("bunx");

    let npx_shim = bin_dir.join("npx");
    ensure_executable_shim(
        &npx_shim,
        &format!(
            "#!/bin/sh\nset -e\nBUNX=\"{}\"\nif [ -x \"$BUNX\" ]; then exec \"$BUNX\" \"$@\"; fi\nif command -v npx >/dev/null 2>&1; then exec \"$(command -v npx)\" \"$@\"; fi\necho 'npx is unavailable (no bunx in {} and npx not found in PATH)' 1>&2\nexit 127\n",
            bunx_path.display(),
            bun_runtime_dir.display()
        ),
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
fn ensure_executable_shim(path: &std::path::Path, body: &str) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    if path.exists() {
        return Ok(());
    }

    let mut file = fs::File::create(path)?;
    file.write_all(body.as_bytes())?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn resolve_base_dir() -> PathBuf {
    match MCPMatePaths::new() {
        Ok(paths) => paths.base_dir().to_path_buf(),
        Err(_) => {
            let home = env::var("HOME").unwrap_or_else(|_| String::from("/"));
            PathBuf::from(home).join(".mcpmate")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_service_environment_preserves_base_entries() {
        let env = merge_service_environment(vec![
            ("MCPMATE_DATA_DIR".to_string(), "/tmp/mcpmate".to_string()),
            ("MCPMATE_API_PORT".to_string(), "8080".to_string()),
        ]);

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
        ]);

        assert_eq!(env.iter().filter(|(key, _)| key == "PATH").count(), 1);
        assert!(
            env.iter()
                .any(|(key, value)| key == "MCPMATE_DATA_DIR" && value == "/tmp/mcpmate")
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn service_environment_includes_path_on_macos() {
        let env = service_environment_entries();
        let path = env
            .iter()
            .find(|(key, _)| key == "PATH")
            .map(|(_, value)| value.clone())
            .expect("PATH entry");

        assert!(path.contains(".mcpmate/bin"));
        assert!(path.contains("/opt/homebrew/bin") || path.contains("/usr/local/bin"));
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn service_environment_is_empty_off_macos() {
        assert!(service_environment_entries().is_empty());
    }
}
