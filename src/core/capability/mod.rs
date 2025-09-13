//! Unified capability query management module
//!
//! Unified entry point based on existing infrastructure, supporting tools, resources, prompts and other capability queries.

pub mod domain;
pub mod integration;
pub mod query;
pub mod unified;

// main public interfaces
pub use domain::{
    CapabilityError, CapabilityItem, CapabilityQuery, CapabilityResult, CapabilityType, DataSource,
    FreshnessRequirement, QueryContext, AffinityKey, ConnectionMode, IsolationMode,
};

pub use unified::{
    UnifiedConnectionManager, InstanceMetadata, InstanceStatus, ConnectionStats,
};

pub use query::{MetricsCollector, UnifiedQueryService, UnifiedQueryServiceBuilder};

pub use integration::{UnifiedQueryAdapter, UnifiedQueryIntegration, migration::MigrationComparison};
