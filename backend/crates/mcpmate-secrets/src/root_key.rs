use std::{
    fmt,
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::PathBuf,
    sync::Arc,
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use ring::rand;
use sha2::{Digest, Sha256};
use thiserror::Error;

const DEVELOPMENT_PROVIDER_ID: &str = "local-encrypted-vault";
const DEVELOPMENT_PROVIDER_KIND: &str = "local_encrypted_vault";
const OS_PROVIDER_KIND: &str = "operating_system_keychain";
const OS_KEYRING_SERVICE: &str = "ai.umate.mcpmate.secure-store";
const OS_KEYRING_USER: &str = "secure-store-root-key";

pub const DEVELOPMENT_ROOT_KEY_ENV: &str = "MCPMATE_SECRETS_LOCAL_KEY";

pub type SecretRootKey = [u8; 32];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootKeyProviderMetadata {
    provider_id: &'static str,
    provider_kind: &'static str,
}

impl RootKeyProviderMetadata {
    pub const fn new(
        provider_id: &'static str,
        provider_kind: &'static str,
    ) -> Self {
        Self {
            provider_id,
            provider_kind,
        }
    }

    pub fn provider_id(&self) -> &'static str {
        self.provider_id
    }

    pub fn provider_kind(&self) -> &'static str {
        self.provider_kind
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SecretRootKeyError {
    #[error("operating-system secure storage is unavailable: {0}")]
    ProviderUnavailable(String),
    #[error("invalid root key material: {0}")]
    InvalidMaterial(String),
    #[error("development root key storage failed: {0}")]
    DevelopmentStorage(String),
}

pub trait SecretRootKeyProvider: fmt::Debug + Send + Sync {
    fn metadata(&self) -> RootKeyProviderMetadata;
    fn load_or_create_root_key(&self) -> Result<SecretRootKey, SecretRootKeyError>;
}

pub fn default_root_key_provider() -> Arc<dyn SecretRootKeyProvider> {
    Arc::new(OperatingSystemRootKeyProvider::new())
}

#[derive(Debug, Clone)]
pub struct OperatingSystemRootKeyProvider {
    service: String,
    user: String,
}

impl OperatingSystemRootKeyProvider {
    pub fn new() -> Self {
        Self::with_keyring_entry(OS_KEYRING_SERVICE, OS_KEYRING_USER)
    }

    pub fn with_keyring_entry(
        service: impl Into<String>,
        user: impl Into<String>,
    ) -> Self {
        Self {
            service: service.into(),
            user: user.into(),
        }
    }
}

impl Default for OperatingSystemRootKeyProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretRootKeyProvider for OperatingSystemRootKeyProvider {
    fn metadata(&self) -> RootKeyProviderMetadata {
        RootKeyProviderMetadata::new(os_provider_id(), OS_PROVIDER_KIND)
    }

    fn load_or_create_root_key(&self) -> Result<SecretRootKey, SecretRootKeyError> {
        load_or_create_os_root_key(&self.service, &self.user)
    }
}

#[derive(Debug, Clone)]
pub struct DevelopmentRootKeyProvider {
    local_key_path: PathBuf,
}

impl DevelopmentRootKeyProvider {
    pub fn new(local_key_path: impl Into<PathBuf>) -> Self {
        Self {
            local_key_path: local_key_path.into(),
        }
    }
}

impl SecretRootKeyProvider for DevelopmentRootKeyProvider {
    fn metadata(&self) -> RootKeyProviderMetadata {
        RootKeyProviderMetadata::new(DEVELOPMENT_PROVIDER_ID, DEVELOPMENT_PROVIDER_KIND)
    }

    fn load_or_create_root_key(&self) -> Result<SecretRootKey, SecretRootKeyError> {
        if let Ok(value) = std::env::var(DEVELOPMENT_ROOT_KEY_ENV) {
            if !value.trim().is_empty() {
                return Ok(derive_key(value.as_bytes()));
            }
        }

        if self.local_key_path.exists() {
            let mut file = OpenOptions::new()
                .read(true)
                .open(&self.local_key_path)
                .map_err(|err| SecretRootKeyError::DevelopmentStorage(err.to_string()))?;
            let mut encoded = String::new();
            file.read_to_string(&mut encoded)
                .map_err(|err| SecretRootKeyError::DevelopmentStorage(err.to_string()))?;
            return decode_root_key(&encoded);
        }

        if let Some(parent) = self.local_key_path.parent() {
            fs::create_dir_all(parent).map_err(|err| SecretRootKeyError::DevelopmentStorage(err.to_string()))?;
        }
        let root = generate_root_key()?;
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&self.local_key_path)
            .map_err(|err| SecretRootKeyError::DevelopmentStorage(err.to_string()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(fs::Permissions::from_mode(0o600))
                .map_err(|err| SecretRootKeyError::DevelopmentStorage(err.to_string()))?;
        }
        file.write_all(STANDARD.encode(root).as_bytes())
            .map_err(|err| SecretRootKeyError::DevelopmentStorage(err.to_string()))?;
        Ok(derive_key(&root))
    }
}

fn generate_root_key() -> Result<[u8; 32], SecretRootKeyError> {
    let rng = rand::SystemRandom::new();
    let mut root = [0_u8; 32];
    rand::SecureRandom::fill(&rng, &mut root)
        .map_err(|_| SecretRootKeyError::InvalidMaterial("root key generation failed".to_string()))?;
    Ok(root)
}

fn decode_root_key(encoded: &str) -> Result<SecretRootKey, SecretRootKeyError> {
    let decoded = STANDARD
        .decode(encoded.trim())
        .map_err(|err| SecretRootKeyError::InvalidMaterial(err.to_string()))?;
    Ok(derive_key(&decoded))
}

fn derive_key(material: &[u8]) -> SecretRootKey {
    Sha256::digest(material).into()
}

#[cfg(target_os = "macos")]
fn os_provider_id() -> &'static str {
    "macos-keychain"
}

#[cfg(target_os = "windows")]
fn os_provider_id() -> &'static str {
    "windows-credential-manager"
}

#[cfg(target_os = "linux")]
fn os_provider_id() -> &'static str {
    "linux-secret-service"
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn os_provider_id() -> &'static str {
    "unsupported-os-secure-storage"
}

#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
fn load_or_create_os_root_key(
    service: &str,
    user: &str,
) -> Result<SecretRootKey, SecretRootKeyError> {
    let entry =
        keyring::Entry::new(service, user).map_err(|err| SecretRootKeyError::ProviderUnavailable(err.to_string()))?;
    match entry.get_password() {
        Ok(encoded) => decode_root_key(&encoded),
        Err(keyring::Error::NoEntry) => {
            let root = generate_root_key()?;
            entry
                .set_password(&STANDARD.encode(root))
                .map_err(|err| SecretRootKeyError::ProviderUnavailable(err.to_string()))?;
            Ok(derive_key(&root))
        }
        Err(err) => Err(SecretRootKeyError::ProviderUnavailable(err.to_string())),
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn load_or_create_os_root_key(
    _service: &str,
    _user: &str,
) -> Result<SecretRootKey, SecretRootKeyError> {
    Err(SecretRootKeyError::ProviderUnavailable(
        "unsupported platform".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    use tempfile::TempDir;

    struct EnvVarGuard {
        key: &'static str,
        original: Option<String>,
    }

    impl EnvVarGuard {
        fn set(
            key: &'static str,
            value: &str,
        ) -> Self {
            let original = std::env::var(key).ok();
            // SAFETY: these serial unit tests own this process key while they run.
            unsafe { std::env::set_var(key, value) };
            Self { key, original }
        }

        fn unset(key: &'static str) -> Self {
            let original = std::env::var(key).ok();
            // SAFETY: these serial unit tests own this process key while they run.
            unsafe { std::env::remove_var(key) };
            Self { key, original }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            // SAFETY: this restores the process key captured by the test guard.
            unsafe {
                match self.original.as_ref() {
                    Some(value) => std::env::set_var(self.key, value),
                    None => std::env::remove_var(self.key),
                }
            }
        }
    }

    #[test]
    fn default_root_key_provider_uses_os_custody_metadata() {
        let metadata = default_root_key_provider().metadata();

        assert_eq!(metadata.provider_kind(), OS_PROVIDER_KIND);
        assert_ne!(metadata.provider_id(), DEVELOPMENT_PROVIDER_ID);
    }

    #[test]
    #[serial_test::serial]
    fn development_provider_uses_env_key_without_local_file() {
        let _env = EnvVarGuard::set(DEVELOPMENT_ROOT_KEY_ENV, "deterministic development key");
        let temp_dir = TempDir::new().expect("temp dir");
        let key_path = temp_dir.path().join("secrets").join("local-root.key");
        let provider = DevelopmentRootKeyProvider::new(&key_path);

        let key = provider.load_or_create_root_key().expect("load env root key");

        assert_eq!(key, derive_key(b"deterministic development key"));
        assert!(!key_path.exists());
    }

    #[test]
    #[serial_test::serial]
    fn development_provider_local_file_requires_explicit_provider() {
        let _env = EnvVarGuard::unset(DEVELOPMENT_ROOT_KEY_ENV);
        let temp_dir = TempDir::new().expect("temp dir");
        let key_path = temp_dir.path().join("secrets").join("local-root.key");
        let provider = DevelopmentRootKeyProvider::new(&key_path);

        let _key = provider
            .load_or_create_root_key()
            .expect("create local development root key");

        assert!(key_path.exists());
    }

    #[test]
    #[serial_test::serial]
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    fn os_root_key_provider_loads_or_creates_root_key_when_enabled() {
        if std::env::var("MCPMATE_RUN_OS_KEYRING_TESTS").as_deref() != Ok("1") {
            return;
        }

        let unique_id = format!(
            "{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time after epoch")
                .as_nanos()
        );
        let service = format!("{OS_KEYRING_SERVICE}.test.{unique_id}");
        let user = format!("{OS_KEYRING_USER}.test");
        let entry = keyring::Entry::new(&service, &user).expect("create OS keyring entry");
        let _ = entry.delete_credential();

        let provider = OperatingSystemRootKeyProvider::with_keyring_entry(&service, &user);
        let first = provider.load_or_create_root_key().expect("create OS-backed root key");
        let second = provider.load_or_create_root_key().expect("load OS-backed root key");

        assert!(first.iter().any(|byte| *byte != 0));
        assert_eq!(first, second);

        let _ = entry.delete_credential();
    }
}
