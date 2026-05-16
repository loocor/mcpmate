//! MCPMate Runtime Module
//!
//! Simplified runtime management with file-system based detection.
//! Provides unified runtime management through RuntimeManager.

pub mod detection; // Runtime availability detection (scoped to spawn PATH)
pub mod downloader; // Simplified downloader
pub mod installer; // Simplified installer
pub mod manager; // Unified runtime manager
pub mod resolver; // Unified command resolution (managed → system PATH)

// Re-export common types from common::env and common::types
pub use crate::common::env::{Architecture, Environment, OperatingSystem, detect_environment};
pub use crate::common::{RuntimeError, RuntimeType};

// Re-export core runtime services
pub use crate::runtime::{
    detection::{RuntimeDetection, RuntimeDetector, RuntimeProbe},
    downloader::RuntimeDownloader,
    installer::RuntimeInstaller,
    manager::{RuntimeInfo, RuntimeManager},
    resolver::{CommandResolver, ResolveSource, ResolvedCommand},
};
