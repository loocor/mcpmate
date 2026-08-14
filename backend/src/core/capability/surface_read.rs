use std::collections::{BTreeMap, HashSet};

use mcpmate_capability_store::{
    CapabilityId, CapabilityKind, CapabilityRefId, CatalogError, EffectiveCapabilityDefinition,
    EffectiveCapabilityRecordV1, Result, SURFACE_MANIFEST_FORMAT_V1, SurfaceManifestEntry, SurfaceManifestId,
};
use rmcp::model::{Prompt, Resource, ResourceTemplate, Tool};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Row, Sqlite};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct PersistedManifestContent {
    format: String,
    consumer_id: String,
    entries: Vec<SurfaceManifestEntry>,
}

#[derive(Clone, Debug)]
pub struct ActiveSurfaceEntry {
    pub ref_id: CapabilityRefId,
    pub capability_id: CapabilityId,
    pub kind: CapabilityKind,
    pub external_key: String,
    pub source_server_id: String,
    pub upstream_key: String,
    pub definition: EffectiveCapabilityDefinition,
}

#[derive(Clone, Debug)]
pub struct ActiveSurface {
    pub consumer_id: String,
    pub publication_id: String,
    pub manifest_id: SurfaceManifestId,
    pub generation: i64,
    pub entries: Vec<ActiveSurfaceEntry>,
}

#[derive(Clone, Debug)]
pub struct ActiveResourceRoute {
    pub source_server_id: String,
    pub external_uri: String,
    pub upstream_uri: String,
    pub upstream_template: Option<String>,
    pub template_arguments: Option<BTreeMap<String, String>>,
}

impl ActiveSurface {
    pub fn tools(&self) -> Vec<Tool> {
        self.entries
            .iter()
            .filter_map(|entry| match &entry.definition {
                EffectiveCapabilityDefinition::Tool(tool) => Some(tool.clone()),
                _ => None,
            })
            .collect()
    }

    pub fn prompts(&self) -> Vec<Prompt> {
        self.entries
            .iter()
            .filter_map(|entry| match &entry.definition {
                EffectiveCapabilityDefinition::Prompt(prompt) => Some(prompt.clone()),
                _ => None,
            })
            .collect()
    }

    pub fn resources(&self) -> Vec<Resource> {
        self.entries
            .iter()
            .filter_map(|entry| match &entry.definition {
                EffectiveCapabilityDefinition::Resource(resource) => Some(resource.clone()),
                _ => None,
            })
            .collect()
    }

    pub fn resource_templates(&self) -> Vec<ResourceTemplate> {
        self.entries
            .iter()
            .filter_map(|entry| match &entry.definition {
                EffectiveCapabilityDefinition::ResourceTemplate(template) => Some(template.clone()),
                _ => None,
            })
            .collect()
    }
}

#[derive(Clone)]
pub struct SurfaceReader {
    pool: Pool<Sqlite>,
}

impl SurfaceReader {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn load(
        &self,
        consumer_id: &str,
    ) -> Result<ActiveSurface> {
        let binding = sqlx::query(
            r#"
            SELECT b.active_publication_id, b.generation, p.consumer_id AS publication_consumer_id,
                   p.manifest_id, m.consumer_id AS manifest_consumer_id, m.canonical_content
            FROM consumer_surface_bindings b
            JOIN surface_publications p ON p.publication_id = b.active_publication_id
            JOIN surface_manifests m ON m.manifest_id = p.manifest_id
            WHERE b.consumer_id = ?
            "#,
        )
        .bind(consumer_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| CatalogError::SurfaceNotFound {
            entity: "active consumer surface",
            id: consumer_id.to_string(),
        })?;
        let publication_consumer_id: String = binding.try_get("publication_consumer_id")?;
        let manifest_consumer_id: String = binding.try_get("manifest_consumer_id")?;
        if publication_consumer_id != consumer_id || manifest_consumer_id != consumer_id {
            return Err(CatalogError::IntegrityMismatch {
                identity: consumer_id.to_string(),
            });
        }
        let manifest_id: SurfaceManifestId = binding.try_get::<String, _>("manifest_id")?.parse()?;
        let canonical_content: Vec<u8> = binding.try_get("canonical_content")?;
        let content: PersistedManifestContent = serde_json::from_slice(&canonical_content)?;
        manifest_id.verify_content(&content)?;
        if serde_json_canonicalizer::to_vec(&content)? != canonical_content {
            return Err(CatalogError::IntegrityMismatch {
                identity: manifest_id.to_string(),
            });
        }
        if content.format != SURFACE_MANIFEST_FORMAT_V1 || content.consumer_id != consumer_id {
            return Err(CatalogError::IntegrityMismatch {
                identity: manifest_id.to_string(),
            });
        }

        let rows = sqlx::query(
            r#"
            SELECT e.position, e.ref_id, e.capability_id, v.canonical_record,
                   r.server_id, r.kind AS ref_kind, r.origin_key
            FROM surface_manifest_entries e
            JOIN capability_refs r ON r.ref_id = e.ref_id
            JOIN capability_versions v
              ON v.ref_id = e.ref_id AND v.capability_id = e.capability_id
            WHERE e.manifest_id = ?
            ORDER BY e.position
            "#,
        )
        .bind(manifest_id.as_str())
        .fetch_all(&self.pool)
        .await?;
        let mut entries = Vec::with_capacity(rows.len());
        let mut external_keys = HashSet::with_capacity(rows.len());
        if rows.len() != content.entries.len() {
            return Err(CatalogError::IntegrityMismatch {
                identity: manifest_id.to_string(),
            });
        }
        for (position, row) in rows.into_iter().enumerate() {
            let ref_id: CapabilityRefId = row.try_get::<String, _>("ref_id")?.parse()?;
            let capability_id: CapabilityId = row.try_get::<String, _>("capability_id")?.parse()?;
            let persisted_position: i64 = row.try_get("position")?;
            if persisted_position != position as i64
                || content.entries[position].ref_id != ref_id
                || content.entries[position].capability_id != capability_id
            {
                return Err(CatalogError::IntegrityMismatch {
                    identity: ref_id.to_string(),
                });
            }
            let canonical_record: Vec<u8> = row.try_get("canonical_record")?;
            capability_id.verify_canonical_content(&canonical_record, &canonical_record)?;
            let record: EffectiveCapabilityRecordV1 = serde_json::from_slice(&canonical_record)?;
            record.validate()?;
            let ref_kind: String = row.try_get("ref_kind")?;
            let source_server_id: String = row.try_get("server_id")?;
            let upstream_key: String = row.try_get("origin_key")?;
            if record.ref_id != ref_id
                || record.source.server_id != source_server_id
                || record.source.kind.as_str() != ref_kind
                || record.source.origin_key != upstream_key
            {
                return Err(CatalogError::IntegrityMismatch {
                    identity: ref_id.to_string(),
                });
            }
            let kind = record.definition.kind();
            let external_key = record.definition.external_key();
            if !external_keys.insert((kind, external_key.clone())) {
                return Err(CatalogError::InvalidSurfaceValue {
                    field: "surface external key",
                    value: external_key,
                });
            }
            entries.push(ActiveSurfaceEntry {
                ref_id,
                capability_id,
                kind,
                external_key,
                source_server_id,
                upstream_key,
                definition: record.definition,
            });
        }
        Ok(ActiveSurface {
            consumer_id: consumer_id.to_string(),
            publication_id: binding.try_get("active_publication_id")?,
            manifest_id,
            generation: binding.try_get("generation")?,
            entries,
        })
    }

    pub async fn require(
        &self,
        kind: CapabilityKind,
        consumer_id: &str,
        external_key: &str,
    ) -> Result<ActiveSurfaceEntry> {
        self.load(consumer_id)
            .await?
            .entries
            .into_iter()
            .find(|entry| entry.kind == kind && entry.external_key == external_key)
            .ok_or_else(|| CatalogError::InvalidSurfaceValue {
                field: "active surface capability",
                value: format!("{consumer_id}/{}/{external_key}", kind.as_str()),
            })
    }

    pub async fn resolve_resource_route(
        &self,
        consumer_id: &str,
        external_uri: &str,
    ) -> Result<ActiveResourceRoute> {
        self.try_resolve_resource_route(consumer_id, external_uri)
            .await?
            .ok_or_else(|| CatalogError::InvalidSurfaceValue {
                field: "active surface resource route",
                value: format!("{consumer_id}/{external_uri}"),
            })
    }

    pub async fn try_resolve_resource_route(
        &self,
        consumer_id: &str,
        external_uri: &str,
    ) -> Result<Option<ActiveResourceRoute>> {
        let surface = self.load(consumer_id).await?;
        if let Some(entry) = surface
            .entries
            .iter()
            .find(|entry| entry.kind == CapabilityKind::Resources && entry.external_key == external_uri)
        {
            return Ok(Some(ActiveResourceRoute {
                source_server_id: entry.source_server_id.clone(),
                external_uri: external_uri.to_string(),
                upstream_uri: entry.upstream_key.clone(),
                upstream_template: None,
                template_arguments: None,
            }));
        }

        let mut matches = Vec::new();
        for entry in surface
            .entries
            .iter()
            .filter(|entry| entry.kind == CapabilityKind::ResourceTemplates)
        {
            let expanded = super::resource_uri::expand_upstream_resource_template(
                &entry.external_key,
                &entry.upstream_key,
                external_uri,
            )
            .map_err(|error| CatalogError::InvalidSurfaceValue {
                field: "active surface resource route",
                value: format!("{consumer_id}/{external_uri}: {error}"),
            })?;
            if let Some((upstream_uri, arguments)) = expanded {
                matches.push(ActiveResourceRoute {
                    source_server_id: entry.source_server_id.clone(),
                    external_uri: external_uri.to_string(),
                    upstream_uri,
                    upstream_template: Some(entry.upstream_key.clone()),
                    template_arguments: Some(arguments),
                });
            }
        }

        match matches.len() {
            1 => Ok(Some(matches.pop().expect("single active Surface Resource route"))),
            0 => Ok(None),
            _ => Err(CatalogError::InvalidSurfaceValue {
                field: "ambiguous active surface resource route",
                value: format!("{consumer_id}/{external_uri}"),
            }),
        }
    }
}
