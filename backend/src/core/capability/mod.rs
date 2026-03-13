//! Unified capability management module
//!
//! Complete MCP capability management including tools, resources, prompts, and unified query services.
//! Integrates the former protocol layer for a cohesive capability-centric architecture.

pub mod domain;
pub mod facade;
pub mod integration;
mod internal;
pub mod naming;
pub mod prompts;
pub mod query;
pub mod resolver;
pub mod resources;
pub mod runtime;
pub mod service;
pub mod tools;

pub use domain::{
    AffinityKey, CapabilityError, CapabilityItem, CapabilityQuery, CapabilityResult, CapabilityType, ConnectionMode,
    DataSource, FreshnessRequirement, IsolationMode, QueryContext,
};
pub use integration::{UnifiedQueryAdapter, UnifiedQueryIntegration, migration::MigrationComparison};
pub use query::{MetricsCollector, UnifiedQueryService, UnifiedQueryServiceBuilder};
pub use service::CapabilityService;
