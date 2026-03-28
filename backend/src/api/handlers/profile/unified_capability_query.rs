//! Shared unified-query helper for token estimate and capability ledger handlers.

use crate::{
    api::handlers::server::common::InspectParams,
    core::capability::{CapabilityItem, CapabilityType, UnifiedQueryAdapter},
};

pub async fn query_unified_capabilities(
    unified_query: &UnifiedQueryAdapter,
    server_id: &str,
    capability_type: CapabilityType,
    params: &InspectParams,
) -> Option<Vec<CapabilityItem>> {
    match unified_query
        .query_capabilities(server_id, capability_type, params)
        .await
    {
        Ok(result) => Some(result.items),
        Err(error) => {
            tracing::warn!(
                server_id = %server_id,
                capability_type = ?capability_type,
                error = %error,
                "Failed to query unified capabilities for token metrics"
            );
            None
        }
    }
}
