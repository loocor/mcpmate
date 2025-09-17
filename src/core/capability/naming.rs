//! Unified naming utilities for capabilities
//! Provides generation, resolution, and uniqueness guarantees for tool/prompt/resource identifiers.

use anyhow::{Context, Result};
use once_cell::sync::OnceCell;
use sqlx::{Pool, Sqlite};
use tracing;

static NAMING_POOL: OnceCell<Pool<Sqlite>> = OnceCell::new();

/// Initialize the global naming store with a database pool.
/// Safe to call multiple times; subsequent calls are ignored.
pub fn initialize(pool: Pool<Sqlite>) {
    if NAMING_POOL.set(pool).is_err() {
        tracing::debug!("Naming store already initialized");
    } else {
        tracing::debug!("Naming store initialized");
    }
}

fn pool() -> &'static Pool<Sqlite> {
    NAMING_POOL
        .get()
        .expect("Naming store not initialized; call naming::initialize first")
}

fn normalize_server_name(server_name: &str) -> String {
    server_name.to_lowercase().replace(' ', "_")
}

/// Capability kinds supported by the naming module.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NamingKind {
    Tool,
    Prompt,
    Resource,
    ResourceTemplate,
}

impl NamingKind {
    fn table(self) -> &'static str {
        match self {
            NamingKind::Tool => "server_tools",
            NamingKind::Prompt => "server_prompts",
            NamingKind::Resource => "server_resources",
            NamingKind::ResourceTemplate => "server_resource_templates",
        }
    }

    fn unique_column(self) -> &'static str {
        match self {
            NamingKind::Tool => "unique_name",
            NamingKind::Prompt => "unique_name",
            NamingKind::Resource => "unique_uri",
            NamingKind::ResourceTemplate => "unique_name",
        }
    }

    fn value_column(self) -> &'static str {
        match self {
            NamingKind::Tool => "tool_name",
            NamingKind::Prompt => "prompt_name",
            NamingKind::Resource => "resource_uri",
            NamingKind::ResourceTemplate => "uri_template",
        }
    }
}

/// Generate a unique identifier for the given capability kind.
pub fn generate_unique_name(
    kind: NamingKind,
    server_name: &str,
    value: &str,
) -> String {
    match kind {
        NamingKind::Tool | NamingKind::Prompt | NamingKind::ResourceTemplate => {
            let normalized = normalize_server_name(server_name);
            let prefix = format!("{normalized}_");
            if value.to_lowercase().starts_with(&prefix) {
                value.to_string()
            } else {
                format!("{normalized}_{value}")
            }
        }
        NamingKind::Resource => {
            let normalized = normalize_server_name(server_name);
            let prefix = format!("{normalized}:");
            if value.to_lowercase().starts_with(&prefix) {
                value.to_string()
            } else {
                format!("{normalized}:{value}")
            }
        }
    }
}

/// Resolve a unique identifier back to its `(server_name, original_value)` pair.
pub async fn resolve_unique_name(
    kind: NamingKind,
    unique: &str,
) -> Result<(String, String)> {
    let query = format!(
        "SELECT server_name, {} FROM {} WHERE {} = ?",
        kind.value_column(),
        kind.table(),
        kind.unique_column()
    );

    let row = sqlx::query_as::<_, (String, String)>(&query)
        .bind(unique)
        .fetch_optional(pool())
        .await
        .context(format!("Failed to resolve unique {:?}: {}", kind, unique))?;

    row.ok_or_else(|| anyhow::anyhow!("Unique {:?} '{}' not found", kind, unique))
}

/// Ensure a unique identifier is collision-free. Non-tool kinds return generated names directly.
pub async fn ensure_unique_name(
    kind: NamingKind,
    server_id: &str,
    server_name: &str,
    value: &str,
) -> Result<String> {
    match kind {
        NamingKind::Tool => ensure_unique_tool_name(server_id, server_name, value).await,
        NamingKind::Prompt | NamingKind::Resource | NamingKind::ResourceTemplate => {
            Ok(generate_unique_name(kind, server_name, value))
        }
    }
}

async fn ensure_unique_tool_name(
    server_id: &str,
    server_name: &str,
    tool_name: &str,
) -> Result<String> {
    let pool = pool();
    let base_name = generate_unique_name(NamingKind::Tool, server_name, tool_name);

    let conflict = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM server_tools
            WHERE unique_name = ?
              AND (server_id != ? OR tool_name != ?)
        )
        "#,
    )
    .bind(&base_name)
    .bind(server_id)
    .bind(tool_name)
    .fetch_one(pool)
    .await
    .context(format!("Failed to check tool name conflicts for '{}'", base_name))?;

    if !conflict {
        return Ok(base_name);
    }

    let mut counter = 1;
    loop {
        let candidate = format!("{base_name}_{counter}");
        let conflict = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM server_tools
                WHERE unique_name = ?
                  AND (server_id != ? OR tool_name != ?)
            )
            "#,
        )
        .bind(&candidate)
        .bind(server_id)
        .bind(tool_name)
        .fetch_one(pool)
        .await
        .context(format!("Failed to check tool name conflicts for '{}'", candidate))?;

        if !conflict {
            tracing::debug!("Resolved tool name collision for '{}' using '{}'", base_name, candidate);
            return Ok(candidate);
        }

        counter += 1;
        if counter > 1000 {
            return Err(anyhow::anyhow!(
                "Failed to generate a unique tool name after 1000 attempts"
            ));
        }
    }
}
