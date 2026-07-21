#![forbid(unsafe_code)]

mod cache;
mod domain;
mod error;
mod schema;
mod sqlite;

pub use cache::{
    DEFAULT_PROJECTION_CAPACITY, DEFAULT_RAW_SNAPSHOT_CAPACITY, DerivedCacheKeyDiagnostic, DerivedCacheMetrics,
    DerivedCapabilityCache, ProjectionEpoch, ProjectionKey, ProjectionNameDomain, ProjectionPayload, RawSnapshotKey,
};
pub use domain::{
    CapabilityFailureObservation, CapabilityKind, CapabilityObservation, CapabilityPayload, CatalogCommit,
    CatalogInvalidation, CatalogRecord, CatalogSnapshot, CatalogStats, DeclarationState, InventoryState,
    KindObservation, SnapshotState,
};
pub use error::{CatalogError, Result};
pub use sqlite::{CapabilityCatalog, SqliteCapabilityCatalog};

pub const RECORD_FORMAT_VERSION: i64 = 1;
