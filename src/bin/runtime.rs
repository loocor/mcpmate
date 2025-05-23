//! MCPMate Runtime Manager - Enhanced Edition
//!
//! This is an enhanced standalone executable for managing MCPMate's runtime environment.
//! It provides advanced features like:
//! - Progress tracking with visual progress bars
//! - Configurable timeouts and retry mechanisms
//! - Verbose logging and detailed error reporting
//!
//! For basic usage, all original commands (install, list, check, path) are supported
//! with enhanced capabilities.

use anyhow::Result;
use clap::Parser;
use mcpmate::runtime::{
    Commands, DownloadConfig, InlineProgressBar, MultiLineProgress, RuntimeManager, RuntimeType,
    download_runtime_with_config, list_runtime, show_runtime_path, supports_inline_progress,
};
use std::sync::{Arc, Mutex};

#[derive(Parser)]
#[command(name = "runtime")]
#[command(about = "MCPMate Runtime Manager with enhanced download features")]
#[command(version)]
#[command(long_about = "
MCPMate Runtime Manager provides comprehensive management of runtime environments
including Node.js, uv (Python), and Bun.js. This enhanced version includes:

• Progress tracking with visual indicators
• Configurable download timeouts and retries
• Verbose logging for troubleshooting

Examples:
  runtime install node --verbose                    # Install Node.js with verbose output
  runtime install uv --timeout 600 --max-retries 5 # Install uv with custom settings
  runtime list                                      # List all installed runtimes
  runtime check node                                # Check if Node.js is installed
  runtime path node                                 # Show path to Node.js installation
")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Install {
            runtime_type,
            version,
            timeout,
            max_retries,
            verbose,
        } => {
            install_runtime_enhanced(
                runtime_type,
                version.as_deref(),
                timeout,
                max_retries,
                verbose,
            )
            .await
        }
        Commands::List => list_all_runtimes().await,
        Commands::Check {
            runtime_type,
            version,
        } => check_runtime(runtime_type, version.as_deref()).await,
        Commands::Path {
            runtime_type,
            version,
        } => show_runtime_path(runtime_type, version.as_deref()),
    }
}

/// Enhanced install with progress tracking and configurable options
async fn install_runtime_enhanced(
    runtime_type: RuntimeType,
    version: Option<&str>,
    timeout: u64,
    max_retries: u32,
    verbose: bool,
) -> Result<()> {
    println!("Installing {} runtime...", runtime_type);

    // check if inline progress is supported
    let use_inline_progress = supports_inline_progress() && !verbose;
    let progress_bar = if use_inline_progress {
        Some(Arc::new(Mutex::new(InlineProgressBar::new())))
    } else {
        None
    };

    let progress_bar_clone = progress_bar.clone();
    let config = DownloadConfig {
        timeout: Some(timeout),
        max_retries,
        progress_callback: Some(Box::new(move |progress| {
            if let Some(bar) = &progress_bar_clone {
                if let Ok(mut bar) = bar.lock() {
                    bar.update(&progress);
                }
            } else {
                // fallback to multi-line mode
                MultiLineProgress::update(&progress);
            }
        })),
        verbose,
    };

    let result = download_runtime_with_config(runtime_type, version, config).await?;

    // finish progress bar display
    if let Some(bar) = progress_bar {
        if let Ok(bar) = bar.lock() {
            bar.finish(&format!(
                "Runtime installed successfully at: {}",
                result.display()
            ));
        }
    } else {
        println!("Runtime installed successfully at: {}", result.display());
    }

    Ok(())
}

/// List all installed runtime environments
async fn list_all_runtimes() -> Result<()> {
    println!("Installed runtime environments:");

    let runtime_manager = RuntimeManager::new()?;

    // list Node.js environments
    println!("\nNode.js:");
    list_runtime(&runtime_manager, RuntimeType::Node)?;

    // list uv/Python environments
    println!("\nuv/Python:");
    list_runtime(&runtime_manager, RuntimeType::Uv)?;

    // list Bun.js environments
    println!("\nBun.js:");
    list_runtime(&runtime_manager, RuntimeType::Bun)?;

    Ok(())
}

/// Check if a specific runtime is available
async fn check_runtime(
    runtime_type: RuntimeType,
    version: Option<&str>,
) -> Result<()> {
    let runtime_manager = RuntimeManager::new()?;

    let available = runtime_manager.is_runtime_available(runtime_type, version)?;
    if available {
        let path = runtime_manager.get_runtime_path(runtime_type, version)?;
        println!(
            "✓ {} {} installed: {}",
            runtime_type.as_str(),
            version.unwrap_or(runtime_type.default_version()),
            path.display()
        );
    } else {
        println!(
            "✗ {} {} not installed",
            runtime_type.as_str(),
            version.unwrap_or(runtime_type.default_version())
        );
    }

    Ok(())
}
