//! MCP resource projection for profile-scoped Skills-style guidance.

use anyhow::{Context, Result};
use rmcp::model::{RawResource, ReadResourceResult, Resource, ResourceContents};
use serde::Serialize;
use sqlx::{Pool, Sqlite};
use std::collections::HashSet;

use crate::config::models::ProfileGuidance;

pub const PROFILE_GUIDANCE_INDEX_URI: &str = "skill://index.json";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProfileGuidanceIndex {
    schema_version: u8,
    kind: &'static str,
    resources: Vec<ProfileGuidanceIndexEntry>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProfileGuidanceIndexEntry {
    uri: String,
    profile_id: String,
    slug: String,
    title: String,
    summary: Option<String>,
    source_uri: Option<String>,
    mime_type: &'static str,
    guidance_schema: ProfileGuidanceIndexSchema,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProfileGuidanceIndexSchema {
    scenario: Option<String>,
    activation: Option<String>,
    capability_refs_count: usize,
    validation_notes: Option<String>,
    avoid: Option<String>,
}

/// Return MCP resources for profile-scoped Skills-style guidance.
pub async fn list_profile_guidance_resources(
    pool: &Pool<Sqlite>,
    profile_ids: &[String],
) -> Result<Vec<Resource>> {
    if profile_ids.is_empty() {
        return Ok(Vec::new());
    }

    let records = list_enabled_guidance_for_profiles(pool, profile_ids).await?;
    let mut resources = vec![resource(
        PROFILE_GUIDANCE_INDEX_URI,
        "Profile guidance index",
        Some("Profile-scoped Skills-style guidance index"),
        Some("application/json"),
    )];

    resources.extend(records.into_iter().map(|record| {
        let uri = profile_guidance_resource_uri(&record.profile_id, &record.slug);
        resource(&uri, &record.title, record.summary.as_deref(), Some("text/markdown"))
    }));

    Ok(resources)
}

/// Read a profile guidance resource when the target profile is visible.
pub async fn read_profile_guidance_resource(
    pool: &Pool<Sqlite>,
    profile_ids: &[String],
    uri: &str,
) -> Result<Option<ReadResourceResult>> {
    if profile_ids.is_empty() || !is_profile_guidance_uri(uri) {
        return Ok(None);
    }

    if uri == PROFILE_GUIDANCE_INDEX_URI {
        let records = list_enabled_guidance_for_profiles(pool, profile_ids).await?;
        let entries = records
            .into_iter()
            .map(|record| ProfileGuidanceIndexEntry {
                uri: profile_guidance_resource_uri(&record.profile_id, &record.slug),
                profile_id: record.profile_id,
                slug: record.slug,
                title: record.title,
                summary: record.summary,
                source_uri: record.source_uri,
                mime_type: "text/markdown",
                guidance_schema: ProfileGuidanceIndexSchema {
                    scenario: record.scenario,
                    activation: record.activation,
                    capability_refs_count: record.capability_refs.len(),
                    validation_notes: record.validation_notes,
                    avoid: record.avoid,
                },
            })
            .collect::<Vec<_>>();
        let index = ProfileGuidanceIndex {
            schema_version: 1,
            kind: "profileGuidanceIndex",
            resources: entries,
        };
        let text = serde_json::to_string_pretty(&index).context("Failed to serialize profile guidance index")?;
        return Ok(Some(text_result(uri, text, "application/json")));
    }

    let Some((profile_id, slug)) = parse_profile_guidance_uri(uri) else {
        return Ok(None);
    };
    let visible_profile_ids = profile_ids.iter().collect::<HashSet<_>>();
    if !visible_profile_ids.contains(&profile_id) {
        return Ok(None);
    }

    let Some(record) = crate::config::profile::guidance::get_profile_guidance(pool, &profile_id, &slug).await? else {
        return Ok(None);
    };
    if !record.enabled {
        return Ok(None);
    }

    Ok(Some(text_result(
        uri,
        render_guidance_markdown(&record),
        "text/markdown",
    )))
}

pub fn profile_guidance_resource_uri(
    profile_id: &str,
    slug: &str,
) -> String {
    format!("skill://profiles/{profile_id}/{slug}/SKILL.md")
}

pub fn is_profile_guidance_uri(uri: &str) -> bool {
    uri == PROFILE_GUIDANCE_INDEX_URI || uri.starts_with("skill://profiles/")
}

fn resource(
    uri: &str,
    name: &str,
    description: Option<&str>,
    mime_type: Option<&str>,
) -> Resource {
    let mut raw = RawResource::new(uri, name);
    raw.title = Some(name.to_string());
    raw.description = description.map(str::to_string);
    raw.mime_type = mime_type.map(str::to_string);
    Resource { raw, annotations: None }
}

fn text_result(
    uri: &str,
    text: String,
    mime_type: &str,
) -> ReadResourceResult {
    ReadResourceResult::new(vec![
        ResourceContents::text(text, uri).with_mime_type(mime_type.to_string()),
    ])
}

fn render_guidance_markdown(record: &ProfileGuidance) -> String {
    let mut out = format!("# {}\n\n", record.title);
    if let Some(summary) = trimmed_non_empty(record.summary.as_deref()) {
        out.push_str(summary);
        out.push_str("\n\n");
    }
    append_section(&mut out, "Scenario", record.scenario.as_deref());
    append_section(&mut out, "Activation", record.activation.as_deref());
    if !record.capability_refs.is_empty() {
        out.push_str("## Capabilities\n\n");
        for capability in &record.capability_refs {
            append_capability_ref(&mut out, capability);
        }
        out.push('\n');
    }
    append_section(&mut out, "Validation", record.validation_notes.as_deref());
    append_section(&mut out, "Avoid", record.avoid.as_deref());
    if let Some(source_uri) = trimmed_non_empty(record.source_uri.as_deref()) {
        out.push_str("Source: ");
        out.push_str(source_uri);
        out.push_str("\n\n");
    }
    out.push_str(record.content_markdown.trim());
    out.push('\n');
    out
}

fn append_section(
    out: &mut String,
    title: &str,
    value: Option<&str>,
) {
    let Some(value) = trimmed_non_empty(value) else {
        return;
    };
    out.push_str("## ");
    out.push_str(title);
    out.push_str("\n\n");
    out.push_str(value);
    out.push_str("\n\n");
}

fn append_capability_ref(
    out: &mut String,
    capability: &crate::config::models::ProfileGuidanceCapabilityRef,
) {
    out.push_str("- ");
    out.push_str(capability.kind.trim());
    out.push_str(": ");
    out.push_str(capability.id.trim());
    if let Some(name) = trimmed_non_empty(capability.name.as_deref()) {
        out.push_str(" (");
        out.push_str(name);
        out.push(')');
    }
    if let Some(server_name) = trimmed_non_empty(capability.server_name.as_deref()) {
        out.push_str(" via ");
        out.push_str(server_name);
    }
    out.push('\n');
}

fn trimmed_non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn parse_profile_guidance_uri(uri: &str) -> Option<(String, String)> {
    let rest = uri.strip_prefix("skill://profiles/")?;
    let (profile_id, rest) = rest.split_once('/')?;
    let slug = rest.strip_suffix("/SKILL.md")?;
    if profile_id.is_empty() || slug.is_empty() {
        return None;
    }
    Some((profile_id.to_string(), slug.to_string()))
}

async fn list_enabled_guidance_for_profiles(
    pool: &Pool<Sqlite>,
    profile_ids: &[String],
) -> Result<Vec<ProfileGuidance>> {
    let mut records = Vec::new();
    for profile_id in profile_ids {
        records.extend(crate::config::profile::guidance::list_enabled_profile_guidance(pool, profile_id).await?);
    }
    records.sort_by(|left, right| {
        left.title
            .cmp(&right.title)
            .then_with(|| left.profile_id.cmp(&right.profile_id))
            .then_with(|| left.slug.cmp(&right.slug))
    });
    Ok(records)
}
