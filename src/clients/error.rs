use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Client template file parsing error: {0}")]
    TemplateParseError(String),

    #[error("Client template index building failed: {0}")]
    TemplateIndexError(String),

    #[error("Path resolution failed: {0}")]
    PathResolutionError(String),

    #[error("Configuration path is not writable: {path}")]
    PathNotWritable { path: PathBuf },

    #[error("Client detection failed: {client_id}")]
    DetectionFailed { client_id: String },

    #[error("Configuration merge conflict: {details}")]
    MergeConflict { details: String },

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("YAML error: {0}")]
    YamlError(#[from] serde_yaml::Error),

    #[error("TOML error: {0}")]
    TomlError(#[from] toml::de::Error),

    #[error("TOML serialization error: {0}")]
    TomlSerializeError(String),

    #[error("Handlebars error: {0}")]
    HandlebarsError(#[from] handlebars::TemplateError),

    #[error("Handlebars render error: {0}")]
    HandlebarsRenderError(#[from] handlebars::RenderError),

    #[error("Template format not supported: {0}")]
    UnsupportedFormat(String),

    #[error("Unknown template field: {0}")]
    UnknownField(String),

    #[error("Configuration storage adapter not registered: {0}")]
    StorageAdapterMissing(String),

    #[error("Renderer not registered: {0}")]
    RendererMissing(String),

    #[error("Template conflict: {identifier}")]
    TemplateConflict { identifier: String },

    #[error("File operation failed: {0}")]
    FileOperationError(String),

    #[error("Data access failed: {0}")]
    DataAccessError(String),

    #[error("Client {identifier} 已被禁用，不允许由 MCPMate 管理")]
    ClientDisabled { identifier: String },
}

pub type ConfigResult<T> = Result<T, ConfigError>;
