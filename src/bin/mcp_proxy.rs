use anyhow::{Context, Result};
use clap::Parser;
use mcp_client::config::{load_rule_config, load_server_config, Config};
use rmcp::{
    model::{ServerCapabilities, ServerInfo, Tool},
    service::{RunningService, ServiceExt},
    tool,
    transport::{
        sse::SseTransport,
        sse_server::{SseServer, SseServerConfig},
        TokioChildProcess,
    },
    RoleClient, ServerHandler,
};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{process::Command, sync::Mutex, time::sleep};
use tracing_subscriber::{self, EnvFilter};

/// Command line arguments for the MCP proxy server
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the MCP configuration file
    #[arg(short, long, default_value = "config/mcp.json")]
    config: PathBuf,

    /// Path to the rule configuration file
    #[arg(short, long, default_value = "config/rule.json5")]
    rule_config: PathBuf,

    /// Port to listen on
    #[arg(short, long, default_value = "8000")]
    port: u16,

    /// Log level
    #[arg(short, long, default_value = "info")]
    log_level: String,
}

/// Connection status for an upstream server
#[derive(Debug, Clone, PartialEq)]
enum ConnectionStatus {
    /// Server is connected and operational
    Connected,
    /// Server is disconnected
    Disconnected,
    /// Server is in the process of connecting
    Connecting,
    /// Server connection failed with an error
    Failed(String),
}

/// Connection to an upstream MCP server
#[derive(Debug)]
struct UpstreamConnection {
    /// TODO: Name of the server
    #[allow(dead_code)]
    server_name: String,
    /// Active service connection
    service: Option<RunningService<RoleClient, ()>>,
    /// Tools provided by this server
    tools: Vec<Tool>,
    /// Last time the server was connected
    last_connected: Instant,
    /// Number of connection attempts
    connection_attempts: u32,
    /// Current connection status
    status: ConnectionStatus,
}

/// Pool of connections to upstream MCP servers
#[derive(Debug)]
struct UpstreamConnectionPool {
    /// Map of server name to connection
    connections: HashMap<String, UpstreamConnection>,
    /// Server configuration
    config: Arc<Config>,
    /// Rule configuration
    rule_config: Arc<HashMap<String, bool>>,
}

impl UpstreamConnectionPool {
    /// Create a new connection pool
    fn new(config: Arc<Config>, rule_config: Arc<HashMap<String, bool>>) -> Self {
        Self {
            connections: HashMap::new(),
            config,
            rule_config,
        }
    }

    /// Initialize the connection pool with all enabled servers
    fn initialize(&mut self) {
        for (name, _server_config) in &self.config.mcp_servers {
            // Skip the proxy server itself
            if name == "proxy" {
                continue;
            }

            // Check if the server is enabled in the rule configuration
            let enabled = self.rule_config.get(name).copied().unwrap_or(false);
            if !enabled {
                tracing::info!("Server '{}' is disabled, skipping", name);
                continue;
            }

            // Create a new connection
            self.connections.insert(
                name.clone(),
                UpstreamConnection {
                    server_name: name.clone(),
                    service: None,
                    tools: Vec::new(),
                    last_connected: Instant::now(),
                    connection_attempts: 0,
                    status: ConnectionStatus::Disconnected,
                },
            );
        }

        tracing::info!(
            "Initialized connection pool with {} enabled servers",
            self.connections.len()
        );
    }

    /// Connect to a specific server
    async fn connect(&mut self, server_name: &str) -> Result<()> {
        // Check if we should connect
        {
            let conn = self.connections.get(server_name).context(format!(
                "Server '{}' not found in connection pool",
                server_name
            ))?;

            // Avoid connecting if already connecting
            if matches!(conn.status, ConnectionStatus::Connecting) {
                return Ok(());
            }
        };

        // Update status and increment connection attempts
        {
            let conn = self.connections.get_mut(server_name).unwrap();
            conn.status = ConnectionStatus::Connecting;
            conn.connection_attempts += 1;
        }

        tracing::info!("Connecting to server '{}'...", server_name);

        // Get the server type
        let server_type = {
            let server_config = self.config.mcp_servers.get(server_name).unwrap();
            server_config.kind.clone()
        };

        // Connect based on server type
        let result = match server_type.as_str() {
            "stdio" => self.connect_stdio(server_name).await,
            "sse" => self.connect_sse(server_name).await,
            _ => {
                let error_msg = format!("Unsupported server type: {}", server_type);
                let conn = self.connections.get_mut(server_name).unwrap();
                conn.status = ConnectionStatus::Failed(error_msg.clone());
                Err(anyhow::anyhow!(error_msg))
            }
        };

        // If there was an error, update the status
        if let Err(e) = &result {
            if let Some(conn) = self.connections.get_mut(server_name) {
                conn.status = ConnectionStatus::Failed(e.to_string());
            }
        }

        result
    }

    /// Connect to a stdio server
    async fn connect_stdio(&mut self, server_name: &str) -> Result<()> {
        // Get server configuration
        let server_config = self.config.mcp_servers.get(server_name).unwrap();

        // Get command and arguments
        let command = server_config
            .command
            .as_ref()
            .context("Command not specified for stdio server")?;

        // Create command
        let mut cmd = Command::new(command);

        // Add arguments if any
        if let Some(args) = &server_config.args {
            cmd.args(args);
        }

        // Add environment variables if any
        if let Some(env) = &server_config.env {
            for (key, value) in env {
                cmd.env(key, value);
            }
        }

        // Connect to the server
        match TokioChildProcess::new(&mut cmd) {
            Ok(child_process) => {
                match ().serve(child_process).await {
                    Ok(service) => {
                        // Get tools
                        match service.list_tools(Default::default()).await {
                            Ok(tools_result) => {
                                // Update connection
                                let conn = self.connections.get_mut(server_name).unwrap();
                                conn.tools = tools_result.tools;
                                conn.service = Some(service);
                                conn.status = ConnectionStatus::Connected;
                                conn.last_connected = Instant::now();
                                tracing::info!(
                                    "Connected to server '{}', found {} tools",
                                    server_name,
                                    conn.tools.len()
                                );
                                Ok(())
                            }
                            Err(e) => {
                                let error_msg = format!("Failed to list tools: {}", e);
                                Err(anyhow::anyhow!(error_msg))
                            }
                        }
                    }
                    Err(e) => {
                        let error_msg = format!("Failed to connect to server: {}", e);
                        Err(anyhow::anyhow!(error_msg))
                    }
                }
            }
            Err(e) => {
                let error_msg = format!("Failed to create child process: {}", e);
                Err(anyhow::anyhow!(error_msg))
            }
        }
    }

    /// Connect to an SSE server
    async fn connect_sse(&mut self, server_name: &str) -> Result<()> {
        // Get server configuration
        let server_config = self.config.mcp_servers.get(server_name).unwrap();

        // Get URL
        let url = server_config
            .url
            .as_ref()
            .context("URL not specified for SSE server")?;

        // Connect to the server
        match SseTransport::start(url).await {
            Ok(transport) => {
                match ().serve(transport).await {
                    Ok(service) => {
                        // Get tools
                        match service.list_tools(Default::default()).await {
                            Ok(tools_result) => {
                                // Update connection
                                let conn = self.connections.get_mut(server_name).unwrap();
                                conn.tools = tools_result.tools;
                                conn.service = Some(service);
                                conn.status = ConnectionStatus::Connected;
                                conn.last_connected = Instant::now();
                                tracing::info!(
                                    "Connected to server '{}', found {} tools",
                                    server_name,
                                    conn.tools.len()
                                );
                                Ok(())
                            }
                            Err(e) => {
                                let error_msg = format!("Failed to list tools: {}", e);
                                Err(anyhow::anyhow!(error_msg))
                            }
                        }
                    }
                    Err(e) => {
                        let error_msg = format!("Failed to connect to server: {}", e);
                        Err(anyhow::anyhow!(error_msg))
                    }
                }
            }
            Err(e) => {
                let error_msg = format!("Failed to create SSE transport: {}", e);
                Err(anyhow::anyhow!(error_msg))
            }
        }
    }

    /// Disconnect from a server
    async fn disconnect(&mut self, server_name: &str) -> Result<()> {
        let conn = self.connections.get_mut(server_name).context(format!(
            "Server '{}' not found in connection pool",
            server_name
        ))?;

        // Take the service out of the connection
        if let Some(service) = conn.service.take() {
            // Try to gracefully close the connection
            if let Err(e) = service.cancel().await {
                tracing::warn!("Error disconnecting from server '{}': {}", server_name, e);
            }
        }

        conn.status = ConnectionStatus::Disconnected;
        tracing::info!("Disconnected from server '{}'", server_name);

        Ok(())
    }

    /// Reconnect to a server
    async fn reconnect(&mut self, server_name: &str) -> Result<()> {
        // First disconnect
        self.disconnect(server_name).await?;

        // Get connection for backoff calculation
        let conn = self.connections.get(server_name).context(format!(
            "Server '{}' not found in connection pool",
            server_name
        ))?;

        // Calculate backoff time using exponential backoff
        let backoff = std::cmp::min(
            30,                                                   // Maximum 30 seconds
            2u64.pow(std::cmp::min(5, conn.connection_attempts)), // Exponential backoff, max 2^5=32 seconds
        );

        tracing::info!(
            "Waiting {}s before reconnecting to '{}'",
            backoff,
            server_name
        );
        sleep(Duration::from_secs(backoff)).await;

        // Reconnect
        self.connect(server_name).await
    }

    /// Connect to all servers (sequential version)
    #[allow(dead_code)]
    async fn connect_all(&mut self) -> Result<()> {
        let server_names: Vec<String> = self.connections.keys().cloned().collect();

        // connect all servers one by one
        for name in server_names {
            if let Err(e) = self.connect(&name).await {
                tracing::error!("Failed to connect to server '{}': {}", name, e);
            }
        }

        Ok(())
    }

    /// Disconnect from all servers
    async fn disconnect_all(&mut self) -> Result<()> {
        let server_names: Vec<String> = self.connections.keys().cloned().collect();

        for name in server_names {
            if let Err(e) = self.disconnect(&name).await {
                tracing::error!("Failed to disconnect from server '{}': {}", name, e);
                // Continue with other servers even if one fails
            }
        }

        Ok(())
    }

    /// TODO: Get all tools from all connected servers
    #[allow(dead_code)]
    fn get_all_tools(&self) -> Vec<Tool> {
        let mut tools = Vec::new();

        for (_, conn) in &self.connections {
            if conn.status == ConnectionStatus::Connected {
                tools.extend(conn.tools.clone());
            }
        }

        tools
    }

    /// Start health check task
    async fn start_health_check(pool: Arc<Mutex<Self>>) {
        tokio::spawn(async move {
            loop {
                // Wait for 30 seconds between health checks
                sleep(Duration::from_secs(30)).await;

                // Lock the pool
                let mut pool_guard = pool.lock().await;

                // Check each connection
                let server_names: Vec<String> = pool_guard.connections.keys().cloned().collect();

                for name in server_names {
                    let conn = match pool_guard.connections.get(&name) {
                        Some(conn) => conn,
                        None => continue,
                    };

                    match conn.status {
                        ConnectionStatus::Disconnected | ConnectionStatus::Failed(_) => {
                            // Try to reconnect
                            tracing::info!("Health check: Attempting to reconnect to '{}'", name);
                            if let Err(e) = pool_guard.connect(&name).await {
                                tracing::warn!(
                                    "Health check: Failed to reconnect to '{}': {}",
                                    name,
                                    e
                                );
                            }
                        }
                        ConnectionStatus::Connected => {
                            // Check if the connection is still valid
                            if let Some(service) = &conn.service {
                                // Try to list tools as a simple ping
                                let result = service.list_tools(Default::default()).await;
                                if result.is_err() {
                                    tracing::warn!("Health check: Server '{}' appears to be disconnected, will reconnect", name);
                                    // Drop the lock before reconnecting to avoid deadlock
                                    drop(pool_guard);

                                    // Get a new lock
                                    let mut new_pool_guard = pool.lock().await;
                                    if let Err(e) = new_pool_guard.reconnect(&name).await {
                                        tracing::error!(
                                            "Health check: Failed to reconnect to '{}': {}",
                                            name,
                                            e
                                        );
                                    }

                                    // Update pool_guard
                                    pool_guard = new_pool_guard;
                                }
                            }
                        }
                        ConnectionStatus::Connecting => {
                            // Connection is in progress, do nothing
                        }
                    }
                }

                // Drop the lock
                drop(pool_guard);
            }
        });
    }
}

/// MCP Proxy Server that aggregates tools from multiple MCP servers
#[derive(Debug, Clone)]
struct ProxyServer {
    /// Connection pool for upstream servers
    connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
}

#[tool(tool_box)]
impl ProxyServer {
    pub fn new(config: Arc<Config>, rule_config: Arc<HashMap<String, bool>>) -> Self {
        // Create connection pool
        let mut pool = UpstreamConnectionPool::new(config, rule_config);

        // Initialize the pool
        pool.initialize();

        let connection_pool = Arc::new(Mutex::new(pool));

        // Start health check task
        tokio::spawn(UpstreamConnectionPool::start_health_check(
            connection_pool.clone(),
        ));

        Self { connection_pool }
    }
}

#[tool(tool_box)]
impl ServerHandler for ProxyServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "MCP Proxy Server that aggregates tools from multiple MCP servers".into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Initialize the tracing subscriber
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive(
                args.log_level
                    .parse()
                    .unwrap_or(tracing::Level::INFO.into()),
            ),
        )
        .init();

    // Load the MCP server and rule configuration
    let config = load_server_config(&args.config)?;
    let rule_config = load_rule_config(&args.rule_config)?;

    // Log the loaded configuration
    tracing::info!("Loaded configuration from: {}", args.config.display());
    tracing::info!(
        "Found {} MCP servers in configuration",
        config.mcp_servers.len()
    );
    tracing::info!(
        "Loaded rule configuration from: {}",
        args.rule_config.display()
    );

    // Create a map of server name to enabled status
    let enabled_servers = rule_config
        .rules
        .iter()
        .map(|(name, rule)| (name.clone(), rule.enabled))
        .collect::<HashMap<String, bool>>();

    // Create proxy server
    let proxy = Arc::new(ProxyServer::new(
        Arc::new(config),
        Arc::new(enabled_servers),
    ));

    // connect to all servers in the background
    tokio::spawn({
        let proxy_clone = proxy.clone();
        async move {
            // wait for a short time to ensure the SSE server is started
            tokio::time::sleep(Duration::from_millis(100)).await;

            // connect to all servers
            let mut pool = proxy_clone.connection_pool.lock().await;

            // connect all servers one by one (not blocking the main thread)
            let server_names: Vec<String> = pool.connections.keys().cloned().collect();
            for name in server_names {
                if let Err(e) = pool.connect(&name).await {
                    tracing::error!("Failed to connect to server '{}': {}", name, e);
                }
            }

            // record the connection status
            let connected_count = pool
                .connections
                .values()
                .filter(|conn| conn.status == ConnectionStatus::Connected)
                .count();

            tracing::info!(
                "Connected to {}/{} upstream servers",
                connected_count,
                pool.connections.len()
            );
        }
    });

    // Start SSE server
    let bind_address = format!("127.0.0.1:{}", args.port).parse()?;
    tracing::info!("Starting SSE server on {}", bind_address);

    let server_config = SseServerConfig {
        bind: bind_address,
        sse_path: "/sse".to_string(),
        post_path: "/message".to_string(),
        ct: Default::default(),
    };

    // Create a factory function that returns a new ProxyServer instance
    let proxy_clone = proxy.clone();
    let factory = move || {
        let p = proxy_clone.clone();
        ProxyServer {
            connection_pool: Arc::clone(&p.connection_pool),
        }
    };

    let server = SseServer::serve_with_config(server_config)
        .await?
        .with_service(factory);

    tracing::info!("Server started. Press Ctrl+C to stop.");

    // Wait for Ctrl+C
    tokio::signal::ctrl_c().await?;
    tracing::info!("Received Ctrl+C, shutting down...");
    server.cancel();

    // Disconnect from all servers
    {
        let mut pool = proxy.connection_pool.lock().await;
        pool.disconnect_all().await?;
        tracing::info!("Disconnected from all upstream servers");
    }

    Ok(())
}
