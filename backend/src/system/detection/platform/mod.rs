// Platform-specific detection implementations

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "macos")]
pub use macos::MacOSDetector as PlatformDetector;

#[cfg(not(target_os = "macos"))]
pub mod generic;

#[cfg(not(target_os = "macos"))]
pub use generic::GenericDetector as PlatformDetector;
