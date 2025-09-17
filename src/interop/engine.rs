//! Interop Engine implementation
//!
//! Core engine that manages MCPMate service lifecycle for cross-language integration

use anyhow::Result;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

use super::types::{PortConfig, ServiceInfo, ServiceStatus, StartupConfig, StartupProgress};
use crate::core::proxy::init::{setup_database, setup_logging, setup_proxy_server_with_params};
use crate::core::proxy::startup::{start_api_server, start_background_connections, start_proxy_server};
use crate::core::proxy::{Args, ProxyServer};

/// MCPMate Interop Engine
///
/// This is the main interface for Swift to interact with MCPMate backend.
/// It provides minimal lifecycle management functionality.
pub struct MCPMateEngine {
    /// Tokio runtime for async operations
    runtime: Option<tokio::runtime::Runtime>,
    /// Proxy server handle
    proxy_handle: Option<Arc<ProxyServer>>,
    /// API server task handle
    api_handle: Option<tokio::task::JoinHandle<()>>,
    /// Current startup progress
    startup_progress: Arc<Mutex<StartupProgress>>,
    /// Service status
    status: Arc<Mutex<ServiceStatus>>,
    /// Whether service is running
    is_running: Arc<AtomicBool>,
    /// Service start time
    start_time: Arc<AtomicU64>,
    /// Configuration
    api_port: u16,
    mcp_port: u16,
}

impl Default for MCPMateEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl MCPMateEngine {
    /// Create a new MCPMate engine instance
    pub fn new() -> Self {
        use crate::common::constants::ports;
        Self {
            runtime: Some(tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime")),
            proxy_handle: None,
            api_handle: None,
            startup_progress: Arc::new(Mutex::new(StartupProgress::default())),
            status: Arc::new(Mutex::new(ServiceStatus::Unknown)),
            is_running: Arc::new(AtomicBool::new(false)),
            start_time: Arc::new(AtomicU64::new(0)),
            api_port: ports::API_PORT,
            mcp_port: ports::MCP_PORT,
        }
    }

    /// Start the MCPMate service
    ///
    /// Note: This function internally converts to StartupConfig for unified processing.
    /// For more advanced configuration options, use start_with_startup_config().
    pub fn start(
        &mut self,
        api_port: u16,
        mcp_port: u16,
    ) -> bool {
        let config = StartupConfig::new(api_port, mcp_port, None, false);
        self.start_with_startup_config(config)
    }

    /// Start the MCPMate service with port configuration
    ///
    /// Note: This function internally converts PortConfig to StartupConfig for unified processing.
    /// For more advanced configuration options, use start_with_startup_config().
    pub fn start_with_config(
        &mut self,
        config: PortConfig,
    ) -> bool {
        // Validate port configuration
        if let Err(e) = config.validate() {
            tracing::error!("Invalid port configuration: {}", e);
            return false;
        }

        // Convert PortConfig to StartupConfig for unified processing
        let startup_config = StartupConfig::new(config.api_port, config.mcp_port, None, false);
        self.start_with_startup_config(startup_config)
    }

    /// Start with full startup configuration
    pub fn start_with_startup_config(
        &mut self,
        config: StartupConfig,
    ) -> bool {
        // Validate configuration
        if let Err(e) = config.validate() {
            tracing::error!("Invalid startup configuration: {}", e);
            return false;
        }

        // Update configuration
        self.api_port = config.api_port();
        self.mcp_port = config.mcp_port();

        // Record start time
        let start_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.start_time.store(start_time, Ordering::Relaxed);

        // Get runtime reference
        let runtime = match &self.runtime {
            Some(rt) => rt,
            None => return false,
        };

        // Clone necessary data for async task
        let startup_progress = Arc::clone(&self.startup_progress);
        let status = Arc::clone(&self.status);
        let is_running = Arc::clone(&self.is_running);

        // Start the service in background
        let _handle = runtime.spawn(async move {
            if let Err(e) = Self::start_service_async_with_config(config, startup_progress, status, is_running).await {
                tracing::error!("Failed to start MCPMate service: {}", e);
            }
        });

        true
    }

    /// Start in minimal mode (API only)
    pub fn start_minimal(
        &mut self,
        api_port: u16,
    ) -> bool {
        let config = StartupConfig::minimal(api_port);
        self.start_with_startup_config(config)
    }

    /// Start with specific profile
    pub fn start_with_profile(
        &mut self,
        api_port: u16,
        mcp_port: u16,
        profile: Vec<String>,
    ) -> bool {
        let config = StartupConfig::with_profile(api_port, mcp_port, profile);
        self.start_with_startup_config(config)
    }

    /// Start with no profile
    pub fn start_no_profile(
        &mut self,
        api_port: u16,
        mcp_port: u16,
    ) -> bool {
        let config = StartupConfig::no_profile(api_port, mcp_port);
        self.start_with_startup_config(config)
    }

    /// Start with default configuration (load default profile)
    pub fn start_default(
        &mut self,
        api_port: u16,
        mcp_port: u16,
    ) -> bool {
        let config = StartupConfig::new(api_port, mcp_port, None, false);
        self.start_with_startup_config(config)
    }

    /// Async service startup implementation with startup configuration
    async fn start_service_async_with_config(
        config: StartupConfig,
        startup_progress: Arc<Mutex<StartupProgress>>,
        status: Arc<Mutex<ServiceStatus>>,
        is_running: Arc<AtomicBool>,
    ) -> Result<()> {
        // Update status to starting
        {
            let mut status_guard = status.lock().await;
            *status_guard = ServiceStatus::Starting;
        }

        // Step 1: Setup logging (10%)
        Self::update_progress(&startup_progress, 0.1, "Setting up logging...").await;

        // Create Args with configuration
        let args = Args {
            mcp_port: config.mcp_port(),
            api_port: config.api_port(),
            log_level: "info".to_string(),
            transport: "uni".to_string(),
            profile: config.profile.clone(),
            minimal: config.minimal,
        };

        tracing::info!("Interop Engine starting with startup config: {:?}", config);

        setup_logging(&args)?;

        // Log configuration information
        tracing::info!(
            "Interop startup with configuration - API port: {}, MCP port: {}, minimal: {}, profile: {:?}",
            config.api_port(),
            config.mcp_port(),
            config.minimal,
            config.profile
        );

        // Step 2: Setup database (30%)
        Self::update_progress(&startup_progress, 0.3, "Initializing database...").await;
        let db = setup_database().await?;

        // Step 3: Setup proxy server with startup mode (50%)
        Self::update_progress(&startup_progress, 0.5, "Setting up proxy server...").await;
        let startup_mode = config.to_startup_mode();
        let (mut proxy, proxy_arc) = setup_proxy_server_with_params(db, &startup_mode).await?;

        // Step 4: Debug environment and command availability (only if not minimal)
        if !config.minimal {
            Self::update_progress(&startup_progress, 0.65, "Debugging environment...").await;
            Self::debug_environment().await;

            // Step 5: Start background connections (70%)
            Self::update_progress(&startup_progress, 0.7, "Starting background connections...").await;
            start_background_connections(&proxy, proxy_arc.clone()).await?;

            // Step 6: Start proxy server (85%)
            Self::update_progress(&startup_progress, 0.85, "Starting proxy server...").await;
            tracing::info!(
                "Starting MCP proxy server on port {} (from Interop config)",
                args.mcp_port
            );
            start_proxy_server(&mut proxy, &args).await?;
        } else {
            tracing::info!("Minimal mode: skipping MCP server and background connections");
            Self::update_progress(&startup_progress, 0.85, "Skipping MCP server (minimal mode)...").await;
        }

        // Step 7: Start API server (100%)
        Self::update_progress(&startup_progress, 1.0, "Starting API server...").await;
        tracing::info!("Starting API server on port {} (from Interop config)", args.api_port);
        let _api_task = start_api_server(proxy_arc.clone(), &args).await?;

        // Mark as running
        {
            let mut status_guard = status.lock().await;
            *status_guard = ServiceStatus::Running;
        }
        is_running.store(true, Ordering::Relaxed);

        // Final progress update
        {
            let mut progress = startup_progress.lock().await;
            progress.percentage = 1.0;
            progress.current_step = if config.minimal {
                "Service ready (minimal mode)".to_string()
            } else {
                "Service ready".to_string()
            };
            progress.is_complete = true;
        }

        tracing::info!(
            "MCPMate service started successfully via Interop with config: {:?}",
            config
        );

        // Keep the service running
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("Received shutdown signal");
            }
        }

        Ok(())
    }

    /// Update startup progress
    async fn update_progress(
        startup_progress: &Arc<Mutex<StartupProgress>>,
        percentage: f32,
        message: &str,
    ) {
        let mut progress = startup_progress.lock().await;
        progress.percentage = percentage;
        progress.current_step = message.to_string();

        tracing::info!("Startup progress: {:.1}% - {}", percentage * 100.0, message);
    }

    /// Debug environment and command availability
    async fn debug_environment() {
        tracing::info!("=== Interop Environment Debug ===");

        // Check current working directory
        if let Ok(cwd) = std::env::current_dir() {
            tracing::info!("Current working directory: {}", cwd.display());
        }

        // Check HOME directory
        if let Ok(home) = std::env::var("HOME") {
            tracing::info!("HOME directory: {}", home);
        }

        // Check database path
        use crate::common::paths::global_paths;
        let db_path = global_paths().database_path();
        tracing::info!("Expected database path: {}", db_path.display());
        tracing::info!("Database exists: {}", db_path.exists());

        // Check .mcpmate directory
        let mcpmate_dir = dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".mcpmate");
        tracing::info!("MCPMate directory: {}", mcpmate_dir.display());
        tracing::info!("MCPMate directory exists: {}", mcpmate_dir.exists());

        // List .mcpmate directory contents if it exists
        if mcpmate_dir.exists() {
            match std::fs::read_dir(&mcpmate_dir) {
                Ok(entries) => {
                    tracing::info!("MCPMate directory contents:");
                    for entry in entries.flatten() {
                        tracing::info!("  - {}", entry.file_name().to_string_lossy());
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to read MCPMate directory: {}", e);
                }
            }
        }

        // Check PATH environment variable
        use crate::common::env::constants;
        if let Ok(path) = std::env::var(constants::PATH) {
            tracing::info!("PATH: {}", path);

            // Check if homebrew paths are in PATH
            let homebrew_paths = [
                "/opt/homebrew/bin",
                "/opt/homebrew/sbin",
                "/usr/local/bin",
                "/usr/local/sbin",
            ];
            for homebrew_path in homebrew_paths {
                if path.contains(homebrew_path) {
                    tracing::info!("✅ PATH contains: {}", homebrew_path);
                } else {
                    tracing::warn!("❌ PATH missing: {}", homebrew_path);
                }
            }
        } else {
            tracing::error!("PATH environment variable not found!");
        }

        // Check sandbox-related environment variables
        for env_var in ["APP_SANDBOX_CONTAINER_ID", "TMPDIR", "NSUnbufferedIO"] {
            if let Ok(value) = std::env::var(env_var) {
                tracing::info!("Sandbox env {}: {}", env_var, value);
            }
        }

        // Test command availability
        let commands = ["uvx", "npx", "which"];
        for cmd in commands {
            match std::process::Command::new(cmd).arg("--version").output() {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    tracing::info!(
                        "Command '{}' test: success={}, stdout={}, stderr={}",
                        cmd,
                        output.status.success(),
                        stdout.trim(),
                        stderr.trim()
                    );
                }
                Err(e) => {
                    tracing::warn!("Command '{}' test failed: {}", cmd, e);
                }
            }
        }

        // Test which command for uvx and npx
        for cmd in ["uvx", "npx"] {
            match std::process::Command::new("which").arg(cmd).output() {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    if output.status.success() && !stdout.trim().is_empty() {
                        tracing::info!("✅ which {}: {}", cmd, stdout.trim());
                    } else {
                        tracing::warn!(
                            "❌ which {} failed: stdout='{}', stderr='{}', status={}",
                            cmd,
                            stdout.trim(),
                            stderr.trim(),
                            output.status
                        );
                    }
                }
                Err(e) => {
                    tracing::error!("❌ which {} command failed: {}", cmd, e);
                }
            }

            // Also test direct access to expected paths
            let expected_paths = [format!("/opt/homebrew/bin/{}", cmd), format!("/usr/local/bin/{}", cmd)];

            for expected_path in expected_paths {
                if std::path::Path::new(&expected_path).exists() {
                    tracing::info!("✅ Direct path exists: {}", expected_path);

                    // Test if it's executable
                    match std::process::Command::new(&expected_path).arg("--version").output() {
                        Ok(output) => {
                            let stdout = String::from_utf8_lossy(&output.stdout);
                            if output.status.success() {
                                tracing::info!(
                                    "✅ {} is executable: {}",
                                    expected_path,
                                    stdout.lines().next().unwrap_or("").trim()
                                );
                            } else {
                                tracing::warn!("❌ {} not executable: status={}", expected_path, output.status);
                            }
                        }
                        Err(e) => {
                            tracing::warn!("❌ {} execution test failed: {}", expected_path, e);
                        }
                    }
                } else {
                    tracing::info!("❌ Direct path not found: {}", expected_path);
                }
            }
        }

        // Test subprocess creation with different approaches
        tracing::info!("=== Subprocess Creation Tests ===");

        // Test 1: Simple echo command
        match std::process::Command::new("echo").arg("test").output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                tracing::info!(
                    "Echo test: success={}, output={}",
                    output.status.success(),
                    stdout.trim()
                );
            }
            Err(e) => {
                tracing::error!("Echo test failed: {}", e);
            }
        }

        // Test 2: Shell command
        match std::process::Command::new("sh")
            .arg("-c")
            .arg("echo 'shell test'")
            .output()
        {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                tracing::info!(
                    "Shell test: success={}, output={}",
                    output.status.success(),
                    stdout.trim()
                );
            }
            Err(e) => {
                tracing::error!("Shell test failed: {}", e);
            }
        }

        // Test 3: Direct npx command with simple args
        match std::process::Command::new("npx").arg("--version").output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                tracing::info!(
                    "NPX direct test: success={}, stdout={}, stderr={}",
                    output.status.success(),
                    stdout.trim(),
                    stderr.trim()
                );
            }
            Err(e) => {
                tracing::error!("NPX direct test failed: {}", e);
            }
        }

        // Test 4: Using absolute path
        match std::process::Command::new("/opt/homebrew/bin/npx")
            .arg("--version")
            .output()
        {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                tracing::info!(
                    "NPX absolute path test: success={}, stdout={}, stderr={}",
                    output.status.success(),
                    stdout.trim(),
                    stderr.trim()
                );
            }
            Err(e) => {
                tracing::error!("NPX absolute path test failed: {}", e);
            }
        }

        // Test 5: Spawn vs output
        tracing::info!("Testing spawn vs output...");
        match std::process::Command::new("echo").arg("spawn test").spawn() {
            Ok(mut child) => match child.wait() {
                Ok(status) => {
                    tracing::info!("Spawn test: success={}", status.success());
                }
                Err(e) => {
                    tracing::error!("Spawn wait failed: {}", e);
                }
            },
            Err(e) => {
                tracing::error!("Spawn test failed: {}", e);
            }
        }

        tracing::info!("=== End Subprocess Creation Tests ===");
        tracing::info!("=== End Interop Environment Debug ===");
    }

    /// Get current startup progress
    pub fn get_startup_progress(&self) -> StartupProgress {
        // Use blocking lock since this is called from Swift
        self.runtime
            .as_ref()
            .unwrap()
            .block_on(async { self.startup_progress.lock().await.clone() })
    }

    /// Check if service is running
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    /// Get service information
    pub fn get_service_info(&self) -> ServiceInfo {
        let start_time = self.start_time.load(Ordering::Relaxed);
        let uptime = if start_time > 0 {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                .saturating_sub(start_time)
        } else {
            0
        };

        ServiceInfo {
            version: env!("CARGO_PKG_VERSION").to_string(),
            api_port: self.api_port,
            mcp_port: self.mcp_port,
            uptime_seconds: uptime,
            is_running: self.is_running(),
            active_connections: 0, // TODO: Get from connection pool
        }
    }

    /// Get startup progress as JSON string
    pub fn get_startup_progress_json(&self) -> String {
        let progress = self.get_startup_progress();
        serde_json::to_string(&progress).unwrap_or_else(|_| "{}".to_string())
    }

    /// Get service info as JSON string
    pub fn get_service_info_json(&self) -> String {
        let info = self.get_service_info();
        serde_json::to_string(&info).unwrap_or_else(|_| "{}".to_string())
    }

    /// Stop the service
    pub fn stop(&mut self) {
        tracing::info!("🛑 Stopping MCPMate service via Interop");

        // Update status
        if let Some(runtime) = &self.runtime {
            runtime.block_on(async {
                let mut status_guard = self.status.lock().await;
                *status_guard = ServiceStatus::Stopping;
                tracing::info!("🔄 Service status updated to Stopping");
            });
        }

        // Abort API task first
        if let Some(api_handle) = self.api_handle.take() {
            tracing::info!("🛑 Aborting API server task");
            api_handle.abort();
        }

        // Disconnect from all servers and stop proxy
        if let Some(proxy) = &self.proxy_handle {
            if let Some(runtime) = &self.runtime {
                runtime.block_on(async {
                    tracing::info!("🛑 Disconnecting from all MCP servers");
                    let mut pool = proxy.connection_pool.lock().await;
                    let _ = pool.disconnect_all().await;
                    drop(pool); // Explicitly drop the pool
                    tracing::info!("✅ All MCP server connections closed");
                });
            }
        }

        // Clear proxy handle to ensure it's dropped
        self.proxy_handle = None;
        tracing::info!("🛑 Proxy handle cleared");

        // Mark as stopped
        self.is_running.store(false, Ordering::Relaxed);

        if let Some(runtime) = &self.runtime {
            runtime.block_on(async {
                let mut status_guard = self.status.lock().await;
                *status_guard = ServiceStatus::Stopped;
            });
        }

        tracing::info!("MCPMate service stopped");
    }
}

impl Drop for MCPMateEngine {
    fn drop(&mut self) {
        if self.is_running() {
            self.stop();
        }
    }
}
