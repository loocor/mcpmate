//! Profile visibility service
//!
//! Centralizes profile-driven capability visibility (tools/resources/prompts)
//! for MCP list responses. Uses ProfileService cache when available and falls
//! back to lightweight SQL queries. Returns allowlists in unique-name space
//! to match proxy aggregation output.

use std::collections::HashSet;
use std::sync::Arc;

use anyhow::Result;

use crate::config::database::Database;
use crate::core::profile::ProfileService;

pub struct ProfileVisibilityService {
    db: Option<Arc<Database>>,                    // fallback when merge cache unavailable
    profile_service: Option<Arc<ProfileService>>, // preferred source (merged caches)
}

impl ProfileVisibilityService {
    pub fn new(
        db: Option<Arc<Database>>,
        profile_service: Option<Arc<ProfileService>>,
    ) -> Self {
        Self { db, profile_service }
    }

    /// Return Some(allowed) if any profile rows exist for tools; otherwise None (no filtering).
    async fn allowed_tools_set(&self) -> Result<Option<HashSet<String>>> {
        // Prefer merged cache
        if let Some(ps) = &self.profile_service {
            if let Some(set) = ps.allowed_tool_unique_set().await {
                return Ok(Some(set));
            }
        }
        let Some(db) = &self.db else { return Ok(None) };

        // Determine if any profile_tool rows exist under active profiles
        let any_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(1)
            FROM profile_tool cst
            JOIN profile cs ON cst.profile_id = cs.id
            WHERE cs.is_active = 1
            "#,
        )
        .fetch_one(&db.pool)
        .await
        .unwrap_or(0);
        if any_count == 0 {
            return Ok(None);
        }

        // Build allowlist using unique_name
        let sql = crate::config::profile::tool::build_enabled_tools_query(None);
        let rows: Vec<(String, String, String, String)> =
            sqlx::query_as(&sql).fetch_all(&db.pool).await.unwrap_or_default();
        let mut set = HashSet::new();
        for (_unique_name, _server_name, _tool_name, _server_id) in rows {
            // build_enabled_tools_query selects: unique_name, server_name, tool_name, server_id
            set.insert(_unique_name);
        }
        Ok(Some(set))
    }

    /// Return Some(allowed) if any profile_resource rows exist; otherwise None (no filtering).
    async fn allowed_resources_set(&self) -> Result<Option<HashSet<String>>> {
        // Prefer merged cache
        if let Some(ps) = &self.profile_service {
            if let Some(set) = ps.allowed_resource_unique_set().await {
                return Ok(Some(set));
            }
        }
        let Some(db) = &self.db else { return Ok(None) };
        let any_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(1)
            FROM profile_resource csr
            JOIN profile cs ON csr.profile_id = cs.id
            WHERE cs.is_active = 1
            "#,
        )
        .fetch_one(&db.pool)
        .await
        .unwrap_or(0);
        if any_count == 0 {
            return Ok(None);
        }

        let sql = crate::config::profile::resource::build_enabled_resources_query(None);
        // Selected columns: server_id, server_name(original), resource_uri
        let rows: Vec<(String, String, String)> = sqlx::query_as(&sql).fetch_all(&db.pool).await.unwrap_or_default();
        let mut set = HashSet::new();
        for (_server_id, server_name_original, upstream_uri) in rows {
            let unique = crate::core::capability::naming::generate_unique_name(
                crate::core::capability::naming::NamingKind::Resource,
                &server_name_original,
                &upstream_uri,
            );
            set.insert(unique);
        }
        Ok(Some(set))
    }

    /// Return Some(allowed) for resource templates (unique template names) if any rows exist; otherwise None
    async fn allowed_resource_templates_unique_set(&self) -> Result<Option<HashSet<String>>> {
        let Some(db) = &self.db else { return Ok(None) };
        let any_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(1)
            FROM profile_resource_template prt
            JOIN profile cs ON prt.profile_id = cs.id
            WHERE cs.is_active = 1
            "#,
        )
        .fetch_one(&db.pool)
        .await
        .unwrap_or(0);
        if any_count == 0 {
            return Ok(None);
        }

        let sql = crate::config::profile::resource_template::build_enabled_resource_templates_query(None);
        let rows: Vec<(String, String, String)> = sqlx::query_as(&sql).fetch_all(&db.pool).await.unwrap_or_default();
        let mut set = HashSet::new();
        for (_server_id, server_name_original, uri_template) in rows {
            // Build template unique-name in template namespace: server_norm_/uri_template
            let unique = crate::core::capability::naming::generate_unique_name(
                crate::core::capability::naming::NamingKind::ResourceTemplate,
                &server_name_original,
                &uri_template,
            );
            set.insert(unique);
        }
        Ok(Some(set))
    }

    /// Return Some(allowed) resource unique-name prefixes derived from enabled templates: server_norm:/prefix
    async fn allowed_resource_prefixes_from_templates(&self) -> Result<Option<HashSet<String>>> {
        let Some(db) = &self.db else { return Ok(None) };
        let any_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(1)
            FROM profile_resource_template prt
            JOIN profile cs ON prt.profile_id = cs.id
            WHERE cs.is_active = 1
            "#,
        )
        .fetch_one(&db.pool)
        .await
        .unwrap_or(0);
        if any_count == 0 {
            return Ok(None);
        }

        let sql = crate::config::profile::resource_template::build_enabled_resource_templates_query(None);
        let rows: Vec<(String, String, String)> = sqlx::query_as(&sql).fetch_all(&db.pool).await.unwrap_or_default();
        let mut set = HashSet::new();
        for (_server_id, server_name_original, uri_template) in rows {
            let prefix = crate::config::profile::resource_template::template_prefix(&uri_template).to_string();
            // Build resource unique-name prefix: server_norm:/prefix
            let unique_prefix = crate::core::capability::naming::generate_unique_name(
                crate::core::capability::naming::NamingKind::Resource,
                &server_name_original,
                &prefix,
            );
            set.insert(unique_prefix);
        }
        Ok(Some(set))
    }

    /// Return Some(allowed) if any profile_prompt rows exist; otherwise None (no filtering).
    async fn allowed_prompts_set(&self) -> Result<Option<HashSet<String>>> {
        // Prefer merged cache
        if let Some(ps) = &self.profile_service {
            if let Some(set) = ps.allowed_prompt_unique_set().await {
                return Ok(Some(set));
            }
        }
        let Some(db) = &self.db else { return Ok(None) };
        let any_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(1)
            FROM profile_prompt csp
            JOIN profile cs ON csp.profile_id = cs.id
            WHERE cs.is_active = 1
            "#,
        )
        .fetch_one(&db.pool)
        .await
        .unwrap_or(0);
        if any_count == 0 {
            return Ok(None);
        }

        let sql = crate::config::profile::prompt::build_enabled_prompts_query(None);
        // Selected columns: server_id, server_name(original), prompt_name
        let rows: Vec<(String, String, String)> = sqlx::query_as(&sql).fetch_all(&db.pool).await.unwrap_or_default();
        let mut set = HashSet::new();
        for (_server_id, server_name_original, upstream_name) in rows {
            let unique = crate::core::capability::naming::generate_unique_name(
                crate::core::capability::naming::NamingKind::Prompt,
                &server_name_original,
                &upstream_name,
            );
            set.insert(unique);
        }
        Ok(Some(set))
    }

    pub async fn filter_tools(
        &self,
        mut tools: Vec<rmcp::model::Tool>,
    ) -> Vec<rmcp::model::Tool> {
        match self.allowed_tools_set().await {
            Ok(Some(allowed)) => {
                let before = tools.len();
                tools.retain(|t| allowed.contains(t.name.as_ref()));
                let after = tools.len();
                tracing::debug!(
                    filtered = (before as i64 - after as i64),
                    kept = after as i64,
                    "ProfileVisibility: tools filtered"
                );
                tools
            }
            _ => tools,
        }
    }

    pub async fn filter_resources(
        &self,
        mut resources: Vec<rmcp::model::Resource>,
    ) -> Vec<rmcp::model::Resource> {
        let allowed_explicit = self.allowed_resources_set().await.unwrap_or(None);
        let allowed_prefixes = self.allowed_resource_prefixes_from_templates().await.unwrap_or(None);

        // If neither explicit nor templates gating exists, do not filter
        if allowed_explicit.is_none() && allowed_prefixes.is_none() {
            return resources;
        }

        // allowed_prefixes 已是 server_norm:/prefix 形式

        let before = resources.len();
        resources.retain(|r| {
            let u = r.raw.uri.as_str();
            // Explicit allow list
            if let Some(ref aset) = allowed_explicit {
                if aset.contains(u) {
                    return true;
                }
            }
            // Template-derived allow prefixes
            if let Some(ref prefixes) = allowed_prefixes {
                if prefixes.iter().any(|p| u.starts_with(p)) {
                    return true;
                }
            }
            // If explicit gating exists but none matched, drop; else keep
            allowed_explicit.is_none()
        });

        let after = resources.len();
        tracing::debug!(
            filtered = (before as i64 - after as i64),
            kept = after as i64,
            "ProfileVisibility: resources filtered (with templates)"
        );
        resources
    }

    pub async fn filter_prompts(
        &self,
        mut prompts: Vec<rmcp::model::Prompt>,
    ) -> Vec<rmcp::model::Prompt> {
        match self.allowed_prompts_set().await {
            Ok(Some(allowed)) => {
                let before = prompts.len();
                prompts.retain(|p| allowed.contains(p.name.as_str()));
                let after = prompts.len();
                tracing::debug!(
                    filtered = (before as i64 - after as i64),
                    kept = after as i64,
                    "ProfileVisibility: prompts filtered"
                );
                prompts
            }
            _ => prompts,
        }
    }

    pub async fn filter_resource_templates(
        &self,
        mut templates: Vec<rmcp::model::ResourceTemplate>,
    ) -> Vec<rmcp::model::ResourceTemplate> {
        match self.allowed_resource_templates_unique_set().await {
            Ok(Some(allowed)) => {
                let before = templates.len();
                templates.retain(|t| allowed.contains(t.raw.name.as_str()));
                let after = templates.len();
                tracing::debug!(
                    filtered = (before as i64 - after as i64),
                    kept = after as i64,
                    "ProfileVisibility: resource templates filtered"
                );
                templates
            }
            _ => templates,
        }
    }
}
