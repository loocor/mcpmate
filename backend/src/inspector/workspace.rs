use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::{common::paths::MCPMatePaths, core::models::MCPServerConfig};

#[derive(Debug, Clone)]
pub struct InspectorWorkspace {
    servers_dir: PathBuf,
    patches_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InspectorServerProvenance {
    Scratch {
        #[serde(skip_serializing_if = "Option::is_none")]
        origin: Option<String>,
    },
    ManagedRegistry {
        server_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        server_name: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectorServerRecord {
    pub id: String,
    pub name: String,
    pub config: MCPServerConfig,
    pub provenance: InspectorServerProvenance,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct InspectorServerRecordInput {
    pub name: String,
    pub config: MCPServerConfig,
    pub provenance: InspectorServerProvenance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InspectorCapabilityPatchKind {
    Tools,
    Prompts,
    Resources,
    ResourceTemplates,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum InspectorPatchTarget {
    ManagedRegistry { server_id: String },
    ScratchWorkspace { record_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectorCapabilityPatchRecord {
    pub id: String,
    pub target: InspectorPatchTarget,
    pub capability_kind: InspectorCapabilityPatchKind,
    pub capability_key: String,
    pub patch: Map<String, Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct InspectorCapabilityPatchInput {
    pub target: InspectorPatchTarget,
    pub capability_kind: InspectorCapabilityPatchKind,
    pub capability_key: String,
    pub patch: Map<String, Value>,
}

impl InspectorWorkspace {
    pub fn new(paths: &MCPMatePaths) -> Self {
        Self {
            servers_dir: paths.inspector_servers_dir(),
            patches_dir: paths.inspector_patches_dir(),
        }
    }

    pub fn from_servers_dir<P: Into<PathBuf>>(servers_dir: P) -> Self {
        let servers_dir = servers_dir.into();
        let patches_dir = servers_dir
            .parent()
            .map(|parent| parent.join("patches"))
            .unwrap_or_else(|| PathBuf::from("patches"));
        Self {
            servers_dir,
            patches_dir,
        }
    }

    pub fn servers_dir(&self) -> &Path {
        &self.servers_dir
    }

    pub fn patches_dir(&self) -> &Path {
        &self.patches_dir
    }

    pub fn create_server_record(
        &self,
        input: InspectorServerRecordInput,
    ) -> Result<InspectorServerRecord> {
        std::fs::create_dir_all(&self.servers_dir).with_context(|| {
            format!(
                "Failed to create Inspector server directory: {}",
                self.servers_dir.display()
            )
        })?;
        let id = self.next_server_record_id(&input.name)?;
        let now = Utc::now();
        let record = InspectorServerRecord {
            id,
            name: input.name,
            config: input.config,
            provenance: input.provenance,
            created_at: now,
            updated_at: now,
        };
        self.save_server_record(&record)?;
        Ok(record)
    }

    pub fn save_server_record(
        &self,
        record: &InspectorServerRecord,
    ) -> Result<()> {
        validate_record_id(&record.id)?;
        std::fs::create_dir_all(&self.servers_dir).with_context(|| {
            format!(
                "Failed to create Inspector server directory: {}",
                self.servers_dir.display()
            )
        })?;

        let path = self.server_record_path(&record.id)?;
        let tmp_path = path.with_extension("json.tmp");
        let payload = serde_json::to_vec_pretty(record).context("Failed to serialize Inspector server record")?;

        std::fs::write(&tmp_path, payload)
            .with_context(|| format!("Failed to write Inspector server record: {}", tmp_path.display()))?;
        std::fs::rename(&tmp_path, &path)
            .with_context(|| format!("Failed to move Inspector server record into place: {}", path.display()))?;
        Ok(())
    }

    pub fn get_server_record(
        &self,
        id: &str,
    ) -> Result<Option<InspectorServerRecord>> {
        validate_record_id(id)?;
        let path = self.server_record_path(id)?;
        if !path.exists() {
            return Ok(None);
        }
        read_server_record(&path).map(Some)
    }

    pub fn list_server_records(&self) -> Result<Vec<InspectorServerRecord>> {
        if !self.servers_dir.exists() {
            return Ok(Vec::new());
        }

        let mut records = Vec::new();
        for entry in std::fs::read_dir(&self.servers_dir).with_context(|| {
            format!(
                "Failed to read Inspector server directory: {}",
                self.servers_dir.display()
            )
        })? {
            let entry = entry.context("Failed to read Inspector server directory entry")?;
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
                continue;
            }
            records.push(read_server_record(&path)?);
        }

        records.sort_by(|left, right| left.name.cmp(&right.name).then_with(|| left.id.cmp(&right.id)));
        Ok(records)
    }

    pub fn delete_server_record(
        &self,
        id: &str,
    ) -> Result<bool> {
        validate_record_id(id)?;
        let path = self.server_record_path(id)?;
        if !path.exists() {
            return Ok(false);
        }

        std::fs::remove_file(&path)
            .with_context(|| format!("Failed to delete Inspector server record: {}", path.display()))?;
        Ok(true)
    }

    fn server_record_path(
        &self,
        id: &str,
    ) -> Result<PathBuf> {
        validate_record_id(id)?;
        Ok(self.servers_dir.join(format!("{id}.json")))
    }

    fn next_server_record_id(
        &self,
        name: &str,
    ) -> Result<String> {
        let base = normalize_server_record_id(name)?;
        let existing = self.list_server_records()?;
        let mut suffix = 1usize;
        loop {
            let candidate = if suffix == 1 {
                base.clone()
            } else {
                format!("{base}-{suffix}")
            };
            let path = self.server_record_path(&candidate)?;
            let name_conflict = suffix == 1 && existing.iter().any(|record| record.name == name);
            if !path.exists() && !name_conflict {
                return Ok(candidate);
            }
            suffix += 1;
        }
    }

    pub fn upsert_capability_patch(
        &self,
        input: InspectorCapabilityPatchInput,
    ) -> Result<InspectorCapabilityPatchRecord> {
        let existing = self.list_capability_patches()?.into_iter().find(|record| {
            record.target == input.target
                && record.capability_kind == input.capability_kind
                && record.capability_key == input.capability_key
        });
        let now = Utc::now();
        let record = InspectorCapabilityPatchRecord {
            id: existing
                .as_ref()
                .map(|record| record.id.clone())
                .unwrap_or_else(|| format!("inspatch-{}", Uuid::new_v4().simple())),
            target: input.target,
            capability_kind: input.capability_kind,
            capability_key: input.capability_key,
            patch: input.patch,
            created_at: existing.map(|record| record.created_at).unwrap_or(now),
            updated_at: now,
        };
        self.save_capability_patch(&record)?;
        Ok(record)
    }

    pub fn save_capability_patch(
        &self,
        record: &InspectorCapabilityPatchRecord,
    ) -> Result<()> {
        validate_record_id(&record.id)?;
        std::fs::create_dir_all(&self.patches_dir).with_context(|| {
            format!(
                "Failed to create Inspector patch directory: {}",
                self.patches_dir.display()
            )
        })?;

        let path = self.capability_patch_path(&record.id)?;
        let tmp_path = path.with_extension("json.tmp");
        let payload = serde_json::to_vec_pretty(record).context("Failed to serialize Inspector capability patch")?;

        std::fs::write(&tmp_path, payload)
            .with_context(|| format!("Failed to write Inspector capability patch: {}", tmp_path.display()))?;
        std::fs::rename(&tmp_path, &path).with_context(|| {
            format!(
                "Failed to move Inspector capability patch into place: {}",
                path.display()
            )
        })?;
        Ok(())
    }

    pub fn list_capability_patches(&self) -> Result<Vec<InspectorCapabilityPatchRecord>> {
        if !self.patches_dir.exists() {
            return Ok(Vec::new());
        }

        let mut records = Vec::new();
        for entry in std::fs::read_dir(&self.patches_dir).with_context(|| {
            format!(
                "Failed to read Inspector patch directory: {}",
                self.patches_dir.display()
            )
        })? {
            let entry = entry.context("Failed to read Inspector patch directory entry")?;
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
                continue;
            }
            records.push(read_capability_patch(&path)?);
        }

        records.sort_by(|left, right| {
            left.capability_key
                .cmp(&right.capability_key)
                .then_with(|| left.id.cmp(&right.id))
        });
        Ok(records)
    }

    fn capability_patch_path(
        &self,
        id: &str,
    ) -> Result<PathBuf> {
        validate_record_id(id)?;
        Ok(self.patches_dir.join(format!("{id}.json")))
    }
}

fn read_server_record(path: &Path) -> Result<InspectorServerRecord> {
    let payload =
        std::fs::read(path).with_context(|| format!("Failed to read Inspector server record: {}", path.display()))?;
    serde_json::from_slice(&payload)
        .with_context(|| format!("Failed to parse Inspector server record: {}", path.display()))
}

fn read_capability_patch(path: &Path) -> Result<InspectorCapabilityPatchRecord> {
    let payload = std::fs::read(path)
        .with_context(|| format!("Failed to read Inspector capability patch: {}", path.display()))?;
    serde_json::from_slice(&payload)
        .with_context(|| format!("Failed to parse Inspector capability patch: {}", path.display()))
}

fn validate_record_id(id: &str) -> Result<()> {
    let valid = !id.is_empty()
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_');
    if valid {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Invalid Inspector server record id: {}", id))
    }
}

fn normalize_server_record_id(name: &str) -> Result<String> {
    let mut normalized = String::new();
    let mut last_was_separator = false;

    for byte in name.trim().bytes() {
        let next = if byte.is_ascii_alphanumeric() {
            Some(byte.to_ascii_lowercase() as char)
        } else if byte == b'-' || byte == b'_' || byte.is_ascii_whitespace() {
            Some('-')
        } else {
            None
        };

        let Some(ch) = next else {
            continue;
        };

        if ch == '-' {
            if normalized.is_empty() || last_was_separator {
                continue;
            }
            last_was_separator = true;
            normalized.push(ch);
        } else {
            last_was_separator = false;
            normalized.push(ch);
        }
    }

    while normalized.ends_with('-') {
        normalized.pop();
    }

    if normalized.is_empty() {
        Err(anyhow::anyhow!(
            "Inspector scratch server name must contain at least one ASCII letter or number"
        ))
    } else {
        Ok(normalized)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use tempfile::tempdir;

    use super::*;
    use crate::common::{paths::MCPMatePaths, server::ServerType};

    #[test]
    fn path_manager_exposes_inspector_workspace_dirs() {
        let tmp = tempdir().expect("tmp dir");
        let paths = MCPMatePaths::from_base_dir(tmp.path().join("mcpmate")).expect("paths");

        assert_eq!(paths.inspector_dir(), tmp.path().join("mcpmate").join("inspector"));
        assert_eq!(
            paths.inspector_servers_dir(),
            tmp.path().join("mcpmate").join("inspector").join("servers")
        );
        assert_eq!(
            paths.inspector_patches_dir(),
            tmp.path().join("mcpmate").join("inspector").join("patches")
        );
    }

    #[test]
    fn scratch_record_roundtrips_without_registry_storage() {
        let tmp = tempdir().expect("tmp dir");
        let workspace = InspectorWorkspace::from_servers_dir(tmp.path().join("inspector").join("servers"));
        let record = workspace
            .create_server_record(InspectorServerRecordInput {
                name: "Scratch Fetch".to_string(),
                config: MCPServerConfig {
                    kind: ServerType::Stdio,
                    command: Some("uvx".to_string()),
                    args: Some(vec!["mcp-server-fetch".to_string()]),
                    url: None,
                    env: Some(HashMap::from([("FETCH_TIMEOUT".to_string(), "10".to_string())])),
                    headers: None,
                },
                provenance: InspectorServerProvenance::Scratch {
                    origin: Some("manual".to_string()),
                },
            })
            .expect("create scratch record");

        assert_eq!(record.id, "scratch-fetch");
        let stored_path = workspace.servers_dir().join(format!("{}.json", record.id));
        assert!(stored_path.exists());

        let loaded = workspace
            .get_server_record(&record.id)
            .expect("load scratch record")
            .expect("record exists");
        assert_eq!(loaded.id, record.id);
        assert_eq!(loaded.name, "Scratch Fetch");
        assert!(matches!(loaded.provenance, InspectorServerProvenance::Scratch { .. }));
        assert_eq!(loaded.config.command.as_deref(), Some("uvx"));
    }

    #[test]
    fn managed_registry_provenance_stays_explicit() {
        let tmp = tempdir().expect("tmp dir");
        let workspace = InspectorWorkspace::from_servers_dir(tmp.path().join("servers"));
        let record = workspace
            .create_server_record(InspectorServerRecordInput {
                name: "Managed Snapshot".to_string(),
                config: MCPServerConfig {
                    kind: ServerType::StreamableHttp,
                    command: None,
                    args: None,
                    url: Some("http://127.0.0.1:9999/mcp".to_string()),
                    env: None,
                    headers: None,
                },
                provenance: InspectorServerProvenance::ManagedRegistry {
                    server_id: "server-managed".to_string(),
                    server_name: Some("Managed Server".to_string()),
                },
            })
            .expect("create managed snapshot");

        let loaded = workspace
            .get_server_record(&record.id)
            .expect("load managed snapshot")
            .expect("record exists");
        assert_eq!(
            loaded.provenance,
            InspectorServerProvenance::ManagedRegistry {
                server_id: "server-managed".to_string(),
                server_name: Some("Managed Server".to_string())
            }
        );
    }

    #[test]
    fn list_records_returns_name_sorted_records() {
        let tmp = tempdir().expect("tmp dir");
        let workspace = InspectorWorkspace::from_servers_dir(tmp.path().join("servers"));

        for name in ["Zulu", "Alpha"] {
            workspace
                .create_server_record(InspectorServerRecordInput {
                    name: name.to_string(),
                    config: MCPServerConfig {
                        kind: ServerType::Stdio,
                        command: Some("node".to_string()),
                        args: None,
                        url: None,
                        env: None,
                        headers: None,
                    },
                    provenance: InspectorServerProvenance::Scratch { origin: None },
                })
                .expect("create record");
        }

        let names = workspace
            .list_server_records()
            .expect("list records")
            .into_iter()
            .map(|record| record.name)
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["Alpha", "Zulu"]);
    }

    #[test]
    fn scratch_record_ids_are_normalized_and_deduped() {
        let tmp = tempdir().expect("tmp dir");
        let workspace = InspectorWorkspace::from_servers_dir(tmp.path().join("servers"));

        let create = || InspectorServerRecordInput {
            name: "Scratch Fetch".to_string(),
            config: MCPServerConfig {
                kind: ServerType::Stdio,
                command: Some("node".to_string()),
                args: None,
                url: None,
                env: None,
                headers: None,
            },
            provenance: InspectorServerProvenance::Scratch { origin: None },
        };

        let first = workspace.create_server_record(create()).expect("create first");
        let second = workspace.create_server_record(create()).expect("create second");

        assert_eq!(first.id, "scratch-fetch");
        assert_eq!(second.id, "scratch-fetch-2");
        assert!(workspace.servers_dir().join("scratch-fetch.json").exists());
        assert!(workspace.servers_dir().join("scratch-fetch-2.json").exists());
    }

    #[test]
    fn scratch_record_id_rejects_names_without_ascii_slug() {
        let tmp = tempdir().expect("tmp dir");
        let workspace = InspectorWorkspace::from_servers_dir(tmp.path().join("servers"));

        let error = workspace
            .create_server_record(InspectorServerRecordInput {
                name: "临时服务".to_string(),
                config: MCPServerConfig {
                    kind: ServerType::Stdio,
                    command: Some("node".to_string()),
                    args: None,
                    url: None,
                    env: None,
                    headers: None,
                },
                provenance: InspectorServerProvenance::Scratch { origin: None },
            })
            .expect_err("non-ascii-only name should fail");

        assert!(
            error
                .to_string()
                .contains("must contain at least one ASCII letter or number")
        );
    }

    #[test]
    fn delete_record_removes_workspace_file() {
        let tmp = tempdir().expect("tmp dir");
        let workspace = InspectorWorkspace::from_servers_dir(tmp.path().join("servers"));
        let record = workspace
            .create_server_record(InspectorServerRecordInput {
                name: "Scratch".to_string(),
                config: MCPServerConfig {
                    kind: ServerType::Stdio,
                    command: Some("node".to_string()),
                    args: None,
                    url: None,
                    env: None,
                    headers: None,
                },
                provenance: InspectorServerProvenance::Scratch { origin: None },
            })
            .expect("create record");

        let stored_path = workspace.servers_dir().join(format!("{}.json", record.id));
        assert!(stored_path.exists());

        assert!(workspace.delete_server_record(&record.id).expect("delete record"));
        assert!(!stored_path.exists());
        assert!(
            workspace
                .get_server_record(&record.id)
                .expect("load deleted record")
                .is_none()
        );
        assert!(
            !workspace
                .delete_server_record(&record.id)
                .expect("delete missing record")
        );
    }

    #[test]
    fn invalid_record_id_is_rejected() {
        let tmp = tempdir().expect("tmp dir");
        let workspace = InspectorWorkspace::from_servers_dir(tmp.path().join("servers"));

        let error = workspace
            .get_server_record("../registry-server")
            .expect_err("path traversal id should fail");
        assert!(error.to_string().contains("Invalid Inspector server record id"));
    }

    #[test]
    fn capability_patch_upsert_replaces_existing_target_key() {
        let tmp = tempdir().expect("tmp dir");
        let workspace = InspectorWorkspace::from_servers_dir(tmp.path().join("inspector").join("servers"));
        let target = InspectorPatchTarget::ScratchWorkspace {
            record_id: "scratch-a".to_string(),
        };

        let first = workspace
            .upsert_capability_patch(InspectorCapabilityPatchInput {
                target: target.clone(),
                capability_kind: InspectorCapabilityPatchKind::Tools,
                capability_key: "echo".to_string(),
                patch: Map::from_iter([("description".to_string(), Value::String("first".to_string()))]),
            })
            .expect("create patch");
        let second = workspace
            .upsert_capability_patch(InspectorCapabilityPatchInput {
                target,
                capability_kind: InspectorCapabilityPatchKind::Tools,
                capability_key: "echo".to_string(),
                patch: Map::from_iter([("description".to_string(), Value::String("second".to_string()))]),
            })
            .expect("replace patch");

        assert_eq!(first.id, second.id);
        assert_eq!(workspace.list_capability_patches().expect("list patches").len(), 1);
        let loaded = workspace
            .list_capability_patches()
            .expect("list patches")
            .into_iter()
            .next()
            .expect("patch");
        assert_eq!(loaded.patch.get("description").and_then(Value::as_str), Some("second"));
        assert!(workspace.patches_dir().join(format!("{}.json", loaded.id)).exists());
    }
}
