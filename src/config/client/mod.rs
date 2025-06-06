// Client management module for MCPMate
// Provides client application detection, configuration generation, and management
// Integrates with existing system/detection module

pub mod builder;
pub mod generator;
pub mod init;
pub mod loader;
pub mod manager;
pub mod models;
pub mod strategy;
pub mod template;
pub mod utils;

pub use builder::ConfigBuilder;
pub use generator::ConfigGenerator;
pub use init::initialize_client_apps;
pub use loader::{ServerInfo, ServerLoader};
pub use manager::ClientManager;
pub use models::*;
pub use strategy::TransportStrategy;
pub use template::TemplateEngine;

// Re-export the unified structures for external use
pub use models::{
    ClientConfigFile, ClientDefinition, ConfigRulesDefinition, DetectionRuleDefinition,
    load_client_config,
};
