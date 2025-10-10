// Profile Resource Template operations
// Contains functions for managing resource templates in profile

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};

use crate::generate_id;

/// Add a resource template to a profile in the database
///
/// If the association is added or updated, it publishes a ResourceTemplateEnabledInProfileChanged event.
pub async fn add_resource_template_to_profile(
    pool: &Pool<Sqlite>,
    profile_id: &str,
    server_id: &str,
    uri_template: &str,
    enabled: bool,
) -> Result<String> {
    let template_id = generate_id!("srst");

    // Check existing enabled state for change detection
    let existing_enabled = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT enabled FROM profile_resource_template
        WHERE profile_id = ? AND server_id = ? AND uri_template = ?
        "#,
    )
    .bind(profile_id)
    .bind(server_id)
    .bind(uri_template)
    .fetch_optional(pool)
    .await
    .context("Failed to get existing resource template enabled status")?;

    // Server name (safe version)
    let server_name = crate::config::operations::server::get_server_name_safe(pool, server_id)
        .await
        .context("Failed to get server name")?;

    let result = sqlx::query(
        r#"
        INSERT INTO profile_resource_template (id, profile_id, server_id, server_name, uri_template, enabled)
        VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT(profile_id, server_id, uri_template) DO UPDATE SET
            server_name = excluded.server_name,
            enabled = excluded.enabled,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(&template_id)
    .bind(profile_id)
    .bind(server_id)
    .bind(&server_name)
    .bind(uri_template)
    .bind(enabled)
    .execute(pool)
    .await
    .context("Failed to add resource template to profile")?;

    let is_new = result.rows_affected() > 0;
    let id_to_return = if is_new {
        template_id.clone()
    } else {
        sqlx::query_scalar::<_, String>(
            r#"
            SELECT id FROM profile_resource_template
            WHERE profile_id = ? AND server_id = ? AND uri_template = ?
            "#,
        )
        .bind(profile_id)
        .bind(server_id)
        .bind(uri_template)
        .fetch_one(pool)
        .await
        .context("Failed to get profile resource template association ID")?
    };

    if is_new || (existing_enabled != Some(enabled)) {
        crate::core::events::EventBus::global().publish(
            crate::core::events::Event::ResourceTemplateEnabledInProfileChanged {
                template_id: id_to_return.clone(),
                uri_template: uri_template.to_string(),
                profile_id: profile_id.to_string(),
                enabled,
            },
        );
    }

    Ok(id_to_return)
}

/// Remove a resource template from a profile in the database
pub async fn remove_resource_template_from_profile(
    pool: &Pool<Sqlite>,
    profile_id: &str,
    server_id: &str,
    uri_template: &str,
) -> Result<bool> {
    let result = sqlx::query(
        r#"
        DELETE FROM profile_resource_template
        WHERE profile_id = ? AND server_id = ? AND uri_template = ?
        "#,
    )
    .bind(profile_id)
    .bind(server_id)
    .bind(uri_template)
    .execute(pool)
    .await
    .context("Failed to remove resource template from profile")?;

    Ok(result.rows_affected() > 0)
}

/// Get all resource templates for a profile
pub async fn get_resource_templates_for_profile(
    pool: &Pool<Sqlite>,
    profile_id: &str,
) -> Result<Vec<crate::config::models::ProfileResource>> {
    let rows = sqlx::query_as::<_, crate::config::models::ProfileResource>(
        r#"
        SELECT id, profile_id, server_id, server_name, uri_template as resource_uri, enabled, created_at, updated_at
        FROM profile_resource_template
        WHERE profile_id = ?
          AND EXISTS (
              SELECT 1
              FROM profile_server ps
              WHERE ps.profile_id = profile_resource_template.profile_id
                AND ps.server_id = profile_resource_template.server_id
          )
        ORDER BY server_name, uri_template
        "#,
    )
    .bind(profile_id)
    .fetch_all(pool)
    .await
    .context("Failed to get resource templates for profile")?;
    Ok(rows)
}

/// Get enabled resource templates for a profile
pub async fn get_enabled_resource_templates_for_profile(
    pool: &Pool<Sqlite>,
    profile_id: &str,
) -> Result<Vec<crate::config::models::ProfileResource>> {
    let rows = sqlx::query_as::<_, crate::config::models::ProfileResource>(
        r#"
        SELECT id, profile_id, server_id, server_name, uri_template as resource_uri, enabled, created_at, updated_at
        FROM profile_resource_template
        WHERE profile_id = ?
          AND enabled = 1
          AND EXISTS (
              SELECT 1
              FROM profile_server ps
              WHERE ps.profile_id = profile_resource_template.profile_id
                AND ps.server_id = profile_resource_template.server_id
          )
        ORDER BY server_name, uri_template
        "#,
    )
    .bind(profile_id)
    .fetch_all(pool)
    .await
    .context("Failed to get enabled resource templates for profile")?;
    Ok(rows)
}

/// Update resource template enabled status in a profile
pub async fn update_resource_template_enabled_status(
    pool: &Pool<Sqlite>,
    profile_id: &str,
    server_id: &str,
    uri_template: &str,
    enabled: bool,
) -> Result<bool> {
    let result = sqlx::query(
        r#"
        UPDATE profile_resource_template
        SET enabled = ?, updated_at = CURRENT_TIMESTAMP
        WHERE profile_id = ? AND server_id = ? AND uri_template = ?
        "#,
    )
    .bind(enabled)
    .bind(profile_id)
    .bind(server_id)
    .bind(uri_template)
    .execute(pool)
    .await
    .context("Failed to update resource template enabled status")?;

    let updated = result.rows_affected() > 0;

    if updated {
        if enabled {
            crate::config::profile::server::ensure_server_enabled_for_profile(pool, profile_id, server_id).await?;
        } else {
            crate::config::profile::server::disable_server_if_all_capabilities_disabled(pool, profile_id, server_id)
                .await?;
        }

        if let Ok(template_id) = sqlx::query_scalar::<_, String>(
            r#"
            SELECT id FROM profile_resource_template
            WHERE profile_id = ? AND server_id = ? AND uri_template = ?
            "#,
        )
        .bind(profile_id)
        .bind(server_id)
        .bind(uri_template)
        .fetch_one(pool)
        .await
        {
            crate::core::events::EventBus::global().publish(
                crate::core::events::Event::ResourceTemplateEnabledInProfileChanged {
                    template_id,
                    uri_template: uri_template.to_string(),
                    profile_id: profile_id.to_string(),
                    enabled,
                },
            );
        }
    }

    Ok(updated)
}

/// Common query builder for enabled resource templates from active profiles
pub fn build_enabled_resource_templates_query(additional_where: Option<&str>) -> String {
    let base_query = r#"
        SELECT DISTINCT sc.id as server_id, sc.name as server_name, prt.uri_template
        FROM profile_resource_template prt
        JOIN profile cs ON prt.profile_id = cs.id
        JOIN server_config sc ON prt.server_id = sc.id
        WHERE cs.is_active = 1
          AND prt.enabled = 1
          AND sc.enabled = 1
    "#;

    match additional_where {
        Some(condition) => format!("{} AND {}", base_query, condition),
        None => base_query.to_string(),
    }
}

/// Extract a simple, robust prefix from a uri_template for matching concrete URIs.
/// Heuristic: take substring up to the first '{' or '*' placeholder.
pub fn template_prefix(uri_template: &str) -> &str {
    let bytes = uri_template.as_bytes();
    for (i, b) in bytes.iter().enumerate() {
        if *b == b'{' || *b == b'*' {
            return &uri_template[..i];
        }
    }
    uri_template
}

/// Determine if a concrete resource URI matches any enabled templates for the given (profile, server)
pub async fn resource_matches_enabled_templates(
    pool: &Pool<Sqlite>,
    profile_id: &str,
    server_id: &str,
    resource_uri: &str,
) -> Result<bool> {
    let rows: Vec<(String,)> = sqlx::query_as(
        r#"SELECT uri_template FROM profile_resource_template WHERE profile_id = ? AND server_id = ? AND enabled = 1"#,
    )
    .bind(profile_id)
    .bind(server_id)
    .fetch_all(pool)
    .await
    .context("Failed to fetch enabled resource templates for profile/server")?;

    for (tpl,) in rows {
        let prefix = template_prefix(&tpl);
        if !prefix.is_empty() && resource_uri.starts_with(prefix) {
            return Ok(true);
        }
    }
    Ok(false)
}
