use anyhow::Result;
use rmcp::model::{CallToolRequestParam, CallToolResult, Tool};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::config::database::Database;
use crate::core::pool::UpstreamConnectionPool;

use super::SuitsService;

/// Trait for built-in MCP services that convert API capabilities
#[async_trait::async_trait]
pub trait BuiltinService: Send + Sync {
    fn name(&self) -> &'static str;

    fn tools(&self) -> Vec<Tool>;

    async fn call_tool(
        &self,
        request: &CallToolRequestParam,
    ) -> Result<CallToolResult>;
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

    pub async fn call_tool(
        &self,
        request: &CallToolRequestParam,
    ) -> Option<Result<CallToolResult>> {
        for service in &self.services {
            let tools = service.tools();
            if tools.iter().any(|t| t.name == request.name) {
                return Some(service.call_tool(request).await);
            }
        }
        None
    }

    pub fn with_mcpmate_services(
        mut self,
        database: Arc<Database>,
        connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
    ) -> Self {
        let suits_service = Arc::new(SuitsService::new(database, connection_pool));
        self.add_service(suits_service);
        self
    }
}
