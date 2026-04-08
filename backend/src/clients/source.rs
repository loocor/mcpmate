use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use json5;
use serde_yaml;
use tokio::sync::RwLock;
use toml;
use walkdir::WalkDir;

use crate::clients::error::{ConfigError, ConfigResult};
use crate::clients::models::{ClientTemplate, DetectionMethod, TemplateFormat};
use crate::common::constants::database::tables;
use crate::system::paths::PathService;
use sqlx::SqlitePool;

/// Template root directory abstract, responsible for parsing and caching base paths
#[derive(Debug, Clone)]
pub struct TemplateRoot {
    root: PathBuf,
}

impl TemplateRoot {
    pub fn resolve() -> ConfigResult<Self> {
        let path_service = PathService::new().map_err(|err| ConfigError::PathResolutionError(err.to_string()))?;

        let root_hint = std::env::var("MCPMATE_TEMPLATE_ROOT")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "~/.mcpmate/client".to_string());

        let root = path_service
            .resolve_user_path(&root_hint)
            .map_err(|err| ConfigError::PathResolutionError(err.to_string()))?;

        Ok(Self { root })
    }

    pub fn new(path: PathBuf) -> Self {
        Self { root: path }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn official_dir(&self) -> PathBuf {
        self.root.join("official")
    }

    pub fn community_dir(&self) -> PathBuf {
        self.root.join("community")
    }

    pub fn user_dir(&self) -> PathBuf {
        self.root.join("user")
    }

    pub fn standards_dir(&self) -> PathBuf {
        self.root.join("standards")
    }

    pub fn ensure_base_dirs(&self) -> ConfigResult<()> {
        for dir in [
            self.root(),
            &self.official_dir(),
            &self.community_dir(),
            &self.user_dir(),
        ] {
            std::fs::create_dir_all(dir).map_err(|err| ConfigError::PathResolutionError(err.to_string()))?;
        }
        Ok(())
    }
}

impl Default for TemplateRoot {
    fn default() -> Self {
        TemplateRoot::resolve().unwrap_or_else(|_| TemplateRoot {
            root: PathBuf::from("~/.mcpmate/client"),
        })
    }
}

/// MCP standards cache, will be supplemented with structure in the future
#[derive(Debug, Default, Clone)]
pub struct McpStandards {
    pub revisions: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TemplatePriority {
    Official,
    Community,
    User,
}

#[derive(Debug, Clone)]
struct TemplateEntry {
    template: ClientTemplate,
    _source_path: PathBuf,
    file_name: String,
    _priority: TemplatePriority,
}

impl TemplateEntry {
    fn identifier(&self) -> &str {
        &self.template.identifier
    }
}

#[async_trait]
pub trait ClientConfigSource: Send + Sync {
    async fn list_client(&self) -> ConfigResult<Vec<ClientTemplate>>;

    async fn get_template(
        &self,
        client_id: &str,
        platform: &str,
    ) -> ConfigResult<Option<ClientTemplate>>;

    async fn get_config_path(
        &self,
        client_id: &str,
        platform: &str,
    ) -> ConfigResult<Option<String>>;

    async fn reload(&self) -> ConfigResult<()>;
}

/// File-based template source implementation, responsible for loading, indexing and hot reload
pub struct FileTemplateSource {
    template_root: TemplateRoot,
    template_index: Arc<RwLock<HashMap<String, TemplateEntry>>>,
    standards: Arc<RwLock<McpStandards>>,
    path_service: PathService,
}

pub struct DbTemplateSource {
    db_pool: Arc<SqlitePool>,
    path_service: PathService,
}

impl DbTemplateSource {
    pub fn new(db_pool: Arc<SqlitePool>) -> ConfigResult<Self> {
        let path_service = PathService::new().map_err(|err| ConfigError::PathResolutionError(err.to_string()))?;
        Ok(Self { db_pool, path_service })
    }

    async fn load_templates(&self) -> ConfigResult<Vec<ClientTemplate>> {
        let rows = sqlx::query_as::<_, (String, String)>(
            &format!(
                "SELECT identifier, payload_json FROM {} ORDER BY identifier",
                tables::CLIENT_TEMPLATE_RUNTIME
            ),
        )
        .fetch_all(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

        rows.into_iter()
            .map(|(_identifier, payload_json)| {
                serde_json::from_str::<ClientTemplate>(&payload_json)
                    .map_err(|err| ConfigError::TemplateParseError(format!("Failed to parse runtime template payload: {}", err)))
            })
            .collect()
    }
}

impl FileTemplateSource {
    pub fn new(template_root: TemplateRoot) -> ConfigResult<Self> {
        template_root.ensure_base_dirs()?;
        let path_service = PathService::new().map_err(|err| ConfigError::PathResolutionError(err.to_string()))?;

        Ok(Self {
            template_root,
            template_index: Arc::new(RwLock::new(HashMap::new())),
            standards: Arc::new(RwLock::new(McpStandards::default())),
            path_service,
        })
    }

    pub async fn bootstrap(template_root: TemplateRoot) -> ConfigResult<Self> {
        let source = Self::new(template_root)?;
        source.reload().await?;
        Ok(source)
    }

    pub fn template_root(&self) -> &TemplateRoot {
        &self.template_root
    }

    fn directory_order(&self) -> Vec<(TemplatePriority, PathBuf)> {
        vec![
            (TemplatePriority::Official, self.template_root.official_dir()),
            (TemplatePriority::Community, self.template_root.community_dir()),
            (TemplatePriority::User, self.template_root.user_dir()),
        ]
    }

    async fn load_templates(&self) -> ConfigResult<HashMap<String, TemplateEntry>> {
        let mut index: HashMap<String, TemplateEntry> = HashMap::new();
        let mut seen_pairs: HashSet<String> = HashSet::new();

        for (priority, dir) in self.directory_order() {
            if !dir.exists() {
                continue;
            }

            let entries = self.load_template_dir(&dir, priority).await?;
            for entry in entries {
                let key = format!("{}::{}", entry.file_name, entry.identifier());
                if !seen_pairs.insert(key) {
                    return Err(ConfigError::TemplateConflict {
                        identifier: format!("{} ({})", entry.identifier(), entry.file_name),
                    });
                }

                index.insert(entry.identifier().to_string(), entry);
            }
        }

        Ok(index)
    }

    async fn load_template_dir(
        &self,
        dir: &Path,
        priority: TemplatePriority,
    ) -> ConfigResult<Vec<TemplateEntry>> {
        let mut entries = Vec::new();

        for entry in WalkDir::new(dir)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.into_path();
            let Some(ext) = path
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.to_ascii_lowercase())
            else {
                continue;
            };

            if !matches!(ext.as_str(), "json" | "json5" | "yaml" | "yml" | "toml") {
                continue;
            }

            let file_name = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_string();

            let mut template = self.parse_template_file(&path, &ext).await?;
            self.apply_extension_defaults(&mut template, &ext);
            self.validate_template(&template, &path)?;

            entries.push(TemplateEntry {
                template,
                _source_path: path,
                file_name,
                _priority: priority,
            });
        }

        Ok(entries)
    }

    async fn parse_template_file(
        &self,
        path: &Path,
        ext: &str,
    ) -> ConfigResult<ClientTemplate> {
        let content = tokio::fs::read_to_string(path).await.map_err(|err| {
            ConfigError::TemplateParseError(format!("Failed to read template file {}: {}", path.display(), err))
        })?;

        let template = match ext {
            "json" => serde_json::from_str(&content).map_err(|err| {
                ConfigError::TemplateParseError(format!("Failed to parse template file {}: {}", path.display(), err))
            })?,
            "json5" => json5::from_str(&content).map_err(|err| {
                ConfigError::TemplateParseError(format!("Failed to parse template file {}: {}", path.display(), err))
            })?,
            "yaml" | "yml" => serde_yaml::from_str(&content).map_err(|err| {
                ConfigError::TemplateParseError(format!("Failed to parse template file {}: {}", path.display(), err))
            })?,
            "toml" => toml::from_str(&content).map_err(|err| {
                ConfigError::TemplateParseError(format!("Failed to parse template file {}: {}", path.display(), err))
            })?,
            _ => {
                return Err(ConfigError::TemplateParseError(format!(
                    "Unsupported template extension {} ({})",
                    ext,
                    path.display()
                )));
            }
        };

        Ok(template)
    }

    fn apply_extension_defaults(
        &self,
        template: &mut ClientTemplate,
        ext: &str,
    ) {
        // The `format` field on ClientTemplate represents the OUTPUT config format
        // (json/json5/toml/yaml). Template files themselves may be authored in
        // JSON, JSON5, YAML or TOML regardless of the output format. Here we
        // only derive a default output format from the file extension when the
        // template did not explicitly specify one (i.e. it remains at the
        // enum default of JSON).
        if matches!(template.format, TemplateFormat::Json) {
            match ext {
                "json5" => template.format = TemplateFormat::Json5,
                "yaml" | "yml" => template.format = TemplateFormat::Yaml,
                "toml" => template.format = TemplateFormat::Toml,
                _ => {}
            }
        }
    }

    fn validate_template(
        &self,
        template: &ClientTemplate,
        path: &Path,
    ) -> ConfigResult<()> {
        if template.identifier.trim().is_empty() {
            return Err(ConfigError::TemplateParseError(format!(
                "Template missing identifier field: {}",
                path.display()
            )));
        }

        if template.config_mapping.container_keys.is_empty() {
            return Err(ConfigError::TemplateParseError(format!(
                "Template {} missing config_mapping.container_keys",
                template.identifier
            )));
        }

        if template.detection.is_empty() {
            return Err(ConfigError::TemplateParseError(format!(
                "Template {} missing detection rules",
                template.identifier
            )));
        }

        Ok(())
    }

    async fn load_standards(&self) -> ConfigResult<McpStandards> {
        let mut standards = McpStandards::default();
        let root = self.template_root.standards_dir();

        if !root.exists() {
            return Ok(standards);
        }

        for entry in WalkDir::new(&root)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.into_path();
            let revision = path
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_string();

            if revision.is_empty() {
                continue;
            }

            let content = tokio::fs::read_to_string(&path).await.map_err(|err| {
                ConfigError::TemplateParseError(format!("Failed to read standard file {}: {}", path.display(), err))
            })?;

            let value = serde_json::from_str(&content).map_err(|err| {
                ConfigError::TemplateParseError(format!("Failed to parse standard file {}: {}", path.display(), err))
            })?;

            standards.revisions.insert(revision, value);
        }

        Ok(standards)
    }
}

#[async_trait]
impl ClientConfigSource for FileTemplateSource {
    async fn list_client(&self) -> ConfigResult<Vec<ClientTemplate>> {
        let guard = self.template_index.read().await;
        Ok(guard.values().map(|entry| entry.template.clone()).collect())
    }

    async fn get_template(
        &self,
        client_id: &str,
        _platform: &str,
    ) -> ConfigResult<Option<ClientTemplate>> {
        let guard = self.template_index.read().await;
        Ok(guard.get(client_id).map(|entry| entry.template.clone()))
    }

    async fn get_config_path(
        &self,
        client_id: &str,
        platform: &str,
    ) -> ConfigResult<Option<String>> {
        let guard = self.template_index.read().await;
        let Some(entry) = guard.get(client_id) else {
            return Ok(None);
        };

        let Some(rules) = entry.template.platform_rules(platform) else {
            return Ok(None);
        };

        for rule in rules {
            let candidate = match rule.method {
                DetectionMethod::FilePath | DetectionMethod::ConfigPath => {
                    rule.config_path.as_ref().or(Some(&rule.value))
                }
                DetectionMethod::BundleId => None,
            };

            if let Some(path) = candidate {
                let resolved = self
                    .path_service
                    .resolve_user_path(path)
                    .map_err(|err| ConfigError::PathResolutionError(err.to_string()))?;
                return Ok(Some(resolved.to_string_lossy().to_string()));
            }
        }

        Ok(None)
    }

    async fn reload(&self) -> ConfigResult<()> {
        let backup = self.template_index.read().await.clone();
        let standards_backup = {
            let guard = self.standards.read().await;
            guard.clone()
        };

        match (self.load_templates().await, self.load_standards().await) {
            (Ok(new_index), Ok(new_standards)) => {
                *self.template_index.write().await = new_index;
                *self.standards.write().await = new_standards;
                Ok(())
            }
            (Err(err), _) | (_, Err(err)) => {
                *self.template_index.write().await = backup;
                *self.standards.write().await = standards_backup;
                Err(err)
            }
        }
    }
}

#[async_trait]
impl ClientConfigSource for DbTemplateSource {
    async fn list_client(&self) -> ConfigResult<Vec<ClientTemplate>> {
        self.load_templates().await
    }

    async fn get_template(
        &self,
        client_id: &str,
        _platform: &str,
    ) -> ConfigResult<Option<ClientTemplate>> {
        let row = sqlx::query_scalar::<_, String>(
            &format!(
                "SELECT payload_json FROM {} WHERE identifier = ?",
                tables::CLIENT_TEMPLATE_RUNTIME
            ),
        )
        .bind(client_id)
        .fetch_optional(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

        row.map(|payload_json| {
            serde_json::from_str::<ClientTemplate>(&payload_json)
                .map_err(|err| ConfigError::TemplateParseError(format!("Failed to parse runtime template payload: {}", err)))
        })
        .transpose()
    }

    async fn get_config_path(
        &self,
        client_id: &str,
        _platform: &str,
    ) -> ConfigResult<Option<String>> {
        let path = sqlx::query_scalar::<_, String>(
            &format!(
                "SELECT config_path FROM {} WHERE identifier = ? AND config_path IS NOT NULL AND TRIM(config_path) <> ''",
                tables::CLIENT
            ),
        )
        .bind(client_id)
        .fetch_optional(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

        path.map(|raw| {
            self.path_service
                .resolve_user_path(&raw)
                .map(|resolved| resolved.to_string_lossy().to_string())
                .map_err(|err| ConfigError::PathResolutionError(err.to_string()))
        })
        .transpose()
    }

    async fn reload(&self) -> ConfigResult<()> {
        Ok(())
    }
}
