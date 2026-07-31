use anyhow::Result;
use rmcp::model::{CallToolRequestParams, CallToolResult, GetPromptRequestParams, GetPromptResult, Prompt, Tool};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::clients::service::ClientConfigService;
use crate::config::database::Database;
use crate::core::pool::UpstreamConnectionPool;
use mcpmate_capability_store::{BUILTIN_CAPABILITY_SOURCE_ID, CapabilityPayload, CatalogRecord};

use super::client::ClientBuiltinContext;
use super::metadata::{with_builtin_prompt_title, with_builtin_tool_title};
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
        self.services
            .iter()
            .flat_map(|service| service.tools())
            .map(with_builtin_tool_title)
            .collect()
    }

    pub fn prompts(&self) -> Vec<Prompt> {
        self.services
            .iter()
            .flat_map(|service| service.prompts())
            .map(with_builtin_prompt_title)
            .collect()
    }

    pub fn catalog_records(&self) -> mcpmate_capability_store::Result<Vec<CatalogRecord>> {
        let mut records = self
            .tools()
            .into_iter()
            .map(|tool| materialize_builtin_catalog_record(CapabilityPayload::Tool(tool)))
            .chain(
                self.prompts()
                    .into_iter()
                    .map(|prompt| materialize_builtin_catalog_record(CapabilityPayload::Prompt(prompt))),
            )
            .collect::<mcpmate_capability_store::Result<Vec<_>>>()?;
        records.sort_by(|left, right| {
            left.kind()
                .as_str()
                .cmp(right.kind().as_str())
                .then(left.upstream_key.cmp(&right.upstream_key))
        });
        Ok(records)
    }

    pub async fn call_tool(
        &self,
        request: &CallToolRequestParams,
    ) -> Option<Result<CallToolResult>> {
        self.call_tool_with_context(request, None).await
    }

    pub async fn call_tool_with_context(
        &self,
        request: &CallToolRequestParams,
        context: Option<&ClientBuiltinContext>,
    ) -> Option<Result<CallToolResult>> {
        let service = self.find_tool_service(request.name.as_ref())?;
        Some(service.call_tool_with_context(request, context).await)
    }

    pub async fn get_prompt(
        &self,
        request: &GetPromptRequestParams,
    ) -> Option<Result<GetPromptResult>> {
        self.get_prompt_with_context(request, None).await
    }

    pub async fn get_prompt_with_context(
        &self,
        request: &GetPromptRequestParams,
        context: Option<&ClientBuiltinContext>,
    ) -> Option<Result<GetPromptResult>> {
        let service = self.find_prompt_service(&request.name)?;
        Some(service.get_prompt_with_context(request, context).await)
    }

    fn find_tool_service(
        &self,
        tool_name: &str,
    ) -> Option<&Arc<dyn BuiltinService>> {
        self.services
            .iter()
            .find(|service| service.tools().iter().any(|tool| tool.name.as_ref() == tool_name))
    }

    fn find_prompt_service(
        &self,
        prompt_name: &str,
    ) -> Option<&Arc<dyn BuiltinService>> {
        self.services
            .iter()
            .find(|service| service.prompts().iter().any(|prompt| prompt.name == prompt_name))
    }

    pub fn with_mcpmate_services(
        mut self,
        database: Arc<Database>,
        connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
        client_config_service: Arc<ClientConfigService>,
    ) -> Self {
        self.add_service(Arc::new(ProfileService::new(database.clone(), connection_pool.clone())));
        self.add_service(Arc::new(ClientService::new(
            database.clone(),
            connection_pool.clone(),
            client_config_service,
        )));
        self.add_service(Arc::new(BrokerService::new(database, connection_pool)));
        self
    }
}

fn materialize_builtin_catalog_record(payload: CapabilityPayload) -> mcpmate_capability_store::Result<CatalogRecord> {
    let origin_key = payload.origin_key().to_string();
    CatalogRecord::materialize(BUILTIN_CAPABILITY_SOURCE_ID, &origin_key, &origin_key, payload)
}
