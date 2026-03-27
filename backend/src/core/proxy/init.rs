//! Initialization logic for core proxy server
//!
//! This module handles the setup and initialization of the proxy server using core modules.

use super::{Args, ProxyServer, args::StartupMode};
use crate::{
    api::handlers::system,
    audit::{AuditRetentionPolicySetting, AuditService, AuditStore, run_retention_worker},
    config::audit_database::AuditDatabase,
    config::database::Database,
    core::{capability::naming, foundation::loader},
};
use anyhow::Result;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::{self, EnvFilter};

/// Setup logging based on command line arguments
/// This function is safe to call multiple times - it will only initialize once
pub fn setup_logging(args: &Args) -> Result<()> {
    // TODO(temporary): This file logging toggle and multiplexer are temporary.
    // Once the audit logging subsystem is implemented, remove MCPMATE_LOG_TO_FILE,
    // the MultiWriter, and all file-path handling here.
    use crate::common::constants::{defaults, env_vars};
    // Create environment filter with smart defaults
    let (env_filter, log_config_msg) = if let Ok(rust_log) = std::env::var("RUST_LOG") {
        // If RUST_LOG is set, respect it completely - no overrides
        let msg = format!("Using RUST_LOG environment variable: {} (full control)", rust_log);
        (EnvFilter::from_default_env(), msg)
    } else {
        // If RUST_LOG is not set, use application defaults with noise reduction
        let default_level = args.log_level.parse().unwrap_or(tracing::Level::INFO.into());
        let msg = format!(
            "No RUST_LOG set, using application default: {} (with noise reduction)",
            args.log_level
        );

        // Create filter with noise reduction for common noisy modules
        let filter = EnvFilter::from_default_env()
            .add_directive(default_level)
            // Keep the really noisy third-party modules quiet
            .add_directive("sqlx=warn".parse().unwrap()) // SQL queries are too verbose
            .add_directive("rmcp=warn".parse().unwrap()) // MCP protocol noise
            .add_directive("hyper=warn".parse().unwrap()) // HTTP client noise
            .add_directive("reqwest=warn".parse().unwrap()) // HTTP requests
            .add_directive("tokio=warn".parse().unwrap()); // Async runtime noise

        (filter, msg)
    };

    // Setup with stdout + optional file logging
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    // Determine whether to enable file logging (env overrides default)
    let mut post_init_warning: Option<String> = None;
    let enable_file_log = match std::env::var(env_vars::MCPMATE_LOG_TO_FILE) {
        Ok(raw) => {
            let v = raw.trim().to_lowercase();
            match v.as_str() {
                "1" | "true" | "on" | "yes" => true,
                "0" | "false" | "off" | "no" => false,
                other => {
                    // Keep backward-compat by defaulting to enabled for unknown values
                    post_init_warning = Some(format!(
                        "Unrecognized {} value ('{}'); defaulting to enabled",
                        env_vars::MCPMATE_LOG_TO_FILE,
                        other
                    ));
                    true
                }
            }
        }
        Err(_) => defaults::LOG_TO_FILE_DEFAULT,
    };

    // Determine log file path (only if enabled)
    let log_file_path = if enable_file_log {
        use crate::common::paths::global_paths;
        let logs_dir = global_paths().logs_dir();
        std::fs::create_dir_all(&logs_dir).ok();
        Some(logs_dir.join("mcpmate.log"))
    } else {
        None
    };

    // Create multi-writer that writes to both stdout and file
    #[derive(Clone)]
    struct MultiWriter {
        file: Option<Arc<Mutex<std::fs::File>>>,
    }

    impl Write for MultiWriter {
        fn write(
            &mut self,
            buf: &[u8],
        ) -> std::io::Result<usize> {
            // Always write to stdout
            std::io::stdout().write_all(buf).ok();

            // Also write to file if available
            if let Some(ref file) = self.file {
                if let Ok(mut f) = file.lock() {
                    f.write_all(buf).ok();
                }
            }
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            std::io::stdout().flush().ok();
            if let Some(ref file) = self.file {
                if let Ok(mut f) = file.lock() {
                    f.flush().ok();
                }
            }
            Ok(())
        }
    }

    let file_handle = match log_file_path {
        Some(ref path) => std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .ok()
            .map(|f| Arc::new(Mutex::new(f))),
        None => None,
    };

    let multi_writer = MultiWriter { file: file_handle };

    let result = tracing_subscriber::fmt()
        .with_writer(move || multi_writer.clone())
        .with_env_filter(env_filter)
        .try_init();

    match result {
        Ok(()) => {
            tracing::info!("Logging system initialized successfully");
            tracing::info!("{}", log_config_msg);
            if let Some(ref path) = log_file_path {
                tracing::info!("Log file: {}", path.display());
            } else {
                tracing::info!(
                    "File logging disabled (set {}=true to enable)",
                    env_vars::MCPMATE_LOG_TO_FILE
                );
            }
            if let Some(msg) = post_init_warning.take() {
                tracing::warn!("{}", msg);
            }
        }
        Err(_) => {
            // Global subscriber already set, this is fine for FFI mode
            tracing::debug!("Logging system already initialized, skipping");
        }
    }

    Ok(())
}

/// Setup database connection and perform necessary migrations
pub async fn setup_database() -> Result<Database> {
    // Initialize server start time
    system::initialize_server_start_time();

    // Debug database path before initialization
    use crate::common::paths::global_paths;
    let db_path = global_paths().database_path();
    tracing::info!("FFI Database setup - Expected path: {}", db_path.display());
    tracing::info!("FFI Database setup - File exists: {}", db_path.exists());

    // Initialize database
    let db = match Database::new().await {
        Ok(db) => {
            tracing::info!("FFI Database initialized successfully");
            db
        }
        Err(e) => {
            tracing::error!("FFI Failed to initialize database: {}", e);
            return Err(anyhow::anyhow!("Failed to initialize database: {}", e));
        }
    };

    // Initialize naming store once the database is ready
    naming::initialize(db.pool.clone());

    // Runtime migration removed - simplified runtime management
    tracing::debug!("Runtime management simplified - no migration needed");

    Ok(db)
}

pub async fn setup_audit_database() -> Result<Option<AuditDatabase>> {
    match AuditDatabase::new().await {
        Ok(database) => {
            tracing::info!("Audit database initialized successfully");
            Ok(Some(database))
        }
        Err(error) => {
            tracing::warn!(error = %error, "Audit database initialization failed; continuing without audit subsystem");
            Ok(None)
        }
    }
}

/// Setup proxy server with startup parameters
pub async fn setup_proxy_server_with_params(
    db: Database,
    audit_db: Option<AuditDatabase>,
    startup_mode: &StartupMode,
) -> Result<(Arc<ProxyServer>, Arc<ProxyServer>)> {
    // Load configuration from database using core loader with startup parameters
    let config = loader::load_server_config_with_params(&db, startup_mode).await?;

    tracing::info!(
        "Loaded configuration from database with startup mode: {:?}",
        startup_mode
    );
    tracing::info!(
        "Found {} enabled MCP servers in database configuration",
        config.mcp_servers.len()
    );

    // Create proxy server using core implementation
    let mut proxy = ProxyServer::new(Arc::new(config));

    // Use the existing database connection
    proxy.set_database(db).await?;
    tracing::info!("Using database connection for tool-level configuration.");

    let audit_store = if let Some(audit_db) = audit_db {
        let audit_db = Arc::new(audit_db);
        let audit_store = Arc::new(AuditStore::from_database(audit_db.as_ref()));
        audit_store.initialize().await?;
        let audit_service = Arc::new(AuditService::new(audit_store.clone()).await?);
        proxy.set_audit_service(audit_db, audit_service);
        tracing::info!("Audit service initialized and attached to proxy server");
        Some(audit_store)
    } else {
        None
    };

    // Create Arc wrappers for the proxy server
    let proxy_arc = Arc::new(proxy.clone());
    ProxyServer::set_global(Arc::new(tokio::sync::Mutex::new(proxy)));

    if let Some(store) = audit_store {
        let cancellation_token = CancellationToken::new();
        let retention_token = cancellation_token.clone();
        tokio::spawn(async move {
            let policy = match store.get_policy().await {
                Ok(p) => p,
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to load audit retention policy, using default");
                    AuditRetentionPolicySetting::default()
                }
            };
            run_retention_worker(store, policy, retention_token).await;
        });
    }

    tracing::info!("Proxy server created, event system will be initialized with handlers");

    Ok((proxy_arc.clone(), proxy_arc))
}

/// Setup proxy server with database and configuration using core modules (legacy function for backward compatibility)
pub async fn setup_proxy_server(db: Database) -> Result<(Arc<ProxyServer>, Arc<ProxyServer>)> {
    setup_proxy_server_with_params(db, None, &StartupMode::Default).await
}
