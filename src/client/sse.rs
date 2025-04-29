use crate::client::utils;
use crate::client::CallToolInput;
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

/// Call a tool on an SSE server and print the result
pub async fn call_tool_sse(input: CallToolInput<'_>) -> anyhow::Result<()> {
    println!("\nConnecting to SSE server for tool call...\n");
    let server_config = input.server_config;
    let url = server_config
        .url
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No url for sse server"))?;

    // Initialize transport
    let transport = rmcp::transport::sse::SseTransport::start(url).await?;

    // Setup client info
    let client_info = rmcp::model::ClientInfo {
        protocol_version: Default::default(),
        capabilities: rmcp::model::ClientCapabilities::default(),
        client_info: rmcp::model::Implementation {
            name: format!("mcp-client-{}", input.server_name),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
    };

    // Initialize client
    let client = client_info.serve(transport).await?;

    // get tool list and check if the tool exists
    let tools = client.list_all_tools().await?;
    let (tool, req) = match utils::prepare_tool_call(&tools, &input) {
        Ok(pair) => pair,
        Err(msg) => {
            println!("{}", msg);
            client.cancel().await?;
            return Ok(());
        }
    };
    println!(
        "Calling tool: {}\nDescription: {:?}",
        tool.name, tool.description
    );

    // call tool
    let result = client.call_tool(req).await;
    match result {
        Ok(res) => {
            if res.is_error.unwrap_or(false) {
                println!("[Tool Error]");
            }
            for (i, content) in res.content.iter().enumerate() {
                utils::print_content(i, content);
            }
        }
        Err(e) => {
            println!("Tool call failed: {e}");
        }
    }
    client.cancel().await?;
    Ok(())
}
