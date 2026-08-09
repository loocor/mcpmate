use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::{Map, Value, json};
use sqlx::{Sqlite, Transaction};
use url::Url;

use super::super::{Migration, MigrationStep};

const SCHEMA: &str = include_str!("v0012_create_structured_server_transport.sql");

pub(super) fn migration() -> Migration {
    Migration::rust(
        12,
        "create structured server transport",
        &[include_str!("v0012_create_structured_server_transport.rs"), SCHEMA],
        CreateStructuredServerTransport,
    )
}

struct CreateStructuredServerTransport;

#[async_trait]
impl MigrationStep for CreateStructuredServerTransport {
    async fn apply(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
    ) -> Result<()> {
        for statement in SCHEMA.split(";\n").filter(|statement| !statement.trim().is_empty()) {
            sqlx::query(statement)
                .execute(&mut **transaction)
                .await
                .context("create structured server transport storage")?;
        }

        let servers: Vec<(String, String, Option<String>, Option<String>)> =
            sqlx::query_as("SELECT id, server_type, command, url FROM server_config ORDER BY id")
                .fetch_all(&mut **transaction)
                .await
                .context("load legacy server configurations")?;
        for (server_id, server_type, command, url) in servers {
            migrate_server(transaction, &server_id, &server_type, command, url).await?;
        }
        Ok(())
    }
}

async fn migrate_server(
    transaction: &mut Transaction<'_, Sqlite>,
    server_id: &str,
    server_type: &str,
    command: Option<String>,
    url: Option<String>,
) -> Result<()> {
    let args: Vec<String> =
        sqlx::query_scalar("SELECT arg_value FROM server_args WHERE server_id = ? ORDER BY arg_index")
            .bind(server_id)
            .fetch_all(&mut **transaction)
            .await
            .context("load legacy server arguments")?;
    let env: Vec<(String, String)> =
        sqlx::query_as("SELECT env_key, env_value FROM server_env WHERE server_id = ? ORDER BY env_key")
            .bind(server_id)
            .fetch_all(&mut **transaction)
            .await
            .context("load legacy server environment")?;
    let headers: Vec<(String, String)> =
        sqlx::query_as("SELECT header_key, header_value FROM server_headers WHERE server_id = ? ORDER BY header_key")
            .bind(server_id)
            .fetch_all(&mut **transaction)
            .await
            .context("load legacy server headers")?;

    let mut ignored_fields = Vec::new();
    let mut diagnostics = Vec::new();
    let draft = match server_type {
        "stdio" => {
            if command.as_deref().is_none_or(|value| value.trim().is_empty()) {
                diagnostics.push("stdio_command_missing");
            }
            if present(&url) {
                ignored_fields.push("url");
            }
            if !headers.is_empty() {
                ignored_fields.push("headers");
            }
            json!({
                "kind": "stdio",
                "command": command,
                "args": args,
                "env": config_values(&env),
            })
        }
        "sse" | "streamable_http" => {
            if url.as_deref().is_none_or(|value| value.trim().is_empty()) {
                diagnostics.push("remote_url_missing");
            } else if !valid_http_endpoint(url.as_deref().expect("checked above")) {
                diagnostics.push("url_invalid");
            }
            if present(&command) {
                ignored_fields.push("command");
            }
            if !args.is_empty() {
                ignored_fields.push("args");
            }
            if !env.is_empty() {
                ignored_fields.push("env");
            }
            json!({
                "kind": "http",
                "protocol": server_type,
                "endpoint": url,
                "headers": config_values(&headers),
            })
        }
        _ => {
            diagnostics.push("transport_unrecognized");
            json!({ "kind": "unrecognized", "declared_type": server_type })
        }
    };
    if !ignored_fields.is_empty() {
        diagnostics.push("transport_field_conflict");
    }
    ignored_fields.sort_unstable();
    diagnostics.sort_unstable();

    sqlx::query("INSERT INTO server_transport (server_id, draft_json) VALUES (?, ?)")
        .bind(server_id)
        .bind(serde_json::to_string(&draft).context("serialize structured transport draft")?)
        .execute(&mut **transaction)
        .await
        .context("store structured transport draft")?;

    if ignored_fields.is_empty() && diagnostics.is_empty() {
        return Ok(());
    }
    let original_shape = json!({
        "server_type": server_type,
        "command_present": present(&command),
        "url_present": present(&url),
        "arg_count": args.len(),
        "env_keys": env.iter().map(|(key, _)| key).collect::<Vec<_>>(),
        "header_keys": headers.iter().map(|(key, _)| key).collect::<Vec<_>>(),
    });
    sqlx::query(
        "INSERT INTO server_config_migration_audit (
            server_id, original_shape_json, ignored_field_names_json, diagnostic_codes_json
         ) VALUES (?, ?, ?, ?)",
    )
    .bind(server_id)
    .bind(serde_json::to_string(&original_shape).context("serialize redacted migration audit")?)
    .bind(serde_json::to_string(&ignored_fields).context("serialize ignored field names")?)
    .bind(serde_json::to_string(&diagnostics).context("serialize migration diagnostics")?)
    .execute(&mut **transaction)
    .await
    .context("store redacted migration audit")?;
    Ok(())
}

fn present(value: &Option<String>) -> bool {
    value.as_deref().is_some_and(|value| !value.trim().is_empty())
}

fn valid_http_endpoint(value: &str) -> bool {
    Url::parse(value).is_ok_and(|url| matches!(url.scheme(), "http" | "https"))
}

fn config_values(values: &[(String, String)]) -> Value {
    let values = values
        .iter()
        .map(|(key, value)| (key.clone(), config_value(value)))
        .collect::<Map<String, Value>>();
    Value::Object(values)
}

fn config_value(value: &str) -> Value {
    let secret_alias = value
        .strip_prefix("[[secret:")
        .and_then(|value| value.strip_suffix("]]"))
        .filter(|alias| !alias.is_empty());
    match secret_alias {
        Some(alias) => json!({ "kind": "secret_ref", "alias": alias }),
        None => json!({ "kind": "literal", "value": value }),
    }
}
