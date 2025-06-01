//! MCPMate Runtime Manager - CLI Edition
//!
//! This is a standalone executable for managing MCPMate's runtime environment via CLI.
//! It focuses on two core commands: `install` and `list`.
//!
//! - `install`: Installs specified runtimes (Node.js, uv, Bun) with progress tracking,
//!   configurable timeouts, retries, verbose logging, and quiet mode for integration.
//! - `list`: Displays the status (availability and path) of all managed runtimes.
//!
//! This CLI directly reflects the simplified API structure where detailed runtime
//! information is primarily accessed via the GET /api/runtime/list endpoint.

use anyhow::Result;
use clap::Parser;
use mcpmate::runtime::{
    RuntimeType,
    cli::handle_install_command,
    get_runtime_path,
    types::{Commands, ExecutionContext},
};

#[derive(Parser)]
#[command(name = "runtime")]
#[command(about = "MCPMate Runtime Manager - CLI Edition (install & list commands only)")]
#[command(version)]
#[command(long_about = r#"
MCPMate Runtime Manager (CLI Edition)

Manages runtime environments (Node.js, uv, Bun) with two core commands:

1. `install`: Installs runtimes with advanced features:
   - Progress tracking with visual indicators
   - Configurable download timeouts and retries
   - Verbose logging for troubleshooting
   - Interactive timeout handling with network diagnostics
   - Quiet mode with event publishing for integration

2. `list`: Displays the status of all managed runtimes:
   - Availability (installed or not)
   - Installation path (if available)

API Alignment:
  This CLI aligns with the simplified API where GET /api/runtime/list (with optional
  query parameters) provides comprehensive runtime information, replacing separate
  check/path endpoints.

Examples:
  runtime install node --verbose                    # Install Node.js with verbose output
  runtime install uv --timeout 600 --interactive    # Install uv with extended timeout and interactive mode
  runtime install bun --max-retries 5               # Install Bun with more retry attempts
  runtime install node --quiet --database /path/to/db.sqlite3  # Quiet mode with database integration
  runtime list                                       # List status of all runtimes (availability and path)
"#)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

// Helper to display status for a single runtime, used by the list command
fn display_single_runtime_status(runtime_type: RuntimeType) -> Result<()> {
    println!("{}:", runtime_type);
    match get_runtime_path(runtime_type, None) {
        Ok(path) => {
            if path.exists() {
                println!("  ✓ Available, Path: {}", path.display());
            } else {
                println!("  ✗ Not Installed (path not found)");
            }
        }
        Err(e) => {
            println!("  ✗ Not Installed: {}", e);
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

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
                ExecutionContext::Cli,
            )
            .await?;
        }
        Commands::List => {
            println!("MCPMate Runtime Status (CLI):");

            display_single_runtime_status(RuntimeType::Node)?;
            println!(); // Add a blank line for separation
            display_single_runtime_status(RuntimeType::Uv)?;
            println!(); // Add a blank line for separation
            display_single_runtime_status(RuntimeType::Bun)?;
        } // Check and Path commands are removed from the Commands enum
          // and therefore no longer matched here.
    }

    Ok(())
}
