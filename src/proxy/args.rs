use clap::Parser;

/// Command line arguments for the MCP proxy server
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Port to listen on for MCP server
    #[arg(short, long, default_value = "8000")]
    pub port: u16,

    /// Port to listen on for API server
    #[arg(long, default_value = "8080")]
    pub api_port: u16,

    /// Log level
    #[arg(short, long, default_value = "info")]
    pub log_level: String,

    /// Transport type (sse, str, or uni)
    #[arg(long, alias = "trans", default_value = "uni")]
    pub transport: String,
}
