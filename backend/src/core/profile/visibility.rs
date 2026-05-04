use std::collections::HashSet;
use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use sha2::{Digest, Sha256};

use crate::clients::models::{CapabilitySource, ClientCapabilityConfig, UnifyDirectExposureConfig};
use crate::config::database::Database;
use crate::config::profile::basic::get_active_profile;
use crate::core::capability::naming::{NamingKind, generate_unique_name, resolve_unique_name};
use crate::core::profile::ProfileService;
use crate::core::proxy::server::ClientContext;
use crate::mcper::{PROFILE_MODE_BUILTIN_TOOL_NAMES, UNIFY_BUILTIN_TOOL_NAMES};

fn builtin_tool_surface_ids(
    config_mode: Option<&str>,
    capability_source: CapabilitySource,
) -> Vec<&'static str> {
    match config_mode {
        Some("unify") => UNIFY_BUILTIN_TOOL_NAMES.to_vec(),
        Some("transparent") => Vec::new(),
        _ => {
            if capability_source == CapabilitySource::Profiles {
                PROFILE_MODE_BUILTIN_TOOL_NAMES.to_vec()
            } else {
                Vec::new()
            }
        }
    }
}

fn collect_sorted_surfaces<T, F>(
    surfaces: &[T],
    format_key: F,
) -> Vec<String>
where
    F: Fn(&T) -> String,
{
    let mut result: Vec<String> = surfaces.iter().map(format_key).collect();
    result.sort();
    result.dedup();
    result
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityKind {
    Tools,
    Resources,
    ResourceTemplates,
    Prompts,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibilityQuery {
    pub client_id: String,
    pub surface_fingerprint: String,
    pub capability_kind: CapabilityKind,
}

#[derive(Debug, Clone)]
pub struct VisibilitySnapshot {
    pub client_id: String,
    pub surface_fingerprint: String,
    pub profile_ids: Vec<String>,
    pub server_ids: Vec<String>,
    pub allowed_tools: HashSet<String>,
    pub allowed_resources: HashSet<String>,
    pub allowed_resource_templates: HashSet<String>,
    pub allowed_prompts: HashSet<String>,
    allowed_resource_prefixes: HashSet<String>,
    has_tool_policy: bool,
    has_resource_policy: bool,
    has_resource_template_policy: bool,
    has_prompt_policy: bool,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct ClientCapabilityRow {
    capability_source: Option<String>,
    selected_profile_ids: Option<String>,
    custom_profile_id: Option<String>,
}

struct ResolvedPolicies {
    allowed_tools: HashSet<String>,
    allowed_resources: HashSet<String>,
    allowed_resource_templates: HashSet<String>,
    allowed_prompts: HashSet<String>,
    allowed_resource_prefixes: HashSet<String>,
    has_tool_policy: bool,
    has_resource_policy: bool,
    has_resource_template_policy: bool,
    has_prompt_policy: bool,
}

impl ResolvedPolicies {
    fn policy_flags(&self) -> [bool; 4] {
        [
            self.has_tool_policy,
            self.has_resource_policy,
            self.has_resource_template_policy,
            self.has_prompt_policy,
        ]
    }
}

pub struct ProfileVisibilityService {
    db: Option<Arc<Database>>,
    _profile_service: Option<Arc<ProfileService>>,
}

impl ProfileVisibilityService {
    pub fn new(
        db: Option<Arc<Database>>,
        profile_service: Option<Arc<ProfileService>>,
    ) -> Self {
        Self {
            db,
            _profile_service: profile_service,
        }
    }

    pub async fn resolve_snapshot(
        &self,
        client_id: &str,
        profile_id_override: Option<&str>,
    ) -> Result<VisibilitySnapshot> {
        let db = self
            .db
            .as_ref()
            .context("Profile visibility requires database access")?;

        let capability_config = self
            .load_client_capability_config(client_id, profile_id_override)
            .await?;

        let profile_ids = self.resolve_profile_ids(&db.pool, &capability_config).await?;
        let server_ids = self
            .resolve_server_ids(&db.pool, capability_config.capability_source, &profile_ids)
            .await?;

        let policies = self.resolve_policies(&db.pool, &server_ids, &profile_ids).await?;
        let surface_fingerprint = compute_surface_fingerprint(&capability_config, &policies, None, None, None);
        let snapshot = build_snapshot(client_id, surface_fingerprint, profile_ids, server_ids, policies);

        tracing::debug!(
            client_id = %client_id,
            fingerprint = %snapshot.surface_fingerprint,
            "Resolved visibility snapshot"
        );

        Ok(snapshot)
    }

    pub async fn resolve_snapshot_for_client(
        &self,
        client_context: &ClientContext,
    ) -> Result<VisibilitySnapshot> {
        if matches!(client_context.config_mode.as_deref(), Some("unify")) {
            return self
                .resolve_unify_snapshot(
                    &client_context.client_id,
                    client_context.config_mode.as_deref(),
                    client_context.unify_workspace.as_ref(),
                )
                .await;
        }

        let capability_config = self.resolve_capability_config_for_client(client_context).await?;
        self.resolve_snapshot_from_config(
            &client_context.client_id,
            &capability_config,
            client_context.config_mode.as_deref(),
            client_context.unify_workspace.as_ref(),
        )
        .await
    }

    pub async fn resolve_capability_config(
        &self,
        client_id: &str,
    ) -> Result<ClientCapabilityConfig> {
        self.load_client_capability_config(client_id, None).await
    }

    pub async fn resolve_capability_config_for_client(
        &self,
        client_context: &ClientContext,
    ) -> Result<ClientCapabilityConfig> {
        if matches!(client_context.config_mode.as_deref(), Some("unify")) {
            return Ok(Self::active_capability_config());
        }

        self.load_client_capability_config(&client_context.client_id, client_context.profile_id.as_deref())
            .await
    }

    async fn resolve_policies(
        &self,
        pool: &sqlx::Pool<sqlx::Sqlite>,
        server_ids: &[String],
        profile_ids: &[String],
    ) -> Result<ResolvedPolicies> {
        let (allowed_tools, has_tool_policy) = self.resolve_allowed_tools(pool, server_ids, profile_ids).await?;
        let (allowed_resources, has_resource_policy) =
            self.resolve_allowed_resources(pool, server_ids, profile_ids).await?;
        let (allowed_resource_templates, allowed_resource_prefixes, has_resource_template_policy) = self
            .resolve_allowed_resource_templates(pool, server_ids, profile_ids)
            .await?;
        let (allowed_prompts, has_prompt_policy) = self.resolve_allowed_prompts(pool, server_ids, profile_ids).await?;

        Ok(ResolvedPolicies {
            allowed_tools,
            allowed_resources,
            allowed_resource_templates,
            allowed_prompts,
            allowed_resource_prefixes,
            has_tool_policy,
            has_resource_policy,
            has_resource_template_policy,
            has_prompt_policy,
        })
    }

    async fn resolve_snapshot_from_config(
        &self,
        client_id: &str,
        capability_config: &ClientCapabilityConfig,
        config_mode: Option<&str>,
        unify_workspace: Option<&UnifyDirectExposureConfig>,
    ) -> Result<VisibilitySnapshot> {
        let db = self
            .db
            .as_ref()
            .context("Profile visibility requires database access")?;

        let profile_ids = self.resolve_profile_ids(&db.pool, capability_config).await?;
        let server_ids = self
            .resolve_server_ids(&db.pool, capability_config.capability_source, &profile_ids)
            .await?;

        let policies = self.resolve_policies(&db.pool, &server_ids, &profile_ids).await?;
        let direct_surface_fingerprint = self.compute_unify_direct_surface_fingerprint(unify_workspace).await?;

        let surface_fingerprint = compute_surface_fingerprint(
            capability_config,
            &policies,
            config_mode,
            direct_surface_fingerprint.as_deref(),
            Some(builtin_tool_surface_ids(
                config_mode,
                capability_config.capability_source,
            )),
        );

        let snapshot = build_snapshot(client_id, surface_fingerprint, profile_ids, server_ids, policies);

        Ok(snapshot)
    }

    async fn resolve_unify_snapshot(
        &self,
        client_id: &str,
        config_mode: Option<&str>,
        unify_workspace: Option<&UnifyDirectExposureConfig>,
    ) -> Result<VisibilitySnapshot> {
        let db = self
            .db
            .as_ref()
            .context("Profile visibility requires database access")?;

        let capability_config = Self::active_capability_config();

        let profile_ids = Vec::new();
        let server_ids = self.resolve_globally_enabled_server_ids(&db.pool).await?;

        let policies = self.resolve_policies(&db.pool, &server_ids, &profile_ids).await?;
        let direct_surface_fingerprint = self.compute_unify_direct_surface_fingerprint(unify_workspace).await?;

        let surface_fingerprint = compute_surface_fingerprint(
            &capability_config,
            &policies,
            config_mode,
            direct_surface_fingerprint.as_deref(),
            Some(builtin_tool_surface_ids(
                config_mode,
                capability_config.capability_source,
            )),
        );

        let snapshot = build_snapshot(client_id, surface_fingerprint, profile_ids, server_ids, policies);

        Ok(snapshot)
    }

    async fn compute_unify_direct_surface_fingerprint(
        &self,
        unify_workspace: Option<&UnifyDirectExposureConfig>,
    ) -> Result<Option<String>> {
        let Some(workspace) = unify_workspace else {
            return Ok(None);
        };

        let tool_surfaces = collect_sorted_surfaces(&workspace.selected_tool_surfaces, |surface| {
            format!("{}\u{1e}{}", surface.server_id, surface.tool_name)
        });
        let prompt_surfaces = collect_sorted_surfaces(&workspace.selected_prompt_surfaces, |surface| {
            format!("{}\u{1e}{}", surface.server_id, surface.prompt_name)
        });
        let resource_surfaces = collect_sorted_surfaces(&workspace.selected_resource_surfaces, |surface| {
            format!("{}\u{1e}{}", surface.server_id, surface.resource_uri)
        });
        let template_surfaces = collect_sorted_surfaces(&workspace.selected_template_surfaces, |surface| {
            format!("{}\u{1e}{}", surface.server_id, surface.uri_template)
        });

        let mut selected_server_ids = workspace.selected_server_ids.clone();
        selected_server_ids.sort();
        selected_server_ids.dedup();

        let mut hasher = Sha256::new();
        hasher.update(workspace.route_mode.as_str());
        hasher.update([0]);
        hasher.update(selected_server_ids.join("\u{1f}"));
        hasher.update([0]);
        hasher.update(tool_surfaces.join("\u{1f}"));
        hasher.update([0]);
        hasher.update(prompt_surfaces.join("\u{1f}"));
        hasher.update([0]);
        hasher.update(resource_surfaces.join("\u{1f}"));
        hasher.update([0]);
        hasher.update(template_surfaces.join("\u{1f}"));
        Ok(Some(format!("{:x}", hasher.finalize())))
    }

    pub async fn filter_tools_for_client(
        &self,
        client_context: &ClientContext,
        tools: Vec<rmcp::model::Tool>,
    ) -> Result<Vec<rmcp::model::Tool>> {
        let snapshot = self.resolve_snapshot_for_client(client_context).await?;
        Ok(self.filter_tools_with_snapshot(&snapshot, tools))
    }

    pub async fn filter_resources_for_client(
        &self,
        client_context: &ClientContext,
        resources: Vec<rmcp::model::Resource>,
        templates: Vec<rmcp::model::ResourceTemplate>,
    ) -> Result<(Vec<rmcp::model::Resource>, Vec<rmcp::model::ResourceTemplate>)> {
        let snapshot = self.resolve_snapshot_for_client(client_context).await?;
        Ok(self.filter_resources_with_snapshot(&snapshot, resources, templates))
    }

    pub async fn filter_prompts_for_client(
        &self,
        client_context: &ClientContext,
        prompts: Vec<rmcp::model::Prompt>,
    ) -> Result<Vec<rmcp::model::Prompt>> {
        let snapshot = self.resolve_snapshot_for_client(client_context).await?;
        Ok(self.filter_prompts_with_snapshot(&snapshot, prompts))
    }

    pub async fn assert_tool_allowed(
        &self,
        client_context: &ClientContext,
        unique_tool_name: &str,
    ) -> Result<()> {
        let snapshot = self.resolve_snapshot_for_client(client_context).await?;
        self.assert_tool_allowed_with_snapshot(&snapshot, unique_tool_name)
            .await
    }

    pub async fn assert_resource_allowed(
        &self,
        client_context: &ClientContext,
        unique_resource_uri: &str,
    ) -> Result<()> {
        let snapshot = self.resolve_snapshot_for_client(client_context).await?;
        self.assert_resource_allowed_with_snapshot(&snapshot, unique_resource_uri)
            .await
    }

    pub async fn assert_prompt_allowed(
        &self,
        client_context: &ClientContext,
        unique_prompt_name: &str,
    ) -> Result<()> {
        let snapshot = self.resolve_snapshot_for_client(client_context).await?;
        self.assert_prompt_allowed_with_snapshot(&snapshot, unique_prompt_name)
            .await
    }

    pub fn filter_tools_with_snapshot(
        &self,
        snapshot: &VisibilitySnapshot,
        mut tools: Vec<rmcp::model::Tool>,
    ) -> Vec<rmcp::model::Tool> {
        if snapshot.server_ids.is_empty() {
            return Vec::new();
        }

        if !snapshot.has_tool_policy {
            return tools;
        }

        tools.retain(|tool| snapshot.allowed_tools.contains(tool.name.as_ref()));
        tools
    }

    pub fn filter_resources_with_snapshot(
        &self,
        snapshot: &VisibilitySnapshot,
        mut resources: Vec<rmcp::model::Resource>,
        mut templates: Vec<rmcp::model::ResourceTemplate>,
    ) -> (Vec<rmcp::model::Resource>, Vec<rmcp::model::ResourceTemplate>) {
        if snapshot.server_ids.is_empty() {
            return (Vec::new(), Vec::new());
        }

        if snapshot.has_resource_policy || snapshot.has_resource_template_policy {
            resources.retain(|resource| resource_allowed_from_snapshot(snapshot, resource.raw.uri.as_str()));
        }

        if snapshot.has_resource_template_policy {
            templates.retain(|template| snapshot.allowed_resource_templates.contains(template.raw.name.as_str()));
        }

        (resources, templates)
    }

    pub fn filter_prompts_with_snapshot(
        &self,
        snapshot: &VisibilitySnapshot,
        mut prompts: Vec<rmcp::model::Prompt>,
    ) -> Vec<rmcp::model::Prompt> {
        if snapshot.server_ids.is_empty() {
            return Vec::new();
        }

        if !snapshot.has_prompt_policy {
            return prompts;
        }

        prompts.retain(|prompt| snapshot.allowed_prompts.contains(prompt.name.as_str()));
        prompts
    }

    pub async fn assert_tool_allowed_with_snapshot(
        &self,
        snapshot: &VisibilitySnapshot,
        unique_tool_name: &str,
    ) -> Result<()> {
        ensure_allowed(
            self.snapshot_allows_tool(snapshot, unique_tool_name).await?,
            format!("Tool '{unique_tool_name}' is not available for this client"),
        )
    }

    pub async fn assert_resource_allowed_with_snapshot(
        &self,
        snapshot: &VisibilitySnapshot,
        unique_resource_uri: &str,
    ) -> Result<()> {
        ensure_allowed(
            self.snapshot_allows_resource(snapshot, unique_resource_uri).await?,
            format!("Resource '{unique_resource_uri}' is not available for this client"),
        )
    }

    pub async fn assert_prompt_allowed_with_snapshot(
        &self,
        snapshot: &VisibilitySnapshot,
        unique_prompt_name: &str,
    ) -> Result<()> {
        ensure_allowed(
            self.snapshot_allows_prompt(snapshot, unique_prompt_name).await?,
            format!("Prompt '{unique_prompt_name}' is not available for this client"),
        )
    }

    async fn snapshot_allows_tool(
        &self,
        snapshot: &VisibilitySnapshot,
        unique_tool_name: &str,
    ) -> Result<bool> {
        if snapshot.server_ids.is_empty() {
            return Ok(false);
        }

        if snapshot.has_tool_policy {
            return Ok(snapshot.allowed_tools.contains(unique_tool_name));
        }

        self.snapshot_allows_server(NamingKind::Tool, snapshot, unique_tool_name)
            .await
    }

    async fn snapshot_allows_resource(
        &self,
        snapshot: &VisibilitySnapshot,
        unique_resource_uri: &str,
    ) -> Result<bool> {
        if snapshot.server_ids.is_empty() {
            return Ok(false);
        }

        if snapshot.has_resource_policy || snapshot.has_resource_template_policy {
            return Ok(resource_allowed_from_snapshot(snapshot, unique_resource_uri));
        }

        self.snapshot_allows_server(NamingKind::Resource, snapshot, unique_resource_uri)
            .await
    }

    async fn snapshot_allows_prompt(
        &self,
        snapshot: &VisibilitySnapshot,
        unique_prompt_name: &str,
    ) -> Result<bool> {
        if snapshot.server_ids.is_empty() {
            return Ok(false);
        }

        if snapshot.has_prompt_policy {
            return Ok(snapshot.allowed_prompts.contains(unique_prompt_name));
        }

        self.snapshot_allows_server(NamingKind::Prompt, snapshot, unique_prompt_name)
            .await
    }

    async fn snapshot_allows_server(
        &self,
        kind: NamingKind,
        snapshot: &VisibilitySnapshot,
        unique_value: &str,
    ) -> Result<bool> {
        let (server_name, _) = resolve_unique_name(kind, unique_value)
            .await
            .with_context(|| format!("Failed to resolve canonical capability name '{unique_value}'"))?;
        let server_id = crate::core::capability::resolver::to_id(&server_name)
            .await
            .with_context(|| format!("Failed to resolve server id for '{server_name}'"))?
            .ok_or_else(|| anyhow!("Server '{server_name}' not found for canonical capability '{unique_value}'"))?;
        Ok(snapshot.server_ids.iter().any(|candidate| candidate == &server_id))
    }

    async fn load_client_capability_config(
        &self,
        client_id: &str,
        profile_id_override: Option<&str>,
    ) -> Result<ClientCapabilityConfig> {
        if let Some(profile_id) = profile_id_override {
            tracing::info!(
                client_id = %client_id,
                profile_id = %profile_id,
                "Using profile_id override from URL parameter"
            );
            return Ok(Self::custom_capability_config(profile_id));
        }

        let db = self
            .db
            .as_ref()
            .context("Profile visibility requires database access")?;

        let row_opt = sqlx::query_as::<_, ClientCapabilityRow>(
            r#"
            SELECT capability_source, selected_profile_ids, custom_profile_id
            FROM client
            WHERE identifier = ?
            "#,
        )
        .bind(client_id)
        .fetch_optional(&db.pool)
        .await
        .with_context(|| format!("Failed to load client capability config for '{client_id}'"))?;

        if let Some(row) = row_opt {
            return ClientCapabilityConfig::from_parts(
                row.capability_source.as_deref(),
                row.selected_profile_ids.as_deref(),
                row.custom_profile_id,
            )
            .map_err(|error| anyhow!(error));
        }

        tracing::warn!(
            client_id = %client_id,
            "Client not configured in database, using active profile as fallback"
        );

        Ok(Self::active_capability_config())
    }

    fn custom_capability_config(profile_id: &str) -> ClientCapabilityConfig {
        ClientCapabilityConfig {
            capability_source: CapabilitySource::Custom,
            selected_profile_ids: vec![],
            custom_profile_id: Some(profile_id.to_string()),
        }
    }

    fn active_capability_config() -> ClientCapabilityConfig {
        ClientCapabilityConfig {
            capability_source: CapabilitySource::Activated,
            selected_profile_ids: vec![],
            custom_profile_id: None,
        }
    }

    async fn resolve_profile_ids(
        &self,
        pool: &sqlx::Pool<sqlx::Sqlite>,
        capability_config: &ClientCapabilityConfig,
    ) -> Result<Vec<String>> {
        let mut profile_ids = match capability_config.capability_source {
            CapabilitySource::Activated => get_active_profile(pool)
                .await
                .context("Failed to load active profiles")?
                .into_iter()
                .filter_map(|profile| profile.id)
                .collect(),
            CapabilitySource::Profiles => capability_config.selected_profile_ids.clone(),
            CapabilitySource::Custom => vec![
                capability_config
                    .custom_profile_id
                    .clone()
                    .ok_or_else(|| anyhow!("Custom capability source requires custom_profile_id"))?,
            ],
        };

        profile_ids.sort();
        profile_ids.dedup();
        Ok(profile_ids)
    }

    async fn resolve_server_ids(
        &self,
        pool: &sqlx::Pool<sqlx::Sqlite>,
        capability_source: CapabilitySource,
        profile_ids: &[String],
    ) -> Result<Vec<String>> {
        let mut server_ids = if profile_ids.is_empty() {
            if capability_source == CapabilitySource::Activated {
                self.resolve_globally_enabled_server_ids(pool).await?
            } else {
                Vec::new()
            }
        } else {
            let placeholders = repeat_placeholders(profile_ids.len());
            let sql = format!(
                r#"
                SELECT DISTINCT sc.id
                FROM server_config sc
                JOIN profile_server ps ON sc.id = ps.server_id
                WHERE ps.profile_id IN ({placeholders})
                  AND ps.enabled = 1
                  AND sc.enabled = 1
                ORDER BY sc.name, sc.id
                "#,
            );

            let mut query = sqlx::query_scalar::<_, String>(&sql);
            for profile_id in profile_ids {
                query = query.bind(profile_id);
            }
            query
                .fetch_all(pool)
                .await
                .context("Failed to resolve visible servers for client snapshot")?
        };

        server_ids.sort();
        server_ids.dedup();
        Ok(server_ids)
    }

    async fn resolve_globally_enabled_server_ids(
        &self,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> Result<Vec<String>> {
        let mut server_ids = sqlx::query_scalar::<_, String>(
            r#"
            SELECT id
            FROM server_config
            WHERE enabled = 1
            ORDER BY name, id
            "#,
        )
        .fetch_all(pool)
        .await
        .context("Failed to load globally enabled servers for visibility snapshot")?;

        server_ids.sort();
        server_ids.dedup();
        Ok(server_ids)
    }

    async fn resolve_allowed_tools(
        &self,
        pool: &sqlx::Pool<sqlx::Sqlite>,
        server_ids: &[String],
        profile_ids: &[String],
    ) -> Result<(HashSet<String>, bool)> {
        if server_ids.is_empty() {
            return Ok((HashSet::new(), false));
        }

        let has_policy = has_profile_rows(pool, "profile_tool", profile_ids).await?;
        let values = if has_policy {
            let profile_placeholders = repeat_placeholders(profile_ids.len());
            let server_placeholders = repeat_placeholders(server_ids.len());
            let sql = format!(
                r#"
                SELECT DISTINCT st.unique_name
                FROM profile_tool pt
                JOIN server_tools st ON pt.server_tool_id = st.id
                JOIN server_config sc ON st.server_id = sc.id
                WHERE pt.profile_id IN ({profile_placeholders})
                  AND st.server_id IN ({server_placeholders})
                  AND pt.enabled = 1
                  AND sc.enabled = 1
                "#,
            );

            let mut query = sqlx::query_scalar::<_, String>(&sql);
            for profile_id in profile_ids {
                query = query.bind(profile_id);
            }
            for server_id in server_ids {
                query = query.bind(server_id);
            }
            query.fetch_all(pool).await.context("Failed to load tool policy rows")?
        } else {
            query_unique_values(pool, "server_tools", "unique_name", "server_id", server_ids)
                .await
                .context("Failed to load visible tools for snapshot")?
        };

        Ok((values.into_iter().collect(), has_policy))
    }

    async fn resolve_allowed_resources(
        &self,
        pool: &sqlx::Pool<sqlx::Sqlite>,
        server_ids: &[String],
        profile_ids: &[String],
    ) -> Result<(HashSet<String>, bool)> {
        if server_ids.is_empty() {
            return Ok((HashSet::new(), false));
        }

        let has_policy = has_profile_rows(pool, "profile_resource", profile_ids).await?;
        let values = if has_policy {
            let profile_placeholders = repeat_placeholders(profile_ids.len());
            let server_placeholders = repeat_placeholders(server_ids.len());
            let sql = format!(
                r#"
                SELECT DISTINCT sc.name, pr.resource_uri
                FROM profile_resource pr
                JOIN server_config sc ON pr.server_id = sc.id
                WHERE pr.profile_id IN ({profile_placeholders})
                  AND pr.server_id IN ({server_placeholders})
                  AND pr.enabled = 1
                  AND sc.enabled = 1
                "#,
            );

            let mut query = sqlx::query_as::<_, (String, String)>(&sql);
            for profile_id in profile_ids {
                query = query.bind(profile_id);
            }
            for server_id in server_ids {
                query = query.bind(server_id);
            }

            query
                .fetch_all(pool)
                .await
                .context("Failed to load resource policy rows")?
                .into_iter()
                .map(|(server_name, resource_uri)| {
                    generate_unique_name(NamingKind::Resource, &server_name, &resource_uri)
                })
                .collect()
        } else {
            query_unique_values(pool, "server_resources", "unique_uri", "server_id", server_ids)
                .await
                .context("Failed to load visible resources for snapshot")?
        };

        Ok((values.into_iter().collect(), has_policy))
    }

    async fn resolve_allowed_resource_templates(
        &self,
        pool: &sqlx::Pool<sqlx::Sqlite>,
        server_ids: &[String],
        profile_ids: &[String],
    ) -> Result<(HashSet<String>, HashSet<String>, bool)> {
        if server_ids.is_empty() {
            return Ok((HashSet::new(), HashSet::new(), false));
        }

        let has_policy = has_profile_rows(pool, "profile_resource_template", profile_ids).await?;
        let (values, prefixes) = if has_policy {
            let profile_placeholders = repeat_placeholders(profile_ids.len());
            let server_placeholders = repeat_placeholders(server_ids.len());
            let sql = format!(
                r#"
                SELECT DISTINCT sc.name, prt.uri_template
                FROM profile_resource_template prt
                JOIN server_config sc ON prt.server_id = sc.id
                WHERE prt.profile_id IN ({profile_placeholders})
                  AND prt.server_id IN ({server_placeholders})
                  AND prt.enabled = 1
                  AND sc.enabled = 1
                "#,
            );

            let mut query = sqlx::query_as::<_, (String, String)>(&sql);
            for profile_id in profile_ids {
                query = query.bind(profile_id);
            }
            for server_id in server_ids {
                query = query.bind(server_id);
            }

            let rows = query
                .fetch_all(pool)
                .await
                .context("Failed to load resource template policy rows")?;

            let values = rows
                .iter()
                .map(|(server_name, uri_template)| {
                    generate_unique_name(NamingKind::ResourceTemplate, server_name, uri_template)
                })
                .collect::<Vec<_>>();
            let prefixes = rows
                .iter()
                .map(|(server_name, uri_template)| {
                    let prefix = crate::config::profile::resource_template::template_prefix(uri_template);
                    generate_unique_name(NamingKind::Resource, server_name, prefix)
                })
                .collect::<Vec<_>>();
            (values, prefixes)
        } else {
            (
                query_unique_values(
                    pool,
                    "server_resource_templates",
                    "unique_name",
                    "server_id",
                    server_ids,
                )
                .await
                .context("Failed to load visible resource templates for snapshot")?,
                Vec::new(),
            )
        };

        Ok((values.into_iter().collect(), prefixes.into_iter().collect(), has_policy))
    }

    async fn resolve_allowed_prompts(
        &self,
        pool: &sqlx::Pool<sqlx::Sqlite>,
        server_ids: &[String],
        profile_ids: &[String],
    ) -> Result<(HashSet<String>, bool)> {
        if server_ids.is_empty() {
            return Ok((HashSet::new(), false));
        }

        let has_policy = has_profile_rows(pool, "profile_prompt", profile_ids).await?;
        let values = if has_policy {
            let profile_placeholders = repeat_placeholders(profile_ids.len());
            let server_placeholders = repeat_placeholders(server_ids.len());
            let sql = format!(
                r#"
                SELECT DISTINCT sc.name, pp.prompt_name
                FROM profile_prompt pp
                JOIN server_config sc ON pp.server_id = sc.id
                WHERE pp.profile_id IN ({profile_placeholders})
                  AND pp.server_id IN ({server_placeholders})
                  AND pp.enabled = 1
                  AND sc.enabled = 1
                "#,
            );

            let mut query = sqlx::query_as::<_, (String, String)>(&sql);
            for profile_id in profile_ids {
                query = query.bind(profile_id);
            }
            for server_id in server_ids {
                query = query.bind(server_id);
            }

            query
                .fetch_all(pool)
                .await
                .context("Failed to load prompt policy rows")?
                .into_iter()
                .map(|(server_name, prompt_name)| generate_unique_name(NamingKind::Prompt, &server_name, &prompt_name))
                .collect()
        } else {
            query_unique_values(pool, "server_prompts", "unique_name", "server_id", server_ids)
                .await
                .context("Failed to load visible prompts for snapshot")?
        };

        Ok((values.into_iter().collect(), has_policy))
    }
}

fn ensure_allowed(
    allowed: bool,
    message: String,
) -> Result<()> {
    if allowed { Ok(()) } else { Err(anyhow!(message)) }
}

fn resource_allowed_from_snapshot(
    snapshot: &VisibilitySnapshot,
    unique_uri: &str,
) -> bool {
    if snapshot.server_ids.is_empty() {
        return false;
    }

    if !snapshot.has_resource_policy && !snapshot.has_resource_template_policy {
        return true;
    }

    if snapshot.has_resource_policy && snapshot.allowed_resources.contains(unique_uri) {
        return true;
    }

    if snapshot.has_resource_template_policy
        && snapshot
            .allowed_resource_prefixes
            .iter()
            .any(|prefix| unique_uri.starts_with(prefix))
    {
        return true;
    }

    false
}

async fn has_profile_rows(
    pool: &sqlx::Pool<sqlx::Sqlite>,
    table: &str,
    profile_ids: &[String],
) -> Result<bool> {
    if profile_ids.is_empty() {
        return Ok(false);
    }

    let placeholders = repeat_placeholders(profile_ids.len());
    let sql = format!("SELECT COUNT(1) FROM {table} WHERE profile_id IN ({placeholders})");
    let mut query = sqlx::query_scalar::<_, i64>(&sql);
    for profile_id in profile_ids {
        query = query.bind(profile_id);
    }
    Ok(query
        .fetch_one(pool)
        .await
        .with_context(|| format!("Failed to query profile rows from '{table}'"))?
        > 0)
}

async fn query_unique_values(
    pool: &sqlx::Pool<sqlx::Sqlite>,
    table: &str,
    value_column: &str,
    filter_column: &str,
    filter_values: &[String],
) -> Result<Vec<String>> {
    if filter_values.is_empty() {
        return Ok(Vec::new());
    }

    let placeholders = repeat_placeholders(filter_values.len());
    let sql = format!(
        "SELECT DISTINCT {value_column} FROM {table} WHERE {filter_column} IN ({placeholders}) ORDER BY {value_column}"
    );
    let mut query = sqlx::query_scalar::<_, String>(&sql);
    for filter_value in filter_values {
        query = query.bind(filter_value);
    }
    query
        .fetch_all(pool)
        .await
        .with_context(|| format!("Failed to load values from '{table}'"))
}

fn repeat_placeholders(count: usize) -> String {
    vec!["?"; count].join(", ")
}

fn compute_surface_fingerprint(
    capability_config: &ClientCapabilityConfig,
    policies: &ResolvedPolicies,
    config_mode: Option<&str>,
    direct_surface_fingerprint: Option<&str>,
    builtin_tools: Option<Vec<&str>>,
) -> String {
    compute_surface_hash(SurfaceFingerprintInput {
        capability_config,
        allowed_tools: &policies.allowed_tools,
        allowed_resources: &policies.allowed_resources,
        allowed_resource_templates: &policies.allowed_resource_templates,
        allowed_resource_prefixes: &policies.allowed_resource_prefixes,
        allowed_prompts: &policies.allowed_prompts,
        policy_flags: policies.policy_flags(),
        config_mode,
        direct_surface_fingerprint,
        builtin_tools,
    })
}

fn build_snapshot(
    client_id: &str,
    surface_fingerprint: String,
    profile_ids: Vec<String>,
    server_ids: Vec<String>,
    policies: ResolvedPolicies,
) -> VisibilitySnapshot {
    VisibilitySnapshot {
        client_id: client_id.to_string(),
        surface_fingerprint,
        profile_ids,
        server_ids,
        allowed_tools: policies.allowed_tools,
        allowed_resources: policies.allowed_resources,
        allowed_resource_templates: policies.allowed_resource_templates,
        allowed_prompts: policies.allowed_prompts,
        allowed_resource_prefixes: policies.allowed_resource_prefixes,
        has_tool_policy: policies.has_tool_policy,
        has_resource_policy: policies.has_resource_policy,
        has_resource_template_policy: policies.has_resource_template_policy,
        has_prompt_policy: policies.has_prompt_policy,
    }
}

struct SurfaceFingerprintInput<'a> {
    capability_config: &'a ClientCapabilityConfig,
    allowed_tools: &'a HashSet<String>,
    allowed_resources: &'a HashSet<String>,
    allowed_resource_templates: &'a HashSet<String>,
    allowed_resource_prefixes: &'a HashSet<String>,
    allowed_prompts: &'a HashSet<String>,
    policy_flags: [bool; 4],
    config_mode: Option<&'a str>,
    direct_surface_fingerprint: Option<&'a str>,
    builtin_tools: Option<Vec<&'a str>>,
}

fn compute_surface_hash(input: SurfaceFingerprintInput<'_>) -> String {
    let mut hasher = Sha256::new();

    hasher.update(input.config_mode.unwrap_or("hosted"));
    hasher.update([0]);
    hasher.update(sorted_values(input.allowed_tools).join("\u{1f}"));
    hasher.update([0]);
    hasher.update(sorted_values(input.allowed_resources).join("\u{1f}"));
    hasher.update([0]);
    hasher.update(sorted_values(input.allowed_resource_templates).join("\u{1f}"));
    hasher.update([0]);
    hasher.update(sorted_values(input.allowed_resource_prefixes).join("\u{1f}"));
    hasher.update([0]);
    hasher.update(sorted_values(input.allowed_prompts).join("\u{1f}"));
    hasher.update([0]);

    let mut builtin_tools = input
        .builtin_tools
        .unwrap_or_default()
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    builtin_tools.sort();
    builtin_tools.dedup();
    hasher.update(builtin_tools.join("\u{1f}"));
    hasher.update([0]);

    hasher.update(input.capability_config.capability_source.as_str());
    hasher.update([0]);

    if let Some(direct_surface_fingerprint) = input.direct_surface_fingerprint {
        hasher.update(direct_surface_fingerprint);
    }
    hasher.update([0]);

    for flag in input.policy_flags {
        hasher.update([u8::from(flag)]);
    }
    format!("{:x}", hasher.finalize())
}

fn sorted_values(values: &HashSet<String>) -> Vec<String> {
    let mut sorted = values.iter().cloned().collect::<Vec<_>>();
    sorted.sort();
    sorted
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::clients::models::CapabilitySource;
    use crate::common::profile::ProfileType;
    use crate::common::server::ServerType;
    use crate::config::{
        client::init::initialize_client_table,
        models::{Profile, Server},
        profile::{self, init::initialize_profile_tables},
        server::{self, init::initialize_server_tables},
    };
    use serial_test::serial;
    use sqlx::sqlite::SqlitePoolOptions;
    use tempfile::TempDir;

    async fn create_visibility_service() -> (TempDir, Arc<Database>, ProfileVisibilityService) {
        let temp_dir = TempDir::new().expect("temp dir");
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("sqlite pool");

        initialize_server_tables(&pool).await.expect("init server tables");
        initialize_profile_tables(&pool).await.expect("init profile tables");
        initialize_client_table(&pool).await.expect("init client table");
        crate::config::client::init::initialize_system_settings(&pool)
            .await
            .expect("init system settings table");

        crate::core::capability::naming::initialize(pool.clone());

        let db = Arc::new(Database {
            pool,
            path: temp_dir.path().join("test.db"),
        });

        let service = ProfileVisibilityService::new(Some(db.clone()), None);
        (temp_dir, db, service)
    }

    async fn insert_profile(
        db: &Arc<Database>,
        name: &str,
        profile_type: ProfileType,
        is_active: bool,
    ) -> String {
        let mut profile = Profile::new(name.to_string(), profile_type);
        profile.is_active = is_active;
        profile::upsert_profile(&db.pool, &profile)
            .await
            .expect("upsert profile")
    }

    async fn insert_server(
        db: &Arc<Database>,
        name: &str,
        capabilities: &str,
    ) -> String {
        let mut server = Server::new(name.to_string(), ServerType::Stdio);
        server.capabilities = Some(capabilities.to_string());
        server::upsert_server(&db.pool, &server).await.expect("upsert server")
    }

    async fn seed_tool(
        db: &Arc<Database>,
        profile_id: &str,
        server_id: &str,
        server_name: &str,
        tool_name: &str,
    ) -> String {
        let unique_name = generate_unique_name(NamingKind::Tool, server_name, tool_name);
        sqlx::query(
            r#"
            INSERT INTO server_tools (id, server_id, server_name, tool_name, unique_name, description)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(crate::generate_id!("stl"))
        .bind(server_id)
        .bind(server_name)
        .bind(tool_name)
        .bind(&unique_name)
        .bind(Option::<String>::None)
        .execute(&db.pool)
        .await
        .expect("insert server tool");
        profile::add_tool_to_profile(&db.pool, profile_id, server_id, tool_name, true)
            .await
            .expect("add tool to profile");
        unique_name
    }

    async fn seed_prompt(
        db: &Arc<Database>,
        profile_id: &str,
        server_id: &str,
        server_name: &str,
        prompt_name: &str,
    ) -> String {
        let unique_name = generate_unique_name(NamingKind::Prompt, server_name, prompt_name);
        sqlx::query(
            r#"
            INSERT INTO server_prompts (id, server_id, server_name, prompt_name, unique_name, description)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(crate::generate_id!("sprm"))
        .bind(server_id)
        .bind(server_name)
        .bind(prompt_name)
        .bind(&unique_name)
        .bind(Option::<String>::None)
        .execute(&db.pool)
        .await
        .expect("insert server prompt");

        profile::add_prompt_to_profile(&db.pool, profile_id, server_id, prompt_name, true)
            .await
            .expect("add prompt to profile");
        unique_name
    }

    async fn seed_resource(
        db: &Arc<Database>,
        profile_id: &str,
        server_id: &str,
        server_name: &str,
        resource_uri: &str,
    ) -> String {
        let unique_uri = generate_unique_name(NamingKind::Resource, server_name, resource_uri);
        sqlx::query(
            r#"
            INSERT INTO server_resources (id, server_id, server_name, resource_uri, unique_uri, name, description, mime_type)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(crate::generate_id!("sres"))
        .bind(server_id)
        .bind(server_name)
        .bind(resource_uri)
        .bind(&unique_uri)
        .bind(Option::<String>::None)
        .bind(Option::<String>::None)
        .bind(Option::<String>::None)
        .execute(&db.pool)
        .await
        .expect("insert server resource");

        profile::add_resource_to_profile(&db.pool, profile_id, server_id, resource_uri, true)
            .await
            .expect("add resource to profile");
        unique_uri
    }

    async fn seed_resource_template(
        db: &Arc<Database>,
        profile_id: &str,
        server_id: &str,
        server_name: &str,
        uri_template: &str,
    ) -> String {
        let unique_name = generate_unique_name(NamingKind::ResourceTemplate, server_name, uri_template);
        sqlx::query(
            r#"
            INSERT INTO server_resource_templates (id, server_id, server_name, uri_template, unique_name, name, description)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(crate::generate_id!("srt"))
        .bind(server_id)
        .bind(server_name)
        .bind(uri_template)
        .bind(&unique_name)
        .bind(uri_template)
        .bind(Option::<String>::None)
        .execute(&db.pool)
        .await
        .expect("insert server resource template");

        profile::add_resource_template_to_profile(&db.pool, profile_id, server_id, uri_template, true)
            .await
            .expect("add resource template to profile");
        unique_name
    }

    async fn insert_client_config(
        db: &Arc<Database>,
        identifier: &str,
        capability_source: CapabilitySource,
        selected_profile_ids: Vec<String>,
        custom_profile_id: Option<String>,
    ) {
        let selected_profile_ids_json = if selected_profile_ids.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&selected_profile_ids).expect("selected profile ids json"))
        };

        sqlx::query(
            r#"
            INSERT INTO client (id, name, identifier, capability_source, selected_profile_ids, custom_profile_id)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(crate::generate_id!("clnt"))
        .bind(identifier)
        .bind(identifier)
        .bind(capability_source.as_str())
        .bind(selected_profile_ids_json)
        .bind(custom_profile_id)
        .execute(&db.pool)
        .await
        .expect("insert client config");
    }

    #[tokio::test]
    #[serial]
    async fn resolve_snapshot_uses_active_profiles_for_activated_mode() {
        let (_temp_dir, db, service) = create_visibility_service().await;

        let active_profile_id = insert_profile(&db, "active", ProfileType::Shared, true).await;
        let inactive_profile_id = insert_profile(&db, "inactive", ProfileType::Shared, false).await;
        let active_server_id = insert_server(&db, "active-server", "tools,prompts,resources").await;
        let inactive_server_id = insert_server(&db, "inactive-server", "tools,prompts,resources").await;

        profile::add_server_to_profile(&db.pool, &active_profile_id, &active_server_id, true)
            .await
            .expect("add active server");
        profile::add_server_to_profile(&db.pool, &inactive_profile_id, &inactive_server_id, true)
            .await
            .expect("add inactive server");

        let active_tool = seed_tool(
            &db,
            &active_profile_id,
            &active_server_id,
            "active-server",
            "tool_alpha",
        )
        .await;
        let _inactive_tool = seed_tool(
            &db,
            &inactive_profile_id,
            &inactive_server_id,
            "inactive-server",
            "tool_beta",
        )
        .await;

        insert_client_config(&db, "client-a", CapabilitySource::Activated, Vec::new(), None).await;

        let snapshot = service
            .resolve_snapshot("client-a", None)
            .await
            .expect("resolve snapshot");

        assert_eq!(snapshot.profile_ids, vec![active_profile_id]);
        assert_eq!(snapshot.server_ids, vec![active_server_id]);
        assert!(snapshot.allowed_tools.contains(&active_tool));
        assert_eq!(snapshot.allowed_tools.len(), 1);
        assert!(!snapshot.surface_fingerprint.is_empty());
    }

    #[tokio::test]
    #[serial]
    async fn resolve_snapshot_uses_selected_profiles_for_profiles_mode() {
        let (_temp_dir, db, service) = create_visibility_service().await;

        let active_profile_id = insert_profile(&db, "active", ProfileType::Shared, true).await;
        let selected_profile_id = insert_profile(&db, "selected", ProfileType::Shared, false).await;
        let active_server_id = insert_server(&db, "active-server", "tools").await;
        let selected_server_id = insert_server(&db, "selected-server", "tools").await;

        profile::add_server_to_profile(&db.pool, &active_profile_id, &active_server_id, true)
            .await
            .expect("add active server");
        profile::add_server_to_profile(&db.pool, &selected_profile_id, &selected_server_id, true)
            .await
            .expect("add selected server");

        let selected_tool = seed_tool(
            &db,
            &selected_profile_id,
            &selected_server_id,
            "selected-server",
            "tool_selected",
        )
        .await;

        insert_client_config(
            &db,
            "client-b",
            CapabilitySource::Profiles,
            vec![selected_profile_id.clone()],
            None,
        )
        .await;

        let snapshot = service
            .resolve_snapshot("client-b", None)
            .await
            .expect("resolve snapshot");

        assert_eq!(snapshot.profile_ids, vec![selected_profile_id]);
        assert_eq!(snapshot.server_ids, vec![selected_server_id]);
        assert!(snapshot.allowed_tools.contains(&selected_tool));
        assert_eq!(snapshot.allowed_tools.len(), 1);
    }

    #[tokio::test]
    #[serial]
    async fn resolve_snapshot_uses_custom_profile_for_custom_mode() {
        let (_temp_dir, db, service) = create_visibility_service().await;

        let custom_profile_id = insert_profile(&db, "custom", ProfileType::HostApp, false).await;
        let custom_server_id = insert_server(&db, "custom-server", "prompts").await;

        profile::add_server_to_profile(&db.pool, &custom_profile_id, &custom_server_id, true)
            .await
            .expect("add custom server");

        let custom_prompt = seed_prompt(
            &db,
            &custom_profile_id,
            &custom_server_id,
            "custom-server",
            "prompt_custom",
        )
        .await;

        insert_client_config(
            &db,
            "client-c",
            CapabilitySource::Custom,
            Vec::new(),
            Some(custom_profile_id.clone()),
        )
        .await;

        let snapshot = service
            .resolve_snapshot("client-c", None)
            .await
            .expect("resolve snapshot");

        assert_eq!(snapshot.profile_ids, vec![custom_profile_id]);
        assert_eq!(snapshot.server_ids, vec![custom_server_id]);
        assert!(snapshot.allowed_prompts.contains(&custom_prompt));
        assert_eq!(snapshot.allowed_prompts.len(), 1);
    }

    #[tokio::test]
    #[serial]
    async fn direct_authorization_uses_same_snapshot_rules_as_list_filtering() {
        let (_temp_dir, db, service) = create_visibility_service().await;

        let profile_id = insert_profile(&db, "selected", ProfileType::Shared, false).await;
        let allowed_server_id = insert_server(&db, "alpha-server", "tools,prompts,resources").await;
        let denied_server_id = insert_server(&db, "beta-server", "tools,prompts,resources").await;

        profile::add_server_to_profile(&db.pool, &profile_id, &allowed_server_id, true)
            .await
            .expect("add allowed server");

        let allowed_tool = seed_tool(&db, &profile_id, &allowed_server_id, "alpha-server", "tool_alpha").await;
        let allowed_prompt = seed_prompt(&db, &profile_id, &allowed_server_id, "alpha-server", "prompt_alpha").await;
        let _allowed_resource = seed_resource(
            &db,
            &profile_id,
            &allowed_server_id,
            "alpha-server",
            "file://workspace/explicit.txt",
        )
        .await;
        let _allowed_template = seed_resource_template(
            &db,
            &profile_id,
            &allowed_server_id,
            "alpha-server",
            "file://workspace/{path}",
        )
        .await;

        let denied_tool = generate_unique_name(NamingKind::Tool, "beta-server", "tool_beta");
        sqlx::query(
            r#"
            INSERT INTO server_tools (id, server_id, server_name, tool_name, unique_name, description)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(crate::generate_id!("stl"))
        .bind(&denied_server_id)
        .bind("beta-server")
        .bind("tool_beta")
        .bind(&denied_tool)
        .bind(Option::<String>::None)
        .execute(&db.pool)
        .await
        .expect("insert denied tool");
        let denied_prompt = generate_unique_name(NamingKind::Prompt, "beta-server", "prompt_beta");
        let denied_resource = generate_unique_name(NamingKind::Resource, "beta-server", "file://other/file.txt");

        sqlx::query(
            r#"
            INSERT INTO server_prompts (id, server_id, server_name, prompt_name, unique_name, description)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(crate::generate_id!("sprm"))
        .bind(&denied_server_id)
        .bind("beta-server")
        .bind("prompt_beta")
        .bind(&denied_prompt)
        .bind(Option::<String>::None)
        .execute(&db.pool)
        .await
        .expect("insert denied prompt");

        sqlx::query(
            r#"
            INSERT INTO server_resources (id, server_id, server_name, resource_uri, unique_uri, name, description, mime_type)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(crate::generate_id!("sres"))
        .bind(&denied_server_id)
        .bind("beta-server")
        .bind("file://other/file.txt")
        .bind(&denied_resource)
        .bind(Option::<String>::None)
        .bind(Option::<String>::None)
        .bind(Option::<String>::None)
        .execute(&db.pool)
        .await
        .expect("insert denied resource");

        insert_client_config(&db, "client-d", CapabilitySource::Profiles, vec![profile_id], None).await;

        let snapshot = service
            .resolve_snapshot("client-d", None)
            .await
            .expect("resolve snapshot");

        assert!(
            service
                .assert_tool_allowed_with_snapshot(&snapshot, &allowed_tool)
                .await
                .is_ok()
        );
        assert!(
            service
                .assert_tool_allowed_with_snapshot(&snapshot, &denied_tool)
                .await
                .is_err()
        );
        assert!(
            service
                .assert_prompt_allowed_with_snapshot(&snapshot, &allowed_prompt)
                .await
                .is_ok()
        );
        assert!(
            service
                .assert_prompt_allowed_with_snapshot(&snapshot, &denied_prompt)
                .await
                .is_err()
        );

        let dynamic_allowed = generate_unique_name(NamingKind::Resource, "alpha-server", "file://workspace/main.rs");
        assert!(
            service
                .assert_resource_allowed_with_snapshot(&snapshot, &dynamic_allowed)
                .await
                .is_ok()
        );
        assert!(
            service
                .assert_resource_allowed_with_snapshot(&snapshot, &denied_resource)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    #[serial]
    async fn unify_snapshot_uses_globally_enabled_servers_without_profile_semantics() {
        let (_temp_dir, db, service) = create_visibility_service().await;

        let active_profile_id = insert_profile(&db, "active", ProfileType::Shared, true).await;
        let selected_profile_id = insert_profile(&db, "selected", ProfileType::Shared, false).await;
        let disabled_server_id = insert_server(&db, "disabled-server", "tools,prompts,resources").await;
        let enabled_server_id = insert_server(&db, "enabled-server", "tools,prompts,resources").await;

        profile::add_server_to_profile(&db.pool, &active_profile_id, &disabled_server_id, true)
            .await
            .expect("add active server");
        profile::add_server_to_profile(&db.pool, &selected_profile_id, &enabled_server_id, true)
            .await
            .expect("add selected server");

        let disabled_tool = seed_tool(
            &db,
            &active_profile_id,
            &disabled_server_id,
            "disabled-server",
            "tool_disabled",
        )
        .await;
        let enabled_tool = seed_tool(
            &db,
            &selected_profile_id,
            &enabled_server_id,
            "enabled-server",
            "tool_enabled",
        )
        .await;

        sqlx::query(
            r#"
            UPDATE server_config
            SET enabled = 0
            WHERE id = ?
            "#,
        )
        .bind(&disabled_server_id)
        .execute(&db.pool)
        .await
        .expect("disable active-profile server globally");

        insert_client_config(
            &db,
            "client-unify",
            CapabilitySource::Profiles,
            vec![selected_profile_id.clone()],
            None,
        )
        .await;

        let client = ClientContext {
            client_id: "client-unify".to_string(),
            session_id: Some("unify-session".to_string()),
            profile_id: None,
            config_mode: Some("unify".to_string()),
            unify_workspace: None,
            surface_fingerprint: None,
            transport: crate::core::proxy::server::ClientTransport::Other,
            source: crate::core::proxy::server::ClientIdentitySource::SessionBinding,
            observed_client_info: None,
        };

        let snapshot = service
            .resolve_snapshot_for_client(&client)
            .await
            .expect("resolve unify snapshot");

        assert!(snapshot.profile_ids.is_empty());
        assert_eq!(snapshot.server_ids, vec![enabled_server_id]);
        assert!(snapshot.allowed_tools.contains(&enabled_tool));
        assert!(!snapshot.allowed_tools.contains(&disabled_tool));
    }
}
