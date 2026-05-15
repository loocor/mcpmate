//! Shared PATH construction helpers for macOS.
//!
//! These functions are used by both the backend resolver and the desktop shell
//! to build the enriched PATH for command resolution.
//!
//! Platform-specific helpers (macOS) are gated with `#[cfg(target_os = "macos")]`.
//! `split_path_entries` and `dedup_and_join` use the platform-appropriate
//! separator via [`std::env::split_paths`] / [`std::env::join_paths`].

use std::collections::HashSet;

/// Split a PATH string into entries using the platform-appropriate separator,
/// trimming whitespace and skipping empty entries.
pub fn split_path_entries(path: &str) -> Vec<String> {
    std::env::split_paths(path)
        .map(|p| p.to_string_lossy().trim().to_string())
        .filter(|entry| !entry.is_empty())
        .collect()
}

/// Join PATH entries with the platform-appropriate separator,
/// deduplicating by first occurrence.
pub fn dedup_and_join(entries: Vec<String>) -> String {
    let mut seen: HashSet<String> = HashSet::with_capacity(entries.len());
    let mut unique = Vec::with_capacity(entries.len());
    for entry in entries {
        if !seen.contains(entry.as_str()) {
            seen.insert(entry.clone());
            unique.push(entry);
        }
    }
    // join_paths returns Err only on malformed entries (Windows path chars),
    // fall back to ":" separator which covers the vast majority of usage.
    std::env::join_paths(&unique)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| unique.join(":"))
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
        // Fish uses `source` instead of `.` and `; and` instead of `&&`
        r#"test -f "${XDG_CONFIG_HOME:-$HOME/.config}/fish/config.fish"; and source "${XDG_CONFIG_HOME:-$HOME/.config}/fish/config.fish" 2>/dev/null"#
    } else {
        ""
    };

    let cmd = if rc_cmd.is_empty() {
        r#"printf '%s' "$PATH""#.to_string()
    } else if shell.contains("fish") {
        // Fish uses begin/end instead of { } for command grouping
        format!(
            "begin; {}; end 2>/dev/null; printf '%s' \"$PATH\"",
            rc_cmd
        )
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

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    #[test]
    fn extract_path_value_double_quoted() {
        let output = r#"export PATH="/usr/local/bin:/usr/bin";"#;
        assert_eq!(
            extract_path_value(output, "PATH=\"", "\";"),
            Some("/usr/local/bin:/usr/bin".to_string())
        );
    }

    #[test]
    fn extract_path_value_single_quoted() {
        let output = r#"export PATH='/usr/local/bin:/usr/bin';"#;
        assert_eq!(
            extract_path_value(output, "PATH='", "';"),
            Some("/usr/local/bin:/usr/bin".to_string())
        );
    }

    #[test]
    fn extract_path_value_not_found() {
        assert_eq!(extract_path_value("echo hello", "PATH=\"", "\";"), None);
    }

    #[test]
    fn extract_path_value_empty() {
        let output = r#"export PATH="""#;
        assert_eq!(extract_path_value(output, "PATH=\"", "\";"), None);
    }

    #[test]
    fn parse_path_helper_output_double_quoted() {
        let output = r#"export PATH="/opt/homebrew/bin:/usr/bin";"#;
        assert_eq!(
            parse_path_helper_output(output),
            Some("/opt/homebrew/bin:/usr/bin".to_string())
        );
    }

    #[test]
    fn parse_path_helper_output_single_quoted() {
        let output = r#"export PATH='/opt/homebrew/bin:/usr/bin';"#;
        assert_eq!(
            parse_path_helper_output(output),
            Some("/opt/homebrew/bin:/usr/bin".to_string())
        );
    }

    #[test]
    fn parse_path_helper_output_missing_marker() {
        assert_eq!(parse_path_helper_output("echo hello"), None);
    }

    #[test]
    fn parse_path_helper_output_whitespace_trimmed() {
        let output = r#"export PATH="  /usr/bin  ";"#;
        assert_eq!(
            parse_path_helper_output(output),
            Some("/usr/bin".to_string())
        );
    }

    #[test]
    fn dedup_and_join_removes_duplicates() {
        let entries = vec![
            "/usr/bin".to_string(),
            "/usr/local/bin".to_string(),
            "/usr/bin".to_string(),
        ];
        let result = dedup_and_join(entries);
        assert_eq!(result, "/usr/bin:/usr/local/bin");
    }

    #[test]
    fn split_path_entries_handles_trailing_colon() {
        let result = split_path_entries("/usr/bin:/usr/local/bin:");
        assert_eq!(result, vec!["/usr/bin", "/usr/local/bin"]);
    }
}
