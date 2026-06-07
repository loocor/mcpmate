use std::collections::HashMap;

use crate::{SecretError, SecretReference, SecretResolver, SecretValue};

#[derive(Clone, Default)]
pub struct InMemorySecretResolver {
    secrets: HashMap<String, String>,
}

impl InMemorySecretResolver {
    pub fn from_pairs<const N: usize>(pairs: [(&str, &str); N]) -> Self {
        let secrets = pairs
            .into_iter()
            .map(|(alias, value)| (alias.to_string(), value.to_string()))
            .collect();
        Self { secrets }
    }
}

impl SecretResolver for InMemorySecretResolver {
    fn resolve_secret(
        &self,
        reference: &SecretReference,
    ) -> Result<SecretValue, SecretError> {
        self.secrets
            .get(reference.alias())
            .cloned()
            .map(SecretValue::new)
            .ok_or_else(|| SecretError::NotFound(reference.alias().to_string()))
    }
}

impl std::fmt::Debug for InMemorySecretResolver {
    fn fmt(
        &self,
        formatter: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        formatter
            .debug_struct("InMemorySecretResolver")
            .field("secret_count", &self.secrets.len())
            .finish()
    }
}
