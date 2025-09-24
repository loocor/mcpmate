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
        let rows: Vec<(String, String)> = sqlx::query_as(&sql).fetch_all(&db.pool).await.unwrap_or_default();
        let mut set = HashSet::new();
        for (server_name, upstream_uri) in rows {
            let unique = crate::core::capability::naming::generate_unique_name(
                crate::core::capability::naming::NamingKind::Resource,
                &server_name,
                &upstream_uri,
            );
            set.insert(unique);
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
        let rows: Vec<(String, String)> = sqlx::query_as(&sql).fetch_all(&db.pool).await.unwrap_or_default();
        let mut set = HashSet::new();
        for (server_name, upstream_name) in rows {
            let unique = crate::core::capability::naming::generate_unique_name(
                crate::core::capability::naming::NamingKind::Prompt,
                &server_name,
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
        match self.allowed_resources_set().await {
            Ok(Some(allowed)) => {
                let before = resources.len();
                resources.retain(|r| allowed.contains(r.raw.uri.as_str()));
                let after = resources.len();
                tracing::debug!(
                    filtered = (before as i64 - after as i64),
                    kept = after as i64,
                    "ProfileVisibility: resources filtered"
                );
                resources
            }
            _ => resources,
        }
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
}
