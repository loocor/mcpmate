#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    #[error("capability catalog database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("capability catalog JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unsupported record format version {actual}; expected {expected}")]
    UnsupportedRecordVersion { actual: i64, expected: i64 },
    #[error("invalid capability catalog value for {field}: {value}")]
    InvalidValue { field: &'static str, value: String },
    #[error("invalid capability catalog timestamp for {field}: {value}")]
    InvalidTimestamp { field: &'static str, value: String },
    #[error("capability catalog snapshot not found for server '{server_id}'")]
    SnapshotNotFound { server_id: String },
    #[error("capability catalog server configuration not found for '{server_id}'")]
    ServerNotFound { server_id: String },
}

pub type Result<T> = std::result::Result<T, CatalogError>;
