// Platform-specific detection implementations

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "macos")]
pub use macos::MacOSDetector as PlatformDetector;

// #[cfg(target_os = "windows")]
// pub mod windows;

// #[cfg(target_os = "windows")]
// pub use windows::WindowsDetector as PlatformDetector;

// #[cfg(target_os = "linux")]
// pub mod linux;

// #[cfg(target_os = "linux")]
// pub use linux::LinuxDetector as PlatformDetector;
