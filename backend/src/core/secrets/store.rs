use std::path::PathBuf;

use anyhow::Result;
use sqlx::{Pool, Sqlite};

pub use mcpmate_secrets::store::{
    LocalSecretStore, SecretCreateInput, SecretKindInput, SecretMetadataView, SecretOriginInput, SecretUpdateInput,
    SecretUsageLocationInput, SecretUsageUpsertInput, SecretUsageView,
};

pub async fn initialize_development_secret_store(pool: Pool<Sqlite>) -> Result<LocalSecretStore> {
    LocalSecretStore::initialize_with_development_root_key(pool, development_root_key_path()?).await
}

pub fn development_root_key_path() -> Result<PathBuf> {
    Ok(crate::common::paths::global_paths()
        .base_dir()
        .join("secrets")
        .join("local-root.key"))
}
