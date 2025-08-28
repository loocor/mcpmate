// Config Suit Prompt operations
// Contains functions for managing prompts in configuration suits

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};
use tracing;

use crate::generate_id;

/// Add a prompt to a configuration suit in the database
///
/// This function adds a prompt to a configuration suit in the database.
/// If the prompt is added or updated, it also publishes a PromptEnabledInSuitChanged event.
pub async fn add_prompt_to_config_suit(
    pool: &Pool<Sqlite>,
    suit_id: &str,
    server_id: &str,
    prompt_name: &str,
    enabled: bool,
) -> Result<String> {
    tracing::debug!(
        "Adding prompt '{}' from server ID {} to configuration suit ID {}, enabled: {}",
        prompt_name,
        server_id,
        suit_id,
        enabled
    );

    // Check if the prompt already exists in this configuration suit
    let existing_prompt = sqlx::query_scalar::<_, String>(
        r#"
        SELECT id FROM config_suit_prompt
        WHERE suit_id = ? AND server_id = ? AND prompt_name = ?
        "#,
    )
    .bind(suit_id)
    .bind(server_id)
    .bind(prompt_name)
    .fetch_optional(pool)
    .await
    .context("Failed to check if prompt exists in configuration suit")?;

    let prompt_id = if let Some(existing_id) = existing_prompt {
        // Update existing prompt
        tracing::debug!(
            "Prompt '{}' already exists in configuration suit, updating enabled status to {}",
            prompt_name,
            enabled
        );

        sqlx::query(
            r#"
            UPDATE config_suit_prompt
            SET enabled = ?, updated_at = CURRENT_TIMESTAMP
            WHERE id = ?
            "#,
        )
        .bind(enabled)
        .bind(&existing_id)
        .execute(pool)
        .await
        .context("Failed to update prompt in configuration suit")?;

        existing_id
    } else {
        // Get the server name (safe version with underscores)
        let server_name = crate::config::operations::server::get_server_name_safe(pool, server_id)
            .await
            .context("Failed to get server name")?;

        // Insert new prompt
        let new_id = generate_id!("spmt");
        tracing::debug!(
            "Inserting new prompt '{}' with ID {} into configuration suit",
            prompt_name,
            new_id
        );

        sqlx::query(
            r#"
            INSERT INTO config_suit_prompt (id, suit_id, server_id, server_name, prompt_name, enabled)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&new_id)
        .bind(suit_id)
        .bind(server_id)
        .bind(&server_name)
        .bind(prompt_name)
        .bind(enabled)
        .execute(pool)
        .await
        .context("Failed to insert prompt into configuration suit")?;

        new_id
    };

    // Publish event for prompt enabled status change
    let event = crate::core::events::Event::PromptEnabledInSuitChanged {
        prompt_id: prompt_id.clone(),
        prompt_name: prompt_name.to_string(),
        suit_id: suit_id.to_string(),
        enabled,
    };

    crate::core::events::EventBus::global().publish(event);

    tracing::debug!(
        "Successfully added/updated prompt '{}' in configuration suit with ID {}",
        prompt_name,
        prompt_id
    );

    Ok(prompt_id)
}

/// Remove a prompt from a configuration suit
pub async fn remove_prompt_from_config_suit(
    pool: &Pool<Sqlite>,
    suit_id: &str,
    server_id: &str,
    prompt_name: &str,
) -> Result<()> {
    tracing::debug!(
        "Removing prompt '{}' from server ID {} from configuration suit ID {}",
        prompt_name,
        server_id,
        suit_id
    );

    let result = sqlx::query(
        r#"
        DELETE FROM config_suit_prompt
        WHERE suit_id = ? AND server_id = ? AND prompt_name = ?
        "#,
    )
    .bind(suit_id)
    .bind(server_id)
    .bind(prompt_name)
    .execute(pool)
    .await
    .context("Failed to remove prompt from configuration suit")?;

    if result.rows_affected() == 0 {
        tracing::warn!(
            "No prompt '{}' found for server ID {} in configuration suit ID {}",
            prompt_name,
            server_id,
            suit_id
        );
    } else {
        tracing::debug!(
            "Successfully removed prompt '{}' from configuration suit",
            prompt_name
        );
    }

    Ok(())
}

/// Get all prompts for a configuration suit
pub async fn get_prompts_for_config_suit(
    pool: &Pool<Sqlite>,
    suit_id: &str,
) -> Result<Vec<crate::config::models::ConfigSuitPrompt>> {
    tracing::debug!(
        "Getting all prompts for configuration suit ID {}",
        suit_id
    );

    let prompts = sqlx::query_as::<_, crate::config::models::ConfigSuitPrompt>(
        r#"
        SELECT id, suit_id, server_id, server_name, prompt_name, enabled, created_at, updated_at
        FROM config_suit_prompt
        WHERE suit_id = ?
        ORDER BY server_name, prompt_name
        "#,
    )
    .bind(suit_id)
    .fetch_all(pool)
    .await
    .context("Failed to get prompts for configuration suit")?;

    Ok(prompts)
}

/// Get enabled prompts for a configuration suit
pub async fn get_enabled_prompts_for_config_suit(
    pool: &Pool<Sqlite>,
    suit_id: &str,
) -> Result<Vec<crate::config::models::ConfigSuitPrompt>> {
    tracing::debug!(
        "Getting enabled prompts for configuration suit ID {}",
        suit_id
    );

    let prompts = sqlx::query_as::<_, crate::config::models::ConfigSuitPrompt>(
        r#"
        SELECT id, suit_id, server_id, server_name, prompt_name, enabled, created_at, updated_at
        FROM config_suit_prompt
        WHERE suit_id = ? AND enabled = 1
        ORDER BY server_name, prompt_name
        "#,
    )
    .bind(suit_id)
    .fetch_all(pool)
    .await
    .context("Failed to get enabled prompts for configuration suit")?;

    Ok(prompts)
}

/// Update prompt enabled status in a configuration suit
pub async fn update_prompt_enabled_status(
    pool: &Pool<Sqlite>,
    prompt_id: &str,
    enabled: bool,
) -> Result<()> {
    tracing::debug!(
        "Updating prompt ID {} enabled status to {}",
        prompt_id,
        enabled
    );

    // Get prompt info for event publishing
    let prompt_info = sqlx::query_as::<_, (String, String, String)>(
        r#"
        SELECT prompt_name, suit_id, server_id
        FROM config_suit_prompt
        WHERE id = ?
        "#,
    )
    .bind(prompt_id)
    .fetch_optional(pool)
    .await
    .context("Failed to get prompt info for event publishing")?;

    let result = sqlx::query(
        r#"
        UPDATE config_suit_prompt
        SET enabled = ?, updated_at = CURRENT_TIMESTAMP
        WHERE id = ?
        "#,
    )
    .bind(enabled)
    .bind(prompt_id)
    .execute(pool)
    .await
    .context("Failed to update prompt enabled status")?;

    if result.rows_affected() == 0 {
        return Err(anyhow::anyhow!("Prompt with ID {} not found", prompt_id));
    }

    // Publish event if we have prompt info
    if let Some((prompt_name, suit_id, _server_id)) = prompt_info {
        let event = crate::core::events::Event::PromptEnabledInSuitChanged {
            prompt_id: prompt_id.to_string(),
            prompt_name,
            suit_id,
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

/// Common query builder for enabled prompts from active configuration suits.
pub fn build_enabled_prompts_query(additional_where: Option<&str>) -> String {
    let base_query = r#"
        SELECT DISTINCT csp.server_name, csp.prompt_name
        FROM config_suit_prompt csp
        JOIN config_suit cs ON csp.suit_id = cs.id
        WHERE cs.is_active = true AND csp.enabled = true
    "#;

    match additional_where {
        Some(condition) => format!("{} AND {}", base_query, condition),
        None => base_query.to_string(),
    }
}