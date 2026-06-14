use anyhow::Result;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use ring::{aead, rand};
use zeroize::Zeroize;

use crate::{SecretError, SecretRootKey, SecretValue};

const VALUE_AAD_PREFIX: &[u8] = b"mcpmate-secrets:v1:value:";
const KEY_AAD_PREFIX: &[u8] = b"mcpmate-secrets:v1:key:";

#[derive(Debug, Clone)]
pub(crate) struct EncryptedSecret {
    pub alias: String,
    pub key_nonce: String,
    pub encrypted_key: String,
    pub nonce: String,
    pub encrypted_value: String,
}

pub(crate) struct EncryptedSecretParts {
    pub key_nonce: String,
    pub encrypted_key: String,
    pub nonce: String,
    pub encrypted_value: String,
}

#[derive(Clone)]
pub(crate) struct EnvelopeCrypto {
    root_key: SecretRootKey,
}

impl Drop for EnvelopeCrypto {
    fn drop(&mut self) {
        self.root_key.zeroize();
    }
}

impl EnvelopeCrypto {
    pub fn new(root_key: SecretRootKey) -> Self {
        Self { root_key }
    }

    pub fn encrypt(
        &self,
        alias: &str,
        plaintext: &str,
    ) -> Result<EncryptedSecretParts> {
        let data_key = generate_data_key()?;
        let (key_nonce, encrypted_key) = encrypt_with_key(&self.root_key, &key_aad(alias), &data_key)?;
        let (nonce, encrypted_value) = encrypt_with_key(&data_key, &value_aad(alias), plaintext.as_bytes())?;

        Ok(EncryptedSecretParts {
            key_nonce,
            encrypted_key,
            nonce,
            encrypted_value,
        })
    }

    pub fn decrypt_secret(
        &self,
        encrypted: &EncryptedSecret,
    ) -> Result<SecretValue, SecretError> {
        let data_key = self.unwrap_data_key(encrypted)?;
        let nonce_bytes = STANDARD
            .decode(&encrypted.nonce)
            .map_err(|err| SecretError::InvalidMetadata(format!("invalid secret nonce: {err}")))?;
        let nonce_array: [u8; 12] = nonce_bytes
            .try_into()
            .map_err(|_| SecretError::InvalidMetadata("invalid secret nonce length".to_string()))?;
        let mut in_out = STANDARD
            .decode(&encrypted.encrypted_value)
            .map_err(|err| SecretError::InvalidMetadata(format!("invalid encrypted secret value: {err}")))?;
        let value_key = aead_key(&data_key).map_err(|err| SecretError::InvalidMetadata(err.to_string()))?;
        let plaintext = value_key
            .open_in_place(
                aead::Nonce::assume_unique_for_key(nonce_array),
                aead::Aad::from(value_aad(&encrypted.alias).as_slice()),
                &mut in_out,
            )
            .map_err(|_| SecretError::DecryptionFailed("secret value authentication failed".to_string()))?;
        let value = std::str::from_utf8(plaintext)
            .map_err(|err| SecretError::InvalidMetadata(format!("secret value is not utf-8: {err}")))?;
        Ok(SecretValue::new(value.to_string()))
    }

    pub(crate) fn unwrap_data_key(
        &self,
        encrypted: &EncryptedSecret,
    ) -> Result<SecretRootKey, SecretError> {
        let nonce_bytes = STANDARD
            .decode(&encrypted.key_nonce)
            .map_err(|err| SecretError::InvalidMetadata(format!("invalid secret key nonce: {err}")))?;
        let key_nonce_array: [u8; 12] = nonce_bytes
            .try_into()
            .map_err(|_| SecretError::InvalidMetadata("invalid secret key nonce length".to_string()))?;
        let mut encrypted_key = STANDARD
            .decode(&encrypted.encrypted_key)
            .map_err(|err| SecretError::InvalidMetadata(format!("invalid encrypted secret key: {err}")))?;
        let key = aead_key(&self.root_key).map_err(|err| SecretError::InvalidMetadata(err.to_string()))?;
        let data_key = key
            .open_in_place(
                aead::Nonce::assume_unique_for_key(key_nonce_array),
                aead::Aad::from(key_aad(&encrypted.alias).as_slice()),
                &mut encrypted_key,
            )
            .map_err(|_| SecretError::DecryptionFailed("secret data key authentication failed".to_string()))?;
        data_key
            .try_into()
            .map_err(|_| SecretError::InvalidMetadata("invalid secret data key length".to_string()))
    }

    pub(crate) fn wrap_data_key(
        &self,
        alias: &str,
        data_key: &SecretRootKey,
    ) -> Result<(String, String)> {
        encrypt_with_key(&self.root_key, &key_aad(alias), data_key)
    }
}

fn aead_key(raw_key: &SecretRootKey) -> Result<aead::LessSafeKey> {
    let unbound = aead::UnboundKey::new(&aead::AES_256_GCM, raw_key).map_err(|_| anyhow::anyhow!("build AEAD key"))?;
    Ok(aead::LessSafeKey::new(unbound))
}

fn generate_data_key() -> Result<SecretRootKey> {
    let rng = rand::SystemRandom::new();
    let mut key = [0_u8; 32];
    rand::SecureRandom::fill(&rng, &mut key).map_err(|_| anyhow::anyhow!("generate secret data key"))?;
    Ok(key)
}

fn encrypt_with_key(
    raw_key: &SecretRootKey,
    aad: &[u8],
    plaintext: &[u8],
) -> Result<(String, String)> {
    let rng = rand::SystemRandom::new();
    let mut nonce_bytes = [0_u8; 12];
    rand::SecureRandom::fill(&rng, &mut nonce_bytes).map_err(|_| anyhow::anyhow!("generate secret nonce"))?;

    let key = aead_key(raw_key)?;
    let nonce = aead::Nonce::assume_unique_for_key(nonce_bytes);
    let mut in_out = plaintext.to_vec();
    key.seal_in_place_append_tag(nonce, aead::Aad::from(aad), &mut in_out)
        .map_err(|_| anyhow::anyhow!("encrypt secret value"))?;

    Ok((STANDARD.encode(nonce_bytes), STANDARD.encode(in_out)))
}

fn key_aad(alias: &str) -> Vec<u8> {
    [KEY_AAD_PREFIX, alias.as_bytes()].concat()
}

fn value_aad(alias: &str) -> Vec<u8> {
    [VALUE_AAD_PREFIX, alias.as_bytes()].concat()
}
