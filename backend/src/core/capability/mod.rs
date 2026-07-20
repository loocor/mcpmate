//! Unified capability management module
//!
//! Complete MCP capability management including tools, resources, prompts, and unified query services.
//! Integrates the former protocol layer for a cohesive capability-centric architecture.

pub(crate) mod aggregate;
pub(crate) mod connection_provider;
pub mod descriptions;
pub mod domain;
pub mod facade;
pub mod index;
pub mod integration;
mod internal;
pub mod naming;
pub mod prompts;
pub mod query;
pub(crate) mod read_service;
pub mod resolver;
pub(crate) mod resource_registry;
pub(crate) mod resource_uri;
pub mod resources;
pub mod runtime;
pub mod service;
pub mod tools;

pub use domain::{
    AffinityKey, CapabilityError, CapabilityItem, CapabilityQuery, CapabilityResult, CapabilityType, ConnectionMode,
    ConnectionSelection, DataSource, FreshnessRequirement, IsolationMode, QueryContext, RuntimeIdentity,
};
pub use integration::{UnifiedQueryAdapter, UnifiedQueryIntegration, migration::MigrationComparison};
pub use query::{MetricsCollector, UnifiedQueryService, UnifiedQueryServiceBuilder};
pub use service::CapabilityService;
