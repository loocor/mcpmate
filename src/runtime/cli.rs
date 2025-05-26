//! Command-line interface handlers for runtime management
//!
//! This module contains the business logic for handling runtime CLI commands,
//! extracted from the main runtime binary for better organization and testability.

use anyhow::Result;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use super::{
    DownloadConfig, DownloadProgress, InlineProgressBar, MultiLineProgress, RuntimeType,
    download_runtime_with_config,
    integration::{
        save_runtime_config_to_db, send_download_progress_events, send_runtime_ready_event,
        send_runtime_setup_failed_event,
    },
    supports_inline_progress, supports_interactive,
    types::ExecutionContext,
};

/// Progress callback type alias
type ProgressCallback = Box<dyn Fn(DownloadProgress) + Send + Sync>;

/// Handle the install command with all its complexity
pub async fn handle_install_command(
    runtime_type: RuntimeType,
    version: Option<String>,
    timeout: u64,
    max_retries: u32,
    verbose: bool,
    interactive: bool,
    quiet: bool,
    database: Option<String>,
    context: ExecutionContext,
) -> Result<()> {
    // Send start event in quiet mode
    if quiet {
        send_download_progress_events(
            runtime_type,
            &crate::runtime::DownloadStage::Initializing,
            version.as_deref(),
        );
    } else {
        println!("Installing {} runtime...", runtime_type);
    }

    // Check if interactive mode is requested but not supported
    if interactive && !supports_interactive() && !quiet {
        println!("⚠️  Interactive mode requested but not supported in this environment");
        println!("💡 Running in non-interactive mode");
    }

    // Create progress bar based on terminal support (not in quiet mode)
    let progress_bar = if !quiet && supports_inline_progress() {
        Some(Arc::new(Mutex::new(InlineProgressBar::new())))
    } else {
        None
    };

    // Setup download configuration with progress callback
    let config = create_download_config(
        timeout,
        max_retries,
        verbose && !quiet,
        interactive && supports_interactive() && !quiet,
        progress_bar.clone(),
        runtime_type,
        version.clone(),
        quiet,
        context,
    );

    // Perform the download
    match download_runtime_with_config(runtime_type, version.as_deref(), config).await {
        Ok(path) => {
            handle_install_success(
                runtime_type,
                version,
                path,
                quiet,
                database,
                progress_bar,
                context,
            )
            .await?;
        }
        Err(e) => {
            handle_install_failure(runtime_type, e, quiet, progress_bar, context).await?;
        }
    }

    Ok(())
}

/// Create download configuration with appropriate progress callback
fn create_download_config(
    timeout: u64,
    max_retries: u32,
    verbose: bool,
    interactive: bool,
    progress_bar: Option<Arc<Mutex<InlineProgressBar>>>,
    runtime_type: RuntimeType,
    version: Option<String>,
    quiet: bool,
    context: ExecutionContext,
) -> DownloadConfig {
    let progress_callback: Option<ProgressCallback> = Some(Box::new({
        let progress_bar = progress_bar.clone();
        let version = version.clone();
        move |progress: DownloadProgress| {
            match context {
                ExecutionContext::Api => {
                    // API mode: always send events
                    send_download_progress_events(
                        runtime_type,
                        &progress.stage,
                        version.as_deref(),
                    );
                }
                ExecutionContext::Cli => {
                    if quiet {
                        // CLI quiet mode: send events
                        send_download_progress_events(
                            runtime_type,
                            &progress.stage,
                            version.as_deref(),
                        );
                    } else if let Some(bar) = &progress_bar {
                        if let Ok(mut bar) = bar.lock() {
                            bar.update(&progress);
                        }
                    } else {
                        // Fallback to multi-line progress
                        MultiLineProgress::update(&progress);
                    }
                }
            }
        }
    }));

    DownloadConfig {
        timeout: Some(timeout),
        max_retries,
        verbose,
        interactive,
        progress_callback,
    }
}

/// Handle successful installation
async fn handle_install_success(
    runtime_type: RuntimeType,
    version: Option<String>,
    path: PathBuf,
    quiet: bool,
    database: Option<String>,
    progress_bar: Option<Arc<Mutex<InlineProgressBar>>>,
    context: ExecutionContext,
) -> Result<()> {
    if let Some(bar) = progress_bar {
        if let Ok(bar) = bar.lock() {
            bar.clear();
        }
    }

    let version_str = version.unwrap_or_else(|| runtime_type.default_version().to_string());

    match context {
        ExecutionContext::Api => {
            // API mode: save to database if provided, return error on failure
            if let Some(db_path) = database {
                if let Err(e) =
                    save_runtime_config_to_db(&db_path, runtime_type, &version_str, &path).await
                {
                    send_runtime_setup_failed_event(
                        runtime_type,
                        &format!("Failed to save config to database: {}", e),
                    );
                    return Err(e);
                }
            }
            // Send success event
            send_runtime_ready_event(runtime_type, &version_str, &path);
        }
        ExecutionContext::Cli => {
            if quiet {
                // Save to database if database path provided
                if let Some(db_path) = database {
                    if let Err(e) =
                        save_runtime_config_to_db(&db_path, runtime_type, &version_str, &path).await
                    {
                        send_runtime_setup_failed_event(
                            runtime_type,
                            &format!("Failed to save config to database: {}", e),
                        );
                        std::process::exit(1);
                    }
                }
                // Send success event
                send_runtime_ready_event(runtime_type, &version_str, &path);
            } else {
                println!("Runtime installed successfully at: {}", path.display());
            }
        }
    }

    Ok(())
}

/// Handle installation failure
async fn handle_install_failure(
    runtime_type: RuntimeType,
    e: anyhow::Error,
    quiet: bool,
    progress_bar: Option<Arc<Mutex<InlineProgressBar>>>,
    context: ExecutionContext,
) -> Result<()> {
    if let Some(bar) = progress_bar {
        if let Ok(bar) = bar.lock() {
            bar.clear();
        }
    }

    match context {
        ExecutionContext::Api => {
            // API mode: send event and return error
            send_runtime_setup_failed_event(runtime_type, &e.to_string());
            Err(e)
        }
        ExecutionContext::Cli => {
            // CLI mode: handle as before with exit
            if quiet {
                send_runtime_setup_failed_event(runtime_type, &e.to_string());
            } else {
                eprintln!("Failed to install runtime: {}", e);

                // Provide helpful suggestions based on error type
                let error_str = e.to_string();
                if error_str.contains("timeout") {
                    eprintln!();
                    eprintln!("💡 Timeout suggestions:");
                    eprintln!("   • Use --timeout <seconds> to increase timeout duration");
                    eprintln!("   • Use --interactive flag for timeout handling options");
                    eprintln!("   • Check your network connection");
                } else if error_str.contains("network") || error_str.contains("DNS") {
                    eprintln!();
                    eprintln!("💡 Network suggestions:");
                    eprintln!("   • Check your internet connection");
                    eprintln!("   • Try using a different DNS server (8.8.8.8, 1.1.1.1)");
                    eprintln!("   • Check if you're behind a firewall or proxy");
                }
            }
            std::process::exit(1);
        }
    }
}
