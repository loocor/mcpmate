#![forbid(unsafe_code)]

mod cache;
mod domain;
mod error;
mod identity;
mod schema;
mod sqlite;
mod surface;
mod surface_store;

pub use cache::{
    DEFAULT_PROJECTION_CAPACITY, DEFAULT_RAW_SNAPSHOT_CAPACITY, DerivedCacheKeyDiagnostic, DerivedCacheMetrics,
    DerivedCapabilityCache, ProjectionEpoch, ProjectionKey, ProjectionNameDomain, ProjectionPayload, RawSnapshotKey,
};
pub use domain::{
    CapabilityFailureObservation, CapabilityKind, CapabilityObservation, CapabilityPayload, CapabilityRefRecord,
    CapabilityRefState, CapabilityVersionChange, CapabilityVersionRecord, CatalogCommit, CatalogDelta,
    CatalogInvalidation, CatalogReconciliation, CatalogRecord, CatalogSnapshot, CatalogStats, DeclarationState,
    InventoryState, KindCompleteness, KindFailureKind, KindObservation, SnapshotState,
};
pub use error::{CatalogError, Result};
pub use identity::{
    BUILTIN_CAPABILITY_SOURCE_ID, CAPABILITY_REF_FORMAT_V1, CapabilityId, CapabilityRefId, CapabilitySourceIdentity,
    EFFECTIVE_CAPABILITY_FORMAT_V1, EffectiveCapabilityDefinition, EffectiveCapabilityRecordV1,
    SURFACE_MANIFEST_FORMAT_V1, SurfaceManifestId,
};
pub use sqlite::{CapabilityCatalog, SqliteCapabilityCatalog};
pub use surface::{
    CapabilityChangeEvent, ConsumerSurfaceBinding, ProposalLifecycle, ReconciliationJobStatus, ReviewLifecycle,
    ReviewOwnerType, ReviewResolutionAction, ReviewTargetKey, RollbackBlock, SurfaceManifest, SurfaceManifestEntry,
    SurfaceManifestEntryInput, SurfaceOutboxEvent, SurfaceProposal, SurfacePublication, SurfaceReconciliationJob,
    SurfaceReviewDecision, SurfaceReviewDecisionDraft, SurfaceReviewFilter, SurfaceReviewItem, SurfaceReviewItemDraft,
    SurfaceReviewOwner, SurfaceReviewRecord,
};
pub use surface_store::SqliteSurfaceStore;

pub const RECORD_FORMAT_VERSION: i64 = 1;
