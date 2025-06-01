// Basic query operations for Config Suits
// Contains read-only operations for configuration suits

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};

use crate::{common::types::ConfigSuitType, config::models::ConfigSuit};

/// Get all configuration suits from the database
pub async fn get_all_config_suits(pool: &Pool<Sqlite>) -> Result<Vec<ConfigSuit>> {
    tracing::debug!("Executing SQL query to get all configuration suits");

    let suits = sqlx::query_as::<_, ConfigSuit>(
        r#"
        SELECT * FROM config_suit
        ORDER BY name
        "#,
    )
    .fetch_all(pool)
    .await
    .context("Failed to fetch configuration suits")?;

    tracing::debug!(
        "Successfully fetched {} configuration suits from database",
        suits.len()
    );
    Ok(suits)
}

/// Get all active configuration suits from the database
pub async fn get_active_config_suits(pool: &Pool<Sqlite>) -> Result<Vec<ConfigSuit>> {
    tracing::debug!("Executing SQL query to get all active configuration suits");

    let suits = sqlx::query_as::<_, ConfigSuit>(
        r#"
        SELECT * FROM config_suit
        WHERE is_active = 1
        ORDER BY priority DESC, created_at ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .context("Failed to fetch active configuration suits")?;

    tracing::debug!(
        "Successfully fetched {} active configuration suits from database",
        suits.len()
    );
    Ok(suits)
}

/// Get the default configuration suit from the database
pub async fn get_default_config_suit(pool: &Pool<Sqlite>) -> Result<Option<ConfigSuit>> {
    tracing::debug!("Executing SQL query to get default configuration suit");

    let suit = sqlx::query_as::<_, ConfigSuit>(
        r#"
        SELECT * FROM config_suit
        WHERE is_default = 1
        LIMIT 1
        "#,
    )
    .fetch_optional(pool)
    .await
    .context("Failed to fetch default configuration suit")?;

    if let Some(ref s) = suit {
        tracing::debug!(
            "Found default configuration suit '{}' with ID {}",
            s.name,
            s.id.as_ref().unwrap_or(&"unknown".to_string())
        );
    } else {
        tracing::debug!("No default configuration suit found");
    }

    Ok(suit)
}

/// Get configuration suits by type from the database
pub async fn get_config_suits_by_type(
    pool: &Pool<Sqlite>,
    suit_type: ConfigSuitType,
) -> Result<Vec<ConfigSuit>> {
    tracing::debug!(
        "Executing SQL query to get configuration suits of type '{}'",
        suit_type.as_str()
    );

    let suits = sqlx::query_as::<_, ConfigSuit>(
        r#"
        SELECT * FROM config_suit
        WHERE type = ?
        ORDER BY name
        "#,
    )
    .bind(suit_type.as_str())
    .fetch_all(pool)
    .await
    .context("Failed to fetch configuration suits by type")?;

    tracing::debug!(
        "Successfully fetched {} configuration suits of type '{}'",
        suits.len(),
        suit_type.as_str()
    );
    Ok(suits)
}

/// Get a specific configuration suit from the database
pub async fn get_config_suit(
    pool: &Pool<Sqlite>,
    id: &str,
) -> Result<Option<ConfigSuit>> {
    tracing::debug!(
        "Executing SQL query to get configuration suit with ID {}",
        id
    );

    let suit = sqlx::query_as::<_, ConfigSuit>(
        r#"
        SELECT * FROM config_suit
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("Failed to fetch configuration suit")?;

    if let Some(ref s) = suit {
        tracing::debug!(
            "Found configuration suit '{}' with ID {}, type: {}",
            s.name,
            id,
            s.suit_type
        );
    } else {
        tracing::debug!("No configuration suit found with ID {}", id);
    }

    Ok(suit)
}

/// Get a specific configuration suit by name from the database
pub async fn get_config_suit_by_name(
    pool: &Pool<Sqlite>,
    name: &str,
) -> Result<Option<ConfigSuit>> {
    tracing::debug!("Executing SQL query to get configuration suit '{}'", name);

    let suit = sqlx::query_as::<_, ConfigSuit>(
        r#"
        SELECT * FROM config_suit
        WHERE name = ?
        "#,
    )
    .bind(name)
    .fetch_optional(pool)
    .await
    .context("Failed to fetch configuration suit by name")?;

    if let Some(ref s) = suit {
        tracing::debug!(
            "Found configuration suit '{}' with ID {}, type: {}",
            name,
            s.id.as_ref().unwrap_or(&"unknown".to_string()),
            s.suit_type
        );
    } else {
        tracing::debug!("No configuration suit found with name '{}'", name);
    }

    Ok(suit)
}
