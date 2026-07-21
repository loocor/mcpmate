use std::collections::{HashMap, HashSet};

use mcpmate_capability_store::{CapabilityPayload, InventoryState, SnapshotState};

use crate::config::database::Database;

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
    database: &Database,
    server_ids: impl IntoIterator<Item = String>,
) -> CapabilityDescriptionIndex {
    let mut index = CapabilityDescriptionIndex::default();
    let server_ids: HashSet<String> = server_ids.into_iter().collect();

    for server_id in server_ids {
        let snapshot = match database.load_capability_snapshot(&server_id).await {
            Ok((snapshot, _)) => snapshot,
            Err(error) => {
                tracing::debug!(
                    server_id = %server_id,
                    error = %error,
                    "Failed to read SQLite capability descriptions"
                );
                None
            }
        };

        let Some(snapshot) = snapshot else {
            continue;
        };
        if snapshot.state != SnapshotState::Ready {
            continue;
        }
        let complete_kinds = snapshot
            .kind_states
            .iter()
            .filter(|state| state.inventory == InventoryState::Complete)
            .map(|state| state.kind)
            .collect::<HashSet<_>>();
        for record in &snapshot.records {
            if !complete_kinds.contains(&record.kind()) {
                continue;
            }
            match &record.payload {
                CapabilityPayload::Tool(tool) => {
                    insert_description(&mut index.tools, &server_id, &tool.name, tool.description.as_deref());
                    insert_description(
                        &mut index.tools,
                        &server_id,
                        &record.external_key,
                        tool.description.as_deref(),
                    );
                }
                CapabilityPayload::Resource(resource) => {
                    insert_description(
                        &mut index.resources,
                        &server_id,
                        &resource.uri,
                        resource.description.as_deref(),
                    );
                    insert_description(
                        &mut index.resources,
                        &server_id,
                        &record.external_key,
                        resource.description.as_deref(),
                    );
                    insert_description(
                        &mut index.resources,
                        &server_id,
                        &resource.name,
                        resource.description.as_deref(),
                    );
                }
                CapabilityPayload::Prompt(prompt) => {
                    insert_description(
                        &mut index.prompts,
                        &server_id,
                        &prompt.name,
                        prompt.description.as_deref(),
                    );
                    insert_description(
                        &mut index.prompts,
                        &server_id,
                        &record.external_key,
                        prompt.description.as_deref(),
                    );
                }
                CapabilityPayload::ResourceTemplate(template) => {
                    insert_description(
                        &mut index.templates,
                        &server_id,
                        &template.uri_template,
                        template.description.as_deref(),
                    );
                    insert_description(
                        &mut index.templates,
                        &server_id,
                        &record.external_key,
                        template.description.as_deref(),
                    );
                    insert_description(
                        &mut index.templates,
                        &server_id,
                        &template.name,
                        template.description.as_deref(),
                    );
                }
            }
        }
    }

    index
}
