use std::collections::{HashMap, HashSet};

use crate::core::cache::{CacheQuery, CacheScope, FreshnessLevel, RedbCacheManager};

#[derive(Default)]
pub struct CapabilityDescriptionIndex {
    tools: HashMap<String, String>,
    resources: HashMap<String, String>,
    prompts: HashMap<String, String>,
    templates: HashMap<String, String>,
}

impl CapabilityDescriptionIndex {
    pub fn tool(
        &self,
        server_id: &str,
        value: &str,
    ) -> Option<String> {
        self.tools.get(&description_key(server_id, value)).cloned()
    }

    pub fn resource(
        &self,
        server_id: &str,
        value: &str,
    ) -> Option<String> {
        self.resources.get(&description_key(server_id, value)).cloned()
    }

    pub fn prompt(
        &self,
        server_id: &str,
        value: &str,
    ) -> Option<String> {
        self.prompts.get(&description_key(server_id, value)).cloned()
    }

    pub fn template(
        &self,
        server_id: &str,
        value: &str,
    ) -> Option<String> {
        self.templates.get(&description_key(server_id, value)).cloned()
    }
}

fn description_key(
    server_id: &str,
    value: &str,
) -> String {
    format!("{}::{}", server_id, value.trim().to_lowercase())
}

fn insert_description(
    descriptions: &mut HashMap<String, String>,
    server_id: &str,
    value: &str,
    description: Option<&str>,
) {
    let Some(description) = description else {
        return;
    };
    let description = description.trim();
    if value.trim().is_empty() || description.is_empty() {
        return;
    }
    descriptions.insert(description_key(server_id, value), description.to_string());
}

pub async fn load_cached_capability_descriptions(
    redb_cache: &RedbCacheManager,
    server_ids: impl IntoIterator<Item = String>,
) -> CapabilityDescriptionIndex {
    let mut index = CapabilityDescriptionIndex::default();
    let server_ids: HashSet<String> = server_ids.into_iter().collect();

    for server_id in server_ids {
        let query = CacheQuery {
            server_id: server_id.clone(),
            freshness_level: FreshnessLevel::Cached,
            include_disabled: true,
            scope: CacheScope::shared_raw(),
        };

        let cached = match redb_cache.get_server_data(&query).await {
            Ok(result) => result.data,
            Err(error) => {
                tracing::debug!(
                    server_id = %server_id,
                    error = %error,
                    "Failed to read cached capability descriptions"
                );
                None
            }
        };

        let Some(cached) = cached else {
            continue;
        };

        for tool in cached.tools {
            insert_description(&mut index.tools, &server_id, &tool.name, tool.description.as_deref());
            if let Some(unique_name) = tool.unique_name.as_deref() {
                insert_description(&mut index.tools, &server_id, unique_name, tool.description.as_deref());
            }
        }

        for resource in cached.resources {
            insert_description(
                &mut index.resources,
                &server_id,
                &resource.uri,
                resource.description.as_deref(),
            );
            if let Some(name) = resource.name.as_deref() {
                insert_description(&mut index.resources, &server_id, name, resource.description.as_deref());
            }
        }

        for prompt in cached.prompts {
            insert_description(
                &mut index.prompts,
                &server_id,
                &prompt.name,
                prompt.description.as_deref(),
            );
        }

        for template in cached.resource_templates {
            insert_description(
                &mut index.templates,
                &server_id,
                &template.uri_template,
                template.description.as_deref(),
            );
            if let Some(name) = template.name.as_deref() {
                insert_description(&mut index.templates, &server_id, name, template.description.as_deref());
            }
        }
    }

    index
}
