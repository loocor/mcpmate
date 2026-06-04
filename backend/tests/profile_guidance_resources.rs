use anyhow::{Context, Result};
use mcpmate::common::profile::ProfileType;
use mcpmate::config::models::Profile;
use mcpmate::config::profile::guidance::{ProfileGuidanceCapabilityRef, ProfileGuidanceDraft, upsert_profile_guidance};
use mcpmate::config::profile::{init::initialize_profile_tables, upsert_profile};
use mcpmate::core::profile::guidance::{
    PROFILE_GUIDANCE_INDEX_URI, list_profile_guidance_resources, read_profile_guidance_resource,
};
use rmcp::model::ResourceContents;
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
    let mut profile = Profile::new(name.to_string(), ProfileType::Scenario);
    profile.is_active = true;
    upsert_profile(pool, &profile).await
}

fn triage_guidance_draft(profile_id: String) -> ProfileGuidanceDraft {
    ProfileGuidanceDraft {
        id: None,
        profile_id,
        slug: "triage".to_string(),
        title: "Triage customer tickets".to_string(),
        summary: Some("Use CRM and ticket tools in order.".to_string()),
        scenario: Some("Customer support triage".to_string()),
        activation: Some("Use when a customer support ticket needs context gathering.".to_string()),
        capability_refs: vec![ProfileGuidanceCapabilityRef {
            kind: "tool".to_string(),
            id: "ticket_lookup".to_string(),
            name: Some("Ticket lookup".to_string()),
            server_name: Some("support".to_string()),
        }],
        validation_notes: Some("Confirm the ticket id before calling tools.".to_string()),
        avoid: Some("Do not contact the customer before reviewing history.".to_string()),
        content_markdown: "## Workflow\nLoad customer context first.".to_string(),
        source_uri: None,
        enabled: true,
    }
}

fn disabled_guidance_draft(profile_id: String) -> ProfileGuidanceDraft {
    ProfileGuidanceDraft {
        id: None,
        profile_id,
        slug: "disabled".to_string(),
        title: "Disabled guidance".to_string(),
        summary: None,
        scenario: None,
        activation: None,
        capability_refs: Vec::new(),
        validation_notes: None,
        avoid: None,
        content_markdown: "Hidden.".to_string(),
        source_uri: None,
        enabled: false,
    }
}

#[tokio::test]
async fn list_profile_guidance_resources_returns_index_and_enabled_skill_resources() -> Result<()> {
    let pool = setup_pool().await?;
    let profile_id = insert_profile(&pool, "customer support").await?;
    let linked_server_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM profile_server WHERE profile_id = ?")
        .bind(&profile_id)
        .fetch_one(&pool)
        .await?;

    assert_eq!(linked_server_count, 0);

    upsert_profile_guidance(&pool, triage_guidance_draft(profile_id.clone())).await?;
    upsert_profile_guidance(&pool, disabled_guidance_draft(profile_id.clone())).await?;

    let resources = list_profile_guidance_resources(&pool, std::slice::from_ref(&profile_id)).await?;
    let uris = resources
        .iter()
        .map(|resource| resource.raw.uri.as_str())
        .collect::<Vec<_>>();

    assert!(uris.contains(&PROFILE_GUIDANCE_INDEX_URI));
    assert!(uris.contains(&format!("skill://profiles/{profile_id}/triage/SKILL.md").as_str()));
    assert!(!uris.contains(&format!("skill://profiles/{profile_id}/disabled/SKILL.md").as_str()));
    Ok(())
}

#[tokio::test]
async fn read_profile_guidance_resource_enforces_visible_profiles() -> Result<()> {
    let pool = setup_pool().await?;
    let profile_id = insert_profile(&pool, "customer support").await?;
    let other_profile_id = insert_profile(&pool, "sales research").await?;

    let mut draft = triage_guidance_draft(profile_id.clone());
    draft.source_uri = Some("https://example.com/skills/triage/SKILL.md".to_string());
    upsert_profile_guidance(&pool, draft).await?;

    let uri = format!("skill://profiles/{profile_id}/triage/SKILL.md");
    let visible = read_profile_guidance_resource(&pool, std::slice::from_ref(&profile_id), &uri).await?;
    let hidden = read_profile_guidance_resource(&pool, &[other_profile_id], &uri).await?;

    let Some(result) = visible else {
        panic!("expected visible profile guidance resource");
    };
    let ResourceContents::TextResourceContents { text, mime_type, .. } = &result.contents[0] else {
        panic!("expected text resource");
    };

    assert_eq!(mime_type.as_deref(), Some("text/markdown"));
    assert!(text.contains("# Triage customer tickets"));
    assert!(text.contains("## Scenario"));
    assert!(text.contains("Customer support triage"));
    assert!(text.contains("## Capabilities"));
    assert!(text.contains("- tool: ticket_lookup"));
    assert!(text.contains("## Avoid"));
    assert!(text.contains("Source: https://example.com/skills/triage/SKILL.md"));
    assert!(hidden.is_none());

    Ok(())
}

#[tokio::test]
async fn read_profile_guidance_index_returns_schema_object() -> Result<()> {
    let pool = setup_pool().await?;
    let profile_id = insert_profile(&pool, "customer support").await?;

    let mut draft = triage_guidance_draft(profile_id.clone());
    draft.activation = None;
    draft.capability_refs = Vec::new();
    draft.validation_notes = None;
    draft.avoid = None;
    upsert_profile_guidance(&pool, draft).await?;

    let Some(result) =
        read_profile_guidance_resource(&pool, std::slice::from_ref(&profile_id), PROFILE_GUIDANCE_INDEX_URI).await?
    else {
        panic!("expected profile guidance index");
    };
    let ResourceContents::TextResourceContents { text, mime_type, .. } = &result.contents[0] else {
        panic!("expected text resource");
    };
    let index: serde_json::Value = serde_json::from_str(text)?;

    assert_eq!(mime_type.as_deref(), Some("application/json"));
    assert_eq!(index["schemaVersion"], 1);
    assert_eq!(index["kind"], "profileGuidanceIndex");
    assert_eq!(index["resources"][0]["profileId"], profile_id);
    assert_eq!(
        index["resources"][0]["uri"],
        format!("skill://profiles/{profile_id}/triage/SKILL.md")
    );
    assert_eq!(index["resources"][0]["mimeType"], "text/markdown");
    assert_eq!(
        index["resources"][0]["guidanceSchema"]["scenario"],
        "Customer support triage"
    );

    Ok(())
}
