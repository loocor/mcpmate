//! Shared PATH construction helpers for macOS.
//!
//! These functions are used by both the backend resolver and the desktop shell
//! to build the enriched PATH for command resolution.
//!
//! Platform-specific helpers (macOS) are gated with `#[cfg(target_os = "macos")]`.
//! `split_path_entries` and `dedup_and_join` are platform-agnostic.

use std::collections::HashSet;

/// Split a colon-separated PATH string into entries, trimming whitespace and
/// skipping empty entries.
pub fn split_path_entries(path: &str) -> Vec<String> {
    path.split(':')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(str::to_string)
        .collect()
}

/// Join PATH entries with `:`, deduplicating by first occurrence.
pub fn dedup_and_join(entries: Vec<String>) -> String {
    let mut seen = HashSet::new();
    entries
        .into_iter()
        .filter(|entry| seen.insert(entry.clone()))
        .collect::<Vec<_>>()
        .join(":")
}

// ── macOS-specific helpers ───────────────────────────────────────────────────

/// Capture the login shell's PATH by running `$SHELL -l -c "printf '%s' \"$PATH\""`
/// after sourcing the shell's RC file (`.zshrc`, `.bashrc`, etc.) to include
/// user-installed tool paths (Bun, Cargo, pip --user, etc.).
#[cfg(target_os = "macos")]
pub fn capture_login_shell_path() -> Option<String> {
    let shell = std::env::var("SHELL")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "/bin/zsh".to_string());

    let rc_cmd = if shell.contains("zsh") {
        r#"test -f "${ZDOTDIR:-$HOME}/.zshrc" && . "${ZDOTDIR:-$HOME}/.zshrc" 2>/dev/null"#
    } else if shell.contains("bash") {
        r#"test -f "$HOME/.bashrc" && . "$HOME/.bashrc" 2>/dev/null"#
    } else if shell.contains("fish") {
        r#"test -f "${XDG_CONFIG_HOME:-$HOME/.config}/fish/config.fish" && . "${XDG_CONFIG_HOME:-$HOME/.config}/fish/config.fish" 2>/dev/null"#
    } else {
        ""
    };

    let cmd = if rc_cmd.is_empty() {
        r#"printf '%s' "$PATH""#.to_string()
    } else {
        format!("{{ {}; }} 2>/dev/null; printf '%s' \"$PATH\"", rc_cmd)
    };

    let output = std::process::Command::new(&shell)
        .arg("-l")
        .arg("-c")
        .arg(&cmd)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!path.is_empty()).then_some(path)
}

/// Run `/usr/libexec/path_helper -s` and extract the resulting PATH.
#[cfg(target_os = "macos")]
pub fn read_path_from_path_helper() -> Option<String> {
    let output = std::process::Command::new("/usr/libexec/path_helper")
        .arg("-s")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_path_helper_output(stdout.as_ref())
}

#[cfg(target_os = "macos")]
pub fn parse_path_helper_output(output: &str) -> Option<String> {
    extract_path_value(output, "PATH=\"", "\";")
        .or_else(|| extract_path_value(output, "PATH='", "';"))
}

#[cfg(target_os = "macos")]
pub fn extract_path_value(output: &str, prefix: &str, end_marker: &str) -> Option<String> {
    let start = output.find(prefix)?;
    let rest = output.get(start + prefix.len()..)?;
    let end = rest.find(end_marker)?;
    let path = rest.get(..end)?.trim();
    (!path.is_empty()).then(|| path.to_string())
}
