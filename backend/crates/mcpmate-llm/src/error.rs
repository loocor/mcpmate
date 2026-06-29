use std::fmt;

pub type LlmResult<T> = Result<T, LlmError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmErrorKind {
    BadRequest,
    NotFound,
    ServiceUnavailable,
    Internal,
}

#[derive(Debug)]
pub struct LlmError {
    kind: LlmErrorKind,
    message: String,
}

impl LlmError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(LlmErrorKind::BadRequest, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(LlmErrorKind::NotFound, message)
    }

    pub fn service_unavailable(message: impl Into<String>) -> Self {
        Self::new(LlmErrorKind::ServiceUnavailable, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(LlmErrorKind::Internal, message)
    }

    pub fn from_anyhow(
        kind: LlmErrorKind,
        err: anyhow::Error,
    ) -> Self {
        Self::new(kind, err.to_string())
    }

    pub fn kind(&self) -> LlmErrorKind {
        self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    fn new(
        kind: LlmErrorKind,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for LlmError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for LlmError {}
