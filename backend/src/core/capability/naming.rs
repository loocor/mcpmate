//! Unified naming utilities for capabilities
//! Provides generation, resolution, and uniqueness guarantees for tool/prompt/resource identifiers.

use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use sqlx::{Pool, Sqlite, Transaction};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::RwLock;
use tracing;

use crate::clients::models::UnifyDirectExposureIntent;

static NAMING_POOL: Lazy<RwLock<Option<Pool<Sqlite>>>> = Lazy::new(|| RwLock::new(None));

/// Begin a naming transaction with the SQLite write lock acquired up front.
///
/// Naming reconciliation reads the complete inventory before updating external
/// identifiers. A deferred transaction can deadlock when concurrent refreshes
/// both finish reading and then try to upgrade to writers. `BEGIN IMMEDIATE`
/// serializes that transition without retrying or weakening error handling.
pub(crate) async fn begin_naming_transaction(pool: &Pool<Sqlite>) -> Result<Transaction<'static, Sqlite>> {
    pool.begin_with("BEGIN IMMEDIATE")
        .await
        .context("Failed to begin capability naming transaction")
}

/// Initialize the global naming store with a database pool.
/// Safe to call multiple times; later calls replace the active pool.
pub fn initialize(pool: Pool<Sqlite>) {
    let mut guard = NAMING_POOL
        .write()
        .expect("Naming store lock poisoned while initializing");
    let replaced = guard.replace(pool).is_some();
    if replaced {
        tracing::debug!("Naming store reinitialized");
    } else {
        tracing::debug!("Naming store initialized");
    }
}

fn pool() -> Pool<Sqlite> {
    NAMING_POOL
        .read()
        .expect("Naming store lock poisoned while reading")
        .clone()
        .expect("Naming store not initialized; call naming::initialize first")
}

fn canonical_namespace(server_name: &str) -> Result<String> {
    crate::config::server::validate_server_namespace(server_name)?;
    Ok(server_name.to_string())
}

fn remove_namespace_tokens(
    server_name: &str,
    value: &str,
) -> Result<String> {
    let namespace = canonical_namespace(server_name)?;
    let namespace_tokens = namespace.split('_').collect::<Vec<_>>();
    let mut token_start = 0;
    let mut value_tokens = value
        .match_indices('_')
        .map(|(separator, _)| {
            let token = (token_start, separator, &value[token_start..separator]);
            token_start = separator + 1;
            token
        })
        .collect::<Vec<_>>();
    value_tokens.push((token_start, value.len(), &value[token_start..]));

    if namespace_tokens.is_empty() || value_tokens.len() < namespace_tokens.len() {
        return Ok(value.to_string());
    }

    let match_start = value_tokens.windows(namespace_tokens.len()).position(|window| {
        window
            .iter()
            .zip(namespace_tokens.iter())
            .all(|((_, _, candidate), namespace)| candidate.eq_ignore_ascii_case(namespace))
    });
    let Some(match_start) = match_start else {
        return Ok(value.to_string());
    };

    let mut remove_start = value_tokens[match_start].0;
    let mut remove_end = value_tokens[match_start + namespace_tokens.len() - 1].1;
    if value.as_bytes().get(remove_end) == Some(&b'_') {
        remove_end += 1;
    } else if remove_start > 0 && value.as_bytes().get(remove_start - 1) == Some(&b'_') {
        remove_start -= 1;
    }

    Ok(format!("{}{}", &value[..remove_start], &value[remove_end..]))
}

fn remove_boundary_token_overlap(
    namespace: &str,
    value: &str,
) -> String {
    let namespace_tokens = namespace.split('_').collect::<Vec<_>>();
    let value_tokens = value.split('_').collect::<Vec<_>>();
    let max_overlap = namespace_tokens.len().min(value_tokens.len());
    let overlap = (1..=max_overlap).rev().find(|overlap| {
        namespace_tokens[namespace_tokens.len() - overlap..]
            .iter()
            .zip(value_tokens[..*overlap].iter())
            .all(|(namespace, candidate)| namespace.eq_ignore_ascii_case(candidate))
    });
    let Some(overlap) = overlap else {
        return value.to_string();
    };

    if overlap == value_tokens.len() {
        return String::new();
    }

    let matched_bytes = value_tokens
        .iter()
        .take(overlap)
        .map(|token| token.len())
        .sum::<usize>();
    value[matched_bytes + overlap..].to_string()
}

/// Capability kinds supported by the internal naming pipeline.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NamingKind {
    Tool,
    Prompt,
    Resource,
    ResourceTemplate,
}

impl NamingKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Tool => "tool",
            Self::Prompt => "prompt",
            Self::Resource => "resource",
            Self::ResourceTemplate => "resource_template",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error(
    "Cannot assign external {kind:?} identifier '{external_identifier}' for server '{server_id}' capability '{upstream_value}': already used by server '{conflicting_server_id}' capability '{conflicting_upstream_value}'"
)]
pub(crate) struct ExternalIdentifierCollision {
    pub(crate) kind: NamingKind,
    pub(crate) external_identifier: String,
    pub(crate) server_id: String,
    pub(crate) upstream_value: String,
    pub(crate) conflicting_server_id: String,
    pub(crate) conflicting_upstream_value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CapabilityRoute {
    pub(crate) server_id: String,
    pub(crate) server_name: String,
    pub(crate) upstream_value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ExternalIdentifierChange {
    pub(crate) kind: NamingKind,
    pub(crate) server_id: String,
    pub(crate) upstream_value: String,
    pub(crate) old_external: String,
    pub(crate) new_external: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ExternalIdentifierRemoval {
    pub(crate) kind: NamingKind,
    pub(crate) server_id: String,
    pub(crate) upstream_value: String,
    pub(crate) old_external: String,
}

#[derive(Clone, Debug)]
struct PersistedCapabilityIdentity {
    id: String,
    server_id: String,
    upstream_value: String,
    old_external: String,
}

#[derive(Clone, Debug)]
pub(crate) struct ExternalIdentifierReconciliation {
    identifiers: BTreeMap<String, String>,
    additions: Vec<String>,
    pub(crate) changes: Vec<ExternalIdentifierChange>,
    pub(crate) removals: Vec<ExternalIdentifierRemoval>,
}

impl ExternalIdentifierReconciliation {
    pub(crate) fn identifier_for(
        &self,
        upstream_value: &str,
    ) -> Result<&str> {
        self.identifiers.get(upstream_value).map(String::as_str).ok_or_else(|| {
            anyhow::anyhow!(
                "Upstream capability '{}' is missing from the naming plan",
                upstream_value
            )
        })
    }

    pub(crate) fn catalog_changed(&self) -> bool {
        !self.additions.is_empty() || !self.changes.is_empty() || !self.removals.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CapabilityIdentity {
    pub kind: NamingKind,
    pub server_id: String,
    pub server_name: String,
    pub upstream_value: String,
    pub external_value: String,
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
fn generate_unique_name(
    kind: NamingKind,
    server_name: &str,
    value: &str,
) -> Result<String> {
    match kind {
        NamingKind::Tool | NamingKind::Prompt | NamingKind::ResourceTemplate => {
            let normalized = canonical_namespace(server_name)?;
            let value = remove_namespace_tokens(server_name, value)?;
            let value = remove_boundary_token_overlap(&normalized, &value);
            if value.is_empty() {
                Ok(normalized)
            } else {
                Ok(format!("{normalized}_{value}"))
            }
        }
        NamingKind::Resource => {
            let normalized = canonical_namespace(server_name)?;
            let prefix = format!("{normalized}:");
            if value.to_lowercase().starts_with(&prefix) {
                Ok(value.to_string())
            } else {
                Ok(format!("{normalized}:{value}"))
            }
        }
    }
}

fn generate_complete_name(
    kind: NamingKind,
    server_name: &str,
    value: &str,
) -> Result<String> {
    let namespace = canonical_namespace(server_name)?;
    match kind {
        NamingKind::Tool | NamingKind::Prompt | NamingKind::ResourceTemplate => Ok(format!("{namespace}_{value}")),
        NamingKind::Resource => {
            let prefix = format!("{namespace}:");
            if value.starts_with(&prefix) {
                Ok(value.to_string())
            } else {
                Ok(format!("{prefix}{value}"))
            }
        }
    }
}

/// Plan deterministic external identifiers from one complete upstream inventory.
///
/// Semantic shortening is used only when it remains unique within the server
/// and capability kind. A collision expands the complete group back to exact
/// upstream values instead of manufacturing opaque suffixes.
pub(crate) fn plan_external_identifiers(
    kind: NamingKind,
    server_name: &str,
    upstream_values: &[String],
) -> Result<BTreeMap<String, String>> {
    canonical_namespace(server_name)?;

    let mut exact_values = HashSet::new();
    let mut preferred_groups = BTreeMap::<String, Vec<&String>>::new();
    for upstream_value in upstream_values {
        if !exact_values.insert(upstream_value.as_str()) {
            return Err(anyhow::anyhow!(
                "Cannot plan {:?} identifiers for server '{}': duplicate upstream value '{}'",
                kind,
                server_name,
                upstream_value
            ));
        }
        let preferred = generate_unique_name(kind, server_name, upstream_value)?;
        preferred_groups.entry(preferred).or_default().push(upstream_value);
    }

    let mut plan = BTreeMap::new();
    let mut assigned = BTreeMap::<String, String>::new();
    for (preferred, group) in preferred_groups {
        let use_complete_names = group.len() > 1;
        for upstream_value in group {
            let external = if use_complete_names {
                generate_complete_name(kind, server_name, upstream_value)?
            } else {
                preferred.clone()
            };
            if let Some(existing_upstream) = assigned.insert(external.clone(), upstream_value.clone()) {
                return Err(anyhow::anyhow!(
                    "Cannot plan {:?} identifiers for server '{}': upstream values '{}' and '{}' both map to '{}'",
                    kind,
                    server_name,
                    existing_upstream,
                    upstream_value,
                    external
                ));
            }
            plan.insert(upstream_value.clone(), external);
        }
    }

    Ok(plan)
}

/// Derive the existing resource-authorization prefix for template output.
///
/// This is not a client routing entrypoint: concrete external resources still
/// resolve through the persisted catalog.
pub(in crate::core) fn external_resource_prefix(
    server_name: &str,
    upstream_prefix: &str,
) -> Result<String> {
    generate_unique_name(NamingKind::Resource, server_name, upstream_prefix)
}

/// Resolve an external capability identifier to its exact upstream route.
pub(crate) async fn resolve_capability_route(
    kind: NamingKind,
    unique: &str,
) -> Result<CapabilityRoute> {
    resolve_capability_route_with_pool(&pool(), kind, unique).await
}

/// Resolve an external identifier through an explicitly supplied catalog.
pub(crate) async fn resolve_capability_route_with_pool(
    catalog: &Pool<Sqlite>,
    kind: NamingKind,
    unique: &str,
) -> Result<CapabilityRoute> {
    let identity = resolve_capability_identity_with_pool(catalog, kind, unique).await?;
    Ok(CapabilityRoute {
        server_id: identity.server_id,
        server_name: identity.server_name,
        upstream_value: identity.upstream_value,
    })
}

/// Resolve a persisted external identifier to its complete capability identity.
async fn resolve_capability_identity_with_pool(
    catalog: &Pool<Sqlite>,
    kind: NamingKind,
    unique: &str,
) -> Result<CapabilityIdentity> {
    let query = format!(
        "SELECT server_id, server_name, {} FROM {} WHERE {} = ?",
        kind.value_column(),
        kind.table(),
        kind.unique_column()
    );

    let row = sqlx::query_as::<_, (String, String, String)>(&query)
        .bind(unique)
        .fetch_optional(catalog)
        .await
        .with_context(|| format!("Failed to resolve external {:?}: {}", kind, unique))?;

    let Some((server_id, server_name, upstream_value)) = row else {
        return Err(anyhow::anyhow!("External {:?} '{}' not found", kind, unique));
    };
    Ok(CapabilityIdentity {
        kind,
        server_id,
        server_name,
        upstream_value,
        external_value: unique.to_string(),
    })
}

/// Load a persisted external identifier from the supplied catalog pool.
pub(crate) async fn load_external_identifier(
    catalog: &Pool<Sqlite>,
    kind: NamingKind,
    server_id: &str,
    upstream_value: &str,
) -> Result<String> {
    let query = format!(
        "SELECT {} FROM {} WHERE server_id = ? AND {} = ?",
        kind.unique_column(),
        kind.table(),
        kind.value_column()
    );
    sqlx::query_scalar::<_, String>(&query)
        .bind(server_id)
        .bind(upstream_value)
        .fetch_optional(catalog)
        .await
        .with_context(|| {
            format!(
                "Failed to load external {:?} identifier for server '{}'",
                kind, server_id
            )
        })?
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Exact upstream {:?} capability '{}' is not registered for server '{}'",
                kind,
                upstream_value,
                server_id
            )
        })
}

/// Load a persisted external identifier from the active catalog.
pub(in crate::core) async fn external_identifier(
    kind: NamingKind,
    server_id: &str,
    upstream_value: &str,
) -> Result<String> {
    load_external_identifier(&pool(), kind, server_id, upstream_value).await
}

/// Reconcile one server's complete capability inventory inside the caller's
/// transaction. Existing rows, new-row insertion, and persisted references
/// therefore either commit together or remain unchanged.
pub(crate) async fn reconcile_external_identifiers(
    tx: &mut Transaction<'_, Sqlite>,
    kind: NamingKind,
    server_id: &str,
    server_name: &str,
    upstream_values: &[String],
) -> Result<ExternalIdentifierReconciliation> {
    reconcile_external_identifiers_internal(tx, kind, server_id, server_name, upstream_values, false).await
}

/// Extend the persisted inventory with explicitly supplied entries.
///
/// This is reserved for narrow single-row setup and profile materialization
/// paths. Authoritative upstream refreshes must use
/// `reconcile_external_identifiers` so missing capabilities leave the catalog.
pub(crate) async fn reconcile_external_identifier_additions(
    tx: &mut Transaction<'_, Sqlite>,
    kind: NamingKind,
    server_id: &str,
    server_name: &str,
    upstream_values: &[String],
) -> Result<ExternalIdentifierReconciliation> {
    reconcile_external_identifiers_internal(tx, kind, server_id, server_name, upstream_values, true).await
}

async fn reconcile_external_identifiers_internal(
    tx: &mut Transaction<'_, Sqlite>,
    kind: NamingKind,
    server_id: &str,
    server_name: &str,
    upstream_values: &[String],
    retain_persisted: bool,
) -> Result<ExternalIdentifierReconciliation> {
    canonical_namespace(server_name)?;
    let mut supplied = HashSet::new();
    for upstream_value in upstream_values {
        if !supplied.insert(upstream_value.as_str()) {
            return Err(anyhow::anyhow!(
                "Cannot reconcile {:?} identifiers for server '{}': duplicate upstream value '{}'",
                kind,
                server_id,
                upstream_value
            ));
        }
    }

    let query = format!(
        "SELECT id, server_id, {}, {} FROM {} WHERE server_id = ? ORDER BY id",
        kind.value_column(),
        kind.unique_column(),
        kind.table()
    );
    let rows = sqlx::query_as::<_, (String, String, String, String)>(&query)
        .bind(server_id)
        .fetch_all(&mut **tx)
        .await
        .with_context(|| {
            format!(
                "Failed to load persisted {:?} identifiers for server '{}'",
                kind, server_id
            )
        })?
        .into_iter()
        .map(
            |(id, server_id, upstream_value, old_external)| PersistedCapabilityIdentity {
                id,
                server_id,
                upstream_value,
                old_external,
            },
        )
        .collect::<Vec<_>>();

    let complete_inventory = if retain_persisted {
        let mut inventory = rows
            .iter()
            .map(|row| row.upstream_value.clone())
            .collect::<BTreeSet<_>>();
        inventory.extend(upstream_values.iter().cloned());
        inventory.into_iter().collect::<Vec<_>>()
    } else {
        upstream_values.to_vec()
    };
    let identifiers = plan_external_identifiers(kind, server_name, &complete_inventory)?;
    let persisted_values = rows
        .iter()
        .map(|row| row.upstream_value.as_str())
        .collect::<HashSet<_>>();
    let additions = identifiers
        .keys()
        .filter(|upstream_value| !persisted_values.contains(upstream_value.as_str()))
        .cloned()
        .collect::<Vec<_>>();

    let other_query = format!(
        "SELECT server_id, {}, {} FROM {} WHERE server_id != ?",
        kind.value_column(),
        kind.unique_column(),
        kind.table()
    );
    let other_identifiers = sqlx::query_as::<_, (String, String, String)>(&other_query)
        .bind(server_id)
        .fetch_all(&mut **tx)
        .await
        .with_context(|| {
            format!(
                "Failed to check external {:?} identifiers outside server '{}'",
                kind, server_id
            )
        })?
        .into_iter()
        .map(|(other_server_id, upstream_value, external)| (external, (other_server_id, upstream_value)))
        .collect::<HashMap<_, _>>();
    for (upstream_value, external) in &identifiers {
        if let Some((other_server_id, other_upstream)) = other_identifiers.get(external) {
            return Err(ExternalIdentifierCollision {
                kind,
                external_identifier: external.clone(),
                server_id: server_id.to_string(),
                upstream_value: upstream_value.clone(),
                conflicting_server_id: other_server_id.clone(),
                conflicting_upstream_value: other_upstream.clone(),
            }
            .into());
        }
    }

    let changed_rows = rows
        .iter()
        .filter_map(|row| {
            identifiers
                .get(&row.upstream_value)
                .filter(|desired| row.old_external.as_str() != desired.as_str())
                .map(|desired| (row, desired))
        })
        .collect::<Vec<_>>();
    let removed_rows = rows
        .iter()
        .filter(|row| !identifiers.contains_key(&row.upstream_value))
        .collect::<Vec<_>>();

    for (row, _) in &changed_rows {
        let temporary = format!("\u{1f}mcpmate-naming:{}:{}", kind.table(), row.id);
        let update = format!("UPDATE {} SET {} = ? WHERE id = ?", kind.table(), kind.unique_column());
        sqlx::query(&update)
            .bind(temporary)
            .bind(&row.id)
            .execute(&mut **tx)
            .await
            .with_context(|| format!("Failed to stage {:?} identifier rebuild for row '{}'", kind, row.id))?;
    }

    let mut removals = Vec::with_capacity(removed_rows.len());
    for row in removed_rows {
        let delete = format!("DELETE FROM {} WHERE id = ?", kind.table());
        sqlx::query(&delete)
            .bind(&row.id)
            .execute(&mut **tx)
            .await
            .with_context(|| format!("Failed to remove stale {:?} row '{}'", kind, row.id))?;
        removals.push(ExternalIdentifierRemoval {
            kind,
            server_id: row.server_id.clone(),
            upstream_value: row.upstream_value.clone(),
            old_external: row.old_external.clone(),
        });
    }

    let mut changes = Vec::with_capacity(changed_rows.len());
    for (row, desired) in changed_rows {
        let update = format!(
            "UPDATE {} SET {} = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            kind.table(),
            kind.unique_column()
        );
        sqlx::query(&update)
            .bind(desired)
            .bind(&row.id)
            .execute(&mut **tx)
            .await
            .with_context(|| format!("Failed to finalize {:?} identifier rebuild for row '{}'", kind, row.id))?;
        changes.push(ExternalIdentifierChange {
            kind,
            server_id: row.server_id.clone(),
            upstream_value: row.upstream_value.clone(),
            old_external: row.old_external.clone(),
            new_external: desired.clone(),
        });
    }

    rewrite_client_direct_exposure_intents(tx, &changes, &removals).await?;
    remove_profile_references_for_deleted_capabilities(tx, &removals).await?;
    Ok(ExternalIdentifierReconciliation {
        identifiers,
        additions,
        changes,
        removals,
    })
}

async fn remove_profile_references_for_deleted_capabilities(
    tx: &mut Transaction<'_, Sqlite>,
    removals: &[ExternalIdentifierRemoval],
) -> Result<()> {
    for removal in removals {
        let profile_reference = match removal.kind {
            NamingKind::Tool => None,
            NamingKind::Prompt => Some(("profile_prompt", "prompt_name")),
            NamingKind::Resource => Some(("profile_resource", "resource_uri")),
            NamingKind::ResourceTemplate => Some(("profile_resource_template", "uri_template")),
        };
        let Some((table, value_column)) = profile_reference else {
            continue;
        };

        let delete = format!("DELETE FROM {table} WHERE server_id = ? AND {value_column} = ?");
        sqlx::query(&delete)
            .bind(&removal.server_id)
            .bind(&removal.upstream_value)
            .execute(&mut **tx)
            .await
            .with_context(|| {
                format!(
                    "Failed to remove stale profile reference for {:?} '{}' from server '{}'",
                    removal.kind, removal.upstream_value, removal.server_id
                )
            })?;
    }

    Ok(())
}

/// Rebuild one server through the naming pipeline without touching other
/// servers. Used by namespace repair after the target's denormalized namespace
/// fields have been updated in the same transaction.
pub(crate) async fn rebuild_server_external_identifiers(
    tx: &mut Transaction<'_, Sqlite>,
    server_id: &str,
    server_name: &str,
) -> Result<Vec<ExternalIdentifierChange>> {
    let mut changes = Vec::new();
    for kind in [
        NamingKind::Tool,
        NamingKind::Prompt,
        NamingKind::Resource,
        NamingKind::ResourceTemplate,
    ] {
        changes.extend(
            reconcile_external_identifier_additions(tx, kind, server_id, server_name, &[])
                .await?
                .changes,
        );
    }
    Ok(changes)
}

async fn rewrite_client_direct_exposure_intents(
    tx: &mut Transaction<'_, Sqlite>,
    changes: &[ExternalIdentifierChange],
    removals: &[ExternalIdentifierRemoval],
) -> Result<()> {
    if changes.is_empty() && removals.is_empty() {
        return Ok(());
    }

    let mut tool_ids = HashMap::new();
    let mut prompt_ids = HashMap::new();
    let mut resource_ids = HashMap::new();
    let mut template_ids = HashMap::new();
    for change in changes {
        let mapping = match change.kind {
            NamingKind::Tool => &mut tool_ids,
            NamingKind::Prompt => &mut prompt_ids,
            NamingKind::Resource => &mut resource_ids,
            NamingKind::ResourceTemplate => &mut template_ids,
        };
        mapping.insert(change.old_external.clone(), change.new_external.clone());
    }
    let mut removed_tool_ids = HashSet::new();
    let mut removed_prompt_ids = HashSet::new();
    let mut removed_resource_ids = HashSet::new();
    let mut removed_template_ids = HashSet::new();
    for removal in removals {
        let removed = match removal.kind {
            NamingKind::Tool => &mut removed_tool_ids,
            NamingKind::Prompt => &mut removed_prompt_ids,
            NamingKind::Resource => &mut removed_resource_ids,
            NamingKind::ResourceTemplate => &mut removed_template_ids,
        };
        removed.insert(removal.old_external.clone());
    }

    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT id, unify_direct_exposure_intent FROM client WHERE unify_direct_exposure_intent IS NOT NULL",
    )
    .fetch_all(&mut **tx)
    .await
    .context("Failed to load Client Direct Exposure intents for capability reconciliation")?;

    for (client_id, raw_intent) in rows {
        let mut intent: UnifyDirectExposureIntent = serde_json::from_str(&raw_intent)
            .with_context(|| format!("Invalid Client Direct Exposure intent for client '{client_id}'"))?;
        let mut changed = remove_ids(&mut intent.capability_ids.tool_ids, &removed_tool_ids)
            | remove_ids(&mut intent.capability_ids.prompt_ids, &removed_prompt_ids)
            | remove_ids(&mut intent.capability_ids.resource_ids, &removed_resource_ids)
            | remove_ids(&mut intent.capability_ids.template_ids, &removed_template_ids);
        changed |= rewrite_ids(&mut intent.capability_ids.tool_ids, &tool_ids)
            | rewrite_ids(&mut intent.capability_ids.prompt_ids, &prompt_ids)
            | rewrite_ids(&mut intent.capability_ids.resource_ids, &resource_ids)
            | rewrite_ids(&mut intent.capability_ids.template_ids, &template_ids);
        if !changed {
            continue;
        }
        let serialized = serde_json::to_string(&intent)
            .with_context(|| format!("Failed to serialize Client Direct Exposure intent for client '{client_id}'"))?;
        sqlx::query("UPDATE client SET unify_direct_exposure_intent = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(serialized)
            .bind(&client_id)
            .execute(&mut **tx)
            .await
            .with_context(|| format!("Failed to rewrite Client Direct Exposure intent for client '{client_id}'"))?;
    }

    Ok(())
}

fn remove_ids(
    values: &mut Vec<String>,
    removed: &HashSet<String>,
) -> bool {
    let previous_len = values.len();
    values.retain(|value| !removed.contains(value));
    values.len() != previous_len
}

fn rewrite_ids(
    values: &mut [String],
    mapping: &HashMap<String, String>,
) -> bool {
    let mut changed = false;
    for value in values {
        if let Some(replacement) = mapping.get(value) {
            value.clone_from(replacement);
            changed = true;
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use sqlx::{Pool, Sqlite, sqlite::SqlitePoolOptions};

    use super::{
        ExternalIdentifierCollision, NamingKind, generate_unique_name, plan_external_identifiers,
        reconcile_external_identifier_additions, reconcile_external_identifiers, resolve_capability_identity_with_pool,
    };

    async fn test_pool() -> Pool<Sqlite> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("connect in-memory catalog");
        for statement in [
            "CREATE TABLE server_tools (id TEXT PRIMARY KEY, server_id TEXT NOT NULL, server_name TEXT NOT NULL, tool_name TEXT NOT NULL, unique_name TEXT NOT NULL UNIQUE, updated_at TEXT)",
            "CREATE TABLE server_prompts (id TEXT PRIMARY KEY, server_id TEXT NOT NULL, server_name TEXT NOT NULL, prompt_name TEXT NOT NULL, unique_name TEXT NOT NULL UNIQUE, updated_at TEXT)",
            "CREATE TABLE server_resources (id TEXT PRIMARY KEY, server_id TEXT NOT NULL, server_name TEXT NOT NULL, resource_uri TEXT NOT NULL, unique_uri TEXT NOT NULL UNIQUE, updated_at TEXT)",
            "CREATE TABLE server_resource_templates (id TEXT PRIMARY KEY, server_id TEXT NOT NULL, server_name TEXT NOT NULL, uri_template TEXT NOT NULL, unique_name TEXT NOT NULL UNIQUE, updated_at TEXT)",
            "CREATE TABLE profile_prompt (id TEXT PRIMARY KEY, profile_id TEXT NOT NULL, server_id TEXT NOT NULL, server_name TEXT NOT NULL, prompt_name TEXT NOT NULL, enabled BOOLEAN NOT NULL)",
            "CREATE TABLE profile_resource (id TEXT PRIMARY KEY, profile_id TEXT NOT NULL, server_id TEXT NOT NULL, server_name TEXT NOT NULL, resource_uri TEXT NOT NULL, enabled BOOLEAN NOT NULL)",
            "CREATE TABLE profile_resource_template (id TEXT PRIMARY KEY, profile_id TEXT NOT NULL, server_id TEXT NOT NULL, server_name TEXT NOT NULL, uri_template TEXT NOT NULL, enabled BOOLEAN NOT NULL)",
            "CREATE TABLE client (id TEXT PRIMARY KEY, unify_direct_exposure_intent TEXT, updated_at TEXT)",
        ] {
            sqlx::query(statement)
                .execute(&pool)
                .await
                .expect("create naming table");
        }
        pool
    }

    async fn insert_tool(
        pool: &Pool<Sqlite>,
        row_id: &str,
        server_id: &str,
        server_name: &str,
        upstream_name: &str,
    ) {
        let mut tx = pool.begin().await.expect("begin tool insert");
        let inventory = [upstream_name.to_string()];
        let reconciliation =
            reconcile_external_identifier_additions(&mut tx, NamingKind::Tool, server_id, server_name, &inventory)
                .await
                .expect("reconcile external tool identifier");
        let external = reconciliation
            .identifier_for(upstream_name)
            .expect("planned tool identifier");
        sqlx::query(
            "INSERT INTO server_tools (id, server_id, server_name, tool_name, unique_name) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(row_id)
        .bind(server_id)
        .bind(server_name)
        .bind(upstream_name)
        .bind(external)
        .execute(&mut *tx)
        .await
        .expect("insert tool mapping");
        tx.commit().await.expect("commit tool insert");
    }

    async fn tool_mapping(pool: &Pool<Sqlite>) -> BTreeMap<String, String> {
        sqlx::query_as::<_, (String, String)>("SELECT tool_name, unique_name FROM server_tools")
            .fetch_all(pool)
            .await
            .expect("load tool mappings")
            .into_iter()
            .collect()
    }

    fn generated(
        kind: NamingKind,
        namespace: &str,
        upstream: &str,
    ) -> String {
        generate_unique_name(kind, namespace, upstream).expect("generate external identifier")
    }

    #[test]
    fn preserves_single_namespace_prefix() {
        assert_eq!(
            generated(NamingKind::Tool, "searxng", "searxng_web_search"),
            "searxng_web_search"
        );
    }

    #[test]
    fn removes_one_embedded_namespace_token_sequence() {
        assert_eq!(
            generated(NamingKind::Tool, "searxng", "get_searxng_status"),
            "searxng_get_status"
        );
    }

    #[test]
    fn does_not_remove_partial_namespace_tokens() {
        assert_eq!(
            generated(NamingKind::Tool, "search", "research_documents"),
            "search_research_documents"
        );
    }

    #[test]
    fn removes_multi_token_namespace_as_a_unit() {
        assert_eq!(
            generated(
                NamingKind::Prompt,
                "sequential_thinking",
                "get_sequential_thinking_prompt"
            ),
            "sequential_thinking_get_prompt"
        );
    }

    #[test]
    fn removes_exact_token_overlap_at_namespace_boundary() {
        assert_eq!(
            generated(NamingKind::Tool, "amap_maps", "maps_around_search"),
            "amap_maps_around_search"
        );
    }

    #[test]
    fn removes_longest_token_overlap_at_namespace_boundary() {
        assert_eq!(
            generated(NamingKind::Tool, "acme_maps_v2", "maps_v2_search"),
            "acme_maps_v2_search"
        );
    }

    #[test]
    fn namespace_token_removal_preserves_unrelated_repeated_separators() {
        assert_eq!(
            generated(NamingKind::Tool, "docs", "read__docs__file"),
            "docs_read___file"
        );
    }

    #[test]
    fn namespace_token_removal_preserves_upstream_trailing_separator() {
        assert_eq!(generated(NamingKind::Tool, "docs", "read__docs"), "docs_read_");
    }

    #[test]
    fn resource_names_remain_namespaced_uris() {
        assert_eq!(
            generated(NamingKind::Resource, "docs", "file:///guide.md"),
            "docs:file:///guide.md"
        );
    }

    #[test]
    fn resource_templates_use_the_same_namespace_pipeline() {
        assert_eq!(
            generated(NamingKind::ResourceTemplate, "docs", "docs_file:///{path}"),
            "docs_file:///{path}"
        );
    }

    #[test]
    fn batch_plan_preserves_upstream_characters() {
        let plan = plan_external_identifiers(
            NamingKind::Tool,
            "search_v2",
            &["Find-Docs".to_string(), "find_docs".to_string()],
        )
        .expect("plan external identifiers");

        assert_eq!(plan["Find-Docs"], "search_v2_Find-Docs");
        assert_eq!(plan["find_docs"], "search_v2_find_docs");
    }

    #[test]
    fn batch_plan_expands_the_complete_collision_group() {
        let plan = plan_external_identifiers(
            NamingKind::Tool,
            "searxng",
            &["get_searxng_status".to_string(), "get_status".to_string()],
        )
        .expect("plan collision group");

        assert_eq!(plan["get_searxng_status"], "searxng_get_searxng_status");
        assert_eq!(plan["get_status"], "searxng_get_status");
        assert!(plan.values().all(|value| !value.ends_with("_a83f")));
    }

    #[test]
    fn batch_plan_is_independent_of_inventory_order() {
        let forward = plan_external_identifiers(
            NamingKind::Tool,
            "searxng",
            &["get_searxng_status".to_string(), "get_status".to_string()],
        )
        .expect("plan forward inventory");
        let reverse = plan_external_identifiers(
            NamingKind::Tool,
            "searxng",
            &["get_status".to_string(), "get_searxng_status".to_string()],
        )
        .expect("plan reverse inventory");

        assert_eq!(forward, reverse);
    }

    #[test]
    fn batch_plan_rejects_noncanonical_namespaces() {
        let error = plan_external_identifiers(NamingKind::Tool, "Sequential Thinking", &["think".to_string()])
            .expect_err("noncanonical namespaces must not be suggested inside naming");

        assert!(error.to_string().contains("Invalid server namespace"));
    }

    #[test]
    fn batch_plan_rejects_duplicate_exact_upstream_values() {
        let error = plan_external_identifiers(NamingKind::Prompt, "docs", &["help".to_string(), "help".to_string()])
            .expect_err("duplicate upstream values must fail explicitly");

        assert!(error.to_string().contains("duplicate upstream"));
    }

    #[tokio::test]
    async fn collision_group_is_independent_of_insertion_order() {
        let first = test_pool().await;
        insert_tool(&first, "tool-1", "server-a", "searxng", "get_searxng_status").await;
        insert_tool(&first, "tool-2", "server-a", "searxng", "searxng_get_status").await;

        let second = test_pool().await;
        insert_tool(&second, "tool-1", "server-a", "searxng", "searxng_get_status").await;
        insert_tool(&second, "tool-2", "server-a", "searxng", "get_searxng_status").await;

        let first_mapping = tool_mapping(&first).await;
        let second_mapping = tool_mapping(&second).await;
        assert_eq!(first_mapping, second_mapping);
        assert_eq!(first_mapping["get_searxng_status"], "searxng_get_searxng_status");
        assert_eq!(first_mapping["searxng_get_status"], "searxng_searxng_get_status");
    }

    #[tokio::test]
    async fn reconciliation_updates_target_collision_group_and_client_intent() {
        let pool = test_pool().await;
        insert_tool(&pool, "tool-1", "server-a", "searxng", "get_searxng_status").await;
        insert_tool(&pool, "tool-b", "server-b", "other", "status").await;
        sqlx::query(
            r#"INSERT INTO client (id, unify_direct_exposure_intent)
               VALUES ('client-1', '{"route_mode":"capability_level","capability_ids":{"tool_ids":["searxng_get_status","other_status"]}}')"#,
        )
        .execute(&pool)
        .await
        .expect("insert client intent");

        let mut tx = pool.begin().await.expect("begin reconciliation");
        let inventory = ["get_searxng_status".to_string(), "get_status".to_string()];
        let reconciliation =
            reconcile_external_identifiers(&mut tx, NamingKind::Tool, "server-a", "searxng", &inventory)
                .await
                .expect("reconcile collision group");
        assert_eq!(
            reconciliation.identifier_for("get_searxng_status").unwrap(),
            "searxng_get_searxng_status"
        );
        assert_eq!(
            reconciliation.identifier_for("get_status").unwrap(),
            "searxng_get_status"
        );
        tx.commit().await.expect("commit reconciliation");

        let mapping = tool_mapping(&pool).await;
        assert_eq!(mapping["get_searxng_status"], "searxng_get_searxng_status");
        assert_eq!(mapping["status"], "other_status");
        let raw: String = sqlx::query_scalar("SELECT unify_direct_exposure_intent FROM client WHERE id = 'client-1'")
            .fetch_one(&pool)
            .await
            .expect("load rewritten client intent");
        let intent: serde_json::Value = serde_json::from_str(&raw).expect("parse client intent");
        assert_eq!(
            intent["capability_ids"]["tool_ids"],
            serde_json::json!(["searxng_get_searxng_status", "other_status"])
        );
    }

    #[tokio::test]
    async fn reconciliation_rolls_back_identifier_and_client_rewrites() {
        let pool = test_pool().await;
        insert_tool(&pool, "tool-1", "server-a", "searxng", "get_searxng_status").await;
        sqlx::query(
            r#"INSERT INTO client (id, unify_direct_exposure_intent)
               VALUES ('client-1', '{"route_mode":"capability_level","capability_ids":{"tool_ids":["searxng_get_status"]}}')"#,
        )
        .execute(&pool)
        .await
        .expect("insert client intent");

        let mut tx = pool.begin().await.expect("begin reconciliation");
        reconcile_external_identifiers(
            &mut tx,
            NamingKind::Tool,
            "server-a",
            "searxng",
            &["get_searxng_status".to_string(), "get_status".to_string()],
        )
        .await
        .expect("reconcile collision group");
        tx.rollback().await.expect("rollback reconciliation");

        let mapping = tool_mapping(&pool).await;
        assert_eq!(mapping["get_searxng_status"], "searxng_get_status");
        let raw: String = sqlx::query_scalar("SELECT unify_direct_exposure_intent FROM client WHERE id = 'client-1'")
            .fetch_one(&pool)
            .await
            .expect("load original client intent");
        let intent: serde_json::Value = serde_json::from_str(&raw).expect("parse client intent");
        assert_eq!(
            intent["capability_ids"]["tool_ids"],
            serde_json::json!(["searxng_get_status"])
        );
    }

    #[tokio::test]
    async fn authoritative_inventory_removes_missing_rows_and_shrinks_collision_groups() {
        let pool = test_pool().await;
        insert_tool(&pool, "tool-1", "server-a", "searxng", "get_searxng_status").await;
        insert_tool(&pool, "tool-2", "server-a", "searxng", "get_status").await;
        sqlx::query(
            r#"INSERT INTO client (id, unify_direct_exposure_intent)
               VALUES ('client-1', '{"route_mode":"capability_level","capability_ids":{"tool_ids":["searxng_get_searxng_status","searxng_get_status"]}}')"#,
        )
        .execute(&pool)
        .await
        .expect("insert client intent");

        let mut tx = pool.begin().await.expect("begin authoritative reconciliation");
        let reconciliation = reconcile_external_identifiers(
            &mut tx,
            NamingKind::Tool,
            "server-a",
            "searxng",
            &["get_searxng_status".to_string()],
        )
        .await
        .expect("reconcile authoritative inventory");
        assert_eq!(
            reconciliation.identifier_for("get_searxng_status").unwrap(),
            "searxng_get_status"
        );
        tx.commit().await.expect("commit authoritative reconciliation");

        let rows = sqlx::query_as::<_, (String, String, String)>(
            "SELECT id, tool_name, unique_name FROM server_tools WHERE server_id = 'server-a' ORDER BY id",
        )
        .fetch_all(&pool)
        .await
        .expect("load converged inventory");
        assert_eq!(
            rows,
            vec![(
                "tool-1".to_string(),
                "get_searxng_status".to_string(),
                "searxng_get_status".to_string(),
            )]
        );
        let raw: String = sqlx::query_scalar("SELECT unify_direct_exposure_intent FROM client WHERE id = 'client-1'")
            .fetch_one(&pool)
            .await
            .expect("load converged client intent");
        let intent: serde_json::Value = serde_json::from_str(&raw).expect("parse client intent");
        assert_eq!(
            intent["capability_ids"]["tool_ids"],
            serde_json::json!(["searxng_get_status"])
        );
    }

    #[tokio::test]
    async fn authoritative_empty_inventory_removes_all_rows() {
        let pool = test_pool().await;
        insert_tool(&pool, "tool-1", "server-a", "searxng", "web_search").await;

        let mut tx = pool.begin().await.expect("begin empty reconciliation");
        reconcile_external_identifiers(&mut tx, NamingKind::Tool, "server-a", "searxng", &[])
            .await
            .expect("reconcile empty inventory");
        tx.commit().await.expect("commit empty reconciliation");

        assert!(tool_mapping(&pool).await.is_empty());
    }

    #[tokio::test]
    async fn authoritative_removal_cleans_non_tool_profile_references() {
        let pool = test_pool().await;
        for statement in [
            "INSERT INTO server_prompts (id, server_id, server_name, prompt_name, unique_name) VALUES ('prompt-1', 'server-a', 'docs', 'help', 'docs_help')",
            "INSERT INTO server_prompts (id, server_id, server_name, prompt_name, unique_name) VALUES ('prompt-2', 'server-a', 'docs', 'summary', 'docs_summary')",
            "INSERT INTO server_resources (id, server_id, server_name, resource_uri, unique_uri) VALUES ('resource-1', 'server-a', 'docs', 'file:///guide.md', 'docs:file:///guide.md')",
            "INSERT INTO server_resources (id, server_id, server_name, resource_uri, unique_uri) VALUES ('resource-2', 'server-a', 'docs', 'file:///readme.md', 'docs:file:///readme.md')",
            "INSERT INTO server_resource_templates (id, server_id, server_name, uri_template, unique_name) VALUES ('template-1', 'server-a', 'docs', 'docs://{id}', 'docs://{id}')",
            "INSERT INTO server_resource_templates (id, server_id, server_name, uri_template, unique_name) VALUES ('template-2', 'server-a', 'docs', 'files://{path}', 'files://{path}')",
            "INSERT INTO profile_prompt (id, profile_id, server_id, server_name, prompt_name, enabled) VALUES ('profile-prompt-1', 'profile-1', 'server-a', 'docs', 'help', 1)",
            "INSERT INTO profile_prompt (id, profile_id, server_id, server_name, prompt_name, enabled) VALUES ('profile-prompt-2', 'profile-1', 'server-a', 'docs', 'summary', 1)",
            "INSERT INTO profile_resource (id, profile_id, server_id, server_name, resource_uri, enabled) VALUES ('profile-resource-1', 'profile-1', 'server-a', 'docs', 'file:///guide.md', 1)",
            "INSERT INTO profile_resource (id, profile_id, server_id, server_name, resource_uri, enabled) VALUES ('profile-resource-2', 'profile-1', 'server-a', 'docs', 'file:///readme.md', 1)",
            "INSERT INTO profile_resource_template (id, profile_id, server_id, server_name, uri_template, enabled) VALUES ('profile-template-1', 'profile-1', 'server-a', 'docs', 'docs://{id}', 1)",
            "INSERT INTO profile_resource_template (id, profile_id, server_id, server_name, uri_template, enabled) VALUES ('profile-template-2', 'profile-1', 'server-a', 'docs', 'files://{path}', 1)",
        ] {
            sqlx::query(statement)
                .execute(&pool)
                .await
                .expect("insert non-tool capability fixture");
        }

        let mut tx = pool.begin().await.expect("begin authoritative removal");
        for (kind, retained) in [
            (NamingKind::Prompt, "summary"),
            (NamingKind::Resource, "file:///readme.md"),
            (NamingKind::ResourceTemplate, "files://{path}"),
        ] {
            reconcile_external_identifiers(&mut tx, kind, "server-a", "docs", &[retained.to_string()])
                .await
                .expect("remove authoritative capability inventory");
        }
        tx.commit().await.expect("commit authoritative removal");

        for (table, value_column, retained) in [
            ("profile_prompt", "prompt_name", "summary"),
            ("profile_resource", "resource_uri", "file:///readme.md"),
            ("profile_resource_template", "uri_template", "files://{path}"),
        ] {
            let values: Vec<String> = sqlx::query_scalar(&format!("SELECT {value_column} FROM {table}"))
                .fetch_all(&pool)
                .await
                .expect("load remaining profile references");
            assert_eq!(
                values,
                [retained],
                "{table} must retain only the authoritative capability"
            );
        }
    }

    #[tokio::test]
    async fn reconciliation_rejects_cross_server_collisions_without_suffixes() {
        let pool = test_pool().await;
        insert_tool(&pool, "tool-1", "server-a", "searxng", "web_search").await;

        let mut tx = pool.begin().await.expect("begin conflicting reconciliation");
        let error = reconcile_external_identifiers(
            &mut tx,
            NamingKind::Tool,
            "server-b",
            "searxng",
            &["web_search".to_string()],
        )
        .await
        .expect_err("cross-server collision must fail explicitly");
        tx.rollback().await.expect("rollback conflicting reconciliation");

        assert!(error.to_string().contains("already used by server 'server-a'"));
        let collision = error
            .downcast_ref::<ExternalIdentifierCollision>()
            .expect("collision must remain typed for namespace remediation");
        assert_eq!(collision.server_id, "server-b");
        assert_eq!(collision.conflicting_server_id, "server-a");
        let mapping = tool_mapping(&pool).await;
        assert_eq!(mapping["web_search"], "searxng_web_search");
        assert!(mapping.values().all(|external| !external.contains("server-b")));
    }

    #[tokio::test]
    async fn external_identifier_resolves_to_server_id_and_exact_upstream_value() {
        let pool = test_pool().await;
        insert_tool(&pool, "tool-1", "server-a", "searxng", "searxng_web_search").await;

        let identity = resolve_capability_identity_with_pool(&pool, NamingKind::Tool, "searxng_web_search")
            .await
            .expect("resolve tool identity");

        assert_eq!(identity.kind, NamingKind::Tool);
        assert_eq!(identity.server_id, "server-a");
        assert_eq!(identity.server_name, "searxng");
        assert_eq!(identity.upstream_value, "searxng_web_search");
        assert_eq!(identity.external_value, "searxng_web_search");
    }

    #[tokio::test]
    async fn every_supported_kind_routes_external_values_to_exact_upstream_values() {
        let pool = test_pool().await;
        for (kind, table, exact_column, external_column, upstream) in [
            (
                NamingKind::Tool,
                "server_tools",
                "tool_name",
                "unique_name",
                "get_searxng_status",
            ),
            (
                NamingKind::Prompt,
                "server_prompts",
                "prompt_name",
                "unique_name",
                "get_searxng_help",
            ),
            (
                NamingKind::Resource,
                "server_resources",
                "resource_uri",
                "unique_uri",
                "file:///searxng/status",
            ),
            (
                NamingKind::ResourceTemplate,
                "server_resource_templates",
                "uri_template",
                "unique_name",
                "searxng://status/{id}",
            ),
        ] {
            let mut tx = pool.begin().await.expect("begin capability insert");
            let reconciliation =
                reconcile_external_identifiers(&mut tx, kind, "server-a", "searxng", &[upstream.to_string()])
                    .await
                    .expect("reconcile external identifier");
            let external = reconciliation
                .identifier_for(upstream)
                .expect("planned capability identifier")
                .to_string();
            let statement = format!(
                "INSERT INTO {table} (id, server_id, server_name, {exact_column}, {external_column}) VALUES (?, ?, ?, ?, ?)"
            );
            sqlx::query(&statement)
                .bind(format!("row-{table}"))
                .bind("server-a")
                .bind("searxng")
                .bind(upstream)
                .bind(&external)
                .execute(&mut *tx)
                .await
                .expect("insert capability mapping");
            tx.commit().await.expect("commit capability insert");

            let identity = resolve_capability_identity_with_pool(&pool, kind, &external)
                .await
                .expect("resolve capability identity");
            assert_eq!(identity.server_id, "server-a");
            assert_eq!(identity.upstream_value, upstream);
            assert_eq!(identity.external_value, external);
        }
    }
}
