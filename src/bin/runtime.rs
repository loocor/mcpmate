//! MCPMate Runtime Manager - Enhanced Edition
//!
//! This is an enhanced standalone executable for managing MCPMate's runtime environment.
//! It provides advanced features like:
//! - Progress tracking with visual progress bars
//! - Configurable timeouts and retry mechanisms
//! - Verbose logging and detailed error reporting
//! - Quiet mode with event publishing for integration with main program
//!
//! For basic usage, all original commands (install, list, check, path) are supported
//! with enhanced capabilities.

use anyhow::Result;
use clap::Parser;
use mcpmate::runtime::{
    Commands, RuntimeManager, RuntimeType, cli::handle_install_command, list_runtime,
    show_runtime_path,
};

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
• Quiet mode with event publishing for main program integration

Examples:
  runtime install node --verbose                    # Install Node.js with verbose output
  runtime install uv --timeout 600 --interactive    # Install uv with extended timeout and interactive mode
  runtime install bun --max-retries 5               # Install Bun with more retry attempts
  runtime install node --quiet --database /path/to/db.sqlite3  # Quiet mode with database integration
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
    let cli = Cli::parse();

    // Initialize tracing only if not in quiet mode
    let is_quiet = matches!(cli.command, Commands::Install { quiet: true, .. });

    if !is_quiet {
        tracing_subscriber::fmt::init();
    }

    match cli.command {
        Commands::Install {
            runtime_type,
            version,
            timeout,
            max_retries,
            verbose,
            interactive,
            quiet,
            database,
        } => {
            handle_install_command(
                runtime_type,
                version,
                timeout,
                max_retries,
                verbose,
                interactive,
                quiet,
                database,
            )
            .await?;
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
