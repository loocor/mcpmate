use anyhow::{Context, Result};
use mcpmate::common::profile::ProfileType;
use mcpmate::config::models::Profile;
use mcpmate::config::profile::guidance::{
    ProfileGuidanceCapabilityRef, ProfileGuidanceDraft, delete_profile_guidance, get_profile_guidance,
    list_profile_guidance, upsert_profile_guidance,
};
use mcpmate::config::profile::{delete_profile, init::initialize_profile_tables, upsert_profile};
use sqlx::sqlite::SqlitePoolOptions;

async fn setup_pool() -> Result<sqlx::Pool<sqlx::Sqlite>> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .context("create in-memory sqlite pool")?;

    sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await?;
    sqlx::query(
        r#"
        CREATE TABLE server_config (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            enabled BOOLEAN NOT NULL DEFAULT 1,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(&pool)
    .await?;
    initialize_profile_tables(&pool).await?;

    Ok(pool)
}

async fn insert_profile(
    pool: &sqlx::Pool<sqlx::Sqlite>,
    name: &str,
) -> Result<String> {
    let mut profile = Profile::new_with_description(
        name.to_string(),
        Some("Scenario for guided MCP capability usage".to_string()),
        ProfileType::Scenario,
    );
    profile.is_active = true;
    upsert_profile(pool, &profile).await
}

fn guidance_draft(
    profile_id: String,
    slug: &str,
    title: &str,
    content_markdown: &str,
) -> ProfileGuidanceDraft {
    ProfileGuidanceDraft {
        id: None,
        profile_id,
        slug: slug.to_string(),
        title: title.to_string(),
        summary: None,
        scenario: None,
        activation: None,
        capability_refs: Vec::new(),
        validation_notes: None,
        avoid: None,
        content_markdown: content_markdown.to_string(),
        source_uri: None,
        enabled: true,
    }
}

#[tokio::test]
async fn initialize_profile_tables_creates_guidance_table() -> Result<()> {
    let pool = setup_pool().await?;

    let found = sqlx::query_scalar::<_, String>(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='profile_guidance'",
    )
    .fetch_optional(&pool)
    .await?;

    assert_eq!(found.as_deref(), Some("profile_guidance"));
    Ok(())
}

#[tokio::test]
async fn upsert_profile_guidance_lists_profile_scoped_records() -> Result<()> {
    let pool = setup_pool().await?;
    let profile_id = insert_profile(&pool, "customer support").await?;
    let other_profile_id = insert_profile(&pool, "sales research").await?;

    let support_id = upsert_profile_guidance(&pool, {
        let mut draft = guidance_draft(
            profile_id.clone(),
            "triage",
            "Triage customer tickets",
            "## Workflow\n1. Load customer context.\n2. Inspect ticket history.",
        );
        draft.summary = Some("Use CRM and ticket tools in order.".to_string());
        draft.scenario = Some("Customer support triage".to_string());
        draft.activation = Some("Use when a customer support ticket needs context gathering.".to_string());
        draft.capability_refs = vec![ProfileGuidanceCapabilityRef {
            kind: "tool".to_string(),
            id: "ticket_lookup".to_string(),
            name: Some("Ticket lookup".to_string()),
            server_name: Some("support".to_string()),
        }];
        draft.validation_notes = Some("Confirm the ticket id before calling tools.".to_string());
        draft.avoid = Some("Do not contact the customer before reviewing history.".to_string());
        draft
    })
    .await?;
    let _ = upsert_profile_guidance(
        &pool,
        ProfileGuidanceDraft {
            id: None,
            profile_id: other_profile_id,
            slug: "research".to_string(),
            title: "Research accounts".to_string(),
            summary: None,
            scenario: None,
            activation: None,
            capability_refs: Vec::new(),
            validation_notes: None,
            avoid: None,
            content_markdown: "Use account research tools.".to_string(),
            source_uri: Some("https://example.com/skills/research/SKILL.md".to_string()),
            enabled: true,
        },
    )
    .await?;

    let support_records = list_profile_guidance(&pool, &profile_id).await?;

    assert_eq!(support_records.len(), 1);
    assert_eq!(support_records[0].id, support_id);
    assert_eq!(support_records[0].slug, "triage");
    assert_eq!(support_records[0].profile_id, profile_id);
    assert_eq!(support_records[0].scenario.as_deref(), Some("Customer support triage"));
    assert_eq!(support_records[0].capability_refs.len(), 1);
    assert_eq!(support_records[0].capability_refs[0].id, "ticket_lookup");
    assert!(support_records[0].enabled);

    let record = get_profile_guidance(&pool, &profile_id, "triage").await?;
    assert_eq!(
        record.as_ref().map(|item| item.title.as_str()),
        Some("Triage customer tickets")
    );

    Ok(())
}

#[tokio::test]
async fn deleting_profile_removes_profile_guidance() -> Result<()> {
    let pool = setup_pool().await?;
    let profile_id = insert_profile(&pool, "coding assistant").await?;

    upsert_profile_guidance(
        &pool,
        guidance_draft(
            profile_id.clone(),
            "review",
            "Review code changes",
            "Prioritize concrete findings before summary.",
        ),
    )
    .await?;

    assert!(delete_profile(&pool, &profile_id).await?);

    let remaining = list_profile_guidance(&pool, &profile_id).await?;
    assert!(remaining.is_empty());
    Ok(())
}

#[tokio::test]
async fn delete_profile_guidance_removes_profile_scoped_record() -> Result<()> {
    let pool = setup_pool().await?;
    let profile_id = insert_profile(&pool, "support").await?;

    upsert_profile_guidance(
        &pool,
        guidance_draft(profile_id.clone(), "triage", "Triage", "Use ticket tools."),
    )
    .await?;

    assert!(delete_profile_guidance(&pool, &profile_id, "triage").await?);
    assert!(!delete_profile_guidance(&pool, &profile_id, "missing").await?);

    let remaining = list_profile_guidance(&pool, &profile_id).await?;
    assert!(remaining.is_empty());
    Ok(())
}
