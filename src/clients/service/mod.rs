// Service module index for client configuration management
// This file intentionally contains no business logic.

pub mod apply;
pub mod backup;
pub mod core;
pub mod list;
pub mod query;
pub mod settings;
pub mod state;

// Re-exports for external callers
pub use core::{
    ApplyOutcome, ClientBackupRecord, ClientConfigService, ClientDescriptor, ClientRenderOptions, ClientRenderResult,
    PreviewOutcome,
};
