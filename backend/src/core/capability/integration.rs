//! Integration adapter - Integrate unified query service into existing architecture

use crate::api::handlers::server::common::InspectParams;
use crate::api::routes::AppState;
use crate::config::database::Database;
use crate::core::cache::RedbCacheManager;
use crate::core::pool::UpstreamConnectionPool;
use std::sync::Arc;

use super::domain::{CapabilityError, CapabilityResult, CapabilityType, QueryContext};
use super::query::{UnifiedQueryService, UnifiedQueryServiceBuilder};

/// Unified query adapter - Lightweight wrapper
pub struct UnifiedQueryAdapter {
    inner: Arc<UnifiedQueryService>,
}

impl UnifiedQueryAdapter {
    /// Build adapter from AppState components
    pub fn from_components(
        cache: Arc<RedbCacheManager>,
        pool: Arc<tokio::sync::Mutex<UpstreamConnectionPool>>,
        database: Arc<Database>,
        app_state: Arc<AppState>,
    ) -> Self {
        let service = UnifiedQueryServiceBuilder::new()
            .with_cache(cache)
            .with_pool(pool)
            .with_database(database)
            .with_app_state(app_state)
            .build()
            .expect("Failed to build UnifiedQueryService");

        Self {
            inner: Arc::new(service),
        }
    }

    /// Query capabilities - simplified interface, compatible with existing code
    pub async fn query_capabilities(
        &self,
        server_id: &str,
        capability_type: CapabilityType,
        params: &InspectParams,
    ) -> Result<CapabilityResult, CapabilityError> {
        // Default to API call scenario
        self.inner
            .query_capabilities(server_id, capability_type, params, QueryContext::ApiCall)
            .await
    }

    /// Get internal service (for advanced usage)
    pub fn inner(&self) -> &Arc<UnifiedQueryService> {
        &self.inner
    }
}

/// Unified query function integration helper
pub struct UnifiedQueryIntegration;

impl UnifiedQueryIntegration {
    /// Check if unified query can be integrated
    pub fn is_ready(app_state: &AppState) -> bool {
        app_state.database.is_some()
    }

    /// Create unified query adapter
    pub fn create_adapter(app_state: &AppState) -> Option<Arc<UnifiedQueryAdapter>> {
        if !Self::is_ready(app_state) {
            return None;
        }

        let cache = app_state.redb_cache.clone();
        let pool = app_state.connection_pool.clone();
        let database = app_state.database.clone()?;
        let app_state_arc = Arc::new(app_state.clone());

        Some(Arc::new(UnifiedQueryAdapter::from_components(
            cache,
            pool,
            database,
            app_state_arc,
        )))
    }
}

/// Migration helper tools
pub mod migration {
    use super::*;

    /// Compare traditional query and unified query results
    pub fn compare_results(
        legacy_result: &crate::api::models::server::ServerToolsResp,
        unified_result: &CapabilityResult,
    ) -> MigrationComparison {
        let legacy_count = legacy_result.data.as_ref().map_or(0, |data| data.items.len());
        let unified_count = unified_result.items.len();

        MigrationComparison {
            item_count_match: legacy_count == unified_count,
            legacy_count,
            unified_count,
            sources_different: true,
        }
    }

    /// Migration comparison result
    #[derive(Debug, Clone)]
    pub struct MigrationComparison {
        pub item_count_match: bool,
        pub legacy_count: usize,
        pub unified_count: usize,
        pub sources_different: bool,
    }

    impl MigrationComparison {
        pub fn is_compatible(&self) -> bool {
            self.item_count_match
        }
    }
}
