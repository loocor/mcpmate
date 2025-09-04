//! Unified error handling module
//!
//! Provides standardized error mapping and handling functions to ensure consistent error handling across all API modules

use crate::api::handlers::ApiError;
use crate::common::constants::errors;
use sqlx::Error as SqlxError;

/// Standardized database error mapping (sqlx::Error)
///
/// Maps sqlx errors to a standardized ApiError, providing consistent error handling
pub fn map_database_error(e: SqlxError) -> ApiError {
    match e {
        SqlxError::Database(db_err) if db_err.is_unique_violation() => {
            ApiError::Conflict(errors::RESOURCE_EXISTS.to_string())
        }
        SqlxError::Database(db_err) if db_err.is_foreign_key_violation() => {
            ApiError::BadRequest(errors::FK_VIOLATION.to_string())
        }
        SqlxError::Database(db_err) if db_err.is_check_violation() => {
            ApiError::BadRequest(errors::CHECK_VIOLATION.to_string())
        }
        SqlxError::RowNotFound => ApiError::NotFound(errors::RESOURCE_NOT_FOUND.to_string()),
        SqlxError::PoolTimedOut => ApiError::InternalError(errors::DB_TIMEOUT.to_string()),
        SqlxError::Io(io_err) => ApiError::InternalError(format!("Database I/O error: {io_err}")),
        _ => ApiError::InternalError(format!("Database error: {e}")),
    }
}

/// Standardized anyhow error mapping
///
/// Maps anyhow errors to a standardized ApiError, ensuring compatibility with existing code
pub fn map_anyhow_error(e: anyhow::Error) -> ApiError {
    // Attempt to downcast to a sqlx error
    if let Some(sqlx_err) = e.downcast_ref::<SqlxError>() {
        return map_database_error_ref(sqlx_err);
    }

    // Default to internal error
    ApiError::InternalError(format!("Operation failed: {e}"))
}

/// Standardized sqlx error reference mapping
///
/// Handles sqlx error references to avoid cloning issues
pub fn map_database_error_ref(e: &SqlxError) -> ApiError {
    match e {
        SqlxError::Database(db_err) if db_err.is_unique_violation() => {
            ApiError::Conflict(errors::RESOURCE_EXISTS.to_string())
        }
        SqlxError::Database(db_err) if db_err.is_foreign_key_violation() => {
            ApiError::BadRequest(errors::FK_VIOLATION.to_string())
        }
        SqlxError::Database(db_err) if db_err.is_check_violation() => {
            ApiError::BadRequest(errors::CHECK_VIOLATION.to_string())
        }
        SqlxError::RowNotFound => ApiError::NotFound(errors::RESOURCE_NOT_FOUND.to_string()),
        SqlxError::PoolTimedOut => ApiError::InternalError(errors::DB_TIMEOUT.to_string()),
        SqlxError::Io(io_err) => ApiError::InternalError(format!("Database I/O error: {io_err}")),
        _ => ApiError::InternalError(format!("Database error: {e}")),
    }
}

/// Standardized internal error wrapping
///
/// Wraps internal error messages, providing consistent error formatting
#[inline]
pub fn internal_error(msg: &str) -> ApiError {
    ApiError::InternalError(msg.to_owned())
}

/// Standardized validation error wrapping
///
/// Wraps validation errors, providing consistent error formatting
#[inline]
pub fn validation_error(
    field: &str,
    message: &str,
) -> ApiError {
    ApiError::BadRequest(format!("Validation error for {field}: {message}"))
}

/// Standardized not found error wrapping
///
/// Wraps resource not found errors, providing consistent error formatting
#[inline]
pub fn not_found_error(
    resource: &str,
    identifier: &str,
) -> ApiError {
    ApiError::NotFound(format!("{resource} '{identifier}' not found"))
}

/// Standardized conflict error wrapping
///
/// Wraps resource conflict errors, providing consistent error formatting
#[inline]
pub fn conflict_error(
    resource: &str,
    identifier: &str,
) -> ApiError {
    ApiError::Conflict(format!("{resource} '{identifier}' already exists"))
}

/// Standardized forbidden error wrapping
///
/// Wraps permission-related errors, providing consistent error formatting
#[inline]
pub fn forbidden_error(operation: &str) -> ApiError {
    ApiError::Forbidden(format!("Operation '{operation}' is not allowed"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::Error as SqlxError;

    #[test]
    fn test_map_database_error_row_not_found() {
        let error = map_database_error(SqlxError::RowNotFound);
        match error {
            ApiError::NotFound(msg) => assert_eq!(msg, "Resource not found"),
            _ => panic!("Expected NotFound error"),
        }
    }

    #[test]
    fn test_internal_error() {
        let error = internal_error("Test error message");
        match error {
            ApiError::InternalError(msg) => assert_eq!(msg, "Test error message"),
            _ => panic!("Expected InternalError"),
        }
    }

    #[test]
    fn test_validation_error() {
        let error = validation_error("name", "cannot be empty");
        match error {
            ApiError::BadRequest(msg) => assert_eq!(msg, "Validation error for name: cannot be empty"),
            _ => panic!("Expected BadRequest error"),
        }
    }

    #[test]
    fn test_not_found_error() {
        let error = not_found_error("Server", "test-server");
        match error {
            ApiError::NotFound(msg) => assert_eq!(msg, "Server 'test-server' not found"),
            _ => panic!("Expected NotFound error"),
        }
    }

    #[test]
    fn test_conflict_error() {
        let error = conflict_error("Server", "test-server");
        match error {
            ApiError::Conflict(msg) => assert_eq!(msg, "Server 'test-server' already exists"),
            _ => panic!("Expected Conflict error"),
        }
    }

    #[test]
    fn test_forbidden_error() {
        let error = forbidden_error("delete server");
        match error {
            ApiError::Forbidden(msg) => assert_eq!(msg, "Operation 'delete server' is not allowed"),
            _ => panic!("Expected Forbidden error"),
        }
    }
}
