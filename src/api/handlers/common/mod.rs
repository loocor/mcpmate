//! Shared API handlers module
//!
//! Includes all API handlers shared tools and error handling logic

pub mod errors;

// Re-export commonly used error handling functions, making them easily accessible to other modules
pub use errors::{
    conflict_error, forbidden_error, internal_error, map_anyhow_error, map_database_error, map_database_error_ref,
    not_found_error, validation_error,
};
