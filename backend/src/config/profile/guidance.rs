// Profile-scoped Skills-style guidance operations.

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};

use crate::{
    common::constants::database::{columns, tables},
    config::models::ProfileGuidance,
    generate_id,
};

pub use crate::config::models::ProfileGuidanceCapabilityRef;

/// Draft payload for creating or updating profile guidance.
#[derive(Debug, Clone)]
pub struct ProfileGuidanceDraft {
    pub id: Option<String>,
    pub profile_id: String,
    pub slug: String,
    pub title: String,
    pub summary: Option<String>,
    pub scenario: Option<String>,
    pub activation: Option<String>,
    pub capability_refs: Vec<ProfileGuidanceCapabilityRef>,
    pub validation_notes: Option<String>,
    pub avoid: Option<String>,
    pub content_markdown: String,
    pub source_uri: Option<String>,
    pub enabled: bool,
}

/// Create or update a profile guidance record by profile-local slug.
pub async fn upsert_profile_guidance(
    pool: &Pool<Sqlite>,
    draft: ProfileGuidanceDraft,
) -> Result<String> {
    let guidance_id = draft.id.unwrap_or_else(|| generate_id!("pgui"));
    let capability_refs_json = serde_json::to_string(&draft.capability_refs)
        .context("Failed to serialize profile guidance capability refs")?;

    sqlx::query(&format!(
        r#"
        INSERT INTO {} (
            {}, {}, slug, title, summary, scenario, activation, capability_refs_json,
            validation_notes, avoid, content_markdown, source_uri, enabled
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT({}, slug) DO UPDATE SET
            title = excluded.title,
            summary = excluded.summary,
            scenario = excluded.scenario,
            activation = excluded.activation,
            capability_refs_json = excluded.capability_refs_json,
            validation_notes = excluded.validation_notes,
            avoid = excluded.avoid,
            content_markdown = excluded.content_markdown,
            source_uri = excluded.source_uri,
            enabled = excluded.enabled,
            updated_at = CURRENT_TIMESTAMP
        "#,
        tables::PROFILE_GUIDANCE,
        columns::ID,
        columns::PROFILE_ID,
        columns::PROFILE_ID
    ))
    .bind(&guidance_id)
    .bind(&draft.profile_id)
    .bind(&draft.slug)
    .bind(&draft.title)
    .bind(&draft.summary)
    .bind(&draft.scenario)
    .bind(&draft.activation)
    .bind(&capability_refs_json)
    .bind(&draft.validation_notes)
    .bind(&draft.avoid)
    .bind(&draft.content_markdown)
    .bind(&draft.source_uri)
    .bind(draft.enabled)
    .execute(pool)
    .await
    .context("Failed to upsert profile guidance")?;

    sqlx::query_scalar::<_, String>(&format!(
        r#"
        SELECT {}
        FROM {}
        WHERE {} = ? AND slug = ?
        "#,
        columns::ID,
        tables::PROFILE_GUIDANCE,
        columns::PROFILE_ID
    ))
    .bind(&draft.profile_id)
    .bind(&draft.slug)
    .fetch_one(pool)
    .await
    .context("Failed to load upserted profile guidance id")
}

/// List guidance records for one profile.
pub async fn list_profile_guidance(
    pool: &Pool<Sqlite>,
    profile_id: &str,
) -> Result<Vec<ProfileGuidance>> {
    sqlx::query_as::<_, ProfileGuidance>(&format!(
        r#"
        SELECT *
        FROM {}
        WHERE {} = ?
        ORDER BY title, slug
        "#,
        tables::PROFILE_GUIDANCE,
        columns::PROFILE_ID
    ))
    .bind(profile_id)
    .fetch_all(pool)
    .await
    .context("Failed to list profile guidance")
}

/// List enabled guidance records for one profile.
pub async fn list_enabled_profile_guidance(
    pool: &Pool<Sqlite>,
    profile_id: &str,
) -> Result<Vec<ProfileGuidance>> {
    sqlx::query_as::<_, ProfileGuidance>(&format!(
        r#"
        SELECT *
        FROM {}
        WHERE {} = ? AND enabled = 1
        ORDER BY title, slug
        "#,
        tables::PROFILE_GUIDANCE,
        columns::PROFILE_ID
    ))
    .bind(profile_id)
    .fetch_all(pool)
    .await
    .context("Failed to list enabled profile guidance")
}

/// Load one profile guidance record by profile-local slug.
pub async fn get_profile_guidance(
    pool: &Pool<Sqlite>,
    profile_id: &str,
    slug: &str,
) -> Result<Option<ProfileGuidance>> {
    sqlx::query_as::<_, ProfileGuidance>(&format!(
        r#"
        SELECT *
        FROM {}
        WHERE {} = ? AND slug = ?
        "#,
        tables::PROFILE_GUIDANCE,
        columns::PROFILE_ID
    ))
    .bind(profile_id)
    .bind(slug)
    .fetch_optional(pool)
    .await
    .context("Failed to get profile guidance")
}

/// Delete one profile guidance record by profile-local slug.
pub async fn delete_profile_guidance(
    pool: &Pool<Sqlite>,
    profile_id: &str,
    slug: &str,
) -> Result<bool> {
    let result = sqlx::query(&format!(
        r#"
        DELETE FROM {}
        WHERE {} = ? AND slug = ?
        "#,
        tables::PROFILE_GUIDANCE,
        columns::PROFILE_ID
    ))
    .bind(profile_id)
    .bind(slug)
    .execute(pool)
    .await
    .context("Failed to delete profile guidance")?;

    Ok(result.rows_affected() > 0)
}
