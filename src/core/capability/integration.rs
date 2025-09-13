//! Integration adapter - Integrate unified query service into existing architecture

use std::sync::Arc;
use crate::core::cache::RedbCacheManager;
use crate::core::pool::UpstreamConnectionPool;
use crate::config::database::Database;
use crate::api::routes::AppState;
use crate::api::handlers::server::common::InspectParams;

use super::query::{UnifiedQueryService, UnifiedQueryServiceBuilder};
use super::domain::{QueryContext, CapabilityResult, CapabilityError, CapabilityType};
use super::UnifiedConnectionManager;

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
        // Create connection manager for unified entry point
        let config = Arc::new(crate::core::models::Config::default());
        let connection_manager = Arc::new(UnifiedConnectionManager::new(pool.clone(), config));
        
        let service = UnifiedQueryServiceBuilder::new()
            .with_cache(cache)
            .with_pool(pool)
            .with_database(database)
            .with_app_state(app_state)
            .with_connection_manager(connection_manager)
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
        self.inner.query_capabilities(server_id, capability_type, params, QueryContext::ApiCall).await
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
        let legacy_count = legacy_result.data.as_ref().map_or(0, |data| data.data.len());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_comparison() {
        let comparison = migration::MigrationComparison {
            item_count_match: true,
            legacy_count: 5,
            unified_count: 5,
            sources_different: false,
        };
        
        assert!(comparison.is_compatible());
        
        let incompatible = migration::MigrationComparison {
            item_count_match: false,
            legacy_count: 5,
            unified_count: 3,
            sources_different: true,
        };
        
        assert!(!incompatible.is_compatible());
    }
}