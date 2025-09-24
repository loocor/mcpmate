//! Warp (SQLite) storage adapter using sqlx
#![allow(unused)]

#[cfg(feature = "warp-sqlite")]
use async_trait::async_trait;
#[cfg(feature = "warp-sqlite")]
use sqlx::sqlite::SqliteConnectOptions;
#[cfg(feature = "warp-sqlite")]
use sqlx::{Row, SqlitePool};
#[cfg(feature = "warp-sqlite")]
use std::str::FromStr;
#[cfg(feature = "warp-sqlite")]
use std::sync::Arc;

#[cfg(feature = "warp-sqlite")]
use crate::clients::error::{ConfigError, ConfigResult};
#[cfg(feature = "warp-sqlite")]
use crate::clients::models::{BackupPolicySetting, ClientTemplate, StorageKind};
#[cfg(feature = "warp-sqlite")]
use crate::clients::source::ClientConfigSource;
#[cfg(feature = "warp-sqlite")]
use crate::clients::storage::{BackupFile, ConfigStorage, DynConfigStorage};
#[cfg(feature = "warp-sqlite")]
use uuid::Uuid;

#[cfg(feature = "warp-sqlite")]
pub struct WarpSqliteStorage {
    config_source: Arc<dyn ClientConfigSource>,
}

#[cfg(feature = "warp-sqlite")]
impl WarpSqliteStorage {
    pub fn new(config_source: Arc<dyn ClientConfigSource>) -> Self {
        Self { config_source }
    }

    async fn resolve_db_path(
        &self,
        template: &ClientTemplate,
    ) -> ConfigResult<std::path::PathBuf> {
        let platform = if cfg!(target_os = "macos") {
            "macos"
        } else if cfg!(target_os = "windows") {
            "windows"
        } else {
            "linux"
        };
        let raw = self
            .config_source
            .get_config_path(&template.identifier, platform)
            .await?
            .ok_or_else(|| {
                ConfigError::PathResolutionError(format!("Failed to resolve Warp DB path for {}", template.identifier))
            })?;
        crate::system::paths::get_path_service()
            .resolve_user_path(&raw)
            .map_err(|e| ConfigError::PathResolutionError(e.to_string()))
    }

    fn backup_sqlite(src: &std::path::Path) -> ConfigResult<std::path::PathBuf> {
        let ts = chrono::Utc::now().format("%Y%m%d-%H%M%S");
        let dest = src.with_file_name(format!("warp.backup.{}.sqlite", ts));
        std::fs::copy(src, &dest).map_err(|e| ConfigError::FileOperationError(e.to_string()))?;
        Ok(dest)
    }

    async fn open_pool(path: &std::path::Path) -> ConfigResult<SqlitePool> {
        let url = format!("sqlite:{}", path.to_string_lossy());
        let opts = SqliteConnectOptions::from_str(&url)
            .map_err(|e| ConfigError::PathResolutionError(e.to_string()))?
            .read_only(false)
            .create_if_missing(true);
        SqlitePool::connect_with(opts)
            .await
            .map_err(|e| ConfigError::DataAccessError(e.to_string()))
    }
}

#[cfg(feature = "warp-sqlite")]
#[async_trait]
impl ConfigStorage for WarpSqliteStorage {
    fn kind(&self) -> StorageKind {
        StorageKind::Custom
    }

    async fn read(
        &self,
        template: &ClientTemplate,
    ) -> ConfigResult<Option<String>> {
        let path = match self.resolve_db_path(template).await {
            Ok(p) => p,
            Err(ConfigError::PathResolutionError(_)) => return Ok(None),
            Err(e) => return Err(e),
        };
        let pool = Self::open_pool(&path).await?;
        let rows = sqlx::query(
            r#"
            select g.data as data,
                   mv.environment_variables as env_json
            from generic_string_objects g
            join object_metadata om
              on om.shareable_object_id = g.id
             and om.object_type='GENERIC_STRING_JSON_MCPSERVER'
            left join mcp_environment_variables mv
              on upper(replace(json_extract(g.data,'$.uuid'),'-','')) = upper(hex(mv.mcp_server_uuid))
            order by g.id
            "#,
        )
        .fetch_all(&pool)
        .await
        .map_err(|e| {
            ConfigError::FileOperationError(format!("Warp storage read failed (possible version change): {}", e))
        })?;
        if rows.is_empty() {
            return Ok(None);
        }
        let mut list = Vec::with_capacity(rows.len());
        for r in rows {
            let s: String = r.get("data");
            let mut v = serde_json::from_str::<serde_json::Value>(&s).unwrap_or(serde_json::json!({}));
            // merge env if present
            if let Ok(Some(env_s)) = r.try_get::<Option<String>, _>("env_json") {
                if let Ok(env_v) = serde_json::from_str::<serde_json::Value>(&env_s) {
                    if let Some(obj) = v.as_object_mut() {
                        obj.insert("env".into(), env_v);
                    }
                }
            }
            list.push(v);
        }
        let value = serde_json::json!({"servers": list});
        Ok(Some(
            serde_json::to_string_pretty(&value).map_err(ConfigError::JsonError)?,
        ))
    }

    async fn write_atomic(
        &self,
        template: &ClientTemplate,
        content: &str,
        policy: &BackupPolicySetting,
    ) -> ConfigResult<Option<String>> {
        let path = self.resolve_db_path(template).await?;
        let pool = Self::open_pool(&path).await?;
        let mut tx = pool
            .begin()
            .await
            .map_err(|e| ConfigError::FileOperationError(format!("Warp storage write failed to begin tx: {}", e)))?;

        let mut backup_path: Option<String> = None;
        if policy.should_backup() {
            let dest = Self::backup_sqlite(&path)?;
            backup_path = Some(dest.to_string_lossy().to_string());
        }

        // Replace-all strategy: clear active/env for old UUIDs if tables exist, then clear OM/GSO
        // (best-effort: ignore errors when tables are absent)
        let _ = sqlx::query("delete from active_mcp_servers").execute(&mut *tx).await;
        let _ = sqlx::query("delete from mcp_environment_variables")
            .execute(&mut *tx)
            .await;
        sqlx::query("delete from object_metadata where object_type='GENERIC_STRING_JSON_MCPSERVER'")
            .execute(&mut *tx)
            .await
            .map_err(|e| ConfigError::FileOperationError(format!("Warp storage write failed (schema?): {}", e)))?;
        // Clean up unreferenced rows (best-effort)
        sqlx::query(
            "delete from generic_string_objects where id not in (select shareable_object_id from object_metadata)",
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| ConfigError::FileOperationError(format!("Warp storage cleanup failed: {}", e)))?;

        let doc: serde_json::Value = serde_json::from_str(content).map_err(ConfigError::JsonError)?;
        let servers = doc
            .get("servers")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        for mut v in servers {
            // Ensure stable uuid field
            let uuid_text = v
                .get("uuid")
                .and_then(|x| x.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
                .map(|u| u.to_string())
                .or_else(|| {
                    let name = v.get("name").and_then(|x| x.as_str()).unwrap_or("");
                    let input = format!("warp://{}/{}", template.identifier, name);
                    Some(Uuid::new_v5(&Uuid::NAMESPACE_URL, input.as_bytes()).to_string())
                })
                .unwrap();
            if v.get("uuid").and_then(|x| x.as_str()).is_none() {
                if let Some(obj) = v.as_object_mut() {
                    obj.insert("uuid".into(), serde_json::Value::String(uuid_text.clone()));
                }
            }
            let data_json = serde_json::to_string(&v).map_err(ConfigError::JsonError)?;
            sqlx::query("insert into generic_string_objects(data) values (?1)")
                .bind(&data_json)
                .execute(&mut *tx)
                .await
                .map_err(|e| ConfigError::FileOperationError(format!("Warp storage write failed: {}", e)))?;
            let row = sqlx::query("select last_insert_rowid() as id")
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| ConfigError::FileOperationError(format!("Warp storage last_rowid failed: {}", e)))?;
            let gso_id: i64 = row.get("id");
            sqlx::query("insert into object_metadata(is_pending, object_type, shareable_object_id, retry_count) values (0,'GENERIC_STRING_JSON_MCPSERVER',?1,0)")
                .bind(gso_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| ConfigError::FileOperationError(format!("Warp storage tag failed: {}", e)))?;

            // Upsert environment variables for this server by UUID
            let uuid = Uuid::parse_str(&uuid_text)
                .map_err(|e| ConfigError::FileOperationError(format!("Warp uuid parse failed: {}", e)))?;
            let uuid_bytes: &[u8] = uuid.as_bytes();
            let env_value = v.get("env").cloned().unwrap_or_else(|| serde_json::json!({}));
            let env_json = serde_json::to_string(&env_value).map_err(ConfigError::JsonError)?;
            let _ = sqlx::query(
                "insert into mcp_environment_variables(mcp_server_uuid, environment_variables) values (?1, ?2) \
                 on conflict(mcp_server_uuid) do update set environment_variables=excluded.environment_variables",
            )
            .bind(uuid_bytes)
            .bind(&env_json)
            .execute(&mut *tx)
            .await;

            // Activate all servers by default; ignore errors if table absent
            let _ = sqlx::query("insert or ignore into active_mcp_servers(mcp_server_uuid) values (?1)")
                .bind(uuid_text.as_str())
                .execute(&mut *tx)
                .await;
        }

        tx.commit()
            .await
            .map_err(|e| ConfigError::FileOperationError(format!("Warp storage commit failed: {}", e)))?;
        // Ensure changes visible to readers when in WAL
        let _ = sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)").execute(&pool).await;
        let _ = sqlx::query("PRAGMA optimize").execute(&pool).await;
        Ok(backup_path)
    }

    async fn list_backups(
        &self,
        template: &ClientTemplate,
    ) -> ConfigResult<Vec<BackupFile>> {
        let path = self.resolve_db_path(template).await?;
        let parent = path.parent().unwrap_or_else(|| std::path::Path::new("."));
        let mut out = Vec::new();
        if let Ok(rd) = std::fs::read_dir(parent) {
            for entry in rd.flatten() {
                let p = entry.path();
                let fname = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if !fname.starts_with("warp.backup.") || !fname.ends_with(".sqlite") {
                    continue;
                }
                let meta = match entry.metadata() {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                out.push(BackupFile {
                    name: fname.to_string(),
                    path: p.clone(),
                    size: meta.len(),
                    modified_at: meta.modified().ok().map(chrono::DateTime::<chrono::Utc>::from),
                });
            }
        }
        Ok(out)
    }

    async fn delete_backup(
        &self,
        template: &ClientTemplate,
        backup_name: &str,
    ) -> ConfigResult<()> {
        let path = self.resolve_db_path(template).await?;
        let p = path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join(backup_name);
        if p.exists() {
            std::fs::remove_file(p).map_err(|e| ConfigError::FileOperationError(e.to_string()))?;
        }
        Ok(())
    }

    async fn restore_backup(
        &self,
        template: &ClientTemplate,
        backup_name: &str,
        _policy: &BackupPolicySetting,
    ) -> ConfigResult<Option<String>> {
        let path = self.resolve_db_path(template).await?;
        let p = path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join(backup_name);
        if !p.exists() {
            return Err(ConfigError::FileOperationError(format!(
                "Backup {} not found",
                backup_name
            )));
        }
        std::fs::copy(&p, &path).map_err(|e| ConfigError::FileOperationError(e.to_string()))?;
        Ok(Some(p.to_string_lossy().to_string()))
    }
}

#[cfg(feature = "warp-sqlite")]
impl WarpSqliteStorage {
    pub fn as_dyn(self) -> DynConfigStorage {
        Arc::new(self)
    }
}
