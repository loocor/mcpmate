use std::time::Duration;

use crate::common::server::ServerType;

pub(crate) const DEFAULT_CAPABILITY_OPERATION_TIMEOUT: Duration = Duration::from_secs(30);
pub(crate) const DEFAULT_DIRECT_STARTUP_TIMEOUT: Duration = Duration::from_secs(60);
pub(crate) const DEFAULT_DOCKER_STARTUP_TIMEOUT: Duration = Duration::from_secs(120);
pub(crate) const DEFAULT_PACKAGE_RUNNER_STARTUP_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Debug, thiserror::Error)]
#[error("server startup timed out after {timeout_ms} ms")]
pub(crate) struct ServerStartupTimeout {
    pub(crate) timeout_ms: u128,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct McpTimeoutPolicy {
    pub(crate) startup: Duration,
    pub(crate) capability_operation: Duration,
}

impl McpTimeoutPolicy {
    pub(crate) fn for_server(
        server_type: ServerType,
        command: Option<&str>,
        operation_override: Option<Duration>,
    ) -> Self {
        let startup = match server_type {
            ServerType::Stdio if command.is_some_and(is_package_runner) => DEFAULT_PACKAGE_RUNNER_STARTUP_TIMEOUT,
            ServerType::Stdio if command.is_some_and(is_docker) => DEFAULT_DOCKER_STARTUP_TIMEOUT,
            ServerType::Stdio | ServerType::Sse | ServerType::StreamableHttp => DEFAULT_DIRECT_STARTUP_TIMEOUT,
        };

        Self {
            startup,
            capability_operation: operation_override.unwrap_or(DEFAULT_CAPABILITY_OPERATION_TIMEOUT),
        }
    }
}

pub(crate) fn is_package_runner(command: &str) -> bool {
    matches!(normalized_executable(command).as_str(), "bunx" | "npx" | "uvx")
}

fn is_docker(command: &str) -> bool {
    normalized_executable(command) == "docker"
}

fn normalized_executable(command: &str) -> String {
    let executable = command
        .trim()
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    executable
        .strip_suffix(".exe")
        .or_else(|| executable.strip_suffix(".cmd"))
        .or_else(|| executable.strip_suffix(".bat"))
        .unwrap_or(&executable)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_CAPABILITY_OPERATION_TIMEOUT, DEFAULT_DIRECT_STARTUP_TIMEOUT, DEFAULT_DOCKER_STARTUP_TIMEOUT,
        DEFAULT_PACKAGE_RUNNER_STARTUP_TIMEOUT, McpTimeoutPolicy, is_package_runner,
    };
    use crate::common::server::ServerType;
    use std::time::Duration;

    #[test]
    fn package_runner_detection_normalizes_paths_and_windows_suffixes() {
        for command in ["uvx", "/managed/bin/bunx", r"C:\runtime\npx.exe"] {
            assert!(is_package_runner(command), "{command} should be a package runner");
        }
        assert!(!is_package_runner("paddleocr_mcp"));
    }

    #[test]
    fn startup_timeout_depends_on_the_server_launch_class() {
        assert_eq!(
            McpTimeoutPolicy::for_server(ServerType::Stdio, Some("uvx"), None).startup,
            DEFAULT_PACKAGE_RUNNER_STARTUP_TIMEOUT
        );
        assert_eq!(
            McpTimeoutPolicy::for_server(ServerType::Stdio, Some("docker"), None).startup,
            DEFAULT_DOCKER_STARTUP_TIMEOUT
        );
        assert_eq!(
            McpTimeoutPolicy::for_server(ServerType::Stdio, Some("paddleocr_mcp"), None).startup,
            DEFAULT_DIRECT_STARTUP_TIMEOUT
        );
        assert_eq!(
            McpTimeoutPolicy::for_server(ServerType::StreamableHttp, None, None).startup,
            DEFAULT_DIRECT_STARTUP_TIMEOUT
        );
    }

    #[test]
    fn capability_operation_override_never_changes_startup_timeout() {
        let policy = McpTimeoutPolicy::for_server(ServerType::Stdio, Some("uvx"), Some(Duration::from_secs(17)));

        assert_eq!(policy.startup, DEFAULT_PACKAGE_RUNNER_STARTUP_TIMEOUT);
        assert_eq!(policy.capability_operation, Duration::from_secs(17));
    }

    #[test]
    fn capability_operations_default_to_thirty_seconds() {
        let policy = McpTimeoutPolicy::for_server(ServerType::Sse, None, None);

        assert_eq!(policy.capability_operation, DEFAULT_CAPABILITY_OPERATION_TIMEOUT);
    }
}
