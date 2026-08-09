use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use serde_json::Value;
use sqlx::{Sqlite, Transaction};

use super::super::{Migration, MigrationStep};

pub(super) fn migration() -> Migration {
    Migration::rust(
        13,
        "canonicalize unrecognized server transport",
        &[include_str!("v0013_canonicalize_unrecognized_server_transport.rs")],
        CanonicalizeUnrecognizedServerTransport,
    )
}

struct CanonicalizeUnrecognizedServerTransport;

#[async_trait]
impl MigrationStep for CanonicalizeUnrecognizedServerTransport {
    async fn apply(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
    ) -> Result<()> {
        let servers: Vec<(String, String, Option<String>)> = sqlx::query_as(
            "SELECT server_config.id, server_config.server_type, server_transport.draft_json
             FROM server_config
             LEFT JOIN server_transport ON server_transport.server_id = server_config.id
             WHERE server_config.server_type NOT IN ('stdio', 'sse', 'streamable_http')
             ORDER BY server_config.id",
        )
        .fetch_all(&mut **transaction)
        .await
        .context("load unrecognized legacy server transport projections")?;

        for (server_id, server_type, draft_json) in servers {
            verify_unrecognized_transport(transaction, &server_id, &server_type, draft_json.as_deref()).await?;
            let result = sqlx::query("UPDATE server_config SET server_type = 'stdio' WHERE id = ? AND server_type = ?")
                .bind(&server_id)
                .bind(&server_type)
                .execute(&mut **transaction)
                .await
                .with_context(|| format!("canonicalize unrecognized server transport projection for '{server_id}'"))?;
            if result.rows_affected() != 1 {
                bail!("unrecognized server transport projection changed while migrating '{server_id}'");
            }
        }

        Ok(())
    }
}

async fn verify_unrecognized_transport(
    transaction: &mut Transaction<'_, Sqlite>,
    server_id: &str,
    server_type: &str,
    draft_json: Option<&str>,
) -> Result<()> {
    let draft_json = draft_json
        .with_context(|| format!("unrecognized server '{server_id}' is missing its structured transport draft"))?;
    let draft: Value = serde_json::from_str(draft_json)
        .with_context(|| format!("decode structured transport draft for unrecognized server '{server_id}'"))?;
    let draft = draft
        .as_object()
        .with_context(|| format!("unrecognized server '{server_id}' has an incompatible structured transport draft"))?;
    if draft.get("kind").and_then(Value::as_str) != Some("unrecognized") {
        bail!("unrecognized server '{server_id}' has an incompatible structured transport draft");
    }
    let declared_type = draft
        .get("declared_type")
        .and_then(Value::as_str)
        .with_context(|| format!("unrecognized server '{server_id}' has an incompatible structured transport draft"))?;
    if declared_type != server_type {
        bail!(
            "unrecognized server '{server_id}' transport draft declared type '{declared_type}' does not match legacy type '{server_type}'"
        );
    }

    let audit: Option<(String, String)> = sqlx::query_as(
        "SELECT original_shape_json, diagnostic_codes_json
         FROM server_config_migration_audit WHERE server_id = ?",
    )
    .bind(server_id)
    .fetch_optional(&mut **transaction)
    .await
    .with_context(|| format!("load migration audit for unrecognized server '{server_id}'"))?;
    let (original_shape_json, diagnostic_codes_json) =
        audit.with_context(|| format!("unrecognized server '{server_id}' is missing its migration audit"))?;
    let original_shape: Value = serde_json::from_str(&original_shape_json)
        .with_context(|| format!("decode migration audit for unrecognized server '{server_id}'"))?;
    let audited_type = original_shape
        .get("server_type")
        .and_then(Value::as_str)
        .with_context(|| format!("unrecognized server '{server_id}' has an incompatible migration audit"))?;
    if audited_type != server_type {
        bail!(
            "unrecognized server '{server_id}' migration audit type '{audited_type}' does not match legacy type '{server_type}'"
        );
    }
    let diagnostics: Value = serde_json::from_str(&diagnostic_codes_json)
        .with_context(|| format!("decode migration diagnostics for unrecognized server '{server_id}'"))?;
    let is_unrecognized = diagnostics
        .as_array()
        .is_some_and(|codes| codes.iter().any(|code| code.as_str() == Some("transport_unrecognized")));
    if !is_unrecognized {
        bail!("unrecognized server '{server_id}' has an incompatible migration audit");
    }
    Ok(())
}
