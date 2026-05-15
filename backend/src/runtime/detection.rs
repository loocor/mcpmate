//! Runtime availability detection scoped to the connection pool's execution environment.
//!
//! Uses the unified resolver (managed → PATH → UV python) to locate commands,
//! then probes each with `--version` to confirm it actually works.

use std::{
    process::Output,
    time::Duration,
};

use super::resolver::CommandResolver;
use crate::common::MCPMatePaths;

const RUNTIME_CHECK_COMMAND_TIMEOUT: Duration = Duration::from_secs(5);

// ── Probe & detection types ──────────────────────────────────────────────

#[derive(Clone, Copy, Debug)]
pub struct RuntimeProbe<'a> {
    pub command: &'a str,
    pub args: &'a [&'a str],
}

#[derive(Debug)]
pub struct RuntimeDetection {
    pub name: String,
    pub available: bool,
    pub version: Option<String>,
    pub path: Option<String>,
    pub resolve_source: Option<super::resolver::ResolveSource>,
}

// ── Detector ─────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct RuntimeDetector {
    resolver: CommandResolver,
}

impl RuntimeDetector {
    /// Create a new runtime detector with explicit paths.
    /// Builds the four-layer enriched PATH once on construction.
    pub fn new(paths: &MCPMatePaths) -> Self {
        Self {
            resolver: CommandResolver::new(paths),
        }
    }

    pub async fn detect(
        &mut self,
        name: &str,
        probes: &[RuntimeProbe<'_>],
    ) -> RuntimeDetection {
        for &probe in probes {
            if let Some(detection) = self.detect_probe(name, probe).await {
                return detection;
            }
        }

        RuntimeDetection {
            name: name.to_string(),
            available: false,
            version: None,
            path: None,
            resolve_source: None,
        }
    }

    async fn detect_probe(
        &mut self,
        name: &str,
        probe: RuntimeProbe<'_>,
    ) -> Option<RuntimeDetection> {
        let resolved = self.resolver.resolve(probe.command)?;

        if let Some(output) = run_runtime_check_command(
            resolved.path.as_os_str(),
            probe.args,
        )
        .await
        {
            return Some(RuntimeDetection {
                name: name.to_string(),
                available: true,
                version: normalize_runtime_version(&output.stdout, &output.stderr),
                path: Some(resolved.path.display().to_string()),
                resolve_source: Some(resolved.source),
            });
        }

        None
    }
}

// ── Command execution helpers ────────────────────────────────────────────

async fn run_runtime_check_command(
    program: &std::ffi::OsStr,
    args: &[&str],
) -> Option<Output> {
    run_command_with_timeout(program, args, RUNTIME_CHECK_COMMAND_TIMEOUT)
        .await
        .filter(|output| output.status.success())
}

async fn run_command_with_timeout(
    program: &std::ffi::OsStr,
    args: &[&str],
    timeout: Duration,
) -> Option<Output> {
    let mut command = tokio::process::Command::new(program);
    command.kill_on_drop(true).args(args);

    match tokio::time::timeout(timeout, command.output()).await {
        Ok(Ok(output)) if output.status.success() => Some(output),
        Ok(Ok(output)) => {
            tracing::warn!(
                "Runtime check command {:?} failed: exit={}",
                program,
                output.status
            );
            None
        }
        Ok(Err(e)) => {
            tracing::warn!("Runtime check command {:?} spawn error: {}", program, e);
            None
        }
        Err(_) => {
            tracing::debug!("Runtime check command {:?} timed out", program);
            None
        }
    }
}

// ── Version parsing ──────────────────────────────────────────────────────

fn normalize_runtime_version(
    stdout: &[u8],
    stderr: &[u8],
) -> Option<String> {
    let stdout = String::from_utf8_lossy(stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(stderr).trim().to_string();
    let raw = if stdout.is_empty() { stderr } else { stdout };
    let trimmed = raw
        .split_once('(')
        .map(|(head, _)| head.trim().to_string())
        .unwrap_or(raw);
    if trimmed.is_empty() { None } else { Some(trimmed) }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Timeout ──────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn runtime_check_command_times_out_for_long_running_process() {
        #[cfg(unix)]
        let result = run_command_with_timeout(std::ffi::OsStr::new("sh"), &["-c", "sleep 1"], Duration::from_millis(50)).await;

        #[cfg(windows)]
        let result = run_command_with_timeout(
            std::ffi::OsStr::new("cmd"),
            &["/C", "ping -n 2 127.0.0.1 >NUL"],
            Duration::from_millis(50),
        )
        .await;

        assert!(result.is_none());
    }

    // ── Version parsing ──────────────────────────────────────────────────────

    #[test]
    fn normalize_runtime_version_extracts_stdout_first() {
        assert_eq!(
            normalize_runtime_version(b"v22.12.0\n", b"").as_deref(),
            Some("v22.12.0")
        );
    }

    #[test]
    fn normalize_runtime_version_falls_back_to_stderr() {
        assert_eq!(
            normalize_runtime_version(b"", b"Python 3.12.2\n").as_deref(),
            Some("Python 3.12.2")
        );
    }

    #[test]
    fn normalize_runtime_version_trims_parenthesized_details() {
        assert_eq!(
            normalize_runtime_version(b"uv 0.5.11 (Homebrew 2024-12-01)\n", b"").as_deref(),
            Some("uv 0.5.11")
        );
    }

    #[test]
    fn normalize_runtime_version_returns_none_for_blank_output() {
        assert!(normalize_runtime_version(b"\n", b"  \n").is_none());
    }

    #[test]
    fn runtime_detection_unavailable_shape_is_explicit() {
        let detection = RuntimeDetection {
            name: "node".to_string(),
            available: false,
            version: None,
            path: None,
            resolve_source: None,
        };

        assert_eq!(detection.name, "node");
        assert!(!detection.available);
        assert!(detection.version.is_none());
        assert!(detection.path.is_none());
    }
}
