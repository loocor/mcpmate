use std::{
    collections::BTreeMap,
    env, fs,
    io::Write,
    path::PathBuf,
};

use anyhow::Result;
use mcpmate::common::path::{dedup_and_join, split_path_entries};
#[cfg(target_os = "macos")]
use mcpmate::common::path::{capture_login_shell_path, read_path_from_path_helper};
#[cfg(target_os = "macos")]
use mcpmate::common::MCPMatePaths;

pub fn configure_process_environment() -> Result<()> {
    const SKIP_BOARD_STATIC: &str = "MCPMATE_SKIP_BOARD_STATIC";

    if env::var_os(SKIP_BOARD_STATIC).is_none() {
        // Safe here because desktop startup sets process env before the Tauri runtime spawns worker threads.
        unsafe {
            env::set_var(SKIP_BOARD_STATIC, "1");
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(initial_path) = env::var("PATH") {
            tracing::debug!(
                "[PATH_DEBUG] Initial PATH before configuration: {}",
                initial_path
            );
        }

        if let Some(path_helper_path) = read_path_from_path_helper() {
            tracing::debug!("[PATH_DEBUG] path_helper returned: {}", path_helper_path);
        } else {
            tracing::debug!("[PATH_DEBUG] path_helper returned None");
        }

        if let Some(shell_path) = capture_login_shell_path() {
            tracing::debug!("[PATH_DEBUG] Login shell PATH: {}", shell_path);
            // Safe here because desktop startup sets process env before the Tauri runtime spawns worker threads.
            unsafe {
                env::set_var("PATH", shell_path);
            }
        } else {
            tracing::debug!("[PATH_DEBUG] capture_login_shell_path returned None");
        }

        if let Ok(final_path) = env::var("PATH") {
            tracing::debug!("[PATH_DEBUG] Final PATH after login shell: {}", final_path);
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

        tracing::debug!(
            "[PATH_DEBUG] ensure_desktop_runtime_path returned: {}",
            path
        );

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
    let bin_dir = base_dir.join("runtimes").join("shim");
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

    if let Some(path) = read_path_from_path_helper() {
        extra_paths.extend(split_path_entries(&path));
    }

    if let Some(path) = capture_login_shell_path() {
        extra_paths.extend(split_path_entries(&path));
    }

    if let Ok(current) = env::var("PATH") {
        extra_paths.extend(split_path_entries(&current));
    }

    Ok(dedup_and_join(extra_paths))
}

#[cfg(target_os = "macos")]
fn npx_shim_body(
    bin_dir: &std::path::Path,
    bunx_path: &std::path::Path,
    bun_runtime_dir: &std::path::Path,
) -> String {
    format!(
        "#!/bin/sh\nset -e\nBUNX=\"{}\"\nSELF_DIR=\"{}\"\nSEARCH_PATH=\"\"\nOLD_IFS=\"$IFS\"\nIFS=:; for entry in $PATH; do\n  if [ \"$entry\" = \"$SELF_DIR\" ]; then\n    continue\n  fi\n  if [ -z \"$SEARCH_PATH\" ]; then\n    SEARCH_PATH=\"$entry\"\n  else\n    SEARCH_PATH=\"$SEARCH_PATH:$entry\"\n  fi\ndone\nIFS=\"$OLD_IFS\"\nif [ -n \"$SEARCH_PATH\" ] && PATH=\"$SEARCH_PATH\" command -v npx >/dev/null 2>&1; then\n  exec env PATH=\"$SEARCH_PATH\" \"$(PATH=\"$SEARCH_PATH\" command -v npx)\" \"$@\"\nfi\nif [ -x \"$BUNX\" ]; then exec \"$BUNX\" \"$@\"; fi\necho 'npx is unavailable (no system npx outside {} and no bunx in {})' 1>&2\nexit 127\n",
        bunx_path.display(),
        bin_dir.display(),
        bin_dir.display(),
        bun_runtime_dir.display(),
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

        assert!(path.contains(".mcpmate/runtimes/shim"));
        assert!(path.contains("/opt/homebrew/bin") || path.contains("/usr/local/bin"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn dedup_and_join_preserves_first_occurrence() {
        let merged = dedup_and_join(vec![
            "/a".to_string(),
            "/b".to_string(),
            "/a".to_string(),
            "/c".to_string(),
        ]);

        assert_eq!(merged, "/a:/b:/c");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn parse_path_helper_output_extracts_path() {
        use mcpmate::common::path::parse_path_helper_output;
        let shell = "PATH=\"/usr/bin:/bin:/opt/homebrew/bin\"; export PATH;\nMANPATH=\"/usr/share/man\"; export MANPATH;";
        let parsed = parse_path_helper_output(shell).expect("parsed path");
        assert_eq!(parsed, "/usr/bin:/bin:/opt/homebrew/bin");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn npx_shim_prefers_system_npx_before_bunx() {
        let body = npx_shim_body(
            std::path::Path::new("/tmp/shim"),
            std::path::Path::new("/tmp/bun/bunx"),
            std::path::Path::new("/tmp/bun"),
        );
        let system_probe = "command -v npx >/dev/null 2>&1";
        let bunx_probe = "if [ -x \"$BUNX\" ]; then exec \"$BUNX\" \"$@\"; fi";
        let system_index = body.find(system_probe).expect("system npx probe");
        let bunx_index = body.find(bunx_probe).expect("bunx fallback probe");
        assert!(system_index < bunx_index);
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
