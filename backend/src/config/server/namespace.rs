use std::fmt;

pub const MAX_SERVER_NAMESPACE_LEN: usize = 64;
pub const SERVER_NAMESPACE_PATTERN: &str = "^[a-z][a-z0-9]*(?:_[a-z0-9]+)*$";
const RESERVED_SERVER_NAMESPACES: &[&str] = &["template"];

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct NamespaceValidationError {
    value: String,
    reason: &'static str,
    suggestion: Option<String>,
}

impl NamespaceValidationError {
    pub fn suggestion(&self) -> Option<&str> {
        self.suggestion.as_deref()
    }
}

impl fmt::Display for NamespaceValidationError {
    fn fmt(
        &self,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(
            formatter,
            "Invalid server namespace '{}': {}. Use lowercase snake_case matching {} (1-{} characters)",
            self.value, self.reason, SERVER_NAMESPACE_PATTERN, MAX_SERVER_NAMESPACE_LEN
        )?;
        if let Some(suggestion) = &self.suggestion {
            write!(formatter, ". Suggested namespace: '{suggestion}'")?;
        }
        Ok(())
    }
}

impl std::error::Error for NamespaceValidationError {}

pub fn validate_server_namespace(namespace: &str) -> Result<(), NamespaceValidationError> {
    let reason = if namespace.is_empty() {
        Some("the namespace cannot be empty")
    } else if namespace.len() > MAX_SERVER_NAMESPACE_LEN {
        Some("the namespace is too long")
    } else if RESERVED_SERVER_NAMESPACES.contains(&namespace) {
        Some("the namespace is reserved by the MCPMate resource address space")
    } else if !is_canonical_server_namespace(namespace) {
        Some("the namespace is not canonical")
    } else {
        None
    };

    match reason {
        Some(reason) => Err(NamespaceValidationError {
            value: namespace.to_string(),
            reason,
            suggestion: suggest_server_namespace(namespace),
        }),
        None => Ok(()),
    }
}

pub fn suggest_server_namespace(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut suggestion = String::with_capacity(trimmed.len());
    let mut separator_pending = false;
    for character in trimmed.chars() {
        if character.is_ascii_alphanumeric() {
            if separator_pending && !suggestion.is_empty() {
                suggestion.push('_');
            }
            separator_pending = false;
            suggestion.push(character.to_ascii_lowercase());
        } else if character.is_ascii_whitespace() || matches!(character, '-' | '.' | '_') {
            separator_pending = !suggestion.is_empty();
        } else {
            return None;
        }
    }

    (is_canonical_server_namespace(&suggestion) && !RESERVED_SERVER_NAMESPACES.contains(&suggestion.as_str()))
        .then_some(suggestion)
}

fn is_canonical_server_namespace(namespace: &str) -> bool {
    if namespace.is_empty() || namespace.len() > MAX_SERVER_NAMESPACE_LEN {
        return false;
    }

    let mut characters = namespace.bytes();
    let Some(first) = characters.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() {
        return false;
    }

    let mut previous_was_separator = false;
    for character in characters {
        if character == b'_' {
            if previous_was_separator {
                return false;
            }
            previous_was_separator = true;
        } else if character.is_ascii_lowercase() || character.is_ascii_digit() {
            previous_was_separator = false;
        } else {
            return false;
        }
    }

    !previous_was_separator
}

#[cfg(test)]
mod tests {
    use super::{suggest_server_namespace, validate_server_namespace};

    #[test]
    fn suggests_visible_canonical_namespace_for_safe_transformations() {
        assert_eq!(
            suggest_server_namespace("  Sequential Thinking-v2  ").as_deref(),
            Some("sequential_thinking_v2")
        );
        assert_eq!(
            suggest_server_namespace("PaddleOCR-VL-1.6").as_deref(),
            Some("paddleocr_vl_1_6")
        );
    }

    #[test]
    fn accepts_digits_after_the_first_letter() {
        assert!(validate_server_namespace("sequential_thinking_v2").is_ok());
        assert!(validate_server_namespace("server_7zip").is_ok());
    }

    #[test]
    fn rejects_non_canonical_or_unsafe_namespaces() {
        for namespace in [
            "SequentialThinking",
            "sequential-thinking",
            "sequential thinking",
            "7zip",
            "server__name",
            "server_",
            "server.name",
            "序列思考",
            "template",
            "",
        ] {
            assert!(
                validate_server_namespace(namespace).is_err(),
                "namespace '{namespace}' must be rejected"
            );
        }
    }

    #[test]
    fn reserved_resource_route_segment_has_no_namespace_suggestion() {
        let error = validate_server_namespace("template").expect_err("reserved namespace must fail");

        assert_eq!(error.suggestion(), None);
        assert!(error.to_string().contains("reserved"));
    }

    #[test]
    fn rejects_namespaces_longer_than_sixty_four_characters() {
        assert!(validate_server_namespace(&"a".repeat(64)).is_ok());
        assert!(validate_server_namespace(&"a".repeat(65)).is_err());
    }

    #[test]
    fn validation_error_includes_a_safe_suggestion_when_available() {
        let error = validate_server_namespace("Sequential Thinking-v2").expect_err("non-canonical namespace must fail");

        assert_eq!(error.suggestion(), Some("sequential_thinking_v2"));
        assert!(
            error
                .to_string()
                .contains("Suggested namespace: 'sequential_thinking_v2'")
        );
    }

    #[test]
    fn suggestion_is_omitted_when_safe_transforms_cannot_produce_a_valid_namespace() {
        assert_eq!(suggest_server_namespace("123"), None);
        assert_eq!(suggest_server_namespace("序列思考"), None);
    }
}
