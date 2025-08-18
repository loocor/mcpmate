// Config Suit Resource operations
// Contains functions for managing resources in configuration suits

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};
use tracing;

use crate::generate_id;

/// Add a resource to a configuration suit in the database
///
/// This function adds a resource to a configuration suit in the database.
/// If the resource is added or updated, it also publishes a ResourceEnabledInSuitChanged event.
pub async fn add_resource_to_config_suit(
    pool: &Pool<Sqlite>,
    config_suit_id: &str,
    server_id: &str,
    resource_uri: &str,
    enabled: bool,
) -> Result<String> {
    tracing::debug!(
        "Adding resource '{}' from server ID {} to configuration suit ID {}, enabled: {}",
        resource_uri,
        server_id,
        config_suit_id,
        enabled
    );

    // Generate an ID for the suit resource
    let resource_id = generate_id!("sres");

    // Check if the resource already exists in the suit and get its current enabled status
    let existing_enabled = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT enabled FROM config_suit_resource
        WHERE config_suit_id = ? AND server_id = ? AND resource_uri = ?
        "#,
    )
    .bind(config_suit_id)
    .bind(server_id)
    .bind(resource_uri)
    .fetch_optional(pool)
    .await
    .context("Failed to get existing resource enabled status")?;

    // Get the server name (safe version with underscores)
    let server_name = crate::config::operations::server::get_server_name_safe(pool, server_id)
        .await
        .context("Failed to get server name")?;

    let result = sqlx::query(
        r#"
        INSERT INTO config_suit_resource (id, config_suit_id, server_id, server_name, resource_uri, enabled)
        VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT(config_suit_id, server_id, resource_uri) DO UPDATE SET
            server_name = excluded.server_name,
            enabled = excluded.enabled,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(&resource_id)
    .bind(config_suit_id)
    .bind(server_id)
    .bind(&server_name)
    .bind(resource_uri)
    .bind(enabled)
    .execute(pool)
    .await
    .context("Failed to add resource to configuration suit")?;

    let is_new = result.rows_affected() > 0;
    let id_to_return = if is_new {
        resource_id.clone()
    } else {
        // If no rows were affected, get the existing ID
        sqlx::query_scalar::<_, String>(
            r#"
            SELECT id FROM config_suit_resource
            WHERE config_suit_id = ? AND server_id = ? AND resource_uri = ?
            "#,
        )
        .bind(config_suit_id)
        .bind(server_id)
        .bind(resource_uri)
        .fetch_one(pool)
        .await
        .context("Failed to get configuration suit resource association ID")?
    };

    // Publish event if the resource is new or its enabled status has changed
    if is_new || (existing_enabled != Some(enabled)) {
        // Publish the event
        crate::core::events::EventBus::global().publish(crate::core::events::Event::ResourceEnabledInSuitChanged {
            resource_id: id_to_return.clone(),
            resource_uri: resource_uri.to_string(),
            suit_id: config_suit_id.to_string(),
            enabled,
        });

        tracing::debug!(
            "Published ResourceEnabledInSuitChanged event for resource '{}' in suit ID {} ({})",
            resource_uri,
            config_suit_id,
            enabled
        );
    }

    Ok(id_to_return)
}

/// Remove a resource from a configuration suit in the database
pub async fn remove_resource_from_config_suit(
    pool: &Pool<Sqlite>,
    config_suit_id: &str,
    server_id: &str,
    resource_uri: &str,
) -> Result<bool> {
    tracing::debug!(
        "Removing resource '{}' from server ID {} from configuration suit ID {}",
        resource_uri,
        server_id,
        config_suit_id
    );

    let result = sqlx::query(
        r#"
        DELETE FROM config_suit_resource
        WHERE config_suit_id = ? AND server_id = ? AND resource_uri = ?
        "#,
    )
    .bind(config_suit_id)
    .bind(server_id)
    .bind(resource_uri)
    .execute(pool)
    .await
    .context("Failed to remove resource from configuration suit")?;

    Ok(result.rows_affected() > 0)
}

/// Get all resources for a configuration suit
pub async fn get_resources_for_config_suit(
    pool: &Pool<Sqlite>,
    config_suit_id: &str,
) -> Result<Vec<crate::config::models::ConfigSuitResource>> {
    tracing::debug!("Getting all resources for configuration suit ID {}", config_suit_id);

    let resources = sqlx::query_as::<_, crate::config::models::ConfigSuitResource>(
        r#"
        SELECT id, config_suit_id, server_id, server_name, resource_uri, enabled, created_at, updated_at
        FROM config_suit_resource
        WHERE config_suit_id = ?
        ORDER BY server_name, resource_uri
        "#,
    )
    .bind(config_suit_id)
    .fetch_all(pool)
    .await
    .context("Failed to get resources for configuration suit")?;

    Ok(resources)
}

/// Get enabled resources for a configuration suit
pub async fn get_enabled_resources_for_config_suit(
    pool: &Pool<Sqlite>,
    config_suit_id: &str,
) -> Result<Vec<crate::config::models::ConfigSuitResource>> {
    tracing::debug!("Getting enabled resources for configuration suit ID {}", config_suit_id);

    let resources = sqlx::query_as::<_, crate::config::models::ConfigSuitResource>(
        r#"
        SELECT id, config_suit_id, server_id, server_name, resource_uri, enabled, created_at, updated_at
        FROM config_suit_resource
        WHERE config_suit_id = ? AND enabled = 1
        ORDER BY server_name, resource_uri
        "#,
    )
    .bind(config_suit_id)
    .fetch_all(pool)
    .await
    .context("Failed to get enabled resources for configuration suit")?;

    Ok(resources)
}

/// Update resource enabled status in a configuration suit
pub async fn update_resource_enabled_status(
    pool: &Pool<Sqlite>,
    config_suit_id: &str,
    server_id: &str,
    resource_uri: &str,
    enabled: bool,
) -> Result<bool> {
    tracing::debug!(
        "Updating resource '{}' enabled status to {} in configuration suit ID {}",
        resource_uri,
        enabled,
        config_suit_id
    );

    let result = sqlx::query(
        r#"
        UPDATE config_suit_resource
        SET enabled = ?, updated_at = CURRENT_TIMESTAMP
        WHERE config_suit_id = ? AND server_id = ? AND resource_uri = ?
        "#,
    )
    .bind(enabled)
    .bind(config_suit_id)
    .bind(server_id)
    .bind(resource_uri)
    .execute(pool)
    .await
    .context("Failed to update resource enabled status")?;

    if result.rows_affected() > 0 {
        // Publish event for the status change
        if let Ok(resource_id) = sqlx::query_scalar::<_, String>(
            r#"
            SELECT id FROM config_suit_resource
            WHERE config_suit_id = ? AND server_id = ? AND resource_uri = ?
            "#,
        )
        .bind(config_suit_id)
        .bind(server_id)
        .bind(resource_uri)
        .fetch_one(pool)
        .await
        {
            crate::core::events::EventBus::global().publish(crate::core::events::Event::ResourceEnabledInSuitChanged {
                resource_id,
                resource_uri: resource_uri.to_string(),
                suit_id: config_suit_id.to_string(),
                enabled,
            });
        }
    }

    Ok(result.rows_affected() > 0)
}

/// Common query builder for enabled resources from active configuration suits.
/// This helper reduces code duplication and ensures consistency.
pub fn build_enabled_resources_query(additional_where: Option<&str>) -> String {
    let base_query = r#"
        SELECT DISTINCT csr.server_name, csr.resource_uri
        FROM config_suit_resource csr
        JOIN config_suit cs ON csr.config_suit_id = cs.id
        WHERE cs.is_active = true AND csr.enabled = true
    "#;

    match additional_where {
        Some(condition) => format!("{} AND {}", base_query, condition),
        None => base_query.to_string(),
    }
}
