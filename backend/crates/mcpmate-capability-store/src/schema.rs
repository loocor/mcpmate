use sqlx::{Pool, Sqlite};

use crate::{CatalogError, Result};

pub(crate) async fn ensure_schema(pool: &Pool<Sqlite>) -> Result<()> {
    mcpmate_migrations::verify_capability_catalog_database(pool)
        .await
        .map_err(|error| CatalogError::IncompatibleSchema {
            details: error.to_string(),
        })
}
