use thiserror::Error;

use crate::SecretResolver;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SecretError {
    #[error("invalid secret reference: {0}")]
    InvalidReference(String),
    #[error("secret '{0}' was not found")]
    NotFound(String),
    #[error("invalid secret metadata: {0}")]
    InvalidMetadata(String),
    #[error("secret provider is unavailable")]
    ProviderUnavailable,
    #[error("unterminated secret placeholder")]
    UnterminatedPlaceholder,
}

const SECRET_PREFIX: &str = "[[secret:";
const SECRET_SUFFIX: &str = "]]";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SecretReference {
    alias: String,
}

impl SecretReference {
    pub fn new(alias: impl Into<String>) -> Result<Self, SecretError> {
        let alias = alias.into();
        let trimmed = alias.trim();
        if trimmed.is_empty() {
            return Err(SecretError::InvalidReference(
                "secret alias cannot be empty".to_string(),
            ));
        }
        if trimmed != alias {
            return Err(SecretError::InvalidReference(
                "secret alias cannot contain surrounding whitespace".to_string(),
            ));
        }
        if !trimmed
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':' | '/'))
        {
            return Err(SecretError::InvalidReference(format!(
                "secret alias '{trimmed}' contains unsupported characters"
            )));
        }
        Ok(Self { alias })
    }

    pub fn alias(&self) -> &str {
        &self.alias
    }

    pub fn placeholder(&self) -> String {
        format!("{SECRET_PREFIX}{}{SECRET_SUFFIX}", self.alias)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct SecretValue {
    value: String,
}

impl SecretValue {
    pub fn new(value: impl Into<String>) -> Self {
        Self { value: value.into() }
    }

    pub fn expose(&self) -> &str {
        &self.value
    }
}

impl std::fmt::Debug for SecretValue {
    fn fmt(
        &self,
        formatter: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        formatter
            .debug_struct("SecretValue")
            .field("value", &"<redacted>")
            .finish()
    }
}

pub fn parse_placeholder(input: &str) -> Result<Option<SecretReference>, SecretError> {
    let Some(alias) = input
        .strip_prefix(SECRET_PREFIX)
        .and_then(|value| value.strip_suffix(SECRET_SUFFIX))
    else {
        return Ok(None);
    };
    SecretReference::new(alias).map(Some)
}

pub fn extract_secret_references(input: &str) -> Result<Vec<SecretReference>, SecretError> {
    let mut references = Vec::new();
    let mut rest = input;

    while let Some(start) = rest.find(SECRET_PREFIX) {
        let after_prefix = &rest[start + SECRET_PREFIX.len()..];
        let Some(end) = after_prefix.find(SECRET_SUFFIX) else {
            return Err(SecretError::UnterminatedPlaceholder);
        };
        references.push(SecretReference::new(&after_prefix[..end])?);
        rest = &after_prefix[end + SECRET_SUFFIX.len()..];
    }

    Ok(references)
}

pub fn resolve_placeholders(
    input: &str,
    resolver: &(impl SecretResolver + ?Sized),
) -> Result<String, SecretError> {
    let mut resolved = String::with_capacity(input.len());
    let mut rest = input;

    while let Some(start) = rest.find(SECRET_PREFIX) {
        resolved.push_str(&rest[..start]);
        let after_prefix = &rest[start + SECRET_PREFIX.len()..];
        let Some(end) = after_prefix.find(SECRET_SUFFIX) else {
            return Err(SecretError::UnterminatedPlaceholder);
        };
        let alias = &after_prefix[..end];
        let reference = SecretReference::new(alias)?;
        let value = resolver.resolve_secret(&reference)?;
        resolved.push_str(value.expose());
        rest = &after_prefix[end + SECRET_SUFFIX.len()..];
    }

    resolved.push_str(rest);
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::{extract_secret_references, parse_placeholder, resolve_placeholders};
    use crate::{SecretError, SecretValue, testing::InMemorySecretResolver};

    #[test]
    fn parses_full_secret_placeholder() {
        let reference = parse_placeholder("[[secret:github_pat]]")
            .expect("placeholder parses")
            .expect("placeholder exists");

        assert_eq!(reference.alias(), "github_pat");
        assert_eq!(reference.placeholder(), "[[secret:github_pat]]");
    }

    #[test]
    fn extracts_embedded_secret_references() {
        let references =
            extract_secret_references("Bearer [[secret:token]] and [[secret:server/key]]").expect("extract refs");

        let aliases = references.iter().map(|reference| reference.alias()).collect::<Vec<_>>();
        assert_eq!(aliases, ["token", "server/key"]);
    }

    #[test]
    fn resolves_embedded_secret_placeholders() {
        let resolver = InMemorySecretResolver::from_pairs([("token", "runtime-token")]);

        let resolved = resolve_placeholders("Bearer [[secret:token]]", &resolver).expect("placeholder resolves");

        assert_eq!(resolved, "Bearer runtime-token");
    }

    #[test]
    fn fails_on_unterminated_placeholder() {
        let resolver = InMemorySecretResolver::from_pairs([("token", "runtime-token")]);

        let err = resolve_placeholders("Bearer [[secret:token", &resolver).expect_err("placeholder should fail");

        assert_eq!(err, SecretError::UnterminatedPlaceholder);
    }

    #[test]
    fn debug_output_redacts_secret_material() {
        let value = SecretValue::new("runtime-token");
        let resolver = InMemorySecretResolver::from_pairs([("token", "runtime-token")]);

        assert!(!format!("{value:?}").contains("runtime-token"));
        assert!(!format!("{resolver:?}").contains("runtime-token"));
    }
}
