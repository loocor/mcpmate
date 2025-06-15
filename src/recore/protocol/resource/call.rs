//! Resource call module
//!
//! Contains functions for handling resource read requests

use anyhow::{Context, Result};
use rmcp::model::{ReadResourceRequestParam, ReadResourceResult};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use tracing;

use crate::recore::pool::UpstreamConnectionPool;
use crate::recore::protocol::resource::types::ResourceMapping;

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

    // Get the connection from the pool
    let pool = connection_pool.lock().await;
    let instances = pool.connections.get(&mapping.server_name).ok_or_else(|| {
        anyhow::anyhow!(
            "Server {} not found for resource {}",
            mapping.server_name,
            uri
        )
    })?;

    let conn = instances.get(&mapping.instance_id).ok_or_else(|| {
        anyhow::anyhow!(
            "Instance {} not found for resource {}",
            mapping.instance_id,
            uri
        )
    })?;

    // Check if the connection has a service
    let service = match &conn.service {
        Some(service) => service,
        None => {
            return Err(anyhow::anyhow!(
                "No service available for instance {} (server: {})",
                mapping.instance_id,
                mapping.server_name
            ));
        }
    };

    // Forward the read request to the upstream server
    let result = service
        .read_resource(ReadResourceRequestParam {
            uri: mapping.upstream_resource_uri.clone(),
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
            rmcp::model::ResourceContents::TextResourceContents {
                mime_type, text, ..
            } => {
                tracing::debug!(
                    "Content {}: text (mime_type: {:?}, length: {} chars)",
                    i,
                    mime_type,
                    text.len()
                );
            }
            rmcp::model::ResourceContents::BlobResourceContents {
                mime_type, blob, ..
            } => {
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
