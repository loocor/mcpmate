//! Unified command resolution for all runtime consumers.
//!
//! Builds an enriched PATH from four layers, then resolves commands through
//! three cascading resolution layers:
//!
//!   1. MCPMate-managed binary in `~/.mcpmate/runtimes/`
//!   2. Enriched PATH search (initial + path_helper + login shell + MCPMate paths)
//!   3. UV-managed Python fallback (resolves `uv` first, then locates Python)
//!
//! Used by: onboarding detection, runtime status API, stdio transport.

use std::{
    env, fs,
    path::PathBuf,
    process::Command as StdCommand,
};

use super::RuntimeManager;
use crate::common::MCPMatePaths;
use crate::common::constants::commands;
use crate::common::path;

// ── Resolution types ─────────────────────────────────────────────────────

/// How a command was resolved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolveSource {
    /// Binary in `~/.mcpmate/runtimes/<type>/`.
    McpMateManaged,
    /// Found on the enriched PATH.
    SystemPath,
    /// Found via UV-managed Python fallback.
    UvPython,
}

/// Result of resolving a command name to an executable path.
#[derive(Clone, Debug)]
pub struct ResolvedCommand {
    /// Absolute path to the resolved executable.
    pub path: PathBuf,
    /// Which resolution layer matched.
    pub source: ResolveSource,
}

// ── Enriched PATH construction ───────────────────────────────────────────
// Mirrors the four-layer model from desktop shell's runtime_env.rs:
//   Layer 1: current process PATH (initial system PATH)
//   Layer 2: /usr/libexec/path_helper -s (macOS system paths)
//   Layer 3: $SHELL -l -c "printf %s $PATH" (user shell profile)
//   Layer 4: MCPMate-specific directories (shim, bun, uv, homebrew)

/// Build the four-layer enriched PATH string.
///
/// When the Core process is spawned by the desktop shell, `env::var("PATH")`
/// already contains the enriched PATH from `ensure_desktop_runtime_path()`.
/// This function detects that signal and returns the PATH as-is, avoiding
/// redundant subprocess calls (path_helper, login shell).
///
/// In standalone/service mode (no MCPMate dirs in PATH), it performs the
/// full four-layer rebuild so command resolution works regardless of how
/// the process was started.
///
/// The effective search order after dedup is:
///   MCPMate dirs → path_helper → login shell → current process PATH.
pub fn build_enriched_path(mcpmate_paths: &MCPMatePaths) -> String {
    let current = env::var("PATH").unwrap_or_default();

    // If PATH already contains MCPMate-specific directories, the desktop
    // shell already enriched it — no need to re-derive.
    let shim_dir = mcpmate_paths.base_dir().join("runtimes").join("shim");
    let shim_str = shim_dir.to_string_lossy();
    if !current.is_empty() && current.contains(shim_str.as_ref()) {
        tracing::debug!(
            "[resolve] PATH already enriched by desktop shell ({} chars), skipping rebuild",
            current.len()
        );
        return current;
    }

    // Standalone/service mode: build enriched PATH from scratch.
    let mut entries: Vec<String> = Vec::new();
    if !current.is_empty() {
        entries.extend(path::split_path_entries(&current));
    }
    append_platform_paths(&mut entries, mcpmate_paths);
    path::dedup_and_join(entries)
}

#[cfg(target_os = "macos")]
fn append_platform_paths(entries: &mut Vec<String>, mcpmate_paths: &MCPMatePaths) {
    let base_dir = mcpmate_paths.base_dir();

    // Layer 4: MCPMate-specific directories (prepend so they take priority)
    let mcpmate_dirs = vec![
        base_dir.join("runtimes").join("shim").display().to_string(),
        base_dir.join("runtimes").join("bun").display().to_string(),
        base_dir.join("runtimes").join("uv").display().to_string(),
        "/opt/homebrew/bin".into(),
        "/usr/local/bin".into(),
    ];
    // Insert MCPMate dirs at the front (highest priority after managed)
    let mut prefixed = mcpmate_dirs;
    prefixed.append(entries);
    *entries = prefixed;

    // Layer 2: macOS path_helper
    if let Some(p) = path::read_path_from_path_helper() {
        entries.extend(path::split_path_entries(&p));
    }

    // Layer 3: login shell PATH
    if let Some(p) = path::capture_login_shell_path() {
        entries.extend(path::split_path_entries(&p));
    }
}

#[cfg(not(target_os = "macos"))]
fn append_platform_paths(_entries: &mut Vec<String>, _mcpmate_paths: &MCPMatePaths) {
    // Non-macOS: only the process PATH is available.
    // Windows / Linux enrichment can be added here when needed.
}

// ── Command resolution ───────────────────────────────────────────────────

/// Stateful resolver that caches the enriched PATH across multiple lookups.
#[derive(Debug)]
pub struct CommandResolver {
    paths: MCPMatePaths,
    enriched_path: String,
}

impl CommandResolver {
    /// Create a new resolver, building the four-layer enriched PATH once.
    pub fn new(paths: &MCPMatePaths) -> Self {
        let enriched_path = build_enriched_path(paths);
        tracing::debug!(
            "[resolve] Enriched PATH ({} entries): {}",
            enriched_path.matches(':').count() + 1,
            enriched_path
        );
        Self {
            paths: paths.clone(),
            enriched_path,
        }
    }

    /// Resolve a command through managed → enriched PATH → UV Python fallback.
    pub fn resolve(&self, command: &str) -> Option<ResolvedCommand> {
        // Layer 1: MCPMate-managed runtime
        let manager = RuntimeManager::with_paths(&self.paths);
        if let Some(path) = manager.get_command_path(command) {
            tracing::debug!(
                command,
                path = %path.display(),
                "[resolve] Layer 1 hit: MCPMate managed"
            );
            return Some(ResolvedCommand {
                path,
                source: ResolveSource::McpMateManaged,
            });
        }

        // Layer 2: enriched PATH search
        if let Some(path) = which_in_path(command, &self.enriched_path) {
            tracing::debug!(
                command,
                path = %path.display(),
                "[resolve] Layer 2 hit: enriched PATH"
            );
            return Some(ResolvedCommand {
                path,
                source: ResolveSource::SystemPath,
            });
        }

        // Layer 3: UV-managed Python fallback (transport-specific commands only)
        if looks_like_python_command(command) {
            if let Some(path) = self.find_uv_python(command) {
                tracing::debug!(
                    command,
                    path = %path.display(),
                    "[resolve] Layer 3 hit: UV Python fallback"
                );
                return Some(ResolvedCommand {
                    path,
                    source: ResolveSource::UvPython,
                });
            }
        }

        tracing::warn!(
            command,
            "[resolve] All layers exhausted — command not found"
        );
        None
    }

    /// Resolve `uv` via Layers 1-2, then locate UV-managed Python.
    fn find_uv_python(&self, command: &str) -> Option<PathBuf> {
        let uv = self.resolve("uv")?;
        try_find_uv_python(&uv.path, command)
    }

    /// The enriched PATH string (for logging or passing to child processes).
    pub fn enriched_path(&self) -> &str {
        &self.enriched_path
    }
}

/// Search for a command in a custom PATH string (colon-separated).
fn which_in_path(command: &str, path: &str) -> Option<PathBuf> {
    for dir in path.split(':') {
        let dir = dir.trim();
        if dir.is_empty() {
            continue;
        }
        let candidate = PathBuf::from(dir).join(command);
        if candidate.is_file() {
            // On Unix, also check that the file is executable
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(meta) = fs::metadata(&candidate) {
                    if meta.permissions().mode() & 0o111 != 0 {
                        return Some(candidate);
                    }
                }
                continue;
            }
            #[cfg(not(unix))]
            {
                return Some(candidate);
            }
        }
    }
    None
}

/// Check whether a command name looks like a Python variant that UV can manage.
fn looks_like_python_command(command: &str) -> bool {
    matches!(
        command.trim().to_lowercase().as_str(),
        commands::PYTHON | commands::PYTHON3 | commands::PY
    )
}

// ── UV Python helper ─────────────────────────────────────────────────────
// Part of the resolver's Layer 3.  Resolves `uv` first, then asks it
// where Python is installed.  Kept as a separate function so it can
// be tested independently.

/// Locate a UV-managed Python binary by asking `uv` where it stores Python.
///
/// Takes the path to a resolved `uv` binary and the original command
/// (`python`, `python3`, or `py`).  Runs `uv python dir` to discover
/// the actual Python directory (which may differ from the default),
/// then returns the most recently installed matching binary.
pub fn try_find_uv_python(uv_path: &std::path::Path, command: &str) -> Option<PathBuf> {
    let cmd_lower = command.trim().to_lowercase();
    if !matches!(cmd_lower.as_str(), "python" | "python3" | "py") {
        return None;
    }

    // Ask uv where it stores Python
    let output = StdCommand::new(uv_path)
        .arg("python")
        .arg("dir")
        .output()
        .ok()?;

    if !output.status.success() {
        tracing::debug!(
            "[uv-python] `uv python dir` failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
        return None;
    }

    let uv_python_dir = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if uv_python_dir.is_empty() {
        return None;
    }

    let uv_python_dir = PathBuf::from(&uv_python_dir);
    if !uv_python_dir.exists() {
        tracing::debug!("[uv-python] Directory does not exist: {}", uv_python_dir.display());
        return None;
    }

    tracing::debug!("[uv-python] Searching in: {}", uv_python_dir.display());

    // Find the most recently modified cpython-* directory
    let mut best_version: Option<PathBuf> = None;
    let mut best_modified = std::time::SystemTime::UNIX_EPOCH;

    for entry in fs::read_dir(&uv_python_dir).ok()? {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                tracing::debug!("[uv-python] Failed to read dir entry: {}", e);
                continue;
            }
        };
        let name_str = entry.file_name().to_string_lossy().to_string();

        if !name_str.starts_with("cpython-")
            || name_str.ends_with(".lock")
            || name_str == ".gitignore"
            || name_str == ".temp"
        {
            continue;
        }

        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if !metadata.is_dir() {
            continue;
        }

        if let Ok(modified) = metadata.modified() {
            if modified > best_modified {
                best_modified = modified;
                best_version = Some(entry.path());
            }
        }
    }

    let version_dir = best_version?;
    let python_path = version_dir.join("bin").join(&cmd_lower);

    if python_path.exists() {
        tracing::info!(
            "[uv-python] Found {} → {}",
            command,
            python_path.display()
        );
        Some(python_path)
    } else {
        tracing::debug!(
            "[uv-python] Binary not found at expected path: {}",
            python_path.display()
        );
        None
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_source_equality() {
        assert_eq!(ResolveSource::McpMateManaged, ResolveSource::McpMateManaged);
        assert_ne!(ResolveSource::McpMateManaged, ResolveSource::SystemPath);
    }

    #[test]
    fn split_path_entries_handles_empty_and_whitespace() {
        let entries = path::split_path_entries("/a::  :/b:");
        assert_eq!(entries, vec!["/a".to_string(), "/b".to_string()]);
    }

    #[test]
    fn dedup_and_join_preserves_first_occurrence() {
        let result = path::dedup_and_join(vec![
            "/a".into(), "/b".into(), "/a".into(), "/c".into(),
        ]);
        assert_eq!(result, "/a:/b:/c");
    }

    #[test]
    fn which_in_path_finds_existing_executable() {
        let dir = tempfile::tempdir().expect("temp dir");
        let bin = dir.path().join("test-cmd");
        fs::write(&bin, "#!/bin/sh\necho ok\n").expect("write");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).expect("chmod");
        }

        let path_str = dir.path().display().to_string();
        let found = which_in_path("test-cmd", &path_str);
        assert!(found.is_some());
        assert_eq!(found.unwrap(), bin);
    }

    #[test]
    fn which_in_path_returns_none_for_missing() {
        let found = which_in_path("nonexistent-command-xyz", "/usr/bin:/usr/local/bin");
        assert!(found.is_none());
    }

    #[test]
    fn try_find_uv_python_rejects_non_python_commands() {
        // Need a dummy path — the function short-circuits on command name
        let dummy = std::path::Path::new("/usr/bin/uv");
        assert!(try_find_uv_python(dummy, "node").is_none());
        assert!(try_find_uv_python(dummy, "npx").is_none());
    }
}
