use cherry_db_manager::{CherryDbManager, DefaultCherryDbManager, ServerRequest};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a manager instance
    let manager = DefaultCherryDbManager::new();
    let db_path = "./leveldb";

    println!("🔍 Cherry DB Manager Basic Usage Example");
    println!("=======================================");

    // Read current MCP configuration
    println!("\n📖 Reading current MCP server configuration...");
    match manager.read_mcp_config(db_path) {
        Ok(config) => {
            println!("✅ Found {} servers", config.servers.len());
        }
        Err(e) => {
            println!("❌ Failed to read config: {}", e);
            return Ok(());
        }
    }

    // List all servers
    println!("\n📋 Listing all MCP servers...");
    match manager.list_servers(db_path) {
        Ok(response) => {
            println!("✅ Found {} servers:", response.total_count);
            for server in &response.servers {
                let status = if server.is_active { "🟢" } else { "🔴" };
                let cmd = server.command.as_deref().unwrap_or("-");
                let url = server.base_url.as_deref().unwrap_or("-");
                println!(
                    "   {} {} [{}] cmd:{} url:{} ({})",
                    status, server.id, server.server_type, cmd, url, server.name
                );
            }
        }
        Err(e) => println!("❌ Failed to list servers: {}", e),
    }

    // Example: Add a new server
    println!("\n➕ Example: Adding a new server...");
    let new_server = ServerRequest {
        id: "example-server".to_string(),
        is_active: false,
        args: Some(vec!["--example".to_string(), "parameter".to_string()]),
        command: Some("node".to_string()),
        server_type: "stdio".to_string(),
        name: "Example Server".to_string(),
        env: None,
        base_url: None,
        headers: None,
        long_running: None,
    };

    // Note: This would actually modify the database in a real scenario
    println!(
        "📝 Would add server: {}",
        serde_json::to_string_pretty(&new_server)?
    );
    println!("   (Database write operations require proper permissions)");

    // Example: Check if server exists
    println!("\n🔍 Checking if '21magic' server exists...");
    match manager.server_exists(db_path, "21magic") {
        Ok(exists) => {
            if exists {
                println!("✅ Server '21magic' exists");
            } else {
                println!("❌ Server '21magic' not found");
            }
        }
        Err(e) => println!("❌ Error checking server: {}", e),
    }

    println!("\n🎉 Example completed!");
    Ok(())
}
