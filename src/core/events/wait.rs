//! Event waiting utilities

use std::time::Duration;

use anyhow::Result;
use tokio::time::timeout;
use tracing::debug;

use super::{Event, EventBus};
use crate::{common::server::ServerType, core::transport::TransportType};

/// Wait for the transport layer of a specific server type to be ready
///
/// This function will wait for a ready event for the specified transport type,
/// or return a timeout error if the event does not occur within the specified timeout.
///
/// # Parameters
/// * `transport_type` - The type of transport layer to wait for
/// * `timeout_ms` - The timeout in milliseconds
///
/// # Returns
/// * `Result<()>` - Success or timeout error
///
/// This function will wait for a ready event for the specified transport type,
/// or return a timeout error if the event does not occur within the specified timeout.
/// If the event occurs within the timeout, it returns Ok(()), otherwise it returns a timeout error.
///
/// # Parameters
/// * `transport_type` - The type of transport layer to wait for
/// * `timeout_ms` - The timeout in milliseconds
///
/// # Returns
/// * `Result<()>` - Success or timeout error
pub async fn wait_for_transport_ready(
    transport_type: TransportType,
    timeout_ms: u64,
) -> Result<()> {
    debug!(
        "Waiting for {:?} transport layer to be ready, timeout {}ms",
        transport_type, timeout_ms
    );

    // Create an async event receiver
    let mut rx = EventBus::global().subscribe_async();

    // Use timeout mechanism to wait for the event
    let wait_future = async {
        loop {
            match rx.recv().await {
                Ok(Event::ServerTransportReady {
                    transport_type: event_type,
                    ready,
                }) => {
                    if event_type == transport_type && ready {
                        debug!("Received {:?} transport layer ready event", transport_type);
                        return Ok(());
                    }
                }
                Ok(_) => {
                    // Ignore other types of events
                    continue;
                }
                Err(e) => {
                    // Broadcast channel error
                    return Err(anyhow::anyhow!("Event receiver error: {}", e));
                }
            }
        }
    };

    // Add timeout
    let timeout_duration = Duration::from_millis(timeout_ms);
    match timeout(timeout_duration, wait_future).await {
        Ok(result) => result,
        Err(_) => {
            debug!(
                "Waiting for {:?} transport layer to be ready timeout ({}ms)",
                transport_type, timeout_ms
            );
            // Timeout is not an error, it's just normal control flow
            // We should continue to try to connect
            Ok(())
        }
    }
}

/// Check if the specified server type needs to wait for the transport layer to be ready
///
/// # Parameters
/// * `server_type` - The server type enum
/// * `transport_type` - The transport type
///
/// # Returns
/// * `bool` - If the server type needs to wait for the transport layer to be ready, return true
pub fn needs_transport_ready_wait(
    server_type: ServerType,
    transport_type: TransportType,
) -> bool {
    match (server_type, transport_type) {
        // SSE and StreamableHttp type servers need to wait for the transport layer to be ready
        (ServerType::Sse, TransportType::Sse) => true,
        (ServerType::StreamableHttp, TransportType::StreamableHttp) => true,
        // Other types do not need to wait
        _ => false,
    }
}
