//! MCPMate Runtime Manager
//!
//! This is a standalone executable for managing MCPMate's runtime environment.
//! It can be used to download, install, and manage various runtime environments,
//! such as Node.js, Python/uv, and Bun.js.

use anyhow::Result;
use clap::Parser;
use mcpmate::runtime::{Commands, RuntimeManager, RuntimeType, list_runtime};

#[derive(Parser)]
#[command(name = "mcpmate-runtime")]
#[command(about = "MCPMate Runtime Manager")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Parse CLI arguments
    let cli = Cli::parse();
    let runtime_manager = RuntimeManager::new()?;

    // Handle commands
    match cli.command {
        Commands::Install {
            runtime_type,
            version,
        } => {
            println!(
                "Installing {} {}...",
                runtime_type.as_str(),
                version.as_deref().unwrap_or(runtime_type.default_version())
            );

            let path = runtime_manager
                .ensure(runtime_type, version.as_deref())
                .await?;
            println!("Installation complete: {}", path.display());
        }

        Commands::List => {
            println!("Installed runtime environments:");

            // List Node.js environments
            println!("\nNode.js:");
            list_runtime(&runtime_manager, RuntimeType::Node)?;

            // List uv/Python environments
            println!("\nuv/Python:");
            list_runtime(&runtime_manager, RuntimeType::Uv)?;

            // List Bun.js environments
            println!("\nBun.js:");
            list_runtime(&runtime_manager, RuntimeType::Bun)?;
        }

        Commands::Check {
            runtime_type,
            version,
        } => {
            let available =
                runtime_manager.is_runtime_available(runtime_type, version.as_deref())?;
            if available {
                let path = runtime_manager.get_runtime_path(runtime_type, version.as_deref())?;
                println!(
                    "✓ {} {} installed: {}",
                    runtime_type.as_str(),
                    version.as_deref().unwrap_or(runtime_type.default_version()),
                    path.display()
                );
            } else {
                println!(
                    "✗ {} {} not installed",
                    runtime_type.as_str(),
                    version.as_deref().unwrap_or(runtime_type.default_version())
                );
            }
        }

        Commands::Path {
            runtime_type,
            version,
        } => {
            let path = runtime_manager.get_runtime_path(runtime_type, version.as_deref())?;
            println!("{}", path.display());
        }
    }

    Ok(())
}
