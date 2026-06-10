use crate::{SecretError, SecretReference, SecretValue};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretKind {
    Generic,
    Token,
    ApiKey,
    Password,
    OAuthClientSecret,
    OAuthAccessToken,
    OAuthRefreshToken,
    UrlCredential,
    HeaderValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretProviderKind {
    LocalEncryptedVault,
    OperatingSystemKeychain,
    EnterpriseKms,
    HardwareSecurityModule,
    ManagedVault,
    Test,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretProviderMetadata {
    provider_id: String,
    kind: SecretProviderKind,
}

impl SecretProviderMetadata {
    pub fn new(
        provider_id: impl Into<String>,
        kind: SecretProviderKind,
    ) -> Result<Self, SecretError> {
        let provider_id = provider_id.into();
        validate_identifier("provider id", &provider_id)?;
        Ok(Self { provider_id, kind })
    }

    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }

    pub fn kind(&self) -> &SecretProviderKind {
        &self.kind
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretMetadata {
    reference: SecretReference,
    kind: SecretKind,
    provider: SecretProviderMetadata,
    version: u64,
}

impl SecretMetadata {
    pub fn new(
        reference: SecretReference,
        kind: SecretKind,
        provider: SecretProviderMetadata,
        version: u64,
    ) -> Result<Self, SecretError> {
        if version == 0 {
            return Err(SecretError::InvalidMetadata(
                "secret version must be greater than zero".to_string(),
            ));
        }
        Ok(Self {
            reference,
            kind,
            provider,
            version,
        })
    }

    pub fn reference(&self) -> &SecretReference {
        &self.reference
    }

    pub fn kind(&self) -> &SecretKind {
        &self.kind
    }

    pub fn provider(&self) -> &SecretProviderMetadata {
        &self.provider
    }

    pub fn version(&self) -> u64 {
        self.version
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretRecord {
    metadata: SecretMetadata,
    value: SecretValue,
}

impl SecretRecord {
    pub fn new(
        metadata: SecretMetadata,
        value: SecretValue,
    ) -> Self {
        Self { metadata, value }
    }

    pub fn metadata(&self) -> &SecretMetadata {
        &self.metadata
    }

    pub fn value(&self) -> &SecretValue {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretUsageLocation {
    StdioCommand,
    StdioArgument { index: usize },
    StdioEnv { name: String },
    StreamableHttpUrl,
    StreamableHttpHeader { name: String },
    OAuthToken,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretUsageRef {
    reference: SecretReference,
    server_id: String,
    location: SecretUsageLocation,
}

impl SecretUsageRef {
    pub fn new(
        reference: SecretReference,
        server_id: impl Into<String>,
        location: SecretUsageLocation,
    ) -> Result<Self, SecretError> {
        let server_id = server_id.into();
        validate_identifier("server id", &server_id)?;
        validate_usage_location(&location)?;
        Ok(Self {
            reference,
            server_id,
            location,
        })
    }

    pub fn reference(&self) -> &SecretReference {
        &self.reference
    }

    pub fn server_id(&self) -> &str {
        &self.server_id
    }

    pub fn location(&self) -> &SecretUsageLocation {
        &self.location
    }
}

/// Resolves secret references at runtime.
///
/// Implementations must not include secret material in their `Debug` output.
pub trait SecretResolver: std::fmt::Debug + Send + Sync {
    fn resolve_secret(
        &self,
        reference: &SecretReference,
    ) -> Result<SecretValue, SecretError>;
}

pub trait SecretStore: SecretResolver {
    fn put_secret(
        &self,
        record: SecretRecord,
    ) -> Result<SecretMetadata, SecretError>;

    fn delete_secret(
        &self,
        reference: &SecretReference,
    ) -> Result<(), SecretError>;

    fn list_secret_metadata(&self) -> Result<Vec<SecretMetadata>, SecretError>;

    fn list_usage_refs(
        &self,
        reference: &SecretReference,
    ) -> Result<Vec<SecretUsageRef>, SecretError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct UnavailableSecretResolver;

impl SecretResolver for UnavailableSecretResolver {
    fn resolve_secret(
        &self,
        _reference: &SecretReference,
    ) -> Result<SecretValue, SecretError> {
        Err(SecretError::ProviderUnavailable)
    }
}

fn validate_identifier(
    label: &str,
    value: &str,
) -> Result<(), SecretError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(SecretError::InvalidMetadata(format!("{label} cannot be empty")));
    }
    if trimmed != value {
        return Err(SecretError::InvalidMetadata(format!(
            "{label} cannot contain surrounding whitespace"
        )));
    }
    Ok(())
}

fn validate_usage_location(location: &SecretUsageLocation) -> Result<(), SecretError> {
    match location {
        SecretUsageLocation::StdioEnv { name } | SecretUsageLocation::StreamableHttpHeader { name } => {
            validate_identifier("usage name", name)
        }
        SecretUsageLocation::StdioCommand
        | SecretUsageLocation::StdioArgument { .. }
        | SecretUsageLocation::StreamableHttpUrl
        | SecretUsageLocation::OAuthToken => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        SecretKind, SecretMetadata, SecretProviderKind, SecretProviderMetadata, SecretRecord, SecretUsageLocation,
        SecretUsageRef,
    };
    use crate::{SecretError, SecretReference, SecretValue};

    #[test]
    fn models_secret_metadata_without_secret_material() {
        let reference = SecretReference::new("server/github/token").expect("valid reference");
        let provider = SecretProviderMetadata::new("local-vault", SecretProviderKind::LocalEncryptedVault)
            .expect("valid provider");
        let metadata = SecretMetadata::new(reference.clone(), SecretKind::Token, provider, 1).expect("valid metadata");
        let record = SecretRecord::new(metadata.clone(), SecretValue::new("runtime-token"));

        assert_eq!(metadata.reference(), &reference);
        assert_eq!(metadata.kind(), &SecretKind::Token);
        assert_eq!(metadata.provider().provider_id(), "local-vault");
        assert_eq!(metadata.version(), 1);
        assert!(!format!("{record:?}").contains("runtime-token"));
    }

    #[test]
    fn models_secret_usage_references_for_runtime_surfaces() {
        let reference = SecretReference::new("server/github/token").expect("valid reference");

        let env_usage = SecretUsageRef::new(
            reference.clone(),
            "github-server",
            SecretUsageLocation::StdioEnv {
                name: "GITHUB_TOKEN".to_string(),
            },
        )
        .expect("valid env usage");
        let header_usage = SecretUsageRef::new(
            reference.clone(),
            "github-server",
            SecretUsageLocation::StreamableHttpHeader {
                name: "Authorization".to_string(),
            },
        )
        .expect("valid header usage");

        assert_eq!(env_usage.reference(), &reference);
        assert_eq!(env_usage.server_id(), "github-server");
        assert_eq!(
            header_usage.location(),
            &SecretUsageLocation::StreamableHttpHeader {
                name: "Authorization".to_string(),
            }
        );
    }

    #[test]
    fn rejects_invalid_metadata_boundaries() {
        let reference = SecretReference::new("server/github/token").expect("valid reference");
        let provider = SecretProviderMetadata::new("local-vault", SecretProviderKind::LocalEncryptedVault)
            .expect("valid provider");

        let err = SecretMetadata::new(reference.clone(), SecretKind::Token, provider, 0)
            .expect_err("version zero is invalid");

        assert_eq!(
            err,
            SecretError::InvalidMetadata("secret version must be greater than zero".to_string())
        );
        assert!(SecretProviderMetadata::new(" local-vault", SecretProviderKind::LocalEncryptedVault).is_err());
        assert!(
            SecretUsageRef::new(
                reference,
                "github-server",
                SecretUsageLocation::StreamableHttpHeader {
                    name: " Authorization".to_string(),
                },
            )
            .is_err()
        );
    }
}
