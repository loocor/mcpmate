// Platform-specific path resolution

#[cfg(target_os = "macos")]
pub mod macos;

// Windows/Linux implementations are temporarily disabled until platform support lands.
// #[cfg(target_os = "windows")]
// pub mod windows;

// #[cfg(target_os = "linux")]
// pub mod linux;
