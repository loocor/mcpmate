//! Shared capability query helper for token estimate and capability ledger handlers.

use crate::{
    api::{handlers::server::common::InspectParams, routes::AppState},
    core::capability::{CapabilityItem, CapabilityType, query},
};

pub async fn query_unified_capabilities(
    state: &AppState,
    server_id: &str,
    capability_type: CapabilityType,
    params: &InspectParams,
) -> Option<Vec<CapabilityItem>> {
    let database = state.database.as_ref()?;
    match query::query_capabilities(
        state.connection_pool.clone(),
        database.clone(),
        server_id,
        capability_type,
        params,
    )
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
