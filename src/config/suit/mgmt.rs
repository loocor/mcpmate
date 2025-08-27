// Management operations for Config Suits
// Contains create, update, delete and status management operations

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};

use crate::{config::models::ConfigSuit, generate_id};

use super::basic::get_config_suit;

/// Create or update a configuration suit in the database
pub async fn upsert_config_suit(
    pool: &Pool<Sqlite>,
    suit: &ConfigSuit,
) -> Result<String> {
    tracing::debug!("Upserting configuration suit '{}', type: {}", suit.name, suit.suit_type);

    // Generate an ID for the suit if it doesn't have one
    let suit_id = if let Some(id) = &suit.id {
        id.clone()
    } else {
        generate_id!("suit")
    };

    let mut tx = pool.begin().await.context("Failed to begin transaction")?;

    let result = sqlx::query(
        r#"
        INSERT INTO config_suit (id, name, description, type, multi_select, priority, is_active, is_default)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(name) DO UPDATE SET
            description = excluded.description,
            type = excluded.type,
            multi_select = excluded.multi_select,
            priority = excluded.priority,
            is_active = excluded.is_active,
            is_default = excluded.is_default,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(&suit_id)
    .bind(&suit.name)
    .bind(&suit.description)
    .bind(suit.suit_type)
    .bind(suit.multi_select)
    .bind(suit.priority)
    .bind(suit.is_active)
    .bind(suit.is_default)
    .execute(&mut *tx)
    .await
    .context("Failed to upsert configuration suit")?;

    let final_suit_id = if result.rows_affected() == 0 {
        // If no rows were affected, get the existing ID
        let existing_id = sqlx::query_scalar::<_, String>(
            r#"
            SELECT id FROM config_suit
            WHERE name = ?
            "#,
        )
        .bind(&suit.name)
        .fetch_one(&mut *tx)
        .await
        .context("Failed to get configuration suit ID")?;

        existing_id
    } else {
        suit_id
    };

    tx.commit().await.context("Failed to commit transaction")?;

    Ok(final_suit_id)
}

/// Update an existing configuration suit by ID
///
/// This function is specifically designed for updating existing suits based on their ID,
/// unlike upsert_config_suit which is designed for creation scenarios with name-based conflict detection.
pub async fn update_config_suit(
    pool: &Pool<Sqlite>,
    suit: &ConfigSuit,
) -> Result<()> {
    let suit_id = suit
        .id
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Configuration suit ID is required for update operation"))?;

    tracing::debug!("Updating configuration suit '{}' with ID '{}'", suit.name, suit_id);

    let result = sqlx::query(
        r#"
        UPDATE config_suit
        SET name = ?, description = ?, type = ?, multi_select = ?, priority = ?, is_active = ?, is_default = ?, updated_at = CURRENT_TIMESTAMP
        WHERE id = ?
        "#,
    )
    .bind(&suit.name)
    .bind(&suit.description)
    .bind(suit.suit_type)
    .bind(suit.multi_select)
    .bind(suit.priority)
    .bind(suit.is_active)
    .bind(suit.is_default)
    .bind(suit_id)
    .execute(pool)
    .await
    .context("Failed to update configuration suit")?;

    if result.rows_affected() == 0 {
        return Err(anyhow::anyhow!("Configuration suit with ID '{}' not found", suit_id));
    }

    tracing::debug!(
        "Successfully updated configuration suit '{}' with ID '{}'",
        suit.name,
        suit_id
    );

    Ok(())
}

/// Set a configuration suit as active or inactive
///
/// This function updates the active status of a configuration suit in the database.
/// If the status is updated, it also publishes a ConfigSuitStatusChanged event.
pub async fn set_config_suit_active(
    pool: &Pool<Sqlite>,
    suit_id: &str,
    active: bool,
) -> Result<()> {
    tracing::debug!(
        "Setting configuration suit with ID {} as {}",
        suit_id,
        if active { "active" } else { "inactive" }
    );

    // Get the configuration suit to check multi_select
    let suit = get_config_suit(pool, suit_id).await?;
    if suit.is_none() {
        return Err(anyhow::anyhow!("Configuration suit with ID {} not found", suit_id));
    }
    let suit = suit.unwrap();

    let mut tx = pool.begin().await.context("Failed to begin transaction")?;

    // If activating and multi_select is false, deactivate all other suits (except default)
    if active && !suit.multi_select {
        sqlx::query(
            r#"
            UPDATE config_suit
            SET is_active = 0,
                updated_at = CURRENT_TIMESTAMP
            WHERE id != ? AND is_default = 0
            "#,
        )
        .bind(suit_id)
        .execute(&mut *tx)
        .await
        .context("Failed to deactivate other configuration suits")?;
    }

    // Update the specified suit
    sqlx::query(
        r#"
        UPDATE config_suit
        SET is_active = ?,
            updated_at = CURRENT_TIMESTAMP
        WHERE id = ?
        "#,
    )
    .bind(active)
    .bind(suit_id)
    .execute(&mut *tx)
    .await
    .context("Failed to update configuration suit active status")?;

    tx.commit().await.context("Failed to commit transaction")?;

    // Publish the event
    crate::core::events::EventBus::global().publish(crate::core::events::Event::ConfigSuitStatusChanged {
        suit_id: suit_id.to_string(),
        enabled: active,
    });

    tracing::info!(
        "Published ConfigSuitStatusChanged event for suit ID {} ({})",
        suit_id,
        active
    );

    Ok(())
}

/// Set a configuration suit as the default
pub async fn set_config_suit_default(
    pool: &Pool<Sqlite>,
    suit_id: &str,
) -> Result<()> {
    tracing::debug!("Setting configuration suit with ID {} as default", suit_id);

    let mut tx = pool.begin().await.context("Failed to begin transaction")?;

    // Clear default flag from all suits
    sqlx::query(
        r#"
        UPDATE config_suit
        SET is_default = 0,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .execute(&mut *tx)
    .await
    .context("Failed to clear default flag from all configuration suits")?;

    // Set the specified suit as default
    sqlx::query(
        r#"
        UPDATE config_suit
        SET is_default = 1,
            updated_at = CURRENT_TIMESTAMP
        WHERE id = ?
        "#,
    )
    .bind(suit_id)
    .execute(&mut *tx)
    .await
    .context("Failed to set configuration suit as default")?;

    tx.commit().await.context("Failed to commit transaction")?;
    Ok(())
}

/// Delete a configuration suit from the database
pub async fn delete_config_suit(
    pool: &Pool<Sqlite>,
    id: &str,
) -> Result<bool> {
    tracing::debug!("Deleting configuration suit with ID {}", id);

    let result = sqlx::query(
        r#"
        DELETE FROM config_suit
        WHERE id = ?
        "#,
    )
    .bind(id)
    .execute(pool)
    .await
    .context("Failed to delete configuration suit")?;

    Ok(result.rows_affected() > 0)
}
