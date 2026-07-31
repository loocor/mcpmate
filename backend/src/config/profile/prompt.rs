use std::str::FromStr;

use anyhow::{Context, Result};
use mcpmate_capability_store::{CapabilityKind, CapabilityRefId, EffectiveCapabilityDefinition};
use sqlx::{Pool, Sqlite};

use crate::config::{
    models::ProfilePrompt,
    profile::capability_ref::{
        load_capability_server_name, load_profile_capability_refs, upsert_profile_capability_ref,
    },
};

pub async fn add_prompt_to_profile(
    pool: &Pool<Sqlite>,
    profile_id: &str,
    server_id: &str,
    ref_id: &str,
    enabled: bool,
) -> Result<String> {
    let ref_id = parse_owned_ref(pool, server_id, ref_id, CapabilityKind::Prompts).await?;
    upsert_profile_capability_ref(pool, profile_id, &ref_id, enabled).await?;
    Ok(ref_id.to_string())
}

pub async fn remove_prompt_from_profile(
    pool: &Pool<Sqlite>,
    profile_id: &str,
    server_id: &str,
    ref_id: &str,
) -> Result<bool> {
    let ref_id = parse_owned_ref(pool, server_id, ref_id, CapabilityKind::Prompts).await?;
    let result = sqlx::query("DELETE FROM profile_capability_refs WHERE profile_id = ? AND ref_id = ?")
        .bind(profile_id)
        .bind(ref_id.as_str())
        .execute(pool)
        .await
        .context("Failed to remove Prompt capability ref from Profile")?;
    Ok(result.rows_affected() == 1)
}

pub async fn get_prompts_for_profile(
    pool: &Pool<Sqlite>,
    profile_id: &str,
) -> Result<Vec<ProfilePrompt>> {
    let relationships = load_profile_capability_refs(pool, profile_id, Some(CapabilityKind::Prompts)).await?;
    let mut prompts = Vec::with_capacity(relationships.len());
    for relationship in relationships {
        let server_name = load_capability_server_name(pool, &relationship.server_id).await?;
        let EffectiveCapabilityDefinition::Prompt(prompt) = relationship.definition else {
            anyhow::bail!(
                "Capability ref '{}' does not contain a Prompt definition",
                relationship.ref_id
            );
        };
        prompts.push(ProfilePrompt {
            id: Some(relationship.ref_id.to_string()),
            profile_id: relationship.profile_id,
            server_id: relationship.server_id,
            server_name,
            prompt_name: relationship.origin_key,
            unique_name: relationship.external_key,
            description: prompt.description,
            enabled: relationship.enabled,
            state: relationship.state.as_str().to_string(),
            state_generation: relationship.state_generation,
        });
    }
    Ok(prompts)
}

pub async fn update_prompt_enabled_status(
    pool: &Pool<Sqlite>,
    profile_id: &str,
    ref_id: &str,
    enabled: bool,
) -> Result<()> {
    let ref_id =
        CapabilityRefId::from_str(ref_id).with_context(|| format!("Invalid Prompt capability ref '{ref_id}'"))?;
    let (_, kind) = load_ref_owner(pool, &ref_id).await?;
    if kind != CapabilityKind::Prompts {
        anyhow::bail!("Capability ref '{}' is not a Prompt", ref_id);
    }
    let result = sqlx::query("UPDATE profile_capability_refs SET enabled = ? WHERE profile_id = ? AND ref_id = ?")
        .bind(enabled)
        .bind(profile_id)
        .bind(ref_id.as_str())
        .execute(pool)
        .await
        .context("Failed to update Prompt capability ref")?;
    if result.rows_affected() != 1 {
        anyhow::bail!("Prompt relationship '{}' not found in Profile '{}'", ref_id, profile_id);
    }
    Ok(())
}

pub async fn get_enabled_prompts_for_profile(
    pool: &Pool<Sqlite>,
    profile_id: &str,
) -> Result<Vec<ProfilePrompt>> {
    Ok(get_prompts_for_profile(pool, profile_id)
        .await?
        .into_iter()
        .filter(|prompt| prompt.enabled && prompt.state == "active")
        .collect())
}

pub fn build_enabled_prompts_query(additional_where: Option<&str>) -> String {
    let base_query = r#"
        SELECT DISTINCT cr.server_id, sc.name AS server_name, cr.origin_key AS prompt_name
        FROM profile_capability_refs pcr
        JOIN capability_refs cr ON cr.ref_id = pcr.ref_id
        JOIN profile p ON p.id = pcr.profile_id
        JOIN server_config sc ON sc.id = cr.server_id
        WHERE p.is_active = 1
          AND pcr.enabled = 1
          AND cr.state = 'active'
          AND cr.kind = 'prompts'
          AND sc.enabled = 1"#;
    match additional_where {
        Some(condition) => format!("{base_query} AND {condition}"),
        None => base_query.to_string(),
    }
}

async fn parse_owned_ref(
    pool: &Pool<Sqlite>,
    server_id: &str,
    ref_id: &str,
    expected_kind: CapabilityKind,
) -> Result<CapabilityRefId> {
    let ref_id = CapabilityRefId::from_str(ref_id).with_context(|| format!("Invalid capability ref '{ref_id}'"))?;
    let (owner_server_id, kind) = load_ref_owner(pool, &ref_id).await?;
    if owner_server_id != server_id || kind != expected_kind {
        anyhow::bail!(
            "Capability ref '{}' is not a {:?} owned by server '{}'",
            ref_id,
            expected_kind,
            server_id
        );
    }
    Ok(ref_id)
}

async fn load_ref_owner(
    pool: &Pool<Sqlite>,
    ref_id: &CapabilityRefId,
) -> Result<(String, CapabilityKind)> {
    let (server_id, kind): (String, String) =
        sqlx::query_as("SELECT server_id, kind FROM capability_refs WHERE ref_id = ?")
            .bind(ref_id.as_str())
            .fetch_optional(pool)
            .await
            .context("Failed to load capability ref")?
            .ok_or_else(|| anyhow::anyhow!("Capability ref '{}' is not registered", ref_id))?;
    let kind = CapabilityKind::parse(&kind).ok_or_else(|| anyhow::anyhow!("Invalid capability kind '{}'", kind))?;
    Ok((server_id, kind))
}
