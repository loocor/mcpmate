// Application detection module
// Handles automatic detection of MCP clientlications

pub mod detector;
pub mod models;
pub mod platform;

pub use detector::AppDetector;
pub use models::{Client, DetectedApp, DetectionMethod, DetectionResult, DetectionRule};
