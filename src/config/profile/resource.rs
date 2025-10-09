// Profile Resource operations
// Contains functions for managing resources in profile

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};
use tracing;

use crate::generate_id;

/// Add a resource to a profile in the database
///
/// This function adds a resource to a profile in the database.
/// If the resource is added or updated, it also publishes a ResourceEnabledInProfileChanged event.
pub async fn add_resource_to_profile(
    pool: &Pool<Sqlite>,
    profile_id: &str,
    server_id: &str,
    resource_uri: &str,
    enabled: bool,
) -> Result<String> {
    tracing::debug!(
        "Adding resource '{}' from server ID {} to profile ID {}, enabled: {}",
        resource_uri,
        server_id,
        profile_id,
        enabled
    );

    // Generate an ID for the profile resource
    let resource_id = generate_id!("sres");

    // Check if the resource already exists in the profile and get its current enabled status
    let existing_enabled = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT enabled FROM profile_resource
        WHERE profile_id = ? AND server_id = ? AND resource_uri = ?
        "#,
    )
    .bind(profile_id)
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
        INSERT INTO profile_resource (id, profile_id, server_id, server_name, resource_uri, enabled)
        VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT(profile_id, server_id, resource_uri) DO UPDATE SET
            server_name = excluded.server_name,
            enabled = excluded.enabled,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(&resource_id)
    .bind(profile_id)
    .bind(server_id)
    .bind(&server_name)
    .bind(resource_uri)
    .bind(enabled)
    .execute(pool)
    .await
    .context("Failed to add resource to profile")?;

    let is_new = result.rows_affected() > 0;
    let id_to_return = if is_new {
        resource_id.clone()
    } else {
        // If no rows were affected, get the existing ID
        sqlx::query_scalar::<_, String>(
            r#"
            SELECT id FROM profile_resource
            WHERE profile_id = ? AND server_id = ? AND resource_uri = ?
            "#,
        )
        .bind(profile_id)
        .bind(server_id)
        .bind(resource_uri)
        .fetch_one(pool)
        .await
        .context("Failed to get profile resource association ID")?
    };

    // Publish event if the resource is new or its enabled status has changed
    if is_new || (existing_enabled != Some(enabled)) {
        // Publish the event
        crate::core::events::EventBus::global().publish(crate::core::events::Event::ResourceEnabledInProfileChanged {
            resource_id: id_to_return.clone(),
            resource_uri: resource_uri.to_string(),
            profile_id: profile_id.to_string(),
            enabled,
        });

        tracing::debug!(
            "Published ResourceEnabledInProfileChanged event for resource '{}' in profile ID {} ({})",
            resource_uri,
            profile_id,
            enabled
        );
    }

    Ok(id_to_return)
}

/// Remove a resource from a profile in the database
pub async fn remove_resource_from_profile(
    pool: &Pool<Sqlite>,
    profile_id: &str,
    server_id: &str,
    resource_uri: &str,
) -> Result<bool> {
    tracing::debug!(
        "Removing resource '{}' from server ID {} from profile ID {}",
        resource_uri,
        server_id,
        profile_id
    );

    let result = sqlx::query(
        r#"
        DELETE FROM profile_resource
        WHERE profile_id = ? AND server_id = ? AND resource_uri = ?
        "#,
    )
    .bind(profile_id)
    .bind(server_id)
    .bind(resource_uri)
    .execute(pool)
    .await
    .context("Failed to remove resource from profile")?;

    Ok(result.rows_affected() > 0)
}

/// Get all resources for a profile
pub async fn get_resources_for_profile(
    pool: &Pool<Sqlite>,
    profile_id: &str,
) -> Result<Vec<crate::config::models::ProfileResource>> {
    tracing::debug!("Getting all resources for profile ID {}", profile_id);

    let resources = sqlx::query_as::<_, crate::config::models::ProfileResource>(
        r#"
        SELECT id, profile_id, server_id, server_name, resource_uri, enabled, created_at, updated_at
        FROM profile_resource
        WHERE profile_id = ?
          AND EXISTS (
              SELECT 1
              FROM profile_server ps
              WHERE ps.profile_id = profile_resource.profile_id
                AND ps.server_id = profile_resource.server_id
          )
        ORDER BY server_name, resource_uri
        "#,
    )
    .bind(profile_id)
    .fetch_all(pool)
    .await
    .context("Failed to get resources for profile")?;

    Ok(resources)
}

/// Get enabled resources for a profile
pub async fn get_enabled_resources_for_profile(
    pool: &Pool<Sqlite>,
    profile_id: &str,
) -> Result<Vec<crate::config::models::ProfileResource>> {
    tracing::debug!("Getting enabled resources for profile ID {}", profile_id);

    let resources = sqlx::query_as::<_, crate::config::models::ProfileResource>(
        r#"
        SELECT id, profile_id, server_id, server_name, resource_uri, enabled, created_at, updated_at
        FROM profile_resource
        WHERE profile_id = ?
          AND enabled = 1
          AND EXISTS (
              SELECT 1
              FROM profile_server ps
              WHERE ps.profile_id = profile_resource.profile_id
                AND ps.server_id = profile_resource.server_id
          )
        ORDER BY server_name, resource_uri
        "#,
    )
    .bind(profile_id)
    .fetch_all(pool)
    .await
    .context("Failed to get enabled resources for profile")?;

    Ok(resources)
}

/// Update resource enabled status in a profile
pub async fn update_resource_enabled_status(
    pool: &Pool<Sqlite>,
    profile_id: &str,
    server_id: &str,
    resource_uri: &str,
    enabled: bool,
) -> Result<bool> {
    tracing::debug!(
        "Updating resource '{}' enabled status to {} in profile ID {}",
        resource_uri,
        enabled,
        profile_id
    );

    let result = sqlx::query(
        r#"
        UPDATE profile_resource
        SET enabled = ?, updated_at = CURRENT_TIMESTAMP
        WHERE profile_id = ? AND server_id = ? AND resource_uri = ?
        "#,
    )
    .bind(enabled)
    .bind(profile_id)
    .bind(server_id)
    .bind(resource_uri)
    .execute(pool)
    .await
    .context("Failed to update resource enabled status")?;

    let updated = result.rows_affected() > 0;

    if updated {
        if enabled {
            crate::config::profile::server::ensure_server_enabled_for_profile(pool, profile_id, server_id).await?;
        } else {
            crate::config::profile::server::disable_server_if_all_capabilities_disabled(pool, profile_id, server_id)
                .await?;
        }

        // Publish event for the status change
        if let Ok(resource_id) = sqlx::query_scalar::<_, String>(
            r#"
            SELECT id FROM profile_resource
            WHERE profile_id = ? AND server_id = ? AND resource_uri = ?
            "#,
        )
        .bind(profile_id)
        .bind(server_id)
        .bind(resource_uri)
        .fetch_one(pool)
        .await
        {
            crate::core::events::EventBus::global().publish(
                crate::core::events::Event::ResourceEnabledInProfileChanged {
                    resource_id,
                    resource_uri: resource_uri.to_string(),
                    profile_id: profile_id.to_string(),
                    enabled,
                },
            );
        }
    }

    Ok(updated)
}

/// Common query builder for enabled resources from active profile.
/// This helper reduces code duplication and ensures consistency.
pub fn build_enabled_resources_query(additional_where: Option<&str>) -> String {
    // Note: select original server name from server_config (sc.name) instead of the
    // underscored safe name stored in profile_resource.csr.server_name to keep
    // unique-name generation consistent with aggregation path.
    let base_query = r#"
        SELECT DISTINCT sc.id as server_id, sc.name as server_name, csr.resource_uri
        FROM profile_resource csr
        JOIN profile cs ON csr.profile_id = cs.id
        JOIN server_config sc ON csr.server_id = sc.id
        WHERE cs.is_active = true
          AND csr.enabled = true
          AND sc.enabled = 1
    "#;

    match additional_where {
        Some(condition) => format!("{} AND {}", base_query, condition),
        None => base_query.to_string(),
    }
}
