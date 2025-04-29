use crate::config::ServerConfig;
use anyhow::{Context, Result};
use rmcp::{
    model::{ClientCapabilities, ClientInfo, Implementation},
    transport::sse::SseTransport,
    ServiceExt,
};

/// handle sse server for tool listing
pub async fn handle_sse_server(server: &str, server_config: &ServerConfig) -> Result<()> {
    println!("\nConnecting to SSE server {}...\n", server);
    let url = server_config
        .url
        .as_ref()
        .context("No url for sse server")?;

    // Initialize transport
    let transport = SseTransport::start(url).await?;

    // Setup client info
    let client_info = ClientInfo {
        protocol_version: Default::default(),
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: format!("mcp-client-{}", server),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
    };

    // Initialize client
    let client = client_info.serve(transport).await?;

    // list tools
    println!("ready to call list_tools ...");
    let tools_result = match tokio::time::timeout(
        tokio::time::Duration::from_secs(10),
        client.list_tools(Default::default()),
    )
    .await
    {
        Ok(Ok(tools)) => {
            println!("list_tools call success");
            tools.tools
        }
        Ok(Err(e)) => {
            println!("list_tools call failed: {e}");
            client.cancel().await?;
            return Ok(());
        }
        Err(_) => {
            println!("list_tools call timeout!");
            client.cancel().await?;
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
    client.cancel().await?;
    Ok(())
}
