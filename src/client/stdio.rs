use anyhow::{Context, Result};
use rmcp::{service::ServiceExt, transport::TokioChildProcess};
use tokio::process::Command;

use super::utils::prepare_command_env;
use crate::config::ServerConfig;

/// handle stdio server for tool listing
pub async fn handle_stdio_server(server: &str, server_config: &ServerConfig) -> Result<()> {
    println!("\nConnecting to server...\n");

    // build the command
    let command_str = server_config
        .command
        .as_ref()
        .with_context(|| format!("Command not specified for server '{}'", server))?;
    let mut command = Command::new(command_str);

    // add args if present
    if let Some(args) = &server_config.args {
        command.args(args);
    }

    // add env vars if present
    if let Some(env_map) = &server_config.env {
        for (key, value) in env_map {
            command.env(key, value);
        }
    }

    // prepare command env
    prepare_command_env(&mut command, command_str);

    // connect to the server
    let service = ().serve(TokioChildProcess::new(&mut command)?).await?;

    // list tools
    println!("ready to call list_all_tools ...");
    let tools_result = match tokio::time::timeout(
        tokio::time::Duration::from_secs(10),
        service.list_all_tools(),
    )
    .await
    {
        Ok(Ok(tools)) => {
            println!("list_all_tools call success, get {} tools", tools.len());
            tools
        }
        Ok(Err(e)) => {
            println!("list_all_tools call failed: {e}");
            service.cancel().await?;
            return Ok(());
        }
        Err(_) => {
            println!("list_all_tools call timeout!");
            service.cancel().await?;
            return Ok(());
        }
    };

    println!("\nAvailable tools:");
    if !tools_result.is_empty() {
        for (i, tool) in tools_result.iter().enumerate() {
            println!("{:02} - {}: {}", i + 1, tool.name, tool.description);
            println!("     Parameters:");
            println!("{}", super::utils::schema_formater(&tool.input_schema));
            println!();
        }
    } else {
        println!("  No tools available");
    }

    // Close the connection
    service.cancel().await?;

    Ok(())
}
