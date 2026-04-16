use std::{ffi::OsString, path::{Path, PathBuf}, time::Duration};

use anyhow::{Context, Result};
use mcpmate::common::global_paths;
use service_manager::{
    RestartPolicy, ServiceInstallCtx, ServiceLabel, ServiceLevel, ServiceManager, ServiceStartCtx,
    ServiceStatus, ServiceStatusCtx, ServiceStopCtx, ServiceUninstallCtx,
};
use tauri::{AppHandle, Manager};

use crate::source_config::DesktopCoreSourceConfig;

#[derive(Debug, serde::Deserialize)]
struct SystemStatusProbe {
    #[serde(default)]
    desktop_managed_token: Option<String>,
}

#[derive(Debug, Clone)]
enum LocalhostCoreProbeResult {
    Unreachable,
    HttpStatus(reqwest::StatusCode),
    InvalidPayload,
    Reachable { desktop_managed_token: Option<String> },
}

impl LocalhostCoreProbeResult {
    fn matches_expected_token(&self, expected_token: Option<&str>) -> bool {
        match (self, expected_token) {
            (Self::Reachable { .. }, None) => true,
            (Self::Reachable { desktop_managed_token }, Some(expected_token)) => {
                desktop_managed_token.as_deref() == Some(expected_token)
            }
            _ => false,
        }
    }

    fn timeout_detail(&self, expected_token: Option<&str>) -> String {
        match (self, expected_token) {
            (Self::Unreachable, _) => {
                "localhost core did not respond on /api/system/status".to_string()
            }
            (Self::HttpStatus(status), _) => format!(
                "localhost core returned HTTP {} from /api/system/status",
                status.as_u16()
            ),
            (Self::InvalidPayload, _) => {
                "localhost core responded, but /api/system/status returned an invalid payload"
                    .to_string()
            }
            (
                Self::Reachable {
                    desktop_managed_token: None,
                },
                Some(_),
            ) => {
                "localhost core responded, but desktop_managed_token was missing"
                    .to_string()
            }
            (
                Self::Reachable {
                    desktop_managed_token: Some(actual_token),
                },
                Some(expected_token),
            ) => format!(
                "localhost core responded, but desktop_managed_token mismatched (expected {}, got {})",
                expected_token, actual_token
            ),
            (Self::Reachable { .. }, None) => {
                "localhost core responded successfully".to_string()
            }
        }
    }
}

impl std::fmt::Display for LocalhostCoreProbeResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreachable => write!(f, "unreachable"),
            Self::HttpStatus(status) => write!(f, "http_status:{}", status.as_u16()),
            Self::InvalidPayload => write!(f, "invalid_payload"),
            Self::Reachable {
                desktop_managed_token: Some(token),
            } => write!(f, "reachable(token={token})"),
            Self::Reachable {
                desktop_managed_token: None,
            } => write!(f, "reachable(token=missing)"),
        }
    }
}

async fn probe_localhost_core_result(api_port: u16) -> LocalhostCoreProbeResult {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(1200))
        .build();

    let Ok(client) = client else {
        return LocalhostCoreProbeResult::Unreachable;
    };

    let url = format!("http://127.0.0.1:{api_port}/api/system/status");
    match client.get(url).send().await {
        Ok(response) if response.status().is_success() => {
            match response.json::<SystemStatusProbe>().await {
                Ok(payload) => LocalhostCoreProbeResult::Reachable {
                    desktop_managed_token: payload.desktop_managed_token,
                },
                Err(_) => LocalhostCoreProbeResult::InvalidPayload,
            }
        }
        Ok(response) => LocalhostCoreProbeResult::HttpStatus(response.status()),
        Err(_) => LocalhostCoreProbeResult::Unreachable,
    }
}

pub async fn describe_localhost_core_probe(api_port: u16) -> String {
    probe_localhost_core_result(api_port).await.to_string()
}

pub const LOCAL_CORE_SERVICE_LABEL: &str = "ai.umate.mcpmate.core";

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LocalCoreServiceStatusKind {
    NotInstalled,
    Stopped,
    Running,
    RunningUnhealthy,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalCoreServiceDiagnosticsView {
    pub startup_log_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub startup_log_tail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalCoreServiceStatusView {
    pub status: LocalCoreServiceStatusKind,
    pub label: String,
    pub detail: String,
    pub level: String,
    pub installed: bool,
    pub running: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<LocalCoreServiceDiagnosticsView>,
}

impl LocalCoreServiceStatusView {
    pub fn is_active_for_menu(&self) -> bool {
        matches!(
            self.status,
            LocalCoreServiceStatusKind::Running | LocalCoreServiceStatusKind::RunningUnhealthy
        )
    }
}

pub fn resolve_service_level() -> ServiceLevel {
    #[cfg(target_os = "windows")]
    {
        ServiceLevel::System
    }

    #[cfg(not(target_os = "windows"))]
    {
        ServiceLevel::User
    }
}

fn level_label(level: ServiceLevel) -> String {
    match level {
        ServiceLevel::System => "system".to_string(),
        ServiceLevel::User => "user".to_string(),
    }
}

fn service_label() -> Result<ServiceLabel> {
    LOCAL_CORE_SERVICE_LABEL
        .parse()
        .context("failed to parse local core service label")
}

fn service_manager() -> Result<Box<dyn ServiceManager>> {
    let mut manager =
        <dyn ServiceManager>::native().context("failed to create native service manager")?;
    manager
        .set_level(resolve_service_level())
        .context("failed to configure service manager level")?;
    Ok(manager)
}

pub fn resolve_local_core_binary(app: &AppHandle) -> Result<PathBuf> {
    let exe_suffix = std::env::consts::EXE_SUFFIX;
    let target = std::env::var("TAURI_ENV_TARGET_TRIPLE")
        .or_else(|_| std::env::var("TARGET"))
        .unwrap_or_else(|_| {
            format!(
                "{}-unknown-{}",
                std::env::consts::ARCH,
                std::env::consts::OS
            )
        });
    let mut candidates: Vec<PathBuf> = Vec::new();
    let push_sidecar_candidates = |candidates: &mut Vec<PathBuf>, base_dir: &std::path::Path| {
        candidates.push(base_dir.join(format!("mcpmate-core-{target}{exe_suffix}")));
        candidates.push(base_dir.join(format!("mcpmate-core{exe_suffix}")));
    };

    // For release builds, check MacOS directory first (where Tauri bundles sidecars)
    // The app bundle structure is: MCPMate.app/Contents/MacOS/mcpmate-core
    if let Ok(resource_dir) = app.path().resource_dir() {
        // Try MacOS directory (sibling to Resources)
        if let Some(contents_dir) = resource_dir.parent() {
            let macos_dir = contents_dir.join("MacOS");
            push_sidecar_candidates(&mut candidates, &macos_dir);

            // Linux/Windows packages commonly place sidecars next to the main executable or
            // under the parent resource directory rather than inside Resources.
            push_sidecar_candidates(&mut candidates, contents_dir);
        }

        // Also check Resources directory (standard Tauri resource location)
        push_sidecar_candidates(&mut candidates, &resource_dir);

        // On Windows/Linux, Tauri places externalBin sidecars in a bin subdirectory
        // alongside the main executable, or directly next to resources. Check these
        // locations to support packaged distributions (MSI, deb, etc.)
        #[cfg(target_os = "windows")]
        if let Some(parent) = resource_dir.parent() {
            let bin_dir = parent.join("bin");
            push_sidecar_candidates(&mut candidates, &bin_dir);
        }

        #[cfg(target_os = "linux")]
        {
            if let Some(parent) = resource_dir.parent() {
                let bin_dir = parent.join("bin");
                push_sidecar_candidates(&mut candidates, &bin_dir);
            }
            // On Linux AppImage or systemd-installed builds, also check /usr/local/bin
            candidates.push(PathBuf::from("/usr/local/bin").join(format!("mcpmate-core{exe_suffix}")));
            candidates.push(PathBuf::from("/usr/bin").join(format!("mcpmate-core{exe_suffix}")));
        }
    }

    if let Ok(current_exe) = std::env::current_exe()
        && let Some(exe_dir) = current_exe.parent()
    {
        push_sidecar_candidates(&mut candidates, exe_dir);
    }

    // For debug builds, check workspace target directories
    if cfg!(debug_assertions) {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = manifest_dir.join("../../..");
        let profile_dir = "debug";
        candidates.push(
            workspace_root
                .join("backend/target")
                .join(&target)
                .join(profile_dir)
                .join(format!("mcpmate{exe_suffix}")),
        );
        candidates.push(
            workspace_root
                .join("backend/target/sidecars")
                .join(format!("mcpmate-core-{target}{exe_suffix}")),
        );
        candidates.push(
            workspace_root
                .join("backend/target/sidecars")
                .join(format!("mcpmate-core{exe_suffix}")),
        );
    }

    candidates
        .into_iter()
        .find(|path| path.exists())
        .context("unable to resolve local MCPMate core service binary")
}

pub fn resolve_local_core_working_dir(binary: &Path, base_dir: &Path) -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        binary
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| base_dir.to_path_buf())
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = binary;
        base_dir.to_path_buf()
    }
}

pub async fn install_local_service(
    app: &AppHandle,
    config: &DesktopCoreSourceConfig,
) -> Result<LocalCoreServiceStatusView> {
    ensure_local_service_definition(app, config)?;
    read_local_service_status(config).await
}

pub fn uninstall_local_service(
    _config: &DesktopCoreSourceConfig,
) -> Result<LocalCoreServiceStatusView> {
    let status = {
        let manager = service_manager()?;
        manager.status(ServiceStatusCtx {
            label: service_label()?,
        })?
    };

    if matches!(status, ServiceStatus::NotInstalled) {
        return Ok(LocalCoreServiceStatusView {
            status: LocalCoreServiceStatusKind::NotInstalled,
            label: "Not Installed".to_string(),
            detail: "The localhost core service has not been installed yet.".to_string(),
            level: level_label(resolve_service_level()),
            installed: false,
            running: false,
            diagnostics: None,
        });
    }

    {
        let manager = service_manager()?;
        let _ = manager.stop(ServiceStopCtx {
            label: service_label()?,
        });
        manager
            .uninstall(ServiceUninstallCtx {
                label: service_label()?,
            })
            .context("failed to uninstall local core service")?;
    }

    Ok(LocalCoreServiceStatusView {
        status: LocalCoreServiceStatusKind::NotInstalled,
        label: "Not Installed".to_string(),
        detail: "The localhost core service was removed from the OS service manager.".to_string(),
        level: level_label(resolve_service_level()),
        installed: false,
        running: false,
        diagnostics: None,
    })
}

fn service_install_ctx(
    app: &AppHandle,
    config: &DesktopCoreSourceConfig,
) -> Result<ServiceInstallCtx> {
    let base_dir = global_paths().base_dir().to_path_buf();
    let label = service_label()?;
    let program = resolve_local_core_binary(app)?;
    let working_directory = resolve_local_core_working_dir(&program, &base_dir);
    let environment = crate::runtime_env::merge_service_environment(vec![
        (
            "MCPMATE_DATA_DIR".to_string(),
            base_dir.display().to_string(),
        ),
        (
            "MCPMATE_API_PORT".to_string(),
            config.localhost.api_port.to_string(),
        ),
        (
            "MCPMATE_MCP_PORT".to_string(),
            config.localhost.mcp_port.to_string(),
        ),
    ]);

    Ok(ServiceInstallCtx {
        label,
        program,
        args: vec![
            OsString::from("--api-port"),
            OsString::from(config.localhost.api_port.to_string()),
            OsString::from("--mcp-port"),
            OsString::from(config.localhost.mcp_port.to_string()),
            OsString::from("--log-level"),
            OsString::from(
                std::env::var("MCPMATE_TAURI_LOG")
                    .ok()
                    .map(|v| v.trim().to_string())
                    .filter(|v| !v.is_empty())
                    .unwrap_or_else(|| "info".to_string()),
            ),
        ],
        contents: None,
        username: None,
        working_directory: Some(working_directory),
        environment: Some(environment),
        autostart: true,
        restart_policy: RestartPolicy::OnFailure {
            delay_secs: Some(5),
            max_retries: None,
            reset_after_secs: Some(3600),
        },
    })
}

pub async fn probe_localhost_core(api_port: u16, expected_token: Option<&str>) -> bool {
    probe_localhost_core_result(api_port)
        .await
        .matches_expected_token(expected_token)
}

pub async fn wait_for_localhost_core(api_port: u16, expected_token: Option<&str>) -> Result<()> {
    let mut last_probe = LocalhostCoreProbeResult::Unreachable;

    for _ in 0..40 {
        let probe = probe_localhost_core_result(api_port).await;
        if probe.matches_expected_token(None) {
            return Ok(());
        }
        last_probe = probe;
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    anyhow::bail!(
        "localhost core service did not become ready in time: {}",
        last_probe.timeout_detail(expected_token)
    )
}

pub async fn wait_for_localhost_core_stopped(api_port: u16) -> bool {
    for _ in 0..20 {
        if !probe_localhost_core(api_port, None).await {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }

    false
}

/// Wait for a port to become available (not in use)
pub async fn wait_for_port_available(port: u16) -> Result<()> {
    use std::net::TcpListener;

    for attempt in 0..30 {
        // Try to bind to the port to check if it's available
        match TcpListener::bind(format!("127.0.0.1:{}", port)) {
            Ok(_) => {
                // Port is available
                return Ok(());
            }
            Err(_) => {
                // Port still in use, wait and retry
                if attempt < 29 {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
        }
    }

    anyhow::bail!("Port {} did not become available after 15 seconds", port)
}

pub async fn read_local_service_status(
    config: &DesktopCoreSourceConfig,
) -> Result<LocalCoreServiceStatusView> {
    let (level, status) = {
        let manager = service_manager()?;
        let level = level_label(manager.level());
        let status = manager
            .status(ServiceStatusCtx {
                label: service_label()?,
            })
            .context("failed to query local core service status")?;
        (level, status)
    };

    let view = match status {
        ServiceStatus::NotInstalled => LocalCoreServiceStatusView {
            status: LocalCoreServiceStatusKind::NotInstalled,
            label: "Not Installed".to_string(),
            detail: "The localhost core service has not been installed yet.".to_string(),
            level,
            installed: false,
            running: false,
            diagnostics: None,
        },
        ServiceStatus::Stopped(reason) => LocalCoreServiceStatusView {
            status: LocalCoreServiceStatusKind::Stopped,
            label: "Stopped".to_string(),
            detail: reason.unwrap_or_else(|| {
                "The localhost core service is installed but not running.".to_string()
            }),
            level,
            installed: true,
            running: false,
            diagnostics: None,
        },
        ServiceStatus::Running => {
            if probe_localhost_core(config.localhost.api_port, None).await {
                LocalCoreServiceStatusView {
                    status: LocalCoreServiceStatusKind::Running,
                    label: "Running".to_string(),
                    detail:
                        "The localhost core service is running and responding to health checks."
                            .to_string(),
                    level,
                    installed: true,
                    running: true,
                    diagnostics: None,
                }
            } else {
                LocalCoreServiceStatusView {
                    status: LocalCoreServiceStatusKind::RunningUnhealthy,
                    label: "Running (Unhealthy)".to_string(),
                    detail: "The service manager reports the localhost core as running, but the API health check is failing.".to_string(),
                    level,
                    installed: true,
                    running: true,
                    diagnostics: None,
                }
            }
        }
    };

    Ok(view)
}

pub fn ensure_local_service_definition(
    app: &AppHandle,
    config: &DesktopCoreSourceConfig,
) -> Result<()> {
    let manager = service_manager()?;
    let status = manager.status(ServiceStatusCtx {
        label: service_label()?,
    })?;
    let install_ctx = service_install_ctx(app, config)?;

    if !matches!(status, ServiceStatus::NotInstalled) {
        let _ = manager.stop(ServiceStopCtx {
            label: service_label()?,
        });
        manager
            .uninstall(ServiceUninstallCtx {
                label: service_label()?,
            })
            .context("failed to remove previous local core service definition")?;
    }

    manager
        .install(install_ctx)
        .context("failed to install local core service definition")?;

    Ok(())
}

pub async fn start_local_service(
    app: &AppHandle,
    config: &DesktopCoreSourceConfig,
) -> Result<LocalCoreServiceStatusView> {
    ensure_local_service_definition(app, config)?;
    {
        let manager = service_manager()?;
        manager
            .start(ServiceStartCtx {
                label: service_label()?,
            })
            .context("failed to start local core service")?;
    }
    wait_for_localhost_core(config.localhost.api_port, None).await?;
    read_local_service_status(config).await
}

pub async fn restart_local_service(
    app: &AppHandle,
    config: &DesktopCoreSourceConfig,
) -> Result<LocalCoreServiceStatusView> {
    ensure_local_service_definition(app, config)?;
    {
        let manager = service_manager()?;
        let _ = manager.stop(ServiceStopCtx {
            label: service_label()?,
        });
        manager
            .start(ServiceStartCtx {
                label: service_label()?,
            })
            .context("failed to restart local core service")?;
    }
    wait_for_localhost_core(config.localhost.api_port, None).await?;
    read_local_service_status(config).await
}

pub async fn stop_local_service(
    config: &DesktopCoreSourceConfig,
) -> Result<LocalCoreServiceStatusView> {
    let label = service_label()?;

    let status = {
        let manager = service_manager()?;
        manager.status(ServiceStatusCtx {
            label: label.clone(),
        })?
    };

    if matches!(
        status,
        ServiceStatus::NotInstalled | ServiceStatus::Stopped(_)
    ) {
        return read_local_service_status(config).await;
    }

    {
        let manager = service_manager()?;
        manager
            .stop(ServiceStopCtx { label })
            .context("failed to stop local core service")?;
    }

    let _ = wait_for_localhost_core_stopped(config.localhost.api_port).await;

    for _ in 0..10 {
        let view = read_local_service_status(config).await?;
        if !view.running {
            return Ok(view);
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }

    read_local_service_status(config).await
}

pub async fn sync_local_service_definition(
    app: &AppHandle,
    config: &DesktopCoreSourceConfig,
) -> Result<LocalCoreServiceStatusView> {
    let current = read_local_service_status(config).await?;
    if !current.installed {
        return Ok(current);
    }

    let should_restart = current.is_active_for_menu();
    ensure_local_service_definition(app, config)?;
    if should_restart {
        start_local_service(app, config).await
    } else {
        read_local_service_status(config).await
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_local_core_working_dir;
    use std::path::Path;

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_working_dir_prefers_binary_parent() {
        let binary = Path::new(r"C:\Program Files\MCPMate\mcpmate-core.exe");
        let base_dir = Path::new(r"C:\Users\tester\AppData\Roaming\MCPMate");

        assert_eq!(
            resolve_local_core_working_dir(binary, base_dir),
            Path::new(r"C:\Program Files\MCPMate")
        );
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn non_windows_working_dir_stays_on_base_dir() {
        let binary = Path::new("/opt/MCPMate/mcpmate-core");
        let base_dir = Path::new("/var/lib/mcpmate");

        assert_eq!(resolve_local_core_working_dir(binary, base_dir), base_dir);
    }
}
