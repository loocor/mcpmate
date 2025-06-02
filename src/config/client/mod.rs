// Client management module for MCPMate
// Provides client application detection, configuration generation, and management
// Integrates with existing system/detection module

pub mod generator;
pub mod init;
pub mod manager;
pub mod models;

pub use generator::ConfigGenerator;
pub use init::initialize_client_apps;
pub use manager::ClientManager;
pub use models::*;

// Re-export the unified structures for external use
pub use models::{
    ClientConfigFile, ClientDefinition, ConfigRulesDefinition, DetectionRuleDefinition,
    load_client_config,
};
