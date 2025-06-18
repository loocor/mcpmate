use anyhow::Result;
use mcpmate::standalone::run_standalone_with_args;

#[tokio::main]
async fn main() -> Result<()> {
    run_standalone_with_args().await
}
