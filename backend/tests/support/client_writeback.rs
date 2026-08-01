use std::{collections::HashMap, sync::Arc};

use mcpmate::clients::models::{
    ClientTemplate, ConfigMapping, FormatRule, MergeStrategy, StorageConfig, StorageKind, TemplateFormat,
};
use mcpmate::clients::{ClientConfigService, ClientRenderOptions, ConfigMode, DbTemplateSource};
use mcpmate_capability_store::SqliteCapabilityCatalog;
use serde_json::{Value, json};
use sqlx::SqlitePool;
use sqlx::sqlite::SqlitePoolOptions;
use tempfile::TempDir;

pub const CLIENT_ID: &str = "writeback-client";

pub struct ClientWritebackFixture {
    _temp_dir: TempDir,
    pub pool: SqlitePool,
    pub service: Arc<ClientConfigService>,
    pub config_path: std::path::PathBuf,
}

impl ClientWritebackFixture {
    pub async fn new() -> Self {
        Self::new_with_template_strategy(MergeStrategy::Replace).await
    }

    pub async fn new_with_template_strategy(template_merge_strategy: MergeStrategy) -> Self {
        let temp_dir = TempDir::new().expect("create temp dir");
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("create database");
        mcpmate::config::client::init::initialize_client_table(&pool)
            .await
            .expect("initialize clients");
        mcpmate::config::server::init::initialize_server_tables(&pool)
            .await
            .expect("initialize servers");
        SqliteCapabilityCatalog::new(pool.clone())
            .ensure_schema()
            .await
            .expect("initialize capability schema");
        mcpmate::config::profile::init::initialize_profile_tables(&pool)
            .await
            .expect("initialize profiles");

        let config_path = temp_dir.path().join("client.json");
        tokio::fs::write(
            &config_path,
            r#"{"mcpServers":{"plugin-owned":{"command":"plugin-command"}}}"#,
        )
        .await
        .expect("write client config");

        let transports = HashMap::from([(
            "streamable_http".to_string(),
            FormatRule {
                template: json!({"url": "{{{url}}}"}),
                selected: Some(true),
                ..FormatRule::default()
            },
        )]);
        let template = ClientTemplate {
            identifier: CLIENT_ID.to_string(),
            display_name: Some("Writeback Client".to_string()),
            format: TemplateFormat::Json,
            storage: StorageConfig {
                kind: StorageKind::File,
                path_strategy: Some("config_path".to_string()),
                adapter: None,
            },
            config_mapping: ConfigMapping {
                container_keys: vec!["mcpServers".to_string()],
                container_type: mcpmate::clients::ContainerType::ObjectMap,
                merge_strategy: template_merge_strategy,
                keep_original_config: false,
                managed_endpoint: None,
                managed_source: Some("profile".to_string()),
                parse: None,
                format_rules: transports.clone(),
            },
            ..ClientTemplate::default()
        };
        sqlx::query(
            "INSERT INTO client_template_runtime (identifier, payload_json, updated_at) \
             VALUES (?, ?, CURRENT_TIMESTAMP)",
        )
        .bind(&template.identifier)
        .bind(serde_json::to_string(&template).expect("serialize template"))
        .execute(&pool)
        .await
        .expect("insert template");

        sqlx::query(
            r#"
            INSERT INTO client (
                id, identifier, name, display_name, config_mode, approval_status,
                config_path, connection_mode, attachment_state, template_identifier,
                config_format, container_type, container_keys, storage_kind,
                storage_path_strategy, merge_strategy, keep_original_config,
                managed_source, transports
            )
            VALUES (
                'writeback-consumer', ?, 'Writeback Client', 'Writeback Client',
                'unify', 'approved', ?, 'local_config_detected', 'attached', ?,
                'json', 'object', '["mcpServers"]', 'file', 'config_path',
                ?, 0, 'profile', ?
            )
            "#,
        )
        .bind(CLIENT_ID)
        .bind(config_path.to_string_lossy().to_string())
        .bind(CLIENT_ID)
        .bind(match template_merge_strategy {
            MergeStrategy::Replace => "replace",
            MergeStrategy::DeepMerge => "deep_merge",
        })
        .bind(serde_json::to_string(&transports).expect("serialize transports"))
        .execute(&pool)
        .await
        .expect("insert client");

        let source = Arc::new(DbTemplateSource::new(Arc::new(pool.clone())).expect("create template source"));
        let service = Arc::new(
            ClientConfigService::with_source(Arc::new(pool.clone()), source)
                .await
                .expect("create client service"),
        );

        Self {
            _temp_dir: temp_dir,
            pool,
            service,
            config_path,
        }
    }

    pub async fn apply_managed(
        &self,
        dry_run: bool,
    ) -> mcpmate::clients::service::ApplyOutcome {
        self.apply_managed_result(dry_run).await.expect("apply managed config")
    }

    pub async fn apply_managed_result(
        &self,
        dry_run: bool,
    ) -> Result<mcpmate::clients::service::ApplyOutcome, mcpmate::clients::ConfigError> {
        self.service
            .apply_with_deferred(ClientRenderOptions {
                client_id: CLIENT_ID.to_string(),
                mode: ConfigMode::Managed,
                profile_id: None,
                server_ids: None,
                dry_run,
            })
            .await
    }

    pub async fn config(&self) -> Value {
        let content = tokio::fs::read_to_string(&self.config_path)
            .await
            .expect("read client config");
        serde_json::from_str(&content).expect("parse client config")
    }
}
