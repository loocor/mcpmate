//! Resource call module
//!
//! Contains functions for handling resource read requests

use anyhow::{Context, Result};
use rmcp::model::{ReadResourceRequestParam, ReadResourceResult};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use tracing;

use crate::core::capability::resources::types::ResourceMapping;
use crate::core::pool::UpstreamConnectionPool;

/// Read a resource from the appropriate upstream server
///
/// This function takes a resource URI and routes the read request to the correct
/// upstream server based on the resource mapping. It handles both text and binary
/// resource content according to the MCP specification.
pub async fn read_upstream_resource(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>,
    resource_mapping: &HashMap<String, ResourceMapping>,
    uri: &str,
) -> Result<ReadResourceResult> {
    tracing::debug!("Reading resource: {}", uri);

    // Find the mapping for this resource URI
    let mapping = resource_mapping
        .get(uri)
        .ok_or_else(|| anyhow::anyhow!("Resource not found: {}", uri))?;

    tracing::debug!(
        "Routing resource read request for '{}' to instance {} (server: {})",
        uri,
        mapping.instance_id,
        mapping.server_name
    );

    // CRITICAL FIX: Get service reference without holding pool lock during network call
    let (service, upstream_resource_uri) = {
        let pool = connection_pool.lock().await;
        let server_id = mapping
            .server_id
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Server ID not available for resource {}", uri))?;
        let instances = pool
            .connections
            .get(server_id)
            .ok_or_else(|| anyhow::anyhow!("Server {} not found for resource {}", server_id, uri))?;

        let conn = instances
            .get(&mapping.instance_id)
            .ok_or_else(|| anyhow::anyhow!("Instance {} not found for resource {}", mapping.instance_id, uri))?;

        // Check if the connection has a service
        let service = match &conn.service {
            Some(service) => service.clone(),
            None => {
                return Err(anyhow::anyhow!(
                    "No service available for instance {} (server: {})",
                    mapping.instance_id,
                    mapping.server_name
                ));
            }
        };

        (service, mapping.upstream_resource_uri.clone())
        // Pool lock is automatically dropped here
    };

    // Now make the network call WITHOUT holding the pool lock
    let result = service
        .read_resource(ReadResourceRequestParam {
            uri: upstream_resource_uri,
        })
        .await
        .context("Failed to read resource from upstream server")?;

    tracing::debug!(
        "Successfully read resource '{}' from server '{}', got {} content items",
        uri,
        mapping.server_name,
        result.contents.len()
    );

    // Log content types for debugging
    for (i, content) in result.contents.iter().enumerate() {
        match content {
            rmcp::model::ResourceContents::TextResourceContents { mime_type, text, .. } => {
                tracing::debug!(
                    "Content {}: text (mime_type: {:?}, length: {} chars)",
                    i,
                    mime_type,
                    text.len()
                );
            }
            rmcp::model::ResourceContents::BlobResourceContents { mime_type, blob, .. } => {
                tracing::debug!(
                    "Content {}: blob (mime_type: {:?}, length: {} bytes base64)",
                    i,
                    mime_type,
                    blob.len()
                );
            }
        }
    }

    Ok(result)
}

/// Validate resource URI format
///
/// This function performs basic validation on resource URIs to ensure they
/// conform to expected formats. It's used for input validation.
pub fn validate_resource_uri(uri: &str) -> Result<()> {
    if uri.is_empty() {
        return Err(anyhow::anyhow!("Resource URI cannot be empty"));
    }

    // Basic URI format validation
    if !uri.contains("://") {
        return Err(anyhow::anyhow!(
            "Resource URI must contain a scheme (e.g., 'file://', 'memory://'): {}",
            uri
        ));
    }

    // Check for common problematic characters
    if uri.contains('\n') || uri.contains('\r') {
        return Err(anyhow::anyhow!(
            "Resource URI cannot contain newline characters: {}",
            uri
        ));
    }

    Ok(())
}
