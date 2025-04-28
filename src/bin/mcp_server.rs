use anyhow::Result;
use rmcp::{
    model::{ServerCapabilities, ServerInfo},
    tool, ServerHandler, ServiceExt, transport::stdio,
};
use serde::{Deserialize, Serialize};
use tracing_subscriber::{self, EnvFilter};

/// A simple calculator request for demonstration
#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct CalculatorRequest {
    #[schemars(description = "First number")]
    pub a: i32,
    #[schemars(description = "Second number")]
    pub b: i32,
}

/// A simple MCP server that provides a calculator tool
#[derive(Debug, Clone)]
pub struct SimpleServer;

#[tool(tool_box)]
impl SimpleServer {
    pub fn new() -> Self {
        Self {}
    }

    #[tool(description = "Add two numbers")]
    fn add(&self, #[tool(aggr)] CalculatorRequest { a, b }: CalculatorRequest) -> String {
        (a + b).to_string()
    }

    #[tool(description = "Subtract second number from first")]
    fn subtract(
        &self,
        #[tool(param)]
        #[schemars(description = "First number")]
        a: i32,
        #[tool(param)]
        #[schemars(description = "Second number")]
        b: i32,
    ) -> String {
        (a - b).to_string()
    }
}

#[tool(tool_box)]
impl ServerHandler for SimpleServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("A simple calculator server that can add and subtract numbers".into()),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            ..Default::default()
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize the tracing subscriber with file and stdout logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("Starting MCP server");

    // Create an instance of our server
    let service = SimpleServer::new().serve(stdio()).await?;

    // Wait for the service to complete
    service.waiting().await?;

    Ok(())
}
