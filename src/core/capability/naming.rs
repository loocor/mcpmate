//! Unified naming utilities for capabilities
//! Provides generation, resolution, and uniqueness guarantees for tool/prompt/resource identifiers.

use anyhow::{Context, Result};
use once_cell::sync::OnceCell;
use sqlx::{Pool, Sqlite};
use tracing;

static NAMING_POOL: OnceCell<Pool<Sqlite>> = OnceCell::new();

/// Tool naming policy inferred from a server's tool list.
/// When `uniform_prefix` is present, all tool names share the same
/// lowercase, underscore-terminated prefix (e.g., "browser_").
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToolNamingPolicy {
    /// When present and all tools share it (strict), we keep vendor prefix and skip server_ prefix.
    pub uniform_prefix: Option<String>,
    /// When present (majority consensus), we strip this compound vendor subprefix from tool names
    /// before applying server_ prefix. Only used when `uniform_prefix` is None.
    pub strip_subprefix: Option<String>,
}

/// Initialize the global naming store with a database pool.
/// Safe to call multiple times; subsequent calls are ignored.
pub fn initialize(pool: Pool<Sqlite>) {
    if NAMING_POOL.set(pool).is_err() {
        tracing::debug!("Naming store already initialized");
    } else {
        tracing::debug!("Naming store initialized");
    }
}

fn pool() -> &'static Pool<Sqlite> {
    NAMING_POOL
        .get()
        .expect("Naming store not initialized; call naming::initialize first")
}

pub(crate) fn normalize_server_name(server_name: &str) -> String {
    server_name.to_lowercase().replace(' ', "_")
}

pub(crate) fn strip_server_prefix(
    kind: NamingKind,
    server_name: &str,
    value: &str,
) -> Option<String> {
    let normalized = normalize_server_name(server_name);
    match kind {
        NamingKind::Tool | NamingKind::Prompt | NamingKind::ResourceTemplate => {
            let prefix = format!("{normalized}_");
            if value.len() > prefix.len() && value.to_ascii_lowercase().starts_with(&prefix) {
                Some(value[prefix.len()..].to_string())
            } else {
                None
            }
        }
        NamingKind::Resource => {
            let prefix = format!("{normalized}:");
            if value.len() > prefix.len() && value.to_ascii_lowercase().starts_with(&prefix) {
                Some(value[prefix.len()..].to_string())
            } else {
                None
            }
        }
    }
}

/// Capability kinds supported by the naming module.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NamingKind {
    Tool,
    Prompt,
    Resource,
    ResourceTemplate,
}

impl NamingKind {
    fn table(self) -> &'static str {
        match self {
            NamingKind::Tool => "server_tools",
            NamingKind::Prompt => "server_prompts",
            NamingKind::Resource => "server_resources",
            NamingKind::ResourceTemplate => "server_resource_templates",
        }
    }

    fn unique_column(self) -> &'static str {
        match self {
            NamingKind::Tool => "unique_name",
            NamingKind::Prompt => "unique_name",
            NamingKind::Resource => "unique_uri",
            NamingKind::ResourceTemplate => "unique_name",
        }
    }

    fn value_column(self) -> &'static str {
        match self {
            NamingKind::Tool => "tool_name",
            NamingKind::Prompt => "prompt_name",
            NamingKind::Resource => "resource_uri",
            NamingKind::ResourceTemplate => "uri_template",
        }
    }
}

/// Generate a unique identifier for the given capability kind.
pub fn generate_unique_name(
    kind: NamingKind,
    server_name: &str,
    value: &str,
) -> String {
    match kind {
        NamingKind::Tool | NamingKind::Prompt | NamingKind::ResourceTemplate => {
            let normalized = normalize_server_name(server_name);
            let prefix = format!("{normalized}_");
            if value.to_lowercase().starts_with(&prefix) {
                value.to_string()
            } else {
                format!("{normalized}_{value}")
            }
        }
        NamingKind::Resource => {
            let normalized = normalize_server_name(server_name);
            let prefix = format!("{normalized}:");
            if value.to_lowercase().starts_with(&prefix) {
                value.to_string()
            } else {
                format!("{normalized}:{value}")
            }
        }
    }
}

/// Infer a uniform vendor/tool prefix from a set of tool names for the same server.
///
/// Rules (strict):
/// - At least 2 tools are required to infer a prefix.
/// - Every tool name must contain an underscore and share the same leading token + underscore.
/// - Comparison is case-insensitive and uses lowercase.
/// - The inferred prefix must not equal the server's normalized prefix (e.g., "gitmcp_").
/// - The prefix token length must be >= 2 to avoid noise (e.g., "x_").
pub fn infer_uniform_tool_prefix<'a, I>(
    server_name: &str,
    tool_names: I,
) -> ToolNamingPolicy
where
    I: IntoIterator<Item = &'a str>,
{
    let normalized_server_prefix = format!("{}_", normalize_server_name(server_name));
    let iter = tool_names
        .into_iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();

    if iter.len() < 2 {
        return ToolNamingPolicy::default();
    }

    // Determine candidate prefix from the first entry
    let first = iter[0].to_ascii_lowercase();
    let Some(idx) = first.find('_') else {
        return ToolNamingPolicy::default();
    }; // no underscore
    let token = &first[..=idx]; // include underscore
    if token.len() < 3 {
        // require at least 2 chars + underscore
        return ToolNamingPolicy::default();
    }
    if token == normalized_server_prefix {
        return ToolNamingPolicy::default();
    }

    // Validate all names share this prefix
    for name in &iter {
        let lname = name.to_ascii_lowercase();
        if !lname.starts_with(token) {
            return ToolNamingPolicy::default();
        }
    }

    ToolNamingPolicy {
        uniform_prefix: Some(token.to_string()),
        strip_subprefix: None,
    }
}

fn should_skip_server_prefix(
    policy: &ToolNamingPolicy,
    tool_name: &str,
) -> bool {
    match &policy.uniform_prefix {
        Some(prefix) => tool_name.to_ascii_lowercase().starts_with(prefix),
        None => false,
    }
}

/// Generate tool name with a naming policy. Falls back to the default rule when no policy applies.
pub fn generate_tool_name_with_policy(
    server_name: &str,
    tool_name: &str,
    policy: &ToolNamingPolicy,
) -> String {
    // If strict uniform prefix exists, preserve the vendor prefix and skip server prefix
    if should_skip_server_prefix(policy, tool_name) {
        return tool_name.to_string();
    }

    // Else, optionally strip a majority-agreed compound subprefix (e.g., "21st_magic_")
    let mut value = tool_name;
    if policy.uniform_prefix.is_none() {
        if let Some(sub) = &policy.strip_subprefix {
            let lower = tool_name.to_ascii_lowercase();
            if lower.starts_with(sub) && tool_name.len() > sub.len() {
                value = &tool_name[sub.len()..];
            }
        }
    }

    generate_unique_name(NamingKind::Tool, server_name, value)
}

/// Infer the full naming policy (uniform or majority compound subprefix) from a set of tool names.
///
/// - First tries strict `uniform_prefix` inference.
/// - If not uniform, computes a majority compound subprefix like "token1_token2_" that appears
///   in >=60% of tools (and at least 2 tools), then sets `strip_subprefix` to that candidate,
///   provided it doesn't match the normalized server prefix.
pub fn infer_tool_naming_policy<'a, I>(
    server_name: &str,
    tool_names: I,
) -> ToolNamingPolicy
where
    I: IntoIterator<Item = &'a str>,
{
    use std::collections::HashMap;

    let collected: Vec<String> = tool_names
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // Try strict uniform rule first
    let uniform = infer_uniform_tool_prefix(server_name, collected.iter().map(|s| s.as_str()));
    if uniform.uniform_prefix.is_some() {
        return uniform;
    }

    let n = collected.len();
    if n < 2 {
        return ToolNamingPolicy::default();
    }

    let normalized_server_prefix = format!("{}_", normalize_server_name(server_name));
    // Count compound prefixes: token1_token2_
    let mut counts: HashMap<String, usize> = HashMap::new();
    for name in collected.iter() {
        let lname = name.to_ascii_lowercase();
        let parts: Vec<&str> = lname.split('_').collect();
        if parts.len() >= 2 && !parts[0].is_empty() && !parts[1].is_empty() {
            let cand = format!("{}_{}_", parts[0], parts[1]);
            *counts.entry(cand).or_default() += 1;
        }
    }

    if counts.is_empty() {
        return ToolNamingPolicy::default();
    }

    let threshold = ((n as f64) * 0.6).ceil() as usize;
    let threshold = threshold.max(2);

    let mut best: Option<(String, usize)> = None;
    for (cand, cnt) in counts.into_iter() {
        if cnt >= threshold && cand != normalized_server_prefix {
            match &mut best {
                Some((b, c)) => {
                    if cnt > *c || (cnt == *c && cand.len() > b.len()) {
                        *b = cand;
                        *c = cnt;
                    }
                }
                None => best = Some((cand, cnt)),
            }
        }
    }

    if let Some((cand, _)) = best {
        ToolNamingPolicy {
            uniform_prefix: None,
            strip_subprefix: Some(cand),
        }
    } else {
        ToolNamingPolicy::default()
    }
}

/// Resolve a unique identifier back to its `(server_name, original_value)` pair.
pub async fn resolve_unique_name(
    kind: NamingKind,
    unique: &str,
) -> Result<(String, String)> {
    let query = format!(
        "SELECT server_name, {} FROM {} WHERE {} = ?",
        kind.value_column(),
        kind.table(),
        kind.unique_column()
    );

    let row = sqlx::query_as::<_, (String, String)>(&query)
        .bind(unique)
        .fetch_optional(pool())
        .await
        .context(format!("Failed to resolve unique {:?}: {}", kind, unique))?;

    let (server_name, mut value) = row.ok_or_else(|| anyhow::anyhow!("Unique {:?} '{}' not found", kind, unique))?;

    if let Some(stripped) = strip_server_prefix(kind, &server_name, &value) {
        value = stripped;
    }

    Ok((server_name, value))
}

/// Ensure a unique identifier is collision-free. Non-tool kinds return generated names directly.
pub async fn ensure_unique_name(
    kind: NamingKind,
    server_id: &str,
    server_name: &str,
    value: &str,
) -> Result<String> {
    match kind {
        NamingKind::Tool => ensure_unique_tool_name(server_id, server_name, value).await,
        NamingKind::Prompt | NamingKind::Resource | NamingKind::ResourceTemplate => {
            Ok(generate_unique_name(kind, server_name, value))
        }
    }
}

async fn ensure_unique_tool_name(
    server_id: &str,
    server_name: &str,
    tool_name: &str,
) -> Result<String> {
    let base_name = generate_unique_name(NamingKind::Tool, server_name, tool_name);
    if !unique_tool_name_conflict(&base_name, server_id, tool_name).await? {
        return Ok(base_name);
    }
    resolve_unique_with_suffix(&base_name, server_id, tool_name).await
}

/// Ensure a unique tool name with a naming policy. This preserves existing behavior
/// when `policy` is `None`, and when `policy` applies it will:
/// - Use the original tool name if it starts with the uniform prefix.
/// - Otherwise prefix with the normalized server name.
/// - On conflict, if we skipped the server prefix due to policy, first try the
///   server-prefixed fallback before appending counters.
pub async fn ensure_unique_tool_name_with_policy(
    server_id: &str,
    server_name: &str,
    tool_name: &str,
    policy: Option<&ToolNamingPolicy>,
) -> Result<String> {
    // Optionally strip compound subprefix first (when not uniform)
    let mut base_value = tool_name.to_string();
    if let Some(p) = policy {
        if p.uniform_prefix.is_none() {
            if let Some(sub) = &p.strip_subprefix {
                let lower = tool_name.to_ascii_lowercase();
                if lower.starts_with(sub) && tool_name.len() > sub.len() {
                    base_value = tool_name[sub.len()..].to_string();
                }
            }
        }
    }

    // Decide base unique name
    let (base_name, skipped_server_prefix) = match policy {
        Some(p) if should_skip_server_prefix(p, &base_value) => (base_value.clone(), true),
        _ => (generate_unique_name(NamingKind::Tool, server_name, &base_value), false),
    };

    if !unique_tool_name_conflict(&base_name, server_id, tool_name).await? {
        return Ok(base_name);
    }

    // Try server-prefixed fallback if we skipped server prefix due to policy
    if skipped_server_prefix {
        let fallback = generate_unique_name(NamingKind::Tool, server_name, tool_name);
        if !unique_tool_name_conflict(&fallback, server_id, tool_name).await? {
            return Ok(fallback);
        }
        return resolve_unique_with_suffix(&fallback, server_id, tool_name).await;
    }

    resolve_unique_with_suffix(&base_name, server_id, tool_name).await
}

/// Check if a tool unique name conflicts with an existing row from another (server_id, tool_name).
async fn unique_tool_name_conflict(
    unique_name: &str,
    server_id: &str,
    tool_name: &str,
) -> Result<bool> {
    let pool = pool();
    let exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM server_tools
            WHERE unique_name = ?
              AND (server_id != ? OR tool_name != ?)
        )
        "#,
    )
    .bind(unique_name)
    .bind(server_id)
    .bind(tool_name)
    .fetch_one(pool)
    .await
    .context(format!("Failed to check tool name conflicts for '{}'", unique_name))?;
    Ok(exists)
}

/// Try appending numeric suffixes to resolve a conflict.
async fn resolve_unique_with_suffix(
    base: &str,
    server_id: &str,
    tool_name: &str,
) -> Result<String> {
    let mut counter = 1;
    loop {
        let candidate = format!("{base}_{counter}");
        if !unique_tool_name_conflict(&candidate, server_id, tool_name).await? {
            tracing::debug!("Resolved tool name collision for '{}' using '{}'", base, candidate);
            return Ok(candidate);
        }
        counter += 1;
        if counter > 1000 {
            return Err(anyhow::anyhow!(
                "Failed to generate a unique tool name after 1000 attempts"
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ToolNamingPolicy, infer_tool_naming_policy, infer_uniform_tool_prefix};

    #[test]
    fn infer_prefix_all_match() {
        let p = infer_uniform_tool_prefix(
            "Playwright",
            ["browser_click", "browser_fill", "browser_open"].iter().copied(),
        );
        assert_eq!(
            p,
            ToolNamingPolicy {
                uniform_prefix: Some("browser_".to_string()),
                strip_subprefix: None
            }
        );
    }

    #[test]
    fn infer_prefix_single_tool_returns_none() {
        let p = infer_uniform_tool_prefix("X", ["browser_open"].iter().copied());
        assert_eq!(
            p,
            ToolNamingPolicy {
                uniform_prefix: None,
                strip_subprefix: None
            }
        );
    }

    #[test]
    fn infer_prefix_mismatch_returns_none() {
        let p = infer_uniform_tool_prefix("GitMCP", ["git_clone", "git_commit", "browser_open"].iter().copied());
        assert_eq!(
            p,
            ToolNamingPolicy {
                uniform_prefix: None,
                strip_subprefix: None
            }
        );
    }

    #[test]
    fn infer_majority_compound_subprefix() {
        let names = [
            "21st_magic_component_builder",
            "21st_magic_component_inspiration",
            "21st_magic_component_refiner",
            "logo_search",
        ];
        let p = infer_tool_naming_policy("21magic", names.iter().copied());
        assert_eq!(p.uniform_prefix, None);
        assert_eq!(p.strip_subprefix, Some("21st_magic_".to_string()));
    }
}
