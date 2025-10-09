// Profile Prompt operations
// Contains functions for managing prompts in profile

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};
use tracing;

use crate::generate_id;

/// Add a prompt to a profile in the database
///
/// This function adds a prompt to a profile in the database.
/// If the prompt is added or updated, it also publishes a PromptEnabledInProfileChanged event.
pub async fn add_prompt_to_profile(
    pool: &Pool<Sqlite>,
    profile_id: &str,
    server_id: &str,
    prompt_name: &str,
    enabled: bool,
) -> Result<String> {
    tracing::debug!(
        "Adding prompt '{}' from server ID {} to profile ID {}, enabled: {}",
        prompt_name,
        server_id,
        profile_id,
        enabled
    );

    // Check if the prompt already exists in this profile
    let existing_prompt = sqlx::query_scalar::<_, String>(
        r#"
        SELECT id FROM profile_prompt
        WHERE profile_id = ? AND server_id = ? AND prompt_name = ?
        "#,
    )
    .bind(profile_id)
    .bind(server_id)
    .bind(prompt_name)
    .fetch_optional(pool)
    .await
    .context("Failed to check if prompt exists in profile")?;

    let prompt_id = if let Some(existing_id) = existing_prompt {
        // Update existing prompt
        tracing::debug!(
            "Prompt '{}' already exists in profile, updating enabled status to {}",
            prompt_name,
            enabled
        );

        sqlx::query(
            r#"
            UPDATE profile_prompt
            SET enabled = ?, updated_at = CURRENT_TIMESTAMP
            WHERE id = ?
            "#,
        )
        .bind(enabled)
        .bind(&existing_id)
        .execute(pool)
        .await
        .context("Failed to update prompt in profile")?;

        existing_id
    } else {
        // Get the server name (safe version with underscores)
        let server_name = crate::config::operations::server::get_server_name_safe(pool, server_id)
            .await
            .context("Failed to get server name")?;

        // Insert new prompt
        let new_id = generate_id!("spmt");
        tracing::debug!("Inserting new prompt '{}' with ID {} into profile", prompt_name, new_id);

        sqlx::query(
            r#"
            INSERT INTO profile_prompt (id, profile_id, server_id, server_name, prompt_name, enabled)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&new_id)
        .bind(profile_id)
        .bind(server_id)
        .bind(&server_name)
        .bind(prompt_name)
        .bind(enabled)
        .execute(pool)
        .await
        .context("Failed to insert prompt into profile")?;

        new_id
    };

    // Publish event for prompt enabled status change
    let event = crate::core::events::Event::PromptEnabledInProfileChanged {
        prompt_id: prompt_id.clone(),
        prompt_name: prompt_name.to_string(),
        profile_id: profile_id.to_string(),
        enabled,
    };

    crate::core::events::EventBus::global().publish(event);

    tracing::debug!(
        "Successfully added/updated prompt '{}' in profile with ID {}",
        prompt_name,
        prompt_id
    );

    Ok(prompt_id)
}

/// Remove a prompt from a profile
pub async fn remove_prompt_from_profile(
    pool: &Pool<Sqlite>,
    profile_id: &str,
    server_id: &str,
    prompt_name: &str,
) -> Result<()> {
    tracing::debug!(
        "Removing prompt '{}' from server ID {} from profile ID {}",
        prompt_name,
        server_id,
        profile_id
    );

    let result = sqlx::query(
        r#"
        DELETE FROM profile_prompt
        WHERE profile_id = ? AND server_id = ? AND prompt_name = ?
        "#,
    )
    .bind(profile_id)
    .bind(server_id)
    .bind(prompt_name)
    .execute(pool)
    .await
    .context("Failed to remove prompt from profile")?;

    if result.rows_affected() == 0 {
        tracing::warn!(
            "No prompt '{}' found for server ID {} in profile ID {}",
            prompt_name,
            server_id,
            profile_id
        );
    } else {
        tracing::debug!("Successfully removed prompt '{}' from profile", prompt_name);
    }

    Ok(())
}

/// Get all prompts for a profile
pub async fn get_prompts_for_profile(
    pool: &Pool<Sqlite>,
    profile_id: &str,
) -> Result<Vec<crate::config::models::ProfilePrompt>> {
    tracing::debug!("Getting all prompts for profile ID {}", profile_id);

    let prompts = sqlx::query_as::<_, crate::config::models::ProfilePrompt>(
        r#"
        SELECT id, profile_id, server_id, server_name, prompt_name, enabled, created_at, updated_at
        FROM profile_prompt
        WHERE profile_id = ?
          AND EXISTS (
              SELECT 1
              FROM profile_server ps
              WHERE ps.profile_id = profile_prompt.profile_id
                AND ps.server_id = profile_prompt.server_id
          )
        ORDER BY server_name, prompt_name
        "#,
    )
    .bind(profile_id)
    .fetch_all(pool)
    .await
    .context("Failed to get prompts for profile")?;

    Ok(prompts)
}

/// Get enabled prompts for a profile
pub async fn get_enabled_prompts_for_profile(
    pool: &Pool<Sqlite>,
    profile_id: &str,
) -> Result<Vec<crate::config::models::ProfilePrompt>> {
    tracing::debug!("Getting enabled prompts for profile ID {}", profile_id);

    let prompts = sqlx::query_as::<_, crate::config::models::ProfilePrompt>(
        r#"
        SELECT id, profile_id, server_id, server_name, prompt_name, enabled, created_at, updated_at
        FROM profile_prompt
        WHERE profile_id = ?
          AND enabled = 1
          AND EXISTS (
              SELECT 1
              FROM profile_server ps
              WHERE ps.profile_id = profile_prompt.profile_id
                AND ps.server_id = profile_prompt.server_id
          )
        ORDER BY server_name, prompt_name
        "#,
    )
    .bind(profile_id)
    .fetch_all(pool)
    .await
    .context("Failed to get enabled prompts for profile")?;

    Ok(prompts)
}

/// Update prompt enabled status in a profile
pub async fn update_prompt_enabled_status(
    pool: &Pool<Sqlite>,
    prompt_id: &str,
    enabled: bool,
) -> Result<()> {
    tracing::debug!("Updating prompt ID {} enabled status to {}", prompt_id, enabled);

    // Get prompt info for event publishing
    let prompt_info = sqlx::query_as::<_, (String, String, String)>(
        r#"
        SELECT prompt_name, profile_id, server_id
        FROM profile_prompt
        WHERE id = ?
        "#,
    )
    .bind(prompt_id)
    .fetch_optional(pool)
    .await
    .context("Failed to get prompt info for event publishing")?;

    let result = sqlx::query(
        r#"
        UPDATE profile_prompt
        SET enabled = ?, updated_at = CURRENT_TIMESTAMP
        WHERE id = ?
        "#,
    )
    .bind(enabled)
    .bind(prompt_id)
    .execute(pool)
    .await
    .context("Failed to update prompt enabled status")?;

    let updated = result.rows_affected();

    if updated == 0 {
        return Err(anyhow::anyhow!("Prompt with ID {} not found", prompt_id));
    }

    // Publish event if we have prompt info
    if let Some((prompt_name, profile_id, server_id)) = prompt_info {
        if enabled {
            crate::config::profile::server::ensure_server_enabled_for_profile(pool, &profile_id, &server_id).await?;
        } else {
            crate::config::profile::server::disable_server_if_all_capabilities_disabled(pool, &profile_id, &server_id)
                .await?;
        }

        let event = crate::core::events::Event::PromptEnabledInProfileChanged {
            prompt_id: prompt_id.to_string(),
            prompt_name,
            profile_id,
            enabled,
        };

        crate::core::events::EventBus::global().publish(event);
    }

    tracing::debug!(
        "Successfully updated prompt ID {} enabled status to {}",
        prompt_id,
        enabled
    );

    Ok(())
}

/// Common query builder for enabled prompts from active profile.
pub fn build_enabled_prompts_query(additional_where: Option<&str>) -> String {
    // Select original server name from server_config to align with aggregator naming
    let base_query = r#"
        SELECT DISTINCT sc.id as server_id, sc.name as server_name, csp.prompt_name
        FROM profile_prompt csp
        JOIN profile cs ON csp.profile_id = cs.id
        JOIN server_config sc ON csp.server_id = sc.id
        WHERE cs.is_active = true
          AND csp.enabled = true
          AND sc.enabled = 1
    "#;

    match additional_where {
        Some(condition) => format!("{} AND {}", base_query, condition),
        None => base_query.to_string(),
    }
}
