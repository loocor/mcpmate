#[cfg(feature = "kv-cherry")]
use cherry_db_manager::{CherryDbManager, DefaultCherryDbManager, McpConfigRequest, ServerRequest};
#[cfg(feature = "kv-cherry")]
use rusty_leveldb::{DB, LdbIterator, Options};

use crate::clients::error::{ConfigError, ConfigResult};
use crate::clients::models::{BackupPolicySetting, ClientTemplate, StorageKind};
use crate::clients::source::ClientConfigSource;
use crate::clients::storage::{BackupFile, ConfigStorage, DynConfigStorage};
use crate::clients::utils::get_nested_value;
use crate::system::paths::get_path_service;
use async_trait::async_trait;
use chrono::Utc;
use nanoid::nanoid;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;

pub struct CherryKvStorage {
    config_source: Arc<dyn ClientConfigSource>,
}

impl CherryKvStorage {
    pub fn new(config_source: Arc<dyn ClientConfigSource>) -> Self {
        Self { config_source }
    }

    fn current_platform() -> &'static str {
        #[cfg(target_os = "macos")]
        {
            "macos"
        }
        #[cfg(target_os = "windows")]
        {
            "windows"
        }
        #[cfg(target_os = "linux")]
        {
            "linux"
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            "unknown"
        }
    }

    async fn resolve_db_dir(
        &self,
        template: &ClientTemplate,
    ) -> ConfigResult<PathBuf> {
        let platform = Self::current_platform();
        let path = self
            .config_source
            .get_config_path(&template.identifier, platform)
            .await?
            .ok_or_else(|| {
                ConfigError::PathResolutionError(format!(
                    "Failed to resolve Cherry DB path for {}",
                    template.identifier
                ))
            })?;

        let path_service = get_path_service();
        let resolved = path_service
            .resolve_user_path(&path)
            .map_err(|e| ConfigError::PathResolutionError(e.to_string()))?;
        Ok(resolved)
    }

    fn backups_dir_for(identifier: &str) -> ConfigResult<PathBuf> {
        let home = dirs::home_dir().ok_or_else(|| ConfigError::PathResolutionError("No home directory".into()))?;
        Ok(home.join(".mcpmate").join("backups").join("client").join(identifier))
    }

    async fn ensure_backups_dir(identifier: &str) -> ConfigResult<PathBuf> {
        let dir = Self::backups_dir_for(identifier)?;
        if !dir.exists() {
            fs::create_dir_all(&dir).await.map_err(ConfigError::IoError)?;
        }
        Ok(dir)
    }

    fn make_backup_name() -> String {
        let ts = Utc::now().format("%Y%m%d%H%M%S");
        format!("config-{}-{}.bak", ts, nanoid!(6))
    }

    async fn write_snapshot(
        identifier: &str,
        content: &str,
    ) -> ConfigResult<PathBuf> {
        let dir = Self::ensure_backups_dir(identifier).await?;
        let name = Self::make_backup_name();
        let path = dir.join(name);
        fs::write(&path, content.as_bytes())
            .await
            .map_err(ConfigError::IoError)?;
        Ok(path)
    }

    async fn prune_snapshots(
        identifier: &str,
        retention: usize,
    ) -> ConfigResult<()> {
        let dir = Self::ensure_backups_dir(identifier).await?;
        let mut entries = fs::read_dir(&dir).await.map_err(ConfigError::IoError)?;
        let mut files = Vec::new();
        while let Some(entry) = entries.next_entry().await.map_err(ConfigError::IoError)? {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("bak") {
                files.push(path);
            }
        }
        if files.len() <= retention {
            return Ok(());
        }
        files.sort();
        let remove = files.len() - retention;
        for path in files.into_iter().take(remove) {
            let _ = fs::remove_file(path).await;
        }
        Ok(())
    }

    #[cfg(feature = "kv-cherry")]
    fn map_db_err(e: cherry_db_manager::CherryDbError) -> ConfigError {
        match e {
            cherry_db_manager::CherryDbError::DatabaseError(msg) => {
                if msg.to_ascii_lowercase().contains("lock") {
                    ConfigError::FileOperationError(
                        "Cherry Studio database is locked; please close Cherry Studio and retry".into(),
                    )
                } else {
                    ConfigError::FileOperationError(msg)
                }
            }
            other => ConfigError::FileOperationError(format!("Cherry DB error: {:?}", other)),
        }
    }

    #[cfg(feature = "kv-cherry")]
    fn decode_utf16_le_bytes(bytes: &[u8]) -> Option<serde_json::Value> {
        if bytes.is_empty() {
            return None;
        }
        let utf16_bytes = &bytes[1..];
        if utf16_bytes.len() % 2 != 0 {
            return None;
        }
        let utf16: Vec<u16> = utf16_bytes
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        let s = String::from_utf16(&utf16).ok()?;
        serde_json::from_str(&s).ok()
    }

    #[cfg(feature = "kv-cherry")]
    fn tolerant_read_mcp_json(db_dir: &str) -> ConfigResult<Option<serde_json::Value>> {
        if !std::path::Path::new(db_dir).exists() {
            return Ok(None);
        }
        let options = Options {
            create_if_missing: false,
            ..Default::default()
        };
        let mut db = DB::open(db_dir, options)
            .map_err(|e| Self::map_db_err(cherry_db_manager::CherryDbError::DatabaseError(format!("{:?}", e))))?;
        let mut it = db
            .new_iter()
            .map_err(|e| Self::map_db_err(cherry_db_manager::CherryDbError::DatabaseError(format!("{:?}", e))))?;
        while let Some((_k, v)) = it.next() {
            if let Some(json) = Self::decode_utf16_le_bytes(&v) {
                if let Some(mcp_str) = json.get("mcp").and_then(|v| v.as_str()) {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(mcp_str) {
                        return Ok(Some(parsed));
                    }
                }
            }
        }
        Ok(None)
    }

    #[cfg(feature = "kv-cherry")]
    fn encode_json_to_bytes(json: &serde_json::Value) -> Vec<u8> {
        let s = json.to_string();
        let utf16: Vec<u16> = s.encode_utf16().collect();
        let mut out = Vec::with_capacity(1 + utf16.len() * 2);
        out.push(0x00);
        for u in utf16 {
            out.extend_from_slice(&u.to_le_bytes());
        }
        out
    }

    #[cfg(feature = "kv-cherry")]
    fn tolerant_write_mcp_json(
        db_dir: &str,
        requests: &[ServerRequest],
    ) -> ConfigResult<()> {
        use serde_json::json;
        let options = Options {
            create_if_missing: false,
            ..Default::default()
        };
        let mut db = DB::open(db_dir, options)
            .map_err(|e| Self::map_db_err(cherry_db_manager::CherryDbError::DatabaseError(format!("{:?}", e))))?;
        let mut it = db
            .new_iter()
            .map_err(|e| Self::map_db_err(cherry_db_manager::CherryDbError::DatabaseError(format!("{:?}", e))))?;
        let (mut key, mut outer): (Option<Vec<u8>>, Option<serde_json::Value>) = (None, None);
        while let Some((k, v)) = it.next() {
            if let Some(j) = Self::decode_utf16_le_bytes(&v) {
                key = Some(k);
                outer = Some(j);
                break;
            }
        }
        let key = key.ok_or_else(|| ConfigError::FileOperationError("Cherry DB entry not found".into()))?;
        let mut json_data = outer.ok_or_else(|| ConfigError::FileOperationError("Cherry DB JSON not found".into()))?;
        let servers: Vec<serde_json::Value> = requests.iter().map(|s| json!({
            "id": s.id, "isActive": s.is_active, "args": s.args, "command": s.command, "type": s.server_type, "name": s.name
        })).collect();
        let updated = json!({"servers": servers});
        json_data["mcp"] = serde_json::Value::String(serde_json::to_string(&updated).map_err(ConfigError::JsonError)?);
        let encoded = Self::encode_json_to_bytes(&json_data);
        db.put(&key, &encoded)
            .map_err(|e| Self::map_db_err(cherry_db_manager::CherryDbError::DatabaseError(format!("{:?}", e))))?;
        Ok(())
    }
}

#[async_trait]
impl ConfigStorage for CherryKvStorage {
    fn kind(&self) -> StorageKind {
        StorageKind::Kv
    }

    async fn read(
        &self,
        template: &ClientTemplate,
    ) -> ConfigResult<Option<String>> {
        #[cfg(not(feature = "kv-cherry"))]
        {
            let _ = template;
            return Err(ConfigError::StorageAdapterMissing(
                "kv-cherry feature is disabled".into(),
            ));
        }

        #[cfg(feature = "kv-cherry")]
        {
            let db_dir = match self.resolve_db_dir(template).await {
                Ok(p) => p,
                Err(ConfigError::PathResolutionError(_)) => return Ok(None),
                Err(e) => return Err(e),
            };
            let path_service = get_path_service();
            if !path_service.validate_path_exists(&db_dir).await.unwrap_or(false) {
                return Ok(None);
            }
            let manager = DefaultCherryDbManager::new();
            match manager.read_mcp_config(db_dir.to_string_lossy().as_ref()) {
                Ok(resp) => {
                    let value = serde_json::json!({"servers": resp.servers});
                    let mut s = serde_json::to_string_pretty(&value).map_err(ConfigError::JsonError)?;
                    s = s.replace("\\/", "/");
                    Ok(Some(s))
                }
                Err(cherry_db_manager::CherryDbError::ConfigNotFound) => {
                    Ok(Some("{\n  \"servers\": []\n}".to_string()))
                }
                Err(cherry_db_manager::CherryDbError::JsonError(_)) => {
                    if let Some(mcp) = Self::tolerant_read_mcp_json(db_dir.to_string_lossy().as_ref())? {
                        let mut s = serde_json::to_string_pretty(&mcp).map_err(ConfigError::JsonError)?;
                        s = s.replace("\\/", "/");
                        Ok(Some(s))
                    } else {
                        Ok(Some("{\n  \"servers\": []\n}".to_string()))
                    }
                }
                Err(e) => {
                    let mapped = Self::map_db_err(e);
                    if let ConfigError::FileOperationError(ref msg) = mapped {
                        if msg.to_ascii_lowercase().contains("locked") {
                            tracing::warn!("Cherry DB locked during read; return None for listing");
                            return Ok(None);
                        }
                    }
                    Err(mapped)
                }
            }
        }
    }

    async fn write_atomic(
        &self,
        template: &ClientTemplate,
        content: &str,
        policy: &BackupPolicySetting,
    ) -> ConfigResult<Option<String>> {
        #[cfg(not(feature = "kv-cherry"))]
        {
            let _ = (template, content, policy);
            return Err(ConfigError::StorageAdapterMissing(
                "kv-cherry feature is disabled".into(),
            ));
        }

        #[cfg(feature = "kv-cherry")]
        {
            let db_dir = self.resolve_db_dir(template).await?;
            let manager = DefaultCherryDbManager::new();
            let mut backup_path: Option<String> = None;
            if policy.should_backup() {
                if let Some(prev) = self.read(template).await? {
                    let path = Self::write_snapshot(&template.identifier, &prev).await?;
                    if let Some(limit) = policy.retention_limit() {
                        Self::prune_snapshots(&template.identifier, limit).await?;
                    }
                    backup_path = Some(path.to_string_lossy().to_string());
                }
            }
            let doc: serde_json::Value = serde_json::from_str(content).map_err(ConfigError::JsonError)?;
            let path = template
                .config_mapping
                .container_keys
                .first()
                .map(|s| s.as_str())
                .unwrap_or("");
            let servers_val = get_nested_value(&doc, path)
                .cloned()
                .unwrap_or_else(|| serde_json::json!([]));
            let servers_arr = servers_val.as_array().cloned().unwrap_or_default();
            let mut requests: Vec<ServerRequest> = Vec::with_capacity(servers_arr.len());
            for item in servers_arr {
                let req: ServerRequest = serde_json::from_value(item).map_err(ConfigError::JsonError)?;
                requests.push(req);
            }
            let req = McpConfigRequest {
                servers: requests.clone(),
            };
            let dbp = db_dir.to_string_lossy();
            match manager.write_mcp_config(dbp.as_ref(), &req) {
                Ok(_) => {}
                Err(cherry_db_manager::CherryDbError::JsonError(_)) => {
                    Self::tolerant_write_mcp_json(dbp.as_ref(), &requests)?;
                }
                Err(e) => return Err(Self::map_db_err(e)),
            }
            Ok(backup_path)
        }
    }

    async fn list_backups(
        &self,
        template: &ClientTemplate,
    ) -> ConfigResult<Vec<BackupFile>> {
        let dir = Self::ensure_backups_dir(&template.identifier).await?;
        let mut it = fs::read_dir(&dir).await.map_err(ConfigError::IoError)?;
        let mut out = Vec::new();
        while let Some(e) = it.next_entry().await.map_err(ConfigError::IoError)? {
            let path = e.path();
            if !path.is_file() {
                continue;
            }
            let name = path.file_name().and_then(|o| o.to_str()).unwrap_or("").to_string();
            if !name.ends_with(".bak") {
                continue;
            }
            let meta = fs::metadata(&path).await.map_err(ConfigError::IoError)?;
            let modified_at = meta.modified().ok().map(chrono::DateTime::<chrono::Utc>::from);
            out.push(BackupFile {
                name,
                path: path.clone(),
                size: meta.len(),
                modified_at,
            });
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    async fn delete_backup(
        &self,
        template: &ClientTemplate,
        backup_name: &str,
    ) -> ConfigResult<()> {
        let dir = Self::ensure_backups_dir(&template.identifier).await?;
        let p = dir.join(backup_name);
        if p.exists() {
            fs::remove_file(p).await.map_err(ConfigError::IoError)?;
        }
        Ok(())
    }

    async fn restore_backup(
        &self,
        template: &ClientTemplate,
        backup_name: &str,
        policy: &BackupPolicySetting,
    ) -> ConfigResult<Option<String>> {
        let dir = Self::ensure_backups_dir(&template.identifier).await?;
        let p = dir.join(backup_name);
        if !p.exists() {
            return Err(ConfigError::FileOperationError(format!(
                "Backup {} not found for {}",
                backup_name, template.identifier
            )));
        }
        let content = fs::read_to_string(&p).await.map_err(ConfigError::IoError)?;
        self.write_atomic(template, &content, policy).await
    }
}

impl CherryKvStorage {
    pub fn as_dyn(self) -> DynConfigStorage {
        Arc::new(self)
    }
}
