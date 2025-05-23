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
    supports_interactive,
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
• Interactive timeout handling with network diagnostics
• Intelligent network connectivity analysis

Examples:
  runtime install node --verbose                    # Install Node.js with verbose output
  runtime install uv --timeout 600 --interactive    # Install uv with extended timeout and interactive mode
  runtime install bun --max-retries 5               # Install Bun with more retry attempts
  runtime list                                       # List all installed runtimes
  runtime check node                                 # Check Node.js installation
  runtime path uv                                    # Get uv installation path
")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Install {
            runtime_type,
            version,
            timeout,
            max_retries,
            verbose,
            interactive,
        } => {
            println!("Installing {} runtime...", runtime_type);

            // Check if interactive mode is requested but not supported
            if interactive && !supports_interactive() {
                println!("⚠️  Interactive mode requested but not supported in this environment");
                println!("💡 Running in non-interactive mode");
            }

            // Create progress bar based on terminal support
            let progress_bar = if supports_inline_progress() {
                Some(Arc::new(Mutex::new(InlineProgressBar::new())))
            } else {
                None
            };

            let progress_bar_clone = progress_bar.clone();
            let config = DownloadConfig {
                timeout: Some(timeout),
                max_retries,
                verbose,
                interactive: interactive && supports_interactive(),
                progress_callback: Some(Box::new(move |progress| {
                    if let Some(bar) = &progress_bar_clone {
                        if let Ok(mut bar) = bar.lock() {
                            bar.update(&progress);
                        }
                    } else {
                        // Fallback to multi-line progress
                        MultiLineProgress::update(&progress);
                    }
                })),
            };

            match download_runtime_with_config(runtime_type, version.as_deref(), config).await {
                Ok(path) => {
                    if let Some(bar) = progress_bar {
                        if let Ok(bar) = bar.lock() {
                            bar.clear();
                        }
                    }
                    println!("Runtime installed successfully at: {}", path.display());
                }
                Err(e) => {
                    if let Some(bar) = progress_bar {
                        if let Ok(bar) = bar.lock() {
                            bar.clear();
                        }
                    }
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

                    std::process::exit(1);
                }
            }
        }
        Commands::List => {
            let manager = RuntimeManager::new()?;
            println!("Installed runtime environments:");

            // List Node.js environments
            println!("\nNode.js:");
            list_runtime(&manager, RuntimeType::Node)?;

            // List uv/Python environments
            println!("\nuv/Python:");
            list_runtime(&manager, RuntimeType::Uv)?;

            // List Bun.js environments
            println!("\nBun.js:");
            list_runtime(&manager, RuntimeType::Bun)?;
        }
        Commands::Check {
            runtime_type,
            version,
        } => {
            let manager = RuntimeManager::new()?;
            match manager.is_runtime_available(runtime_type, version.as_deref()) {
                Ok(true) => {
                    println!("✓ {} is available", runtime_type);
                    if let Ok(path) = manager.get_runtime_path(runtime_type, version.as_deref()) {
                        println!("  Path: {}", path.display());
                    }
                }
                Ok(false) => {
                    println!("✗ {} is not installed", runtime_type);
                    println!("  Run 'runtime install {}' to install it", runtime_type);
                }
                Err(e) => {
                    eprintln!("Error checking runtime: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Path {
            runtime_type,
            version,
        } => match show_runtime_path(runtime_type, version.as_deref()) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("Error getting runtime path: {}", e);
                std::process::exit(1);
            }
        },
    }

    Ok(())
}
