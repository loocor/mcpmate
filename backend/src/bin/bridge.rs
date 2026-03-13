use anyhow::Result;
use clap::Parser;
use mcpmate::common::constants::branding;
use reqwest::{
    Client as HttpClient,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use rmcp::{
    ClientHandler, ErrorData as McpError, RoleClient, RoleServer, ServerHandler,
    model::{
        CallToolRequestParam, CallToolResult, CancelledNotificationParam, ClientCapabilities, ClientInfo,
        CompleteRequestParam, CompleteResult, GetPromptRequestParam, GetPromptResult, InitializeRequestParam,
        InitializeResult, ListPromptsResult, ListResourceTemplatesResult, ListResourcesResult, ListToolsResult,
        PaginatedRequestParam, ProtocolVersion, ReadResourceRequestParam, ReadResourceResult, ServerCapabilities,
        ServerInfo, SetLevelRequestParam, SubscribeRequestParam, UnsubscribeRequestParam,
    },
    serve_server,
    service::{NotificationContext, RequestContext, ServiceExt},
    transport::{
        StreamableHttpClientTransport,
        io,
        sse_client::{SseClientConfig, SseClientTransport},
        // streamable_http_client::StreamableHttpClientTransportConfig (constructed via helper),
    },
};
use std::{
    future::Future,
    sync::{Arc, RwLock},
};
use tokio::sync::Mutex;
use tracing_subscriber::{self, EnvFilter};

/// Command line arguments for the bridge.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// URL of the upstream MCP server to connect to (Streamable HTTP endpoint).
    #[arg(
        short = 'u',
        short_alias = 's',
        long = "upstream-url",
        alias = "sse-url",
        default_value = "http://127.0.0.1:8000/mcp"
    )]
    upstream_url: String,

    /// Upstream bearer token used for Streamable HTTP or SSE (without the 'Bearer ' prefix).
    /// If provided with 'Bearer ' prefix, it will be stripped.
    #[arg(long = "upstream-bearer")]
    upstream_bearer: Option<String>,

    /// Log level
    #[arg(short, long, default_value = "info")]
    log_level: String,
}

#[derive(Default)]
struct NotificationState {
    tool_list_changed: bool,
    prompt_list_changed: bool,
    resource_list_changed: bool,
    cancelled: Vec<CancelledNotificationParam>,
    progress: Vec<rmcp::model::ProgressNotificationParam>,
    logging: Vec<rmcp::model::LoggingMessageNotificationParam>,
    resource_updates: Vec<rmcp::model::ResourceUpdatedNotificationParam>,
}

#[derive(Default)]
struct BridgeNotifications {
    inner: Mutex<NotificationState>,
}

impl BridgeNotifications {
    async fn mark_tool_list_changed(&self) {
        self.inner.lock().await.tool_list_changed = true;
    }

    async fn mark_prompt_list_changed(&self) {
        self.inner.lock().await.prompt_list_changed = true;
    }

    async fn mark_resource_list_changed(&self) {
        self.inner.lock().await.resource_list_changed = true;
    }

    async fn push_cancelled(
        &self,
        param: CancelledNotificationParam,
    ) {
        self.inner.lock().await.cancelled.push(param);
    }

    async fn push_progress(
        &self,
        param: rmcp::model::ProgressNotificationParam,
    ) {
        self.inner.lock().await.progress.push(param);
    }

    async fn push_logging(
        &self,
        param: rmcp::model::LoggingMessageNotificationParam,
    ) {
        self.inner.lock().await.logging.push(param);
    }

    async fn push_resource_update(
        &self,
        param: rmcp::model::ResourceUpdatedNotificationParam,
    ) {
        self.inner.lock().await.resource_updates.push(param);
    }

    async fn take(&self) -> NotificationState {
        let mut guard = self.inner.lock().await;
        std::mem::take(&mut *guard)
    }
}

#[derive(Default)]
struct BridgeRuntimeStore {
    inner: Mutex<Option<BridgeRuntime>>,
}

struct BridgeRuntime {
    service: Arc<rmcp::service::RunningService<RoleClient, BridgeClient>>,
}

#[derive(Clone)]
struct BridgeServer {
    upstream_url: String,
    upstream_bearer: Option<String>,
    notifications: Arc<BridgeNotifications>,
    runtime: Arc<BridgeRuntimeStore>,
    server_info: Arc<RwLock<Option<ServerInfo>>>,
}

fn map_service_error(error: rmcp::service::ServiceError) -> McpError {
    match error {
        rmcp::service::ServiceError::McpError(err) => err,
        rmcp::service::ServiceError::TransportSend(err) => {
            McpError::internal_error(format!("Upstream transport send error: {err}"), None)
        }
        rmcp::service::ServiceError::TransportClosed => McpError::internal_error("Upstream transport closed", None),
        rmcp::service::ServiceError::UnexpectedResponse => {
            McpError::internal_error("Unexpected upstream response", None)
        }
        rmcp::service::ServiceError::Cancelled { reason } => McpError::internal_error(
            format!("Upstream request cancelled: {}", reason.unwrap_or_default()),
            None,
        ),
        rmcp::service::ServiceError::Timeout { timeout } => {
            McpError::internal_error(format!("Upstream request timed out after {:?}", timeout), None)
        }
        _ => McpError::internal_error(format!("Unhandled upstream service error: {error}"), None),
    }
}

impl BridgeRuntimeStore {
    async fn get(&self) -> Option<Arc<rmcp::service::RunningService<RoleClient, BridgeClient>>> {
        let guard = self.inner.lock().await;
        guard.as_ref().map(|runtime| runtime.service.clone())
    }

    async fn set(
        &self,
        service: Arc<rmcp::service::RunningService<RoleClient, BridgeClient>>,
    ) {
        let mut guard = self.inner.lock().await;
        *guard = Some(BridgeRuntime { service });
    }

    async fn is_some(&self) -> bool {
        self.inner.lock().await.is_some()
    }
}

enum UpstreamKind {
    Streamable(String),
    Sse(String),
}

fn remap_sse_to_mcp(url: &str) -> Option<String> {
    let mut parsed = reqwest::Url::parse(url).ok()?;
    let trimmed = parsed.path().trim_end_matches('/');
    if let Some(stripped) = trimmed.strip_suffix("/sse") {
        let mut new_path = if stripped.is_empty() {
            String::from("/")
        } else {
            stripped.to_string()
        };
        if !new_path.ends_with('/') {
            new_path.push('/');
        }
        new_path.push_str("mcp");
        parsed.set_path(&new_path);
        Some(parsed.to_string())
    } else {
        None
    }
}

fn resolve_upstream_kind(url: &str) -> UpstreamKind {
    if let Ok(parsed) = reqwest::Url::parse(url) {
        let path = parsed.path().trim_end_matches('/');
        if path == "/mcp" || path.ends_with("/mcp") {
            return UpstreamKind::Streamable(url.to_string());
        }
    }

    if let Some(remapped) = remap_sse_to_mcp(url) {
        UpstreamKind::Streamable(remapped)
    } else {
        UpstreamKind::Sse(url.to_string())
    }
}

impl BridgeServer {
    fn new(
        upstream_url: String,
        upstream_bearer: Option<String>,
        notifications: Arc<BridgeNotifications>,
        runtime: Arc<BridgeRuntimeStore>,
        server_info: Arc<RwLock<Option<ServerInfo>>>,
    ) -> Self {
        Self {
            upstream_url,
            upstream_bearer,
            notifications,
            runtime,
            server_info,
        }
    }

    async fn ensure_runtime(&self) -> Result<Arc<rmcp::service::RunningService<RoleClient, BridgeClient>>, McpError> {
        if let Some(service) = self.runtime.get().await {
            return Ok(service);
        }
        self.establish_upstream().await?;
        self.runtime
            .get()
            .await
            .ok_or_else(|| McpError::internal_error("Upstream MCP service is not available", None))
    }

    async fn establish_upstream(&self) -> Result<(), McpError> {
        if self.runtime.is_some().await {
            return Ok(());
        }

        let protocol_version = ProtocolVersion::LATEST.to_string();
        let mut default_headers = HeaderMap::new();
        let header_value = HeaderValue::from_str(&protocol_version).map_err(|err| {
            tracing::error!("Invalid MCP protocol version header: {err}");
            McpError::internal_error("Invalid MCP protocol version header", None)
        })?;
        default_headers.insert(HeaderName::from_static("mcp-protocol-version"), header_value);

        let http_client = HttpClient::builder()
            .default_headers(default_headers)
            .build()
            .map_err(|err| {
                tracing::error!("Failed to create upstream HTTP client: {err}");
                McpError::internal_error("Failed to create upstream HTTP client", None)
            })?;

        let notifications = self.notifications.clone();
        let client_handler = BridgeClient::new(notifications);
        let client_handler_for_streamable = client_handler.clone();

        let upstream_kind = resolve_upstream_kind(&self.upstream_url);

        let service_result = match upstream_kind {
            UpstreamKind::Streamable(stream_url) => {
                tracing::info!(upstream = %stream_url, "Using streamable HTTP upstream");
                let cfg = mcpmate::common::http::make_streamable_config_with_bearer(
                    &stream_url,
                    self.upstream_bearer.as_deref(),
                );
                let transport = StreamableHttpClientTransport::with_client(http_client.clone(), cfg);
                client_handler_for_streamable.serve(transport).await.map_err(|err| {
                    tracing::error!("Failed to initialize upstream MCP client (streamable): {err}");
                    McpError::internal_error(format!("Failed to initialize upstream MCP client: {err}"), None)
                })
            }
            UpstreamKind::Sse(sse_url) => {
                tracing::info!(upstream = %sse_url, "Using SSE upstream");
                let transport = SseClientTransport::start_with_client(
                    http_client.clone(),
                    SseClientConfig {
                        sse_endpoint: sse_url.clone().into(),
                        ..Default::default()
                    },
                )
                .await
                .map_err(|err| {
                    tracing::error!("Failed to create SSE transport: {err}");
                    McpError::internal_error(format!("Failed to initialize upstream MCP client: {err}"), None)
                })?;
                client_handler.serve(transport).await.map_err(|err| {
                    tracing::error!("Failed to initialize upstream MCP client (sse): {err}");
                    McpError::internal_error(format!("Failed to initialize upstream MCP client: {err}"), None)
                })
            }
        };

        match service_result {
            Ok(service) => {
                let service = Arc::new(service);
                if let Some(info) = service.peer().peer_info().cloned() {
                    let mut guard = self.server_info.write().expect("server_info poisoned");
                    *guard = Some(info);
                }
                self.runtime.set(service).await;
                tracing::info!("Successfully initialized upstream MCP client");
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    async fn upstream_request<F, Fut, T>(
        &self,
        f: F,
    ) -> Result<T, McpError>
    where
        F: FnOnce(Arc<rmcp::service::RunningService<RoleClient, BridgeClient>>) -> Fut,
        Fut: Future<Output = Result<T, rmcp::service::ServiceError>>,
    {
        let service = self.ensure_runtime().await?;
        f(service).await.map_err(map_service_error)
    }

    async fn forward_request<F, Fut, T>(
        &self,
        context: &RequestContext<RoleServer>,
        f: F,
    ) -> Result<T, McpError>
    where
        F: FnOnce(Arc<rmcp::service::RunningService<RoleClient, BridgeClient>>) -> Fut,
        Fut: Future<Output = Result<T, rmcp::service::ServiceError>>,
    {
        self.flush_notifications(context).await?;
        let result = self.upstream_request(f).await?;
        self.flush_notifications(context).await?;
        Ok(result)
    }

    async fn forward_notification<F, Fut>(
        &self,
        f: F,
    ) where
        F: FnOnce(Arc<rmcp::service::RunningService<RoleClient, BridgeClient>>) -> Fut,
        Fut: Future<Output = Result<(), rmcp::service::ServiceError>>,
    {
        if let Err(err) = self.upstream_request(f).await {
            tracing::warn!("Failed to forward notification upstream: {err}");
        }
    }

    async fn flush_notifications(
        &self,
        context: &RequestContext<RoleServer>,
    ) -> Result<(), McpError> {
        let pending = self.notifications.take().await;
        let peer = context.peer.clone();

        if pending.tool_list_changed {
            peer.notify_tool_list_changed().await.map_err(map_service_error)?;
        }
        if pending.prompt_list_changed {
            peer.notify_prompt_list_changed().await.map_err(map_service_error)?;
        }
        if pending.resource_list_changed {
            peer.notify_resource_list_changed().await.map_err(map_service_error)?;
        }
        for update in pending.resource_updates {
            peer.notify_resource_updated(update).await.map_err(map_service_error)?;
        }
        for progress in pending.progress {
            peer.notify_progress(progress).await.map_err(map_service_error)?;
        }
        for cancelled in pending.cancelled {
            peer.notify_cancelled(cancelled).await.map_err(map_service_error)?;
        }
        for logging in pending.logging {
            peer.notify_logging_message(logging).await.map_err(map_service_error)?;
        }
        Ok(())
    }

    fn downstream_server_info(&self) -> ServerInfo {
        if let Some(info) = self.server_info.read().expect("server_info poisoned").clone() {
            info
        } else {
            ServerInfo {
                server_info: branding::bridge::create_server_implementation(),
                capabilities: ServerCapabilities::builder().build(),
                instructions: Some(branding::bridge::DESCRIPTION.to_string()),
                protocol_version: ProtocolVersion::LATEST,
            }
        }
    }
}

/// Bridge client forwards upstream notifications into shared state.
#[derive(Clone)]
struct BridgeClient {
    notifications: Arc<BridgeNotifications>,
}

impl BridgeClient {
    fn new(notifications: Arc<BridgeNotifications>) -> Self {
        Self { notifications }
    }
}

impl ClientHandler for BridgeClient {
    fn get_info(&self) -> ClientInfo {
        let appid = std::env::var("APPID").unwrap_or_default();
        ClientInfo {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: ClientCapabilities::default(),
            client_info: branding::bridge::create_client_implementation(&appid),
        }
    }

    async fn on_tool_list_changed(
        &self,
        _context: NotificationContext<RoleClient>,
    ) {
        self.notifications.mark_tool_list_changed().await;
    }

    async fn on_resource_list_changed(
        &self,
        _context: NotificationContext<RoleClient>,
    ) {
        self.notifications.mark_resource_list_changed().await;
    }

    async fn on_prompt_list_changed(
        &self,
        _context: NotificationContext<RoleClient>,
    ) {
        self.notifications.mark_prompt_list_changed().await;
    }

    async fn on_resource_updated(
        &self,
        params: rmcp::model::ResourceUpdatedNotificationParam,
        _context: NotificationContext<RoleClient>,
    ) {
        self.notifications.push_resource_update(params).await;
    }

    async fn on_progress(
        &self,
        params: rmcp::model::ProgressNotificationParam,
        _context: NotificationContext<RoleClient>,
    ) {
        self.notifications.push_progress(params).await;
    }

    async fn on_cancelled(
        &self,
        params: rmcp::model::CancelledNotificationParam,
        _context: NotificationContext<RoleClient>,
    ) {
        self.notifications.push_cancelled(params).await;
    }

    async fn on_logging_message(
        &self,
        params: rmcp::model::LoggingMessageNotificationParam,
        _context: NotificationContext<RoleClient>,
    ) {
        self.notifications.push_logging(params).await;
    }
}

/// The bridge server simply proxies upstream capabilities.
#[allow(clippy::manual_async_fn)]
impl ServerHandler for BridgeServer {
    fn get_info(&self) -> ServerInfo {
        self.downstream_server_info()
    }

    fn initialize(
        &self,
        request: InitializeRequestParam,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<InitializeResult, McpError>> + Send + '_ {
        async move {
            if context.peer.peer_info().is_none() {
                context.peer.set_peer_info(request);
            }
            self.ensure_runtime().await?;
            Ok(self.downstream_server_info())
        }
    }

    fn list_prompts(
        &self,
        request: Option<PaginatedRequestParam>,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListPromptsResult, McpError>> + Send + '_ {
        async move {
            self.forward_request(&context, move |service| {
                let req = request;
                async move { service.list_prompts(req).await }
            })
            .await
        }
    }

    fn list_tools(
        &self,
        request: Option<PaginatedRequestParam>,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListToolsResult, McpError>> + Send + '_ {
        async move {
            self.forward_request(&context, move |service| {
                let req = request;
                async move { service.list_tools(req).await }
            })
            .await
        }
    }

    fn call_tool(
        &self,
        request: CallToolRequestParam,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<CallToolResult, McpError>> + Send + '_ {
        async move {
            self.forward_request(&context, move |service| {
                let req = request;
                async move { service.call_tool(req).await }
            })
            .await
        }
    }

    fn list_resources(
        &self,
        request: Option<PaginatedRequestParam>,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListResourcesResult, McpError>> + Send + '_ {
        async move {
            self.forward_request(&context, move |service| {
                let req = request;
                async move { service.list_resources(req).await }
            })
            .await
        }
    }

    fn list_resource_templates(
        &self,
        request: Option<PaginatedRequestParam>,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListResourceTemplatesResult, McpError>> + Send + '_ {
        async move {
            self.forward_request(&context, move |service| {
                let req = request;
                async move { service.list_resource_templates(req).await }
            })
            .await
        }
    }

    fn read_resource(
        &self,
        request: ReadResourceRequestParam,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ReadResourceResult, McpError>> + Send + '_ {
        async move {
            self.forward_request(&context, move |service| {
                let req = request;
                async move { service.read_resource(req).await }
            })
            .await
        }
    }

    fn subscribe(
        &self,
        request: SubscribeRequestParam,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<(), McpError>> + Send + '_ {
        async move {
            self.forward_request(&context, move |service| {
                let req = request;
                async move { service.subscribe(req).await }
            })
            .await
        }
    }

    fn unsubscribe(
        &self,
        request: UnsubscribeRequestParam,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<(), McpError>> + Send + '_ {
        async move {
            self.forward_request(&context, move |service| {
                let req = request;
                async move { service.unsubscribe(req).await }
            })
            .await
        }
    }

    fn get_prompt(
        &self,
        request: GetPromptRequestParam,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<GetPromptResult, McpError>> + Send + '_ {
        async move {
            self.forward_request(&context, move |service| {
                let req = request;
                async move { service.get_prompt(req).await }
            })
            .await
        }
    }

    fn complete(
        &self,
        request: CompleteRequestParam,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<CompleteResult, McpError>> + Send + '_ {
        async move {
            self.forward_request(&context, move |service| {
                let req = request;
                async move { service.complete(req).await }
            })
            .await
        }
    }

    fn set_level(
        &self,
        request: SetLevelRequestParam,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<(), McpError>> + Send + '_ {
        async move {
            self.forward_request(&context, move |service| {
                let req = request;
                async move { service.set_level(req).await }
            })
            .await
        }
    }

    fn on_cancelled(
        &self,
        notification: CancelledNotificationParam,
        _context: NotificationContext<RoleServer>,
    ) -> impl Future<Output = ()> + Send + '_ {
        async move {
            self.forward_notification(move |service| {
                let param = notification;
                async move { service.notify_cancelled(param).await }
            })
            .await;
        }
    }

    fn on_progress(
        &self,
        notification: rmcp::model::ProgressNotificationParam,
        _context: NotificationContext<RoleServer>,
    ) -> impl Future<Output = ()> + Send + '_ {
        async move {
            self.forward_notification(move |service| {
                let param = notification;
                async move { service.notify_progress(param).await }
            })
            .await;
        }
    }

    fn on_initialized(
        &self,
        _context: NotificationContext<RoleServer>,
    ) -> impl Future<Output = ()> + Send + '_ {
        async move {
            self.forward_notification(|service| async move { service.notify_initialized().await })
                .await;
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            EnvFilter::from_default_env().add_directive(args.log_level.parse().unwrap_or(tracing::Level::INFO.into())),
        )
        .init();

    tracing::info!("Starting stdio ↔ MCP bridge");
    tracing::info!("Using protocol version: {}", ProtocolVersion::LATEST);

    let notifications = Arc::new(BridgeNotifications::default());
    let runtime = Arc::new(BridgeRuntimeStore::default());
    let server_info = Arc::new(RwLock::new(None));

    let bearer_from_env = std::env::var("MCPMATE_UPSTREAM_BEARER").ok();
    let bridge_server = BridgeServer::new(
        args.upstream_url.clone(),
        args.upstream_bearer.clone().or(bearer_from_env),
        notifications,
        runtime,
        server_info,
    );

    tracing::info!("Connecting to upstream MCP server at {}", args.upstream_url);
    if let Err(err) = bridge_server.establish_upstream().await {
        tracing::error!("Failed to connect to upstream MCP server: {}", err);
        tracing::warn!("Bridge will start but report upstream service as unavailable until reconnection succeeds");
    }

    let stdio_transport = io::stdio();
    tracing::info!("Created stdio transport");

    tracing::info!("Initializing stdio server...");
    let server = match serve_server(bridge_server, stdio_transport).await {
        Ok(server) => {
            tracing::info!("Successfully initialized stdio server");
            server
        }
        Err(err) => {
            tracing::error!("Failed to initialize stdio server: {}", err);
            return Err(anyhow::anyhow!("Failed to initialize stdio server: {}", err));
        }
    };

    tracing::info!("Bridge is running. Waiting for stdio server to exit...");
    match server.waiting().await {
        Ok(_) => tracing::info!("Stdio server exited normally"),
        Err(err) => tracing::error!("Stdio server exited with error: {}", err),
    }

    tracing::info!("Bridge shut down");
    Ok(())
}
