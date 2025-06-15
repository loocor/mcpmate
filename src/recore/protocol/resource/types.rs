//! Resource protocol types
//!
//! Contains type definitions for resource mapping and related functionality

use rmcp::model::{Resource, ResourceTemplate};

/// Resource mapping information
///
/// This struct represents the mapping between a resource URI and the server/instance
/// that provides it. It is used to route resource requests to the appropriate upstream server.
/// Unlike tools, resources use URI as unique identifier, so no complex naming management is needed.
#[derive(Debug, Clone)]
pub struct ResourceMapping {
    /// Name of the server that provides this resource
    pub server_name: String,
    /// ID of the instance that provides this resource
    pub instance_id: String,
    /// Original resource definition
    pub resource: Resource,
    /// Original upstream resource URI (without any modifications)
    pub upstream_resource_uri: String,
}

/// Resource template mapping information
///
/// This struct represents the mapping between a resource template and the server/instance
/// that provides it. Used for resources/templates/list aggregation.
#[derive(Debug, Clone)]
pub struct ResourceTemplateMapping {
    /// Name of the server that provides this resource template
    pub server_name: String,
    /// ID of the instance that provides this resource template
    pub instance_id: String,
    /// Original resource template definition
    pub resource_template: ResourceTemplate,
}
