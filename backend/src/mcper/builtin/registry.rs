use anyhow::Result;
use rmcp::model::{CallToolRequestParams, CallToolResult, GetPromptRequestParams, GetPromptResult, Prompt, Tool};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::clients::service::ClientConfigService;
use crate::config::database::Database;
use crate::core::pool::UpstreamConnectionPool;

use super::client::ClientBuiltinContext;
use super::{BrokerService, ClientService, ProfileService};

/// Trait for built-in MCP services that convert API capabilities
#[async_trait::async_trait]
pub trait BuiltinService: Send + Sync {
    fn name(&self) -> &'static str;

    fn tools(&self) -> Vec<Tool>;

    fn prompts(&self) -> Vec<Prompt> {
        Vec::new()
    }

    async fn call_tool(
        &self,
        request: &CallToolRequestParams,
    ) -> Result<CallToolResult>;

    async fn get_prompt(
        &self,
        _request: &GetPromptRequestParams,
    ) -> Result<GetPromptResult> {
        Err(anyhow::anyhow!("Prompt not supported by builtin service"))
    }

    async fn call_tool_with_context(
        &self,
        request: &CallToolRequestParams,
        context: Option<&ClientBuiltinContext>,
    ) -> Result<CallToolResult> {
        let _ = context;
        self.call_tool(request).await
    }

    async fn get_prompt_with_context(
        &self,
        request: &GetPromptRequestParams,
        context: Option<&ClientBuiltinContext>,
    ) -> Result<GetPromptResult> {
        let _ = context;
        self.get_prompt(request).await
    }
}

/// Registry for managing built-in services
#[derive(Default)]
pub struct BuiltinServiceRegistry {
    services: Vec<Arc<dyn BuiltinService>>,
}

impl std::fmt::Debug for BuiltinServiceRegistry {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("BuiltinServiceRegistry")
            .field("services", &format!("{} services", self.services.len()))
            .finish()
    }
}

impl BuiltinServiceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_service(
        &mut self,
        service: Arc<dyn BuiltinService>,
    ) {
        self.services.push(service);
    }

    pub fn tools(&self) -> Vec<Tool> {
        self.services.iter().flat_map(|service| service.tools()).collect()
    }

    pub fn prompts(&self) -> Vec<Prompt> {
        self.services.iter().flat_map(|service| service.prompts()).collect()
    }

    pub async fn call_tool(
        &self,
        request: &CallToolRequestParams,
    ) -> Option<Result<CallToolResult>> {
        for service in &self.services {
            let tools = service.tools();
            if tools.iter().any(|t| t.name == request.name) {
                return Some(service.call_tool(request).await);
            }
        }
        None
    }

    pub async fn call_tool_with_context(
        &self,
        request: &CallToolRequestParams,
        context: Option<&ClientBuiltinContext>,
    ) -> Option<Result<CallToolResult>> {
        for service in &self.services {
            let tools = service.tools();
            if tools.iter().any(|t| t.name == request.name) {
                return Some(service.call_tool_with_context(request, context).await);
            }
        }
        None
    }

    pub async fn get_prompt(
        &self,
        request: &GetPromptRequestParams,
    ) -> Option<Result<GetPromptResult>> {
        for service in &self.services {
            let prompts = service.prompts();
            if prompts.iter().any(|prompt| prompt.name == request.name) {
                return Some(service.get_prompt(request).await);
            }
        }
        None
    }

    pub async fn get_prompt_with_context(
        &self,
        request: &GetPromptRequestParams,
        context: Option<&ClientBuiltinContext>,
    ) -> Option<Result<GetPromptResult>> {
        for service in &self.services {
            let prompts = service.prompts();
            if prompts.iter().any(|prompt| prompt.name == request.name) {
                return Some(service.get_prompt_with_context(request, context).await);
            }
        }
        None
    }

    pub fn with_mcpmate_services(
        mut self,
        database: Arc<Database>,
        connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
        client_config_service: Arc<ClientConfigService>,
    ) -> Self {
        let profile_service = Arc::new(ProfileService::new(database.clone(), connection_pool.clone()));
        let client_service = Arc::new(ClientService::new(
            database.clone(),
            connection_pool.clone(),
            client_config_service,
        ));
        let broker_service = Arc::new(BrokerService::new(database, connection_pool));
        self.add_service(profile_service);
        self.add_service(client_service);
        self.add_service(broker_service);
        self
    }
}
