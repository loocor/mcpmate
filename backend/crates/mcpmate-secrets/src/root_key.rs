use std::{
    fmt,
    fs::{self, OpenOptions},
    io::{Read, Write},
    num::NonZeroU32,
    path::{Path, PathBuf},
    sync::Arc,
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use ring::{aead, pbkdf2, rand};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use zeroize::Zeroize;

use crate::constants::{DEVELOPMENT_PROVIDER_ID, DEVELOPMENT_PROVIDER_KIND, OS_PROVIDER_KIND};

const OS_KEYRING_SERVICE: &str = "ai.umate.mcpmate.secure-store";
const OS_KEYRING_USER: &str = "secure-store-root-key";
pub const PASSPHRASE_PROVIDER_ID: &str = "master-password-local-root-key";
pub const PASSPHRASE_PROVIDER_KIND: &str = "passphrase_wrapped_root_key";
pub const LOCAL_FILE_PROVIDER_ID: &str = "local-file-root-key";
pub const LOCAL_FILE_PROVIDER_KIND: &str = "local_file_root_key";
const PASSPHRASE_FILE_VERSION: u32 = 1;
const PASSPHRASE_KDF_NAME: &str = "pbkdf2-hmac-sha256";
const PASSPHRASE_KDF_ITERATIONS: u32 = 600_000;
const PASSPHRASE_ROOT_KEY_AAD: &[u8] = b"mcpmate-secrets:v1:passphrase-root-key";
const PASSPHRASE_SALT_LEN: usize = 16;
const AES_256_GCM_NONCE_LEN: usize = 12;

pub const DEVELOPMENT_ROOT_KEY_ENV: &str = "MCPMATE_SECRETS_LOCAL_KEY";

pub type SecretRootKey = [u8; 32];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootKeyProviderMode {
    OperatingSystem,
    Passphrase,
    LocalFile,
    Development,
    Custom,
}

impl RootKeyProviderMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OperatingSystem => "operating_system",
            Self::Passphrase => "passphrase",
            Self::LocalFile => "local_file",
            Self::Development => "development",
            Self::Custom => "custom",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootKeySecurityLevel {
    Recommended,
    UserManaged,
    BasicLocal,
    Development,
    Custom,
}

impl RootKeySecurityLevel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Recommended => "recommended",
            Self::UserManaged => "user_managed",
            Self::BasicLocal => "basic_local",
            Self::Development => "development",
            Self::Custom => "custom",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootKeyProviderMetadata {
    provider_id: &'static str,
    provider_kind: &'static str,
    mode: RootKeyProviderMode,
    security_level: RootKeySecurityLevel,
}

impl RootKeyProviderMetadata {
    pub const fn new(
        provider_id: &'static str,
        provider_kind: &'static str,
    ) -> Self {
        Self::with_mode(
            provider_id,
            provider_kind,
            RootKeyProviderMode::Custom,
            RootKeySecurityLevel::Custom,
        )
    }

    pub const fn with_mode(
        provider_id: &'static str,
        provider_kind: &'static str,
        mode: RootKeyProviderMode,
        security_level: RootKeySecurityLevel,
    ) -> Self {
        Self {
            provider_id,
            provider_kind,
            mode,
            security_level,
        }
    }

    pub fn provider_id(&self) -> &'static str {
        self.provider_id
    }

    pub fn provider_kind(&self) -> &'static str {
        self.provider_kind
    }

    pub fn mode(&self) -> RootKeyProviderMode {
        self.mode
    }

    pub fn security_level(&self) -> RootKeySecurityLevel {
        self.security_level
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SecretRootKeyError {
    #[error("operating-system secure storage is unavailable: {0}")]
    ProviderUnavailable(String),
    #[error("root key material is missing: {0}")]
    MissingMaterial(String),
    #[error("invalid root key material: {0}")]
    InvalidMaterial(String),
    #[error("local root key storage failed: {0}")]
    LocalStorage(String),
    #[error("development root key storage failed: {0}")]
    DevelopmentStorage(String),
}

pub trait SecretRootKeyProvider: fmt::Debug + Send + Sync {
    fn metadata(&self) -> RootKeyProviderMetadata;
    fn load_existing_root_key(&self) -> Result<SecretRootKey, SecretRootKeyError>;
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

    /// Store an existing root key in the OS keyring.
    /// Used during provider migration.
    pub fn set_root_key(
        &self,
        root_key: &SecretRootKey,
    ) -> Result<(), SecretRootKeyError> {
        let entry = keyring::Entry::new(&self.service, &self.user)
            .map_err(|err| SecretRootKeyError::ProviderUnavailable(format!("keyring entry: {err}")))?;
        entry
            .set_password(&STANDARD.encode(root_key))
            .map_err(|err| SecretRootKeyError::ProviderUnavailable(format!("keyring set: {err}")))?;
        Ok(())
    }
}

impl Default for OperatingSystemRootKeyProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretRootKeyProvider for OperatingSystemRootKeyProvider {
    fn metadata(&self) -> RootKeyProviderMetadata {
        RootKeyProviderMetadata::with_mode(
            os_provider_id(),
            OS_PROVIDER_KIND,
            RootKeyProviderMode::OperatingSystem,
            RootKeySecurityLevel::Recommended,
        )
    }

    fn load_or_create_root_key(&self) -> Result<SecretRootKey, SecretRootKeyError> {
        load_or_create_os_root_key(&self.service, &self.user)
    }

    fn load_existing_root_key(&self) -> Result<SecretRootKey, SecretRootKeyError> {
        load_existing_os_root_key(&self.service, &self.user)
    }
}

#[derive(Clone)]
pub struct PassphraseRootKeyProvider {
    wrapped_key_path: PathBuf,
    passphrase: String,
}

impl fmt::Debug for PassphraseRootKeyProvider {
    fn fmt(
        &self,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        formatter
            .debug_struct("PassphraseRootKeyProvider")
            .field("wrapped_key_path", &self.wrapped_key_path)
            .field("passphrase", &"<redacted>")
            .finish()
    }
}

impl PassphraseRootKeyProvider {
    pub fn new(
        wrapped_key_path: impl Into<PathBuf>,
        passphrase: impl Into<String>,
    ) -> Self {
        Self {
            wrapped_key_path: wrapped_key_path.into(),
            passphrase: passphrase.into(),
        }
    }

    /// Store an existing root key wrapped with this provider's passphrase.
    /// Used during provider migration — writes the wrapped key file.
    pub fn set_root_key(
        &self,
        root_key: &SecretRootKey,
    ) -> Result<(), SecretRootKeyError> {
        if self.passphrase.is_empty() {
            return Err(SecretRootKeyError::InvalidMaterial(
                "passphrase cannot be empty".to_string(),
            ));
        }
        if let Some(parent) = self.wrapped_key_path.parent() {
            fs::create_dir_all(parent).map_err(|err| SecretRootKeyError::LocalStorage(err.to_string()))?;
        }
        let salt = generate_random_bytes(PASSPHRASE_SALT_LEN)?;
        let nonce = generate_random_bytes(AES_256_GCM_NONCE_LEN)?;
        let wrapping_key = derive_passphrase_wrapping_key(&self.passphrase, &salt, PASSPHRASE_KDF_ITERATIONS)?;
        let encrypted_root_key = encrypt_root_material(root_key, &wrapping_key, &nonce)?;
        let serialized = serde_json::to_vec_pretty(&PassphraseRootKeyFile {
            version: PASSPHRASE_FILE_VERSION,
            kdf: PASSPHRASE_KDF_NAME.to_string(),
            iterations: PASSPHRASE_KDF_ITERATIONS,
            salt: STANDARD.encode(salt),
            nonce: STANDARD.encode(nonce),
            encrypted_root_key: STANDARD.encode(encrypted_root_key),
        })
        .map_err(|err| SecretRootKeyError::LocalStorage(err.to_string()))?;
        write_secret_file_replace(&self.wrapped_key_path, &serialized)?;
        Ok(())
    }
}

impl Drop for PassphraseRootKeyProvider {
    fn drop(&mut self) {
        self.passphrase.zeroize();
    }
}

impl SecretRootKeyProvider for PassphraseRootKeyProvider {
    fn metadata(&self) -> RootKeyProviderMetadata {
        RootKeyProviderMetadata::with_mode(
            PASSPHRASE_PROVIDER_ID,
            PASSPHRASE_PROVIDER_KIND,
            RootKeyProviderMode::Passphrase,
            RootKeySecurityLevel::UserManaged,
        )
    }

    fn load_or_create_root_key(&self) -> Result<SecretRootKey, SecretRootKeyError> {
        load_or_create_passphrase_root_key(&self.wrapped_key_path, &self.passphrase)
    }

    fn load_existing_root_key(&self) -> Result<SecretRootKey, SecretRootKeyError> {
        load_existing_passphrase_root_key(&self.wrapped_key_path, &self.passphrase)
    }
}

#[derive(Debug, Clone)]
pub struct LocalFileRootKeyProvider {
    local_key_path: PathBuf,
}

impl LocalFileRootKeyProvider {
    pub fn new(local_key_path: impl Into<PathBuf>) -> Self {
        Self {
            local_key_path: local_key_path.into(),
        }
    }

    /// Store an existing root key as a local file.
    /// Used during provider migration.
    pub fn set_root_key(
        &self,
        root_key: &SecretRootKey,
    ) -> Result<(), SecretRootKeyError> {
        if let Some(parent) = self.local_key_path.parent() {
            fs::create_dir_all(parent).map_err(|err| SecretRootKeyError::LocalStorage(err.to_string()))?;
        }
        write_secret_file_replace(&self.local_key_path, STANDARD.encode(root_key).as_bytes())?;
        Ok(())
    }
}

impl SecretRootKeyProvider for LocalFileRootKeyProvider {
    fn metadata(&self) -> RootKeyProviderMetadata {
        RootKeyProviderMetadata::with_mode(
            LOCAL_FILE_PROVIDER_ID,
            LOCAL_FILE_PROVIDER_KIND,
            RootKeyProviderMode::LocalFile,
            RootKeySecurityLevel::BasicLocal,
        )
    }

    fn load_or_create_root_key(&self) -> Result<SecretRootKey, SecretRootKeyError> {
        load_or_create_local_file_root_key(&self.local_key_path)
    }

    fn load_existing_root_key(&self) -> Result<SecretRootKey, SecretRootKeyError> {
        load_existing_local_file_root_key(&self.local_key_path)
    }
}

#[derive(Debug, Clone)]
pub struct DevelopmentRootKeyProvider {
    fallback: LocalFileRootKeyProvider,
}

impl DevelopmentRootKeyProvider {
    pub fn new(local_key_path: impl Into<PathBuf>) -> Self {
        Self {
            fallback: LocalFileRootKeyProvider::new(local_key_path),
        }
    }
}

impl SecretRootKeyProvider for DevelopmentRootKeyProvider {
    fn metadata(&self) -> RootKeyProviderMetadata {
        RootKeyProviderMetadata::with_mode(
            DEVELOPMENT_PROVIDER_ID,
            DEVELOPMENT_PROVIDER_KIND,
            RootKeyProviderMode::Development,
            RootKeySecurityLevel::Development,
        )
    }

    fn load_or_create_root_key(&self) -> Result<SecretRootKey, SecretRootKeyError> {
        if let Ok(value) = std::env::var(DEVELOPMENT_ROOT_KEY_ENV) {
            if !value.trim().is_empty() {
                return Ok(derive_key(value.as_bytes()));
            }
        }

        self.fallback.load_or_create_root_key().map_err(|err| match err {
            SecretRootKeyError::LocalStorage(message) => SecretRootKeyError::DevelopmentStorage(message),
            other => other,
        })
    }

    fn load_existing_root_key(&self) -> Result<SecretRootKey, SecretRootKeyError> {
        if let Ok(value) = std::env::var(DEVELOPMENT_ROOT_KEY_ENV) {
            if !value.trim().is_empty() {
                return Ok(derive_key(value.as_bytes()));
            }
        }

        self.fallback.load_existing_root_key().map_err(|err| match err {
            SecretRootKeyError::LocalStorage(message) => SecretRootKeyError::DevelopmentStorage(message),
            other => other,
        })
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
    if decoded.len() != 32 {
        return Err(SecretRootKeyError::InvalidMaterial(format!(
            "root key material must be 32 bytes, got {}",
            decoded.len()
        )));
    }
    Ok(derive_key(&decoded))
}

fn derive_key(material: &[u8]) -> SecretRootKey {
    Sha256::digest(material).into()
}

fn load_or_create_local_file_root_key(local_key_path: &Path) -> Result<SecretRootKey, SecretRootKeyError> {
    if local_key_path.exists() {
        return read_local_file_root_key(local_key_path);
    }

    if let Some(parent) = local_key_path.parent() {
        fs::create_dir_all(parent).map_err(|err| SecretRootKeyError::LocalStorage(err.to_string()))?;
    }
    let root = generate_root_key()?;
    write_secret_file(local_key_path, STANDARD.encode(root).as_bytes())?;
    Ok(derive_key(&root))
}

fn load_existing_local_file_root_key(local_key_path: &Path) -> Result<SecretRootKey, SecretRootKeyError> {
    if !local_key_path.exists() {
        return Err(SecretRootKeyError::MissingMaterial(format!(
            "local root key file '{}' does not exist",
            local_key_path.display()
        )));
    }
    read_local_file_root_key(local_key_path)
}

fn read_local_file_root_key(local_key_path: &Path) -> Result<SecretRootKey, SecretRootKeyError> {
    let mut file = OpenOptions::new()
        .read(true)
        .open(local_key_path)
        .map_err(|err| SecretRootKeyError::LocalStorage(err.to_string()))?;
    let mut encoded = String::new();
    file.read_to_string(&mut encoded)
        .map_err(|err| SecretRootKeyError::LocalStorage(err.to_string()))?;
    decode_root_key(&encoded)
}

#[derive(Debug, Serialize, Deserialize)]
struct PassphraseRootKeyFile {
    version: u32,
    kdf: String,
    iterations: u32,
    salt: String,
    nonce: String,
    encrypted_root_key: String,
}

fn load_or_create_passphrase_root_key(
    wrapped_key_path: &Path,
    passphrase: &str,
) -> Result<SecretRootKey, SecretRootKeyError> {
    if passphrase.is_empty() {
        return Err(SecretRootKeyError::InvalidMaterial(
            "passphrase cannot be empty".to_string(),
        ));
    }

    if wrapped_key_path.exists() {
        return read_passphrase_root_key(wrapped_key_path, passphrase);
    }

    if let Some(parent) = wrapped_key_path.parent() {
        fs::create_dir_all(parent).map_err(|err| SecretRootKeyError::LocalStorage(err.to_string()))?;
    }

    let root = generate_root_key()?;
    let salt = generate_random_bytes(PASSPHRASE_SALT_LEN)?;
    let nonce = generate_random_bytes(AES_256_GCM_NONCE_LEN)?;
    let wrapping_key = derive_passphrase_wrapping_key(passphrase, &salt, PASSPHRASE_KDF_ITERATIONS)?;
    let encrypted_root_key = encrypt_root_material(&root, &wrapping_key, &nonce)?;
    let serialized = serde_json::to_vec_pretty(&PassphraseRootKeyFile {
        version: PASSPHRASE_FILE_VERSION,
        kdf: PASSPHRASE_KDF_NAME.to_string(),
        iterations: PASSPHRASE_KDF_ITERATIONS,
        salt: STANDARD.encode(salt),
        nonce: STANDARD.encode(nonce),
        encrypted_root_key: STANDARD.encode(encrypted_root_key),
    })
    .map_err(|err| SecretRootKeyError::LocalStorage(err.to_string()))?;
    write_secret_file(wrapped_key_path, &serialized)?;
    Ok(derive_key(&root))
}

fn load_existing_passphrase_root_key(
    wrapped_key_path: &Path,
    passphrase: &str,
) -> Result<SecretRootKey, SecretRootKeyError> {
    if passphrase.is_empty() {
        return Err(SecretRootKeyError::InvalidMaterial(
            "passphrase cannot be empty".to_string(),
        ));
    }

    if !wrapped_key_path.exists() {
        return Err(SecretRootKeyError::MissingMaterial(format!(
            "passphrase root key file '{}' does not exist",
            wrapped_key_path.display()
        )));
    }

    read_passphrase_root_key(wrapped_key_path, passphrase)
}

fn read_passphrase_root_key(
    wrapped_key_path: &Path,
    passphrase: &str,
) -> Result<SecretRootKey, SecretRootKeyError> {
    let mut file = OpenOptions::new()
        .read(true)
        .open(wrapped_key_path)
        .map_err(|err| SecretRootKeyError::LocalStorage(err.to_string()))?;
    let mut serialized = String::new();
    file.read_to_string(&mut serialized)
        .map_err(|err| SecretRootKeyError::LocalStorage(err.to_string()))?;
    unwrap_passphrase_root_key(&serialized, passphrase)
}

fn unwrap_passphrase_root_key(
    serialized: &str,
    passphrase: &str,
) -> Result<SecretRootKey, SecretRootKeyError> {
    let file: PassphraseRootKeyFile =
        serde_json::from_str(serialized).map_err(|err| SecretRootKeyError::InvalidMaterial(err.to_string()))?;
    if file.version != PASSPHRASE_FILE_VERSION {
        return Err(SecretRootKeyError::InvalidMaterial(format!(
            "unsupported passphrase root key file version {}",
            file.version
        )));
    }
    if file.kdf != PASSPHRASE_KDF_NAME {
        return Err(SecretRootKeyError::InvalidMaterial(format!(
            "unsupported passphrase root key kdf '{}'",
            file.kdf
        )));
    }

    let salt = STANDARD
        .decode(file.salt)
        .map_err(|err| SecretRootKeyError::InvalidMaterial(err.to_string()))?;
    let nonce = STANDARD
        .decode(file.nonce)
        .map_err(|err| SecretRootKeyError::InvalidMaterial(err.to_string()))?;
    let encrypted_root_key = STANDARD
        .decode(file.encrypted_root_key)
        .map_err(|err| SecretRootKeyError::InvalidMaterial(err.to_string()))?;
    let wrapping_key = derive_passphrase_wrapping_key(passphrase, &salt, file.iterations)?;
    let root = decrypt_root_material(&encrypted_root_key, &wrapping_key, &nonce)?;
    Ok(derive_key(&root))
}

fn derive_passphrase_wrapping_key(
    passphrase: &str,
    salt: &[u8],
    iterations: u32,
) -> Result<SecretRootKey, SecretRootKeyError> {
    let iterations = NonZeroU32::new(iterations)
        .ok_or_else(|| SecretRootKeyError::InvalidMaterial("passphrase kdf iterations cannot be zero".to_string()))?;
    let mut key = [0_u8; 32];
    pbkdf2::derive(
        pbkdf2::PBKDF2_HMAC_SHA256,
        iterations,
        salt,
        passphrase.as_bytes(),
        &mut key,
    );
    Ok(key)
}

fn encrypt_root_material(
    root: &SecretRootKey,
    wrapping_key: &SecretRootKey,
    nonce: &[u8],
) -> Result<Vec<u8>, SecretRootKeyError> {
    let key = aead_key(wrapping_key)?;
    let nonce = aead_nonce(nonce)?;
    let mut in_out = root.to_vec();
    key.seal_in_place_append_tag(nonce, aead::Aad::from(PASSPHRASE_ROOT_KEY_AAD), &mut in_out)
        .map_err(|_| SecretRootKeyError::InvalidMaterial("encrypt passphrase root key".to_string()))?;
    Ok(in_out)
}

fn decrypt_root_material(
    encrypted: &[u8],
    wrapping_key: &SecretRootKey,
    nonce: &[u8],
) -> Result<SecretRootKey, SecretRootKeyError> {
    let key = aead_key(wrapping_key)?;
    let nonce = aead_nonce(nonce)?;
    let mut in_out = encrypted.to_vec();
    let plaintext = key
        .open_in_place(nonce, aead::Aad::from(PASSPHRASE_ROOT_KEY_AAD), &mut in_out)
        .map_err(|_| SecretRootKeyError::InvalidMaterial("passphrase did not unwrap root key".to_string()))?;
    plaintext
        .try_into()
        .map_err(|_| SecretRootKeyError::InvalidMaterial("invalid root key length".to_string()))
}

fn aead_key(raw_key: &SecretRootKey) -> Result<aead::LessSafeKey, SecretRootKeyError> {
    let key = aead::UnboundKey::new(&aead::AES_256_GCM, raw_key)
        .map_err(|_| SecretRootKeyError::InvalidMaterial("invalid wrapping key".to_string()))?;
    Ok(aead::LessSafeKey::new(key))
}

fn aead_nonce(nonce: &[u8]) -> Result<aead::Nonce, SecretRootKeyError> {
    let nonce: [u8; AES_256_GCM_NONCE_LEN] = nonce
        .try_into()
        .map_err(|_| SecretRootKeyError::InvalidMaterial("invalid root key nonce length".to_string()))?;
    Ok(aead::Nonce::assume_unique_for_key(nonce))
}

fn generate_random_bytes(len: usize) -> Result<Vec<u8>, SecretRootKeyError> {
    let rng = rand::SystemRandom::new();
    let mut bytes = vec![0_u8; len];
    rand::SecureRandom::fill(&rng, &mut bytes)
        .map_err(|_| SecretRootKeyError::InvalidMaterial("secure random generation failed".to_string()))?;
    Ok(bytes)
}

fn write_secret_file(
    path: &Path,
    bytes: &[u8],
) -> Result<(), SecretRootKeyError> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|err| SecretRootKeyError::LocalStorage(err.to_string()))?;
    write_secret_file_contents(&mut file, bytes)
}

/// Overwrite an existing secret file during provider migration.
fn write_secret_file_replace(
    path: &Path,
    bytes: &[u8],
) -> Result<(), SecretRootKeyError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| SecretRootKeyError::LocalStorage(err.to_string()))?;
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .map_err(|err| SecretRootKeyError::LocalStorage(err.to_string()))?;
    write_secret_file_contents(&mut file, bytes)
}

fn write_secret_file_contents(
    file: &mut std::fs::File,
    bytes: &[u8],
) -> Result<(), SecretRootKeyError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|err| SecretRootKeyError::LocalStorage(err.to_string()))?;
    }
    file.write_all(bytes)
        .map_err(|err| SecretRootKeyError::LocalStorage(err.to_string()))
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

#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
fn load_existing_os_root_key(
    service: &str,
    user: &str,
) -> Result<SecretRootKey, SecretRootKeyError> {
    let entry =
        keyring::Entry::new(service, user).map_err(|err| SecretRootKeyError::ProviderUnavailable(err.to_string()))?;
    match entry.get_password() {
        Ok(encoded) => decode_root_key(&encoded),
        Err(keyring::Error::NoEntry) => Err(SecretRootKeyError::MissingMaterial(
            "operating-system root key entry does not exist".to_string(),
        )),
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

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn load_existing_os_root_key(
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
        assert_eq!(metadata.mode(), RootKeyProviderMode::OperatingSystem);
        assert_eq!(metadata.security_level(), RootKeySecurityLevel::Recommended);
    }

    #[test]
    fn passphrase_provider_uses_user_managed_metadata() {
        let temp_dir = TempDir::new().expect("temp dir");
        let key_path = temp_dir.path().join("secrets").join("passphrase-root-key.json");
        let provider = PassphraseRootKeyProvider::new(&key_path, "correct horse battery staple");
        let metadata = provider.metadata();

        assert_eq!(metadata.provider_id(), PASSPHRASE_PROVIDER_ID);
        assert_eq!(metadata.provider_kind(), PASSPHRASE_PROVIDER_KIND);
        assert_eq!(metadata.mode(), RootKeyProviderMode::Passphrase);
        assert_eq!(metadata.security_level(), RootKeySecurityLevel::UserManaged);
    }

    #[test]
    fn local_file_provider_uses_basic_local_metadata() {
        let temp_dir = TempDir::new().expect("temp dir");
        let key_path = temp_dir.path().join("secrets").join("local-root.key");
        let provider = LocalFileRootKeyProvider::new(&key_path);
        let metadata = provider.metadata();

        assert_eq!(metadata.provider_id(), LOCAL_FILE_PROVIDER_ID);
        assert_eq!(metadata.provider_kind(), LOCAL_FILE_PROVIDER_KIND);
        assert_eq!(metadata.mode(), RootKeyProviderMode::LocalFile);
        assert_eq!(metadata.security_level(), RootKeySecurityLevel::BasicLocal);
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
    fn local_file_provider_load_existing_missing_file_does_not_create_root_key() {
        let temp_dir = TempDir::new().expect("temp dir");
        let key_path = temp_dir.path().join("secrets").join("missing-local-root.key");

        let err = LocalFileRootKeyProvider::new(&key_path)
            .load_existing_root_key()
            .expect_err("missing local root key should fail");

        assert!(matches!(err, SecretRootKeyError::MissingMaterial(_)));
        assert!(!key_path.exists());
    }

    #[test]
    #[serial_test::serial]
    fn local_file_provider_rejects_short_root_material() {
        let temp_dir = TempDir::new().expect("temp dir");
        let key_path = temp_dir.path().join("secrets").join("local-root.key");
        fs::create_dir_all(key_path.parent().expect("parent dir")).expect("create parent");
        fs::write(&key_path, STANDARD.encode([42_u8])).expect("write short material");

        let err = LocalFileRootKeyProvider::new(&key_path)
            .load_or_create_root_key()
            .expect_err("short local root material should fail");

        assert_invalid_material(err);
    }

    #[test]
    #[serial_test::serial]
    fn local_file_provider_rejects_long_root_material() {
        let temp_dir = TempDir::new().expect("temp dir");
        let key_path = temp_dir.path().join("secrets").join("local-root.key");
        fs::create_dir_all(key_path.parent().expect("parent dir")).expect("create parent");
        fs::write(&key_path, STANDARD.encode([7_u8; 33])).expect("write long material");

        let err = LocalFileRootKeyProvider::new(&key_path)
            .load_or_create_root_key()
            .expect_err("long local root material should fail");

        assert_invalid_material(err);
    }

    #[test]
    #[serial_test::serial]
    fn local_file_provider_rejects_non_base64_root_material() {
        let temp_dir = TempDir::new().expect("temp dir");
        let key_path = temp_dir.path().join("secrets").join("local-root.key");
        fs::create_dir_all(key_path.parent().expect("parent dir")).expect("create parent");
        fs::write(&key_path, "not base64").expect("write invalid material");

        let err = LocalFileRootKeyProvider::new(&key_path)
            .load_or_create_root_key()
            .expect_err("invalid local root material should fail");

        assert_invalid_material(err);
    }

    #[test]
    #[serial_test::serial]
    fn passphrase_provider_reopens_wrapped_root_key_with_same_passphrase() {
        let temp_dir = TempDir::new().expect("temp dir");
        let key_path = temp_dir.path().join("secrets").join("passphrase-root-key.json");
        let first_provider = PassphraseRootKeyProvider::new(&key_path, "correct horse battery staple");
        let second_provider = PassphraseRootKeyProvider::new(&key_path, "correct horse battery staple");

        let first = first_provider
            .load_or_create_root_key()
            .expect("create passphrase root key");
        let second = second_provider
            .load_or_create_root_key()
            .expect("load passphrase root key");

        assert_eq!(first, second);
        assert!(key_path.exists());
    }

    #[test]
    #[serial_test::serial]
    fn passphrase_provider_load_existing_missing_file_does_not_create_root_key() {
        let temp_dir = TempDir::new().expect("temp dir");
        let key_path = temp_dir.path().join("secrets").join("missing-passphrase-root-key.json");

        let err = PassphraseRootKeyProvider::new(&key_path, "correct horse battery staple")
            .load_existing_root_key()
            .expect_err("missing passphrase root key should fail");

        assert!(matches!(err, SecretRootKeyError::MissingMaterial(_)));
        assert!(!key_path.exists());
    }

    #[test]
    #[serial_test::serial]
    fn passphrase_set_root_key_replaces_existing_wrapped_file() {
        let temp_dir = TempDir::new().expect("temp dir");
        let key_path = temp_dir.path().join("secrets").join("passphrase-root-key.json");
        PassphraseRootKeyProvider::new(&key_path, "old password")
            .load_or_create_root_key()
            .expect("create initial passphrase root key");

        let replacement = generate_root_key().expect("replacement root key");
        PassphraseRootKeyProvider::new(&key_path, "new password")
            .set_root_key(&replacement)
            .expect("replace wrapped root key");

        let loaded = PassphraseRootKeyProvider::new(&key_path, "new password")
            .load_or_create_root_key()
            .expect("load replaced root key");

        assert_eq!(loaded, derive_key(&replacement));
    }

    #[test]
    #[serial_test::serial]
    fn passphrase_provider_rejects_wrong_passphrase() {
        let temp_dir = TempDir::new().expect("temp dir");
        let key_path = temp_dir.path().join("secrets").join("passphrase-root-key.json");
        PassphraseRootKeyProvider::new(&key_path, "correct horse battery staple")
            .load_or_create_root_key()
            .expect("create passphrase root key");

        let err = PassphraseRootKeyProvider::new(&key_path, "wrong password")
            .load_or_create_root_key()
            .expect_err("wrong passphrase should not unwrap root key");

        assert!(matches!(err, SecretRootKeyError::InvalidMaterial(_)));
    }

    #[test]
    #[serial_test::serial]
    fn passphrase_provider_rejects_empty_passphrase() {
        let temp_dir = TempDir::new().expect("temp dir");
        let key_path = temp_dir.path().join("secrets").join("passphrase-root-key.json");

        let err = PassphraseRootKeyProvider::new(&key_path, "")
            .load_or_create_root_key()
            .expect_err("empty passphrase should fail");

        assert_invalid_material(err);
    }

    #[test]
    #[serial_test::serial]
    fn passphrase_provider_rejects_unsupported_version() {
        let temp_dir = TempDir::new().expect("temp dir");
        let key_path = temp_dir.path().join("secrets").join("passphrase-root-key.json");
        let mut file = create_passphrase_root_key_file(&key_path);
        file.version = PASSPHRASE_FILE_VERSION + 1;
        write_passphrase_root_key_file(&key_path, &file);

        let err = PassphraseRootKeyProvider::new(&key_path, "correct horse battery staple")
            .load_or_create_root_key()
            .expect_err("unsupported version should fail");

        assert_invalid_material(err);
    }

    #[test]
    #[serial_test::serial]
    fn passphrase_provider_rejects_unsupported_kdf() {
        let temp_dir = TempDir::new().expect("temp dir");
        let key_path = temp_dir.path().join("secrets").join("passphrase-root-key.json");
        let mut file = create_passphrase_root_key_file(&key_path);
        file.kdf = "scrypt".to_string();
        write_passphrase_root_key_file(&key_path, &file);

        let err = PassphraseRootKeyProvider::new(&key_path, "correct horse battery staple")
            .load_or_create_root_key()
            .expect_err("unsupported kdf should fail");

        assert_invalid_material(err);
    }

    #[test]
    #[serial_test::serial]
    fn passphrase_provider_rejects_zero_iterations() {
        let temp_dir = TempDir::new().expect("temp dir");
        let key_path = temp_dir.path().join("secrets").join("passphrase-root-key.json");
        let mut file = create_passphrase_root_key_file(&key_path);
        file.iterations = 0;
        write_passphrase_root_key_file(&key_path, &file);

        let err = PassphraseRootKeyProvider::new(&key_path, "correct horse battery staple")
            .load_or_create_root_key()
            .expect_err("zero iterations should fail");

        assert_invalid_material(err);
    }

    #[test]
    #[serial_test::serial]
    fn passphrase_provider_rejects_invalid_nonce_length() {
        let temp_dir = TempDir::new().expect("temp dir");
        let key_path = temp_dir.path().join("secrets").join("passphrase-root-key.json");
        let mut file = create_passphrase_root_key_file(&key_path);
        file.nonce = STANDARD.encode([1_u8; AES_256_GCM_NONCE_LEN - 1]);
        write_passphrase_root_key_file(&key_path, &file);

        let err = PassphraseRootKeyProvider::new(&key_path, "correct horse battery staple")
            .load_or_create_root_key()
            .expect_err("invalid nonce should fail");

        assert_invalid_material(err);
    }

    #[test]
    #[serial_test::serial]
    fn passphrase_provider_rejects_corrupted_ciphertext() {
        let temp_dir = TempDir::new().expect("temp dir");
        let key_path = temp_dir.path().join("secrets").join("passphrase-root-key.json");
        let mut file = create_passphrase_root_key_file(&key_path);
        let mut ciphertext = STANDARD.decode(&file.encrypted_root_key).expect("decode ciphertext");
        ciphertext[0] ^= 0x01;
        file.encrypted_root_key = STANDARD.encode(ciphertext);
        write_passphrase_root_key_file(&key_path, &file);

        let err = PassphraseRootKeyProvider::new(&key_path, "correct horse battery staple")
            .load_or_create_root_key()
            .expect_err("corrupted ciphertext should fail");

        assert_invalid_material(err);
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

    fn assert_invalid_material(err: SecretRootKeyError) {
        assert!(matches!(err, SecretRootKeyError::InvalidMaterial(_)));
    }

    fn create_passphrase_root_key_file(key_path: &Path) -> PassphraseRootKeyFile {
        PassphraseRootKeyProvider::new(key_path, "correct horse battery staple")
            .load_or_create_root_key()
            .expect("create passphrase root key");
        let serialized = fs::read_to_string(key_path).expect("read passphrase root key file");
        serde_json::from_str(&serialized).expect("parse passphrase root key file")
    }

    fn write_passphrase_root_key_file(
        key_path: &Path,
        file: &PassphraseRootKeyFile,
    ) {
        let serialized = serde_json::to_vec_pretty(file).expect("serialize passphrase root key file");
        fs::write(key_path, serialized).expect("write passphrase root key file");
    }
}
