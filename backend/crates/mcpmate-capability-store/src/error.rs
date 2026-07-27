#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    #[error("capability catalog database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("capability catalog JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unsupported record format version {actual}; expected {expected}")]
    UnsupportedRecordVersion { actual: i64, expected: i64 },
    #[error("incompatible capability schema: {details}")]
    IncompatibleSchema { details: String },
    #[error("invalid capability catalog value for {field}: {value}")]
    InvalidValue { field: &'static str, value: String },
    #[error("invalid capability catalog timestamp for {field}: {value}")]
    InvalidTimestamp { field: &'static str, value: String },
    #[error("capability catalog snapshot not found for server '{server_id}'")]
    SnapshotNotFound { server_id: String },
    #[error("capability catalog server configuration not found for '{server_id}'")]
    ServerNotFound { server_id: String },
    #[error("invalid {identity_kind} identity '{value}'")]
    InvalidIdentity { identity_kind: &'static str, value: String },
    #[error("capability source kind {source_kind:?} does not match payload kind {payload_kind:?}")]
    CapabilityKindMismatch {
        source_kind: crate::CapabilityKind,
        payload_kind: crate::CapabilityKind,
    },
    #[error("unsupported effective capability format {actual}; expected {expected}")]
    UnsupportedEffectiveCapabilityFormat { actual: String, expected: &'static str },
    #[error("canonical content integrity mismatch for '{identity}'")]
    IntegrityMismatch { identity: String },
    #[error("duplicate capability origin ({server_id}, {kind:?}, {origin_key})")]
    DuplicateOrigin {
        server_id: String,
        kind: crate::CapabilityKind,
        origin_key: String,
    },
    #[error("invalid surface value for {field}: {value}")]
    InvalidSurfaceValue { field: &'static str, value: String },
    #[error("duplicate capability ref '{ref_id}' in surface manifest")]
    DuplicateManifestRef { ref_id: String },
    #[error("concurrent update rejected for {entity} '{id}'")]
    ConcurrencyConflict { entity: &'static str, id: String },
    #[error("{entity} '{id}' was not found")]
    SurfaceNotFound { entity: &'static str, id: String },
}

pub type Result<T> = std::result::Result<T, CatalogError>;
