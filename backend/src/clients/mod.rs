pub mod adapters;
pub mod analyzer;
pub mod detector;
pub mod engine;
pub mod error;
pub mod models;
pub mod renderer;
pub mod service;
pub mod source;
pub mod storage;
pub mod utils;

pub use analyzer::analyze_config_content;
pub use detector::{ClientDetector, DetectedClient};
pub use engine::{TemplateEngine, TemplateExecutionResult};
pub use error::ConfigError;
pub use models::{
    CapabilitySource, ClientCapabilityConfig, ClientTemplate, ConfigMode, ContainerType, DetectionMethod,
    ManagedEndpointConfig, TemplateFormat,
};
pub use service::{
    ClientConfigService, ClientDescriptor, ClientRenderOptions, ClientRenderResult, HostedClientReapplySummary,
};
pub use source::{ClientConfigSource, DbTemplateSource, FileTemplateSource, TemplateRoot};
pub use utils::{get_nested_value, set_nested_value};
