//! Cache CRUD operations implementation

use anyhow::Result;
use redb::{Database, ReadableMultimapTable, ReadableTable};
use tracing::debug;

use super::{schema::*, types::*};

/// CRUD operations for cache data
pub struct CacheOperations<'a> {
    db: &'a Database,
}

impl<'a> CacheOperations<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Generate cache key from server_id and instance_type
    ///
    /// Format: "{server_id}#{instance_type_key}"
    /// Examples:
    /// - "srv_123#production"
    /// - "srv_123#validation_api"
    /// - "srv_123#exploration_session1"
    fn generate_cache_key(
        &self,
        server_id: &str,
        instance_type: &InstanceType,
    ) -> String {
        let instance_key = match instance_type {
            InstanceType::Production => "production".to_string(),
        };
        format!("{}#{}", server_id, instance_key)
    }

    /// Store server data in cache
    pub fn store_server_data(
        &self,
        server_data: &CachedServerData,
    ) -> Result<(), CacheError> {
        let write_txn = self.db.begin_write()?;
        let cache_key = self.generate_cache_key(&server_data.server_id, &server_data.instance_type());

        {
            // Store main server data using composite key (server_id + instance_type)
            let mut servers_table = write_txn.open_table(SERVERS_TABLE)?;
            let serialized = bincode::serialize(server_data)?;
            servers_table.insert(cache_key.as_str(), serialized.as_slice())?;

            // Store individual tools with indexing
            let mut tools_table = write_txn.open_table(TOOLS_TABLE)?;
            let mut tools_index = write_txn.open_multimap_table(SERVER_TOOLS_INDEX)?;

            for tool in &server_data.tools {
                let key = (server_data.server_id.as_str(), tool.name.as_str());
                let serialized = bincode::serialize(tool)?;
                tools_table.insert(key, serialized.as_slice())?;
                tools_index.insert(&*server_data.server_id, &*tool.name)?;
            }

            // Store individual resources with indexing
            let mut resources_table = write_txn.open_table(RESOURCES_TABLE)?;
            let mut resources_index = write_txn.open_multimap_table(SERVER_RESOURCES_INDEX)?;

            for resource in &server_data.resources {
                let key = (server_data.server_id.as_str(), resource.uri.as_str());
                let serialized = bincode::serialize(resource)?;
                resources_table.insert(key, serialized.as_slice())?;
                resources_index.insert(&*server_data.server_id, &*resource.uri)?;
            }

            // Store individual prompts with indexing
            let mut prompts_table = write_txn.open_table(PROMPTS_TABLE)?;
            let mut prompts_index = write_txn.open_multimap_table(SERVER_PROMPTS_INDEX)?;

            for prompt in &server_data.prompts {
                let key = (server_data.server_id.as_str(), prompt.name.as_str());
                let serialized = bincode::serialize(prompt)?;
                prompts_table.insert(key, serialized.as_slice())?;
                prompts_index.insert(&*server_data.server_id, &*prompt.name)?;
            }

            // Store resource templates with indexing
            let mut templates_table = write_txn.open_table(RESOURCE_TEMPLATES_TABLE)?;
            let mut templates_index = write_txn.open_multimap_table(SERVER_RESOURCE_TEMPLATES_INDEX)?;

            for template in &server_data.resource_templates {
                let key = (server_data.server_id.as_str(), template.uri_template.as_str());
                let serialized = bincode::serialize(template)?;
                templates_table.insert(key, serialized.as_slice())?;
                templates_index.insert(&*server_data.server_id, &*template.uri_template)?;
            }
        }

        write_txn.commit()?;
        tracing::info!("[CACHE][STORE] key={} server_id={}", cache_key, server_data.server_id);
        Ok(())
    }

    /// Retrieve server data from cache
    pub fn get_server_data(
        &self,
        query: &CacheQuery,
    ) -> Result<Option<CachedServerData>, CacheError> {
        let read_txn = self.db.begin_read()?;
        let servers_table = read_txn.open_table(SERVERS_TABLE)?;

        // Generate composite key for lookup
        let cache_key = self.generate_cache_key(&query.server_id, &query.instance_type());
        tracing::info!("[CACHE][LOOKUP] key={}", cache_key);

        if let Some(data) = servers_table.get(cache_key.as_str())? {
            let server_data: CachedServerData = bincode::deserialize(data.value())?;
            tracing::info!("[CACHE][HIT] key={}", cache_key);
            Ok(Some(server_data))
        } else {
            tracing::info!("[CACHE][MISS] key={}", cache_key);
            Ok(None)
        }
    }

    /// Get server tools
    pub fn get_server_tools(
        &self,
        server_id: &str,
        include_disabled: bool,
    ) -> Result<Vec<CachedToolInfo>, CacheError> {
        let read_txn = self.db.begin_read()?;
        let tools_table = read_txn.open_table(TOOLS_TABLE)?;
        let tools_index = read_txn.open_multimap_table(SERVER_TOOLS_INDEX)?;

        let mut tools = Vec::new();

        let tool_names = tools_index.get(server_id)?;
        for tool_name_result in tool_names {
            let tool_name = tool_name_result?;
            let key = (server_id, tool_name.value());

            if let Some(data) = tools_table.get(key)? {
                let tool: CachedToolInfo = bincode::deserialize(data.value())?;

                if include_disabled || tool.enabled {
                    tools.push(tool);
                }
            }
        }

        Ok(tools)
    }

    /// Get server resources
    pub fn get_server_resources(
        &self,
        server_id: &str,
        include_disabled: bool,
    ) -> Result<Vec<CachedResourceInfo>, CacheError> {
        let read_txn = self.db.begin_read()?;
        let resources_table = read_txn.open_table(RESOURCES_TABLE)?;
        let resources_index = read_txn.open_multimap_table(SERVER_RESOURCES_INDEX)?;

        let mut resources = Vec::new();

        let resource_uris = resources_index.get(server_id)?;
        for resource_uri_result in resource_uris {
            let resource_uri = resource_uri_result?;
            let key = (server_id, resource_uri.value());

            if let Some(data) = resources_table.get(key)? {
                let resource: CachedResourceInfo = bincode::deserialize(data.value())?;

                if include_disabled || resource.enabled {
                    resources.push(resource);
                }
            }
        }

        Ok(resources)
    }

    /// Get server prompts
    pub fn get_server_prompts(
        &self,
        server_id: &str,
        include_disabled: bool,
    ) -> Result<Vec<CachedPromptInfo>, CacheError> {
        let read_txn = self.db.begin_read()?;
        let prompts_table = read_txn.open_table(PROMPTS_TABLE)?;
        let prompts_index = read_txn.open_multimap_table(SERVER_PROMPTS_INDEX)?;

        let mut prompts = Vec::new();

        let prompt_names = prompts_index.get(server_id)?;
        for prompt_name_result in prompt_names {
            let prompt_name = prompt_name_result?;
            let key = (server_id, prompt_name.value());

            if let Some(data) = prompts_table.get(key)? {
                let prompt: CachedPromptInfo = bincode::deserialize(data.value())?;

                if include_disabled || prompt.enabled {
                    prompts.push(prompt);
                }
            }
        }

        Ok(prompts)
    }

    /// Get server resource templates
    pub fn get_server_resource_templates(
        &self,
        server_id: &str,
        include_disabled: bool,
    ) -> Result<Vec<CachedResourceTemplateInfo>, CacheError> {
        let read_txn = self.db.begin_read()?;
        let templates_table = read_txn.open_table(RESOURCE_TEMPLATES_TABLE)?;
        let templates_index = read_txn.open_multimap_table(SERVER_RESOURCE_TEMPLATES_INDEX)?;

        let mut templates = Vec::new();

        let template_uris = templates_index.get(server_id)?;
        for template_uri_result in template_uris {
            let template_uri = template_uri_result?;
            let key = (server_id, template_uri.value());

            if let Some(data) = templates_table.get(key)? {
                let template: CachedResourceTemplateInfo = bincode::deserialize(data.value())?;

                if include_disabled || template.enabled {
                    templates.push(template);
                }
            }
        }

        Ok(templates)
    }

    /// Remove server data from cache
    pub fn remove_server_data(
        &self,
        server_id: &str,
    ) -> Result<(), CacheError> {
        let write_txn = self.db.begin_write()?;

        {
            // Remove main server data (keys are composite: "{server_id}#{instance}")
            let mut servers_table = write_txn.open_table(SERVERS_TABLE)?;
            let keys: Vec<String> = servers_table
                .iter()?
                .map(|item| {
                    let (key, _) = item?;
                    Ok(key.value().to_string())
                })
                .collect::<Result<Vec<_>, CacheError>>()?;
            for k in keys {
                if k == server_id || k.starts_with(&format!("{}#", server_id)) {
                    servers_table.remove(&*k)?;
                }
            }

            // Remove tools
            let mut tools_table = write_txn.open_table(TOOLS_TABLE)?;
            let mut tools_index = write_txn.open_multimap_table(SERVER_TOOLS_INDEX)?;

            let tool_names = tools_index.get(server_id)?;
            for tool_name_result in tool_names {
                let tool_name = tool_name_result?;
                let key = (server_id, tool_name.value());
                tools_table.remove(key)?;
            }
            tools_index.remove_all(server_id)?;

            // Remove resources
            let mut resources_table = write_txn.open_table(RESOURCES_TABLE)?;
            let mut resources_index = write_txn.open_multimap_table(SERVER_RESOURCES_INDEX)?;

            let resource_uris = resources_index.get(server_id)?;
            for resource_uri_result in resource_uris {
                let resource_uri = resource_uri_result?;
                let key = (server_id, resource_uri.value());
                resources_table.remove(key)?;
            }
            resources_index.remove_all(server_id)?;

            // Remove prompts
            let mut prompts_table = write_txn.open_table(PROMPTS_TABLE)?;
            let mut prompts_index = write_txn.open_multimap_table(SERVER_PROMPTS_INDEX)?;

            let prompt_names = prompts_index.get(server_id)?;
            for prompt_name_result in prompt_names {
                let prompt_name = prompt_name_result?;
                let key = (server_id, prompt_name.value());
                prompts_table.remove(key)?;
            }
            prompts_index.remove_all(server_id)?;

            // Remove resource templates
            let mut templates_table = write_txn.open_table(RESOURCE_TEMPLATES_TABLE)?;
            let mut templates_index = write_txn.open_multimap_table(SERVER_RESOURCE_TEMPLATES_INDEX)?;

            let template_uris = templates_index.get(server_id)?;
            for template_uri_result in template_uris {
                let template_uri = template_uri_result?;
                let key = (server_id, template_uri.value());
                templates_table.remove(key)?;
            }
            templates_index.remove_all(server_id)?;

            // Remove fingerprint
            let mut fingerprints_table = write_txn.open_table(FINGERPRINTS_TABLE)?;
            fingerprints_table.remove(server_id)?;
        }

        write_txn.commit()?;
        debug!("Removed server data for: {}", server_id);
        Ok(())
    }

    /// Clear all cache data
    pub fn clear_all(&self) -> Result<(), CacheError> {
        let write_txn = self.db.begin_write()?;

        {
            // Clear all tables by iterating and removing all entries
            let mut servers_table = write_txn.open_table(SERVERS_TABLE)?;
            let mut tools_table = write_txn.open_table(TOOLS_TABLE)?;
            let mut resources_table = write_txn.open_table(RESOURCES_TABLE)?;
            let mut prompts_table = write_txn.open_table(PROMPTS_TABLE)?;
            let mut templates_table = write_txn.open_table(RESOURCE_TEMPLATES_TABLE)?;
            let mut fingerprints_table = write_txn.open_table(FINGERPRINTS_TABLE)?;
            let mut metadata_table = write_txn.open_table(INSTANCE_METADATA_TABLE)?;

            // Clear multimap indexes
            let mut tools_index = write_txn.open_multimap_table(SERVER_TOOLS_INDEX)?;
            let mut resources_index = write_txn.open_multimap_table(SERVER_RESOURCES_INDEX)?;
            let mut prompts_index = write_txn.open_multimap_table(SERVER_PROMPTS_INDEX)?;
            let mut templates_index = write_txn.open_multimap_table(SERVER_RESOURCE_TEMPLATES_INDEX)?;

            // Remove all entries from main tables
            let server_keys: Vec<String> = servers_table
                .iter()?
                .map(|item| {
                    let (key, _) = item?;
                    Ok(key.value().to_string())
                })
                .collect::<Result<Vec<_>, CacheError>>()?;

            for key in server_keys {
                servers_table.remove(&*key)?;
            }

            let tool_keys: Vec<(String, String)> = tools_table
                .iter()?
                .map(|item| {
                    let (key, _) = item?;
                    let (server_id, tool_name) = key.value();
                    Ok((server_id.to_string(), tool_name.to_string()))
                })
                .collect::<Result<Vec<_>, CacheError>>()?;

            for (server_id, tool_name) in tool_keys {
                tools_table.remove((server_id.as_str(), tool_name.as_str()))?;
            }

            let resource_keys: Vec<(String, String)> = resources_table
                .iter()?
                .map(|item| {
                    let (key, _) = item?;
                    let (server_id, resource_uri) = key.value();
                    Ok((server_id.to_string(), resource_uri.to_string()))
                })
                .collect::<Result<Vec<_>, CacheError>>()?;

            for (server_id, resource_uri) in resource_keys {
                resources_table.remove((server_id.as_str(), resource_uri.as_str()))?;
            }

            let prompt_keys: Vec<(String, String)> = prompts_table
                .iter()?
                .map(|item| {
                    let (key, _) = item?;
                    let (server_id, prompt_name) = key.value();
                    Ok((server_id.to_string(), prompt_name.to_string()))
                })
                .collect::<Result<Vec<_>, CacheError>>()?;

            for (server_id, prompt_name) in prompt_keys {
                prompts_table.remove((server_id.as_str(), prompt_name.as_str()))?;
            }

            let template_keys: Vec<(String, String)> = templates_table
                .iter()?
                .map(|item| {
                    let (key, _) = item?;
                    let (server_id, template_uri) = key.value();
                    Ok((server_id.to_string(), template_uri.to_string()))
                })
                .collect::<Result<Vec<_>, CacheError>>()?;

            for (server_id, template_uri) in template_keys {
                templates_table.remove((server_id.as_str(), template_uri.as_str()))?;
            }

            let fingerprint_keys: Vec<String> = fingerprints_table
                .iter()?
                .map(|item| {
                    let (key, _) = item?;
                    Ok(key.value().to_string())
                })
                .collect::<Result<Vec<_>, CacheError>>()?;

            for key in fingerprint_keys {
                fingerprints_table.remove(&*key)?;
            }

            let metadata_keys: Vec<String> = metadata_table
                .iter()?
                .map(|item| {
                    let (key, _) = item?;
                    Ok(key.value().to_string())
                })
                .collect::<Result<Vec<_>, CacheError>>()?;

            for key in metadata_keys {
                metadata_table.remove(&*key)?;
            }

            // Clear multimap indexes
            let server_ids: Vec<String> = tools_index
                .iter()?
                .map(|item| {
                    let (key, _) = item?;
                    Ok(key.value().to_string())
                })
                .collect::<Result<Vec<_>, CacheError>>()?;

            for server_id in &server_ids {
                tools_index.remove_all(server_id.as_str())?;
                resources_index.remove_all(server_id.as_str())?;
                prompts_index.remove_all(server_id.as_str())?;
                templates_index.remove_all(server_id.as_str())?;
            }
        }

        write_txn.commit()?;
        debug!("Cache cleared successfully");
        Ok(())
    }
}
