// System-owned default Profile normalization.

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};

use crate::{common::profile::ProfileType, generate_id};

use super::{DEFAULT_ANCHOR_INITIAL_NAME, DEFAULT_ANCHOR_ROLE, DEFAULT_PROFILE_DESCRIPTION};

/// Ensure the system default anchor exists and satisfies its invariant contract.
pub async fn ensure_default_anchor_profile_id(pool: &Pool<Sqlite>) -> Result<String> {
    normalize_default_anchor_profile(pool).await
}

/// Normalize the system-owned default anchor under one serialized write transaction.
pub(crate) async fn normalize_default_anchor_profile(pool: &Pool<Sqlite>) -> Result<String> {
    let mut transaction = pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .context("Failed to begin default anchor normalization")?;
    let current = sqlx::query_as::<_, (String, String, String, bool, bool, i64)>(
        r#"
        SELECT id, type, role, is_active, is_default, authoring_generation
        FROM profile
        WHERE is_default = 1 OR role = 'default_anchor'
        ORDER BY CASE WHEN role = 'default_anchor' THEN 0 ELSE 1 END, created_at
        LIMIT 1
        "#,
    )
    .fetch_optional(&mut *transaction)
    .await
    .context("Failed to load default anchor Profile")?;

    let profile_id = if let Some((id, profile_type, role, is_active, is_default, generation)) = current {
        let needs_update = profile_type != ProfileType::Shared.as_str()
            || role != DEFAULT_ANCHOR_ROLE.as_str()
            || !is_active
            || !is_default;
        if needs_update {
            let updated = sqlx::query(
                r#"
                UPDATE profile
                SET type = 'shared',
                    role = 'default_anchor',
                    is_active = 1,
                    is_default = 1,
                    authoring_generation = authoring_generation + 1,
                    updated_at = CURRENT_TIMESTAMP
                WHERE id = ? AND authoring_generation = ?
                "#,
            )
            .bind(&id)
            .bind(generation)
            .execute(&mut *transaction)
            .await
            .context("Failed to normalize default anchor Profile")?;
            if updated.rows_affected() != 1 {
                return Err(anyhow::anyhow!("Default anchor Profile changed during normalization"));
            }
        }
        id
    } else {
        let id = generate_id!("prof");
        sqlx::query(
            r#"
            INSERT INTO profile (
                id, name, description, type, role,
                priority, is_active, is_default, authoring_generation
            ) VALUES (?, ?, ?, 'shared', 'default_anchor', 0, 1, 1, 0)
            "#,
        )
        .bind(&id)
        .bind(DEFAULT_ANCHOR_INITIAL_NAME)
        .bind(DEFAULT_PROFILE_DESCRIPTION)
        .execute(&mut *transaction)
        .await
        .context("Failed to create default anchor Profile")?;
        id
    };

    transaction
        .commit()
        .await
        .context("Failed to commit default anchor normalization")?;
    Ok(profile_id)
}
