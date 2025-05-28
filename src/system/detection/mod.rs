// Application detection module
// Handles automatic detection of MCP client applications

pub mod detector;
pub mod models;
pub mod platform;

pub use detector::AppDetector;
pub use models::{ClientApp, DetectedApp, DetectionRule, DetectionMethod, DetectionResult};
