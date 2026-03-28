use std::{
    ffi::OsString,
    path::PathBuf,
    time::Duration,
};

use anyhow::{Context, Result};
use mcpmate::common::global_paths;
use service_manager::{
    RestartPolicy, ServiceInstallCtx, ServiceLabel, ServiceLevel, ServiceManager,
    ServiceStartCtx, ServiceStatus, ServiceStatusCtx, ServiceStopCtx, ServiceUninstallCtx,
};
use tauri::{AppHandle, Manager};

use crate::source_config::DesktopCoreSourceConfig;

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
pub struct LocalCoreServiceStatusView {
    pub status: LocalCoreServiceStatusKind,
    pub label: String,
    pub detail: String,
    pub level: String,
    pub installed: bool,
    pub running: bool,
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
    let mut manager = <dyn ServiceManager>::native().context("failed to create native service manager")?;
    manager
        .set_level(resolve_service_level())
        .context("failed to configure service manager level")?;
    Ok(manager)
}

pub fn resolve_local_core_binary(app: &AppHandle) -> Result<PathBuf> {
    let exe_suffix = std::env::consts::EXE_SUFFIX;
    let target = std::env::var("TAURI_ENV_TARGET_TRIPLE")
        .or_else(|_| std::env::var("TARGET"))
        .unwrap_or_else(|_| format!("{}-unknown-{}", std::env::consts::ARCH, std::env::consts::OS));
    let mut candidates: Vec<PathBuf> = Vec::new();

    // For release builds, check MacOS directory first (where Tauri bundles sidecars)
    // The app bundle structure is: MCPMate.app/Contents/MacOS/mcpmate-core
    if let Ok(resource_dir) = app.path().resource_dir() {
        // Try MacOS directory (sibling to Resources)
        if let Some(contents_dir) = resource_dir.parent() {
            let macos_dir = contents_dir.join("MacOS");
            candidates.push(macos_dir.join(format!("mcpmate-core-{target}{exe_suffix}")));
            candidates.push(macos_dir.join(format!("mcpmate-core{exe_suffix}")));
        }

        // Also check Resources directory (standard Tauri resource location)
        candidates.push(resource_dir.join(format!("mcpmate-core-{target}{exe_suffix}")));
        candidates.push(resource_dir.join(format!("mcpmate-core{exe_suffix}")));
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

pub async fn install_local_service(
    app: &AppHandle,
    config: &DesktopCoreSourceConfig,
) -> Result<LocalCoreServiceStatusView> {
    ensure_local_service_definition(app, config)?;
    read_local_service_status(config).await
}

pub fn uninstall_local_service(_config: &DesktopCoreSourceConfig) -> Result<LocalCoreServiceStatusView> {
	let status = {
		let manager = service_manager()?;
		manager.status(ServiceStatusCtx { label: service_label()? })?
	};

	if matches!(status, ServiceStatus::NotInstalled) {
		return Ok(LocalCoreServiceStatusView {
			status: LocalCoreServiceStatusKind::NotInstalled,
			label: "Not Installed".to_string(),
			detail: "The localhost core service has not been installed yet.".to_string(),
			level: level_label(resolve_service_level()),
			installed: false,
			running: false,
		});
	}

	{
		let manager = service_manager()?;
		let _ = manager.stop(ServiceStopCtx { label: service_label()? });
		manager
			.uninstall(ServiceUninstallCtx { label: service_label()? })
			.context("failed to uninstall local core service")?;
	}

	Ok(LocalCoreServiceStatusView {
		status: LocalCoreServiceStatusKind::NotInstalled,
		label: "Not Installed".to_string(),
		detail: "The localhost core service was removed from the OS service manager.".to_string(),
		level: level_label(resolve_service_level()),
		installed: false,
		running: false,
	})
}

fn service_install_ctx(app: &AppHandle, config: &DesktopCoreSourceConfig) -> Result<ServiceInstallCtx> {
    let base_dir = global_paths().base_dir().to_path_buf();
    let label = service_label()?;
    let program = resolve_local_core_binary(app)?;

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
        working_directory: Some(base_dir.clone()),
        environment: Some(vec![
            ("MCPMATE_DATA_DIR".to_string(), base_dir.display().to_string()),
            (
                "MCPMATE_API_PORT".to_string(),
                config.localhost.api_port.to_string(),
            ),
            (
                "MCPMATE_MCP_PORT".to_string(),
                config.localhost.mcp_port.to_string(),
            ),
        ]),
        autostart: true,
        restart_policy: RestartPolicy::OnFailure {
            delay_secs: Some(5),
            max_retries: None,
            reset_after_secs: Some(3600),
        },
    })
}

pub async fn probe_localhost_core(api_port: u16) -> bool {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(500))
        .build();

    let Ok(client) = client else {
        return false;
    };

    let url = format!("http://127.0.0.1:{api_port}/api/system/status");
    match client.get(url).send().await {
        Ok(response) => response.status().is_success(),
        Err(_) => false,
    }
}

pub async fn wait_for_localhost_core(api_port: u16) -> Result<()> {
    for _ in 0..20 {
        if probe_localhost_core(api_port).await {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }

    anyhow::bail!("localhost core service did not become ready in time")
}

pub async fn wait_for_localhost_core_stopped(api_port: u16) -> bool {
    for _ in 0..20 {
        if !probe_localhost_core(api_port).await {
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

pub async fn read_local_service_status(config: &DesktopCoreSourceConfig) -> Result<LocalCoreServiceStatusView> {
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
        },
        ServiceStatus::Stopped(reason) => LocalCoreServiceStatusView {
            status: LocalCoreServiceStatusKind::Stopped,
            label: "Stopped".to_string(),
            detail: reason.unwrap_or_else(|| "The localhost core service is installed but not running.".to_string()),
            level,
            installed: true,
            running: false,
        },
        ServiceStatus::Running => {
            if probe_localhost_core(config.localhost.api_port).await {
                LocalCoreServiceStatusView {
                    status: LocalCoreServiceStatusKind::Running,
                    label: "Running".to_string(),
                    detail: "The localhost core service is running and responding to health checks.".to_string(),
                    level,
                    installed: true,
                    running: true,
                }
            } else {
                LocalCoreServiceStatusView {
                    status: LocalCoreServiceStatusKind::RunningUnhealthy,
                    label: "Running (Unhealthy)".to_string(),
                    detail: "The service manager reports the localhost core as running, but the API health check is failing.".to_string(),
                    level,
                    installed: true,
                    running: true,
                }
            }
        }
    };

    Ok(view)
}

pub fn ensure_local_service_definition(app: &AppHandle, config: &DesktopCoreSourceConfig) -> Result<()> {
    let manager = service_manager()?;
    let status = manager.status(ServiceStatusCtx { label: service_label()? })?;
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

pub async fn start_local_service(app: &AppHandle, config: &DesktopCoreSourceConfig) -> Result<LocalCoreServiceStatusView> {
    ensure_local_service_definition(app, config)?;
    {
        let manager = service_manager()?;
        manager
            .start(ServiceStartCtx {
                label: service_label()?,
            })
            .context("failed to start local core service")?;
    }
    wait_for_localhost_core(config.localhost.api_port).await?;
    read_local_service_status(config).await
}

pub async fn restart_local_service(app: &AppHandle, config: &DesktopCoreSourceConfig) -> Result<LocalCoreServiceStatusView> {
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
    wait_for_localhost_core(config.localhost.api_port).await?;
    read_local_service_status(config).await
}

pub async fn stop_local_service(config: &DesktopCoreSourceConfig) -> Result<LocalCoreServiceStatusView> {
    let label = service_label()?;

    let status = {
        let manager = service_manager()?;
        manager.status(ServiceStatusCtx { label: label.clone() })?
    };

    if matches!(status, ServiceStatus::NotInstalled | ServiceStatus::Stopped(_)) {
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
