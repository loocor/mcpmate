use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use once_cell::sync::OnceCell;
use rmcp::{
    ClientHandler, ErrorData as McpError, RoleClient, RoleServer, ServerHandler,
    model::{
        CallToolRequestParam, CallToolResult, ClientCapabilities, ClientInfo, Implementation, ProtocolVersion,
        ServerCapabilities, ServerInfo,
    },
    serve_server,
    service::{NotificationContext, RequestContext, ServiceExt},
    transport::{SseClientTransport, io},
};
use tokio::sync::Mutex;
use tracing_subscriber::{self, EnvFilter};

// Global variable to store SSE client
static SSE_CLIENT: OnceCell<Mutex<Option<rmcp::service::RunningService<RoleClient, BridgeClient>>>> = OnceCell::new();

/// A client handler for the bridge client
#[derive(Clone, Debug)]
struct BridgeClient {
    /// Flag indicating whether the tool list has changed
    tool_list_changed: Arc<Mutex<bool>>,
}

impl BridgeClient {
    /// Create a new bridge client
    fn new() -> Self {
        Self {
            tool_list_changed: Arc::new(Mutex::new(false)),
        }
    }
}

impl ClientHandler for BridgeClient {
    fn get_info(&self) -> ClientInfo {
        // get the appid from the environment variable
        let appid = std::env::var("APPID").unwrap_or_default();

        ClientInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ClientCapabilities::default(),
            client_info: Implementation {
                name: format!("mcpmate-bridge::{appid}"),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        }
    }

    async fn on_tool_list_changed(
        &self,
        _context: NotificationContext<RoleClient>,
    ) {
        tracing::info!("Received tool list changed notification from upstream server");

        // Set the tool list changed flag
        let mut flag = self.tool_list_changed.lock().await;
        *flag = true;

        // The notification will be forwarded to downstream clients
        // when they call list_tools or call_tool
    }

    // Implement other notification handlers with default behavior
    // These are provided by the trait with default implementations,
    // but we're explicitly implementing them for clarity

    async fn on_resource_list_changed(
        &self,
        _context: NotificationContext<RoleClient>,
    ) {
        tracing::debug!("Received resource list changed notification (ignored)");
        // We don't handle resource list changes in the bridge
    }

    async fn on_prompt_list_changed(
        &self,
        _context: NotificationContext<RoleClient>,
    ) {
        tracing::debug!("Received prompt list changed notification (ignored)");
        // We don't handle prompt list changes in the bridge
    }

    async fn on_resource_updated(
        &self,
        params: rmcp::model::ResourceUpdatedNotificationParam,
        _context: NotificationContext<RoleClient>,
    ) {
        tracing::debug!(
            "Received resource updated notification for URI: {} (ignored)",
            params.uri
        );
        // We don't handle resource updates in the bridge
    }

    async fn on_progress(
        &self,
        params: rmcp::model::ProgressNotificationParam,
        _context: NotificationContext<RoleClient>,
    ) {
        tracing::debug!("Received progress notification: {:?} (ignored)", params);
        // We don't handle progress notifications in the bridge
    }

    async fn on_cancelled(
        &self,
        params: rmcp::model::CancelledNotificationParam,
        _context: NotificationContext<RoleClient>,
    ) {
        tracing::debug!("Received cancelled notification: {:?} (ignored)", params);
        // We don't handle cancelled notifications in the bridge
    }

    async fn on_logging_message(
        &self,
        params: rmcp::model::LoggingMessageNotificationParam,
        _context: NotificationContext<RoleClient>,
    ) {
        tracing::debug!("Received logging message: {:?} (ignored)", params);
        // We don't handle logging messages in the bridge
    }
}

/// Command line arguments for the stdio to SSE bridge
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// URL of the SSE server to connect to
    #[arg(short, long, default_value = "http://127.0.0.1:8000/sse")]
    sse_url: String,

    /// Log level
    #[arg(short, long, default_value = "info")]
    log_level: String,
}

/// A bridge server that forwards requests to an SSE server
#[derive(Clone)]
struct BridgeServer {
    /// The URL of the upstream SSE server
    sse_url: String,
}

impl BridgeServer {
    /// Create a new bridge server
    fn new(sse_url: String) -> Self {
        Self { sse_url }
    }

    /// Check if the tool list has changed and send a notification if needed
    async fn check_tool_list_changed(
        &self,
        context: RequestContext<RoleServer>,
    ) -> Result<(), McpError> {
        // Get the global SSE client
        if let Some(client_mutex) = SSE_CLIENT.get() {
            let client_guard = client_mutex.lock().await;

            if let Some(sse_client) = &*client_guard {
                // Get the client handler
                let client_handler = sse_client.service();

                // Check if the tool list has changed
                let tool_list_changed = {
                    let flag = client_handler.tool_list_changed.lock().await;
                    *flag
                };

                // If the tool list has changed, send a notification
                if tool_list_changed {
                    // Send the notification
                    if let Err(e) = context.peer.notify_tool_list_changed().await {
                        tracing::error!("Failed to send tool list changed notification: {e:?}");
                        return Err(McpError::internal_error(
                            format!("Failed to send tool list changed notification: {e:?}"),
                            None,
                        ));
                    }

                    // Reset the flag
                    let mut flag = client_handler.tool_list_changed.lock().await;
                    *flag = false;

                    tracing::info!("Sent tool list changed notification to downstream client");
                }
            }
        }

        Ok(())
    }

    /// Connect to the upstream SSE server
    async fn connect_to_sse(&self) -> Result<(), McpError> {
        // initialize the global SSE client
        if SSE_CLIENT.get().is_none() {
            let mutex = Mutex::new(None);
            SSE_CLIENT.set(mutex).unwrap();
        }

        let client_mutex = SSE_CLIENT.get().unwrap();
        let mut client_guard = client_mutex.lock().await;

        // if already connected, do not repeat the connection
        if client_guard.is_some() {
            return Ok(());
        }

        // create client handler
        let client_handler = BridgeClient::new();

        // create SSE transport
        let sse_transport = match SseClientTransport::start(self.sse_url.as_str()).await {
            Ok(transport) => {
                tracing::info!("Successfully connected to SSE server");
                transport
            }
            Err(e) => {
                tracing::error!("Failed to connect to SSE server: {}", e);
                return Err(McpError::internal_error("Failed to connect to SSE server", None));
            }
        };

        // initialize SSE client
        *client_guard = match client_handler.serve(sse_transport).await {
            Ok(client) => {
                tracing::info!("Successfully initialized SSE client");
                Some(client)
            }
            Err(e) => {
                tracing::error!("Failed to initialize SSE client: {}", e);
                return Err(McpError::internal_error("Failed to initialize SSE client", None));
            }
        };

        Ok(())
    }
}

/// A server handler for the bridge server
impl ServerHandler for BridgeServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_tool_list_changed() // Enable tool list changed notifications
                .build(),
            server_info: Implementation {
                name: "mcpmate-bridge".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some("This is a bridge server that forwards requests to an SSE server.".to_string()),
        }
    }

    async fn list_tools(
        &self,
        request: Option<rmcp::model::PaginatedRequestParam>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<rmcp::model::ListToolsResult, McpError> {
        // Check if the tool list has changed and send a notification if needed
        if let Err(e) = self.check_tool_list_changed(ctx.clone()).await {
            tracing::warn!("Failed to check tool list changed: {}", e);
            // Continue with the request even if the notification failed
        }

        // get the global SSE client
        if let Some(client_mutex) = SSE_CLIENT.get() {
            let client_guard = client_mutex.lock().await;

            if let Some(sse_client) = &*client_guard {
                match sse_client.list_tools(request).await {
                    Ok(upstream_result) => Ok(upstream_result),
                    Err(e) => {
                        tracing::error!("Failed to get tools from SSE server: {}", e);
                        // report upstream service error
                        Err(McpError::internal_error(
                            format!("Upstream SSE service error: {e:?}"),
                            None,
                        ))
                    }
                }
            } else {
                // report upstream service not available
                Err(McpError::internal_error("Upstream SSE service is not available", None))
            }
        } else {
            // report upstream service not available
            Err(McpError::internal_error("Upstream SSE service is not available", None))
        }
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        // Check if the tool list has changed and send a notification if needed
        if let Err(e) = self.check_tool_list_changed(ctx.clone()).await {
            tracing::warn!("Failed to check tool list changed: {}", e);
            // Continue with the request even if the notification failed
        }

        // get the global SSE client
        if let Some(client_mutex) = SSE_CLIENT.get() {
            let client_guard = client_mutex.lock().await;

            if let Some(sse_client) = &*client_guard {
                // create CallToolRequestParam
                let call_request = CallToolRequestParam {
                    name: request.name.clone(),
                    arguments: request.arguments.clone(),
                };

                match sse_client.call_tool(call_request).await {
                    Ok(result) => Ok(result),
                    Err(e) => {
                        tracing::error!("Failed to call tool on SSE server: {}", e);
                        // report upstream service error
                        Err(McpError::internal_error(
                            format!("Upstream SSE service error: {e:?}"),
                            None,
                        ))
                    }
                }
            } else {
                // report upstream service not available
                Err(McpError::internal_error("Upstream SSE service is not available", None))
            }
        } else {
            // report upstream service not available
            Err(McpError::internal_error("Upstream SSE service is not available", None))
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Initialize the tracing subscriber to write logs to stderr instead of stdout
    // This is critical for stdio mode as stdout is used for JSON-RPC communication
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr) // Redirect logs to stderr
        .with_env_filter(
            EnvFilter::from_default_env().add_directive(args.log_level.parse().unwrap_or(tracing::Level::INFO.into())),
        )
        .init();

    tracing::info!("Starting stdio to SSE bridge");
    tracing::info!("Using protocol version: 2024-11-05 for compatibility");

    // Create a new bridge server
    let bridge_server = BridgeServer::new(args.sse_url.clone());

    // try to connect to the upstream SSE server
    tracing::info!("Connecting to upstream SSE server at {}", args.sse_url);
    match bridge_server.connect_to_sse().await {
        Ok(_) => tracing::info!("Successfully connected to upstream SSE server"),
        Err(e) => {
            // record the error but continue to start the service
            tracing::error!("Failed to connect to upstream SSE server: {}", e);
            tracing::warn!("Bridge will start but will report upstream service as unavailable");
        }
    }

    // Create stdio transport
    let stdio_transport = io::stdio();
    tracing::info!("Created stdio transport");

    // Serve the bridge server over stdio
    tracing::info!("Initializing stdio server...");

    // Use rmcp's serve_server function to create and serve our bridge server
    let server = match serve_server(bridge_server, stdio_transport).await {
        Ok(server) => {
            tracing::info!("Successfully initialized stdio server");
            server
        }
        Err(e) => {
            tracing::error!("Failed to initialize stdio server: {}", e);
            return Err(anyhow::anyhow!("Failed to initialize stdio server: {}", e));
        }
    };

    // Wait for the server to exit
    tracing::info!("Bridge is now running. Waiting for stdio server to exit...");
    match server.waiting().await {
        Ok(_) => tracing::info!("Stdio server exited normally"),
        Err(e) => tracing::error!("Stdio server exited with error: {}", e),
    }

    tracing::info!("Bridge shut down");
    Ok(())
}
