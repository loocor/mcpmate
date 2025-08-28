//! Unified validation framework
//!
//! Provides standardized validation patterns to eliminate code duplication
//! and ensure consistent validation logic across all modules.

use std::collections::HashMap;

/// Validation result for a single field
#[derive(Debug, Clone)]
pub struct FieldValidation {
    /// Field name
    pub field: String,
    /// Whether the field is valid
    pub is_valid: bool,
    /// Error message if validation failed
    pub error: Option<String>,
}

impl FieldValidation {
    /// Create a successful validation result
    pub fn success(field: &str) -> Self {
        Self {
            field: field.to_string(),
            is_valid: true,
            error: None,
        }
    }

    /// Create a failed validation result
    pub fn failure(field: &str, error: &str) -> Self {
        Self {
            field: field.to_string(),
            is_valid: false,
            error: Some(error.to_string()),
        }
    }
}

/// Validation result for multiple fields
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Individual field validations
    pub fields: Vec<FieldValidation>,
    /// Overall validation status
    pub is_valid: bool,
}

impl ValidationResult {
    /// Create a new validation result
    pub fn new() -> Self {
        Self {
            fields: Vec::new(),
            is_valid: true,
        }
    }

    /// Add a field validation result
    pub fn add_field(&mut self, field_validation: FieldValidation) {
        if !field_validation.is_valid {
            self.is_valid = false;
        }
        self.fields.push(field_validation);
    }

    /// Get all error messages
    pub fn get_errors(&self) -> Vec<String> {
        self.fields
            .iter()
            .filter_map(|f| f.error.as_ref())
            .cloned()
            .collect()
    }

    /// Get errors as a formatted string
    pub fn get_error_message(&self) -> Option<String> {
        let errors = self.get_errors();
        if errors.is_empty() {
            None
        } else {
            Some(errors.join("; "))
        }
    }

    /// Get errors grouped by field
    pub fn get_field_errors(&self) -> HashMap<String, String> {
        self.fields
            .iter()
            .filter_map(|f| {
                f.error.as_ref().map(|err| (f.field.clone(), err.clone()))
            })
            .collect()
    }
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Standard validation functions
pub struct Validator;

impl Validator {
    /// Validate that a string is not empty
    pub fn not_empty(field: &str, value: &str) -> FieldValidation {
        if value.trim().is_empty() {
            FieldValidation::failure(field, "Field cannot be empty")
        } else {
            FieldValidation::success(field)
        }
    }

    /// Validate string length constraints
    pub fn length_range(field: &str, value: &str, min: usize, max: usize) -> FieldValidation {
        let len = value.len();
        if len < min {
            FieldValidation::failure(field, &format!("Field must be at least {} characters", min))
        } else if len > max {
            FieldValidation::failure(field, &format!("Field must be at most {} characters", max))
        } else {
            FieldValidation::success(field)
        }
    }

    /// Validate that a string contains only alphanumeric characters and underscores
    pub fn alphanumeric_underscore(field: &str, value: &str) -> FieldValidation {
        if value.chars().all(|c| c.is_alphanumeric() || c == '_') {
            FieldValidation::success(field)
        } else {
            FieldValidation::failure(field, "Field can only contain letters, numbers, and underscores")
        }
    }

    /// Validate that a string is a valid identifier (starts with letter/underscore, then alphanumeric/underscore)
    pub fn valid_identifier(field: &str, value: &str) -> FieldValidation {
        if value.is_empty() {
            return FieldValidation::failure(field, "Identifier cannot be empty");
        }

        let first_char = value.chars().next().unwrap();
        if !first_char.is_alphabetic() && first_char != '_' {
            return FieldValidation::failure(field, "Identifier must start with a letter or underscore");
        }

        if value.chars().all(|c| c.is_alphanumeric() || c == '_') {
            FieldValidation::success(field)
        } else {
            FieldValidation::failure(field, "Identifier can only contain letters, numbers, and underscores")
        }
    }

    /// Validate that a number is within a range
    pub fn number_range<T>(field: &str, value: T, min: T, max: T) -> FieldValidation
    where
        T: PartialOrd + std::fmt::Display,
    {
        if value < min {
            FieldValidation::failure(field, &format!("Value must be at least {}", min))
        } else if value > max {
            FieldValidation::failure(field, &format!("Value must be at most {}", max))
        } else {
            FieldValidation::success(field)
        }
    }

    /// Validate that a port number is valid
    pub fn valid_port(field: &str, port: u16) -> FieldValidation {
        if port == 0 {
            FieldValidation::failure(field, "Port cannot be 0")
        } else if port < 1024 {
            FieldValidation::failure(field, "Port below 1024 requires root privileges")
        } else {
            FieldValidation::success(field)
        }
    }

    /// Validate that a URL is well-formed
    pub fn valid_url(field: &str, url: &str) -> FieldValidation {
        if url.is_empty() {
            return FieldValidation::failure(field, "URL cannot be empty");
        }

        // Basic URL validation - starts with http:// or https://
        if url.starts_with("http://") || url.starts_with("https://") {
            FieldValidation::success(field)
        } else {
            FieldValidation::failure(field, "URL must start with http:// or https://")
        }
    }

    /// Validate that a file path exists
    pub fn path_exists(field: &str, path: &str) -> FieldValidation {
        if std::path::Path::new(path).exists() {
            FieldValidation::success(field)
        } else {
            FieldValidation::failure(field, "Path does not exist")
        }
    }

    /// Validate that a value is one of the allowed options
    pub fn one_of(field: &str, value: &str, allowed: &[&str]) -> FieldValidation {
        if allowed.contains(&value) {
            FieldValidation::success(field)
        } else {
            FieldValidation::failure(
                field,
                &format!("Value must be one of: {}", allowed.join(", "))
            )
        }
    }
}

/// Validation builder for chaining multiple validations
pub struct ValidationBuilder {
    result: ValidationResult,
}

impl ValidationBuilder {
    /// Create a new validation builder
    pub fn new() -> Self {
        Self {
            result: ValidationResult::new(),
        }
    }

    /// Add a validation result
    pub fn add(mut self, validation: FieldValidation) -> Self {
        self.result.add_field(validation);
        self
    }

    /// Add multiple validation results
    pub fn add_all(mut self, validations: Vec<FieldValidation>) -> Self {
        for validation in validations {
            self.result.add_field(validation);
        }
        self
    }

    /// Build the final validation result
    pub fn build(self) -> ValidationResult {
        self.result
    }
}

impl Default for ValidationBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_validation_success() {
        let validation = FieldValidation::success("test_field");
        assert!(validation.is_valid);
        assert_eq!(validation.field, "test_field");
        assert!(validation.error.is_none());
    }

    #[test]
    fn test_field_validation_failure() {
        let validation = FieldValidation::failure("test_field", "Test error");
        assert!(!validation.is_valid);
        assert_eq!(validation.field, "test_field");
        assert_eq!(validation.error, Some("Test error".to_string()));
    }

    #[test]
    fn test_validation_result() {
        let mut result = ValidationResult::new();
        assert!(result.is_valid);

        result.add_field(FieldValidation::success("field1"));
        assert!(result.is_valid);

        result.add_field(FieldValidation::failure("field2", "Error message"));
        assert!(!result.is_valid);

        let errors = result.get_errors();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0], "Error message");
    }

    #[test]
    fn test_validator_not_empty() {
        let valid = Validator::not_empty("name", "test");
        assert!(valid.is_valid);

        let invalid = Validator::not_empty("name", "");
        assert!(!invalid.is_valid);

        let whitespace = Validator::not_empty("name", "   ");
        assert!(!whitespace.is_valid);
    }

    #[test]
    fn test_validator_length_range() {
        let valid = Validator::length_range("name", "test", 2, 10);
        assert!(valid.is_valid);

        let too_short = Validator::length_range("name", "a", 2, 10);
        assert!(!too_short.is_valid);

        let too_long = Validator::length_range("name", "this is too long", 2, 10);
        assert!(!too_long.is_valid);
    }

    #[test]
    fn test_validator_valid_identifier() {
        let valid = Validator::valid_identifier("name", "valid_name");
        assert!(valid.is_valid);

        let valid_underscore = Validator::valid_identifier("name", "_private");
        assert!(valid_underscore.is_valid);

        let invalid_start = Validator::valid_identifier("name", "123invalid");
        assert!(!invalid_start.is_valid);

        let invalid_chars = Validator::valid_identifier("name", "invalid-name");
        assert!(!invalid_chars.is_valid);
    }

    #[test]
    fn test_validator_valid_port() {
        let valid = Validator::valid_port("port", 8080);
        assert!(valid.is_valid);

        let zero = Validator::valid_port("port", 0);
        assert!(!zero.is_valid);

        let privileged = Validator::valid_port("port", 80);
        assert!(!privileged.is_valid);
    }

    #[test]
    fn test_validation_builder() {
        let result = ValidationBuilder::new()
            .add(Validator::not_empty("name", "test"))
            .add(Validator::valid_port("port", 8080))
            .add(Validator::length_range("description", "short", 1, 100))
            .build();

        assert!(result.is_valid);
        assert_eq!(result.fields.len(), 3);
    }

    #[test]
    fn test_validation_builder_with_errors() {
        let result = ValidationBuilder::new()
            .add(Validator::not_empty("name", ""))
            .add(Validator::valid_port("port", 0))
            .build();

        assert!(!result.is_valid);
        assert_eq!(result.get_errors().len(), 2);
    }
}
