// Management operations for Profile
// Contains create, update, delete and status management operations

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};

use crate::{
    config::{models::Profile, profile::is_primary_default_profile},
    generate_id,
};

use super::basic::get_profile;

/// Create or update a profile in the database
pub async fn upsert_profile(
    pool: &Pool<Sqlite>,
    profile: &Profile,
) -> Result<String> {
    tracing::debug!("Upserting profile '{}', type: {}", profile.name, profile.profile_type);

    // Generate an ID for the profile if it doesn't have one
    let profile_id = if let Some(id) = &profile.id {
        id.clone()
    } else {
        generate_id!("prof")
    };

    let mut tx = pool.begin().await.context("Failed to begin transaction")?;

    let result = sqlx::query(
        r#"
        INSERT INTO profile (id, name, description, type, multi_select, priority, is_active, is_default)
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
    .bind(&profile_id)
    .bind(&profile.name)
    .bind(&profile.description)
    .bind(profile.profile_type)
    .bind(profile.multi_select)
    .bind(profile.priority)
    .bind(profile.is_active)
    .bind(profile.is_default)
    .execute(&mut *tx)
    .await
    .context("Failed to upsert profile")?;

    let final_profile_id = if result.rows_affected() == 0 {
        // If no rows were affected, get the existing ID

        sqlx::query_scalar::<_, String>(
            r#"
            SELECT id FROM profile
            WHERE name = ?
            "#,
        )
        .bind(&profile.name)
        .fetch_one(&mut *tx)
        .await
        .context("Failed to get profile ID")?
    } else {
        profile_id
    };

    tx.commit().await.context("Failed to commit transaction")?;

    Ok(final_profile_id)
}

/// Update an existing profile by ID
///
/// This function is specifically designed for updating existing profile based on their ID,
/// unlike upsert_profile which is designed for creation scenarios with name-based conflict detection.
pub async fn update_profile(
    pool: &Pool<Sqlite>,
    profile: &Profile,
) -> Result<()> {
    let profile_id = profile
        .id
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Profile ID is required for update operation"))?;

    tracing::debug!("Updating profile '{}' with ID '{}'", profile.name, profile_id);

    let result = sqlx::query(
        r#"
        UPDATE profile
        SET name = ?, description = ?, type = ?, multi_select = ?, priority = ?, is_active = ?, is_default = ?, updated_at = CURRENT_TIMESTAMP
        WHERE id = ?
        "#,
    )
    .bind(&profile.name)
    .bind(&profile.description)
    .bind(profile.profile_type)
    .bind(profile.multi_select)
    .bind(profile.priority)
    .bind(profile.is_active)
    .bind(profile.is_default)
    .bind(profile_id)
    .execute(pool)
    .await
    .context("Failed to update profile")?;

    if result.rows_affected() == 0 {
        return Err(anyhow::anyhow!("Profile with ID '{}' not found", profile_id));
    }

    tracing::debug!(
        "Successfully updated profile '{}' with ID '{}'",
        profile.name,
        profile_id
    );

    Ok(())
}

/// Set a profile as active or inactive
///
/// This function updates the active status of a profile in the database.
/// If the status is updated, it also publishes a ProfileStatusChanged event.
pub async fn set_profile_active(
    pool: &Pool<Sqlite>,
    profile_id: &str,
    active: bool,
) -> Result<()> {
    tracing::debug!(
        "Setting profile with ID {} as {}",
        profile_id,
        if active { "active" } else { "inactive" }
    );

    // Get the profile to check multi_select
    let profile = get_profile(pool, profile_id).await?;
    if profile.is_none() {
        return Err(anyhow::anyhow!("Profile with ID {} not found", profile_id));
    }
    let profile = profile.unwrap();

    // Disallow deactivating the default profile
    if is_primary_default_profile(&profile) && !active {
        return Err(anyhow::anyhow!("The system default profile cannot be deactivated"));
    }

    let mut tx = pool.begin().await.context("Failed to begin transaction")?;

    // If activating and multi_select is false, deactivate all other profile (except default)
    if active && !profile.multi_select {
        sqlx::query(
            r#"
            UPDATE profile
            SET is_active = 0,
                updated_at = CURRENT_TIMESTAMP
            WHERE id != ? AND is_default = 0
            "#,
        )
        .bind(profile_id)
        .execute(&mut *tx)
        .await
        .context("Failed to deactivate other profile")?;
    }

    // Update the specified profile
    if active {
        sqlx::query(
            r#"
            UPDATE profile
            SET is_active = 1,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = ?
            "#,
        )
        .bind(profile_id)
        .execute(&mut *tx)
        .await
        .context("Failed to update profile active status")?;
    } else {
        sqlx::query(
            r#"
            UPDATE profile
            SET is_active = 0,
                is_default = 0,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = ?
            "#,
        )
        .bind(profile_id)
        .execute(&mut *tx)
        .await
        .context("Failed to update profile active status")?;
    }

    tx.commit().await.context("Failed to commit transaction")?;

    // Publish the event
    crate::core::events::EventBus::global().publish(crate::core::events::Event::ProfileStatusChanged {
        profile_id: profile_id.to_string(),
        enabled: active,
    });

    tracing::info!(
        "Published ProfileStatusChanged event for profile ID {} ({})",
        profile_id,
        active
    );

    Ok(())
}

/// Set a profile as the default
pub async fn set_profile_default(
    pool: &Pool<Sqlite>,
    profile_id: &str,
) -> Result<()> {
    tracing::debug!("Setting profile with ID {} as default", profile_id);

    let mut tx = pool.begin().await.context("Failed to begin transaction")?;

    // Set the specified profile as default and ensure it remains active
    sqlx::query(
        r#"
        UPDATE profile
        SET is_default = 1,
            is_active = 1,
            updated_at = CURRENT_TIMESTAMP
        WHERE id = ?
        "#,
    )
    .bind(profile_id)
    .execute(&mut *tx)
    .await
    .context("Failed to set profile as default")?;

    tx.commit().await.context("Failed to commit transaction")?;
    Ok(())
}

/// Delete a profile from the database
pub async fn delete_profile(
    pool: &Pool<Sqlite>,
    id: &str,
) -> Result<bool> {
    tracing::debug!("Deleting profile with ID {}", id);

    // Prevent deleting the default profile at the data layer as well
    if let Some(p) = get_profile(pool, id).await? {
        if is_primary_default_profile(&p) {
            return Err(anyhow::anyhow!("Cannot delete the system default profile"));
        }
    }

    let result = sqlx::query(
        r#"
        DELETE FROM profile
        WHERE id = ?
        "#,
    )
    .bind(id)
    .execute(pool)
    .await
    .context("Failed to delete profile")?;

    Ok(result.rows_affected() > 0)
}
