use std::collections::HashMap;
use std::sync::Arc;

use handlebars::{
    Context as HbContext, Handlebars, Helper, HelperResult, Output, RenderContext, RenderError, RenderErrorReason,
};
use serde_json::{Map, Value};

use crate::clients::error::{ConfigError, ConfigResult};
use crate::clients::models::{
    BackupPolicySetting, ClientTemplate, ConfigMode, ContainerType, ServerTemplateInput, StorageKind, TemplateFormat,
};
use crate::clients::renderer::{ConfigDiff, DynConfigRenderer};
use crate::clients::source::ClientConfigSource;
use crate::clients::storage::{DynConfigStorage, FileConfigStorage};
use crate::common::get_bridge_path;
use crate::system::config::get_runtime_port_config;

/// Configuration rendering result
#[derive(Debug, Clone)]
pub enum TemplateExecutionResult {
    Applied {
        backup_path: Option<String>,
        content: String,
    },
    DryRun {
        diff: ConfigDiff,
        content: String,
    },
}

/// Template rendering request
pub struct RenderRequest<'a> {
    pub client_id: &'a str,
    pub servers: &'a [ServerTemplateInput],
    pub mode: ConfigMode,
    pub profile_id: Option<&'a str>,
    pub dry_run: bool,
    pub backup_policy: &'a BackupPolicySetting,
}

/// Template engine, responsible for coordinating template, renderer and storage adapter
pub struct TemplateEngine {
    handlebars: Handlebars<'static>,
    config_source: Arc<dyn ClientConfigSource>,
    renderers: HashMap<TemplateFormat, DynConfigRenderer>,
    storages: HashMap<StorageKind, DynConfigStorage>,
}

impl TemplateEngine {
    pub fn new(config_source: Arc<dyn ClientConfigSource>) -> Self {
        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(true);
        register_builtin_helpers(&mut handlebars);

        Self {
            handlebars,
            config_source,
            renderers: HashMap::new(),
            storages: HashMap::new(),
        }
    }

    /// Build a template engine with default renderer and file storage
    pub fn with_defaults(config_source: Arc<dyn ClientConfigSource>) -> Self {
        let mut engine = Self::new(config_source.clone());
        engine.register_renderer(crate::clients::renderer::StructuredRenderer::new(TemplateFormat::Json));
        engine.register_renderer(crate::clients::renderer::StructuredRenderer::new(TemplateFormat::Json5));
        engine.register_renderer(crate::clients::renderer::StructuredRenderer::new(TemplateFormat::Toml));
        engine.register_renderer(crate::clients::renderer::StructuredRenderer::new(TemplateFormat::Yaml));
        engine.register_storage(Arc::new(FileConfigStorage::new(config_source)));
        engine
    }

    pub fn register_renderer(
        &mut self,
        renderer: DynConfigRenderer,
    ) {
        self.renderers.insert(renderer.format(), renderer);
    }

    pub fn register_storage(
        &mut self,
        storage: DynConfigStorage,
    ) {
        self.storages.insert(storage.kind(), storage);
    }

    pub fn handlebars(&self) -> &Handlebars<'static> {
        &self.handlebars
    }

    pub fn handlebars_mut(&mut self) -> &mut Handlebars<'static> {
        &mut self.handlebars
    }

    pub fn config_source(&self) -> &Arc<dyn ClientConfigSource> {
        &self.config_source
    }

    fn resolve_renderer(
        &self,
        template: &ClientTemplate,
    ) -> ConfigResult<DynConfigRenderer> {
        self.renderers
            .get(&template.format)
            .cloned()
            .ok_or_else(|| ConfigError::RendererMissing(template.format.as_str().to_string()))
    }

    fn resolve_storage(
        &self,
        template: &ClientTemplate,
    ) -> ConfigResult<DynConfigStorage> {
        self.storages
            .get(&template.storage.kind)
            .cloned()
            .ok_or_else(|| ConfigError::StorageAdapterMissing(format!("{:?}", template.storage.kind)))
    }

    pub(crate) fn storage_for_template(
        &self,
        template: &ClientTemplate,
    ) -> ConfigResult<DynConfigStorage> {
        self.resolve_storage(template)
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

    async fn get_client_template(
        &self,
        client_id: &str,
    ) -> ConfigResult<ClientTemplate> {
        let platform = Self::current_platform();
        self.config_source
            .get_template(client_id, platform)
            .await?
            .ok_or_else(|| {
                ConfigError::TemplateIndexError(format!("Client {} not found in platform {}", client_id, platform))
            })
    }

    pub async fn render_config(
        &self,
        request: RenderRequest<'_>,
    ) -> ConfigResult<TemplateExecutionResult> {
        let template = self.get_client_template(request.client_id).await?;
        let renderer = self.resolve_renderer(&template)?;
        let storage = self.resolve_storage(&template)?;

        let fragment = match request.mode {
            ConfigMode::Native => self.render_native_config(&template, request.servers)?,
            ConfigMode::Managed => self.render_managed_config(&template, request.profile_id)?,
        };

        let existing = storage.read(&template).await?.unwrap_or_default();
        let merged = renderer.merge(&existing, &fragment, &template)?;

        if request.dry_run {
            let diff = renderer.diff(&existing, &merged)?;
            Ok(TemplateExecutionResult::DryRun { diff, content: merged })
        } else {
            let backup_path = storage.write_atomic(&template, &merged, request.backup_policy).await?;
            Ok(TemplateExecutionResult::Applied {
                backup_path,
                content: merged,
            })
        }
    }

    fn render_native_config(
        &self,
        template: &ClientTemplate,
        servers: &[ServerTemplateInput],
    ) -> ConfigResult<Value> {
        self.render_container(template, servers)
    }

    fn render_managed_config(
        &self,
        template: &ClientTemplate,
        profile_id: Option<&str>,
    ) -> ConfigResult<Value> {
        let managed = self.build_managed_server(template, profile_id)?;
        self.render_container(template, &[managed])
    }

    fn render_container(
        &self,
        template: &ClientTemplate,
        servers: &[ServerTemplateInput],
    ) -> ConfigResult<Value> {
        match template.config_mapping.container_type {
            ContainerType::ObjectMap => self.render_object_map(template, servers),
            ContainerType::Mixed => self.render_object_map(template, servers),
            ContainerType::Array => self.render_array(template, servers),
        }
    }

    fn render_object_map(
        &self,
        template: &ClientTemplate,
        servers: &[ServerTemplateInput],
    ) -> ConfigResult<Value> {
        let mut container = Map::new();
        for server in servers {
            let server_config = self.render_server_config(template, server)?;
            container.insert(server.name.clone(), server_config);
        }
        Ok(Value::Object(container))
    }

    fn render_array(
        &self,
        template: &ClientTemplate,
        servers: &[ServerTemplateInput],
    ) -> ConfigResult<Value> {
        let mut items = Vec::new();
        for server in servers {
            let mut config = self.render_server_config(template, server)?;
            if let Value::Object(ref mut map) = config {
                if !map.contains_key("name") {
                    map.insert("name".to_string(), Value::String(server.name.clone()));
                }
            }
            items.push(config);
        }
        Ok(Value::Array(items))
    }

    fn render_server_config(
        &self,
        template: &ClientTemplate,
        server: &ServerTemplateInput,
    ) -> ConfigResult<Value> {
        let format_rule = template
            .config_mapping
            .format_rules
            .get(&server.transport)
            .ok_or_else(|| {
                ConfigError::TemplateParseError(format!(
                    "Client {} missing format rule for transport {}",
                    template.identifier, server.transport
                ))
            })?;

        let context = serde_json::to_value(server)?;
        let mut rendered = self.render_value(&format_rule.template, &context)?;

        if format_rule.requires_type_field {
            if let Value::Object(ref mut obj) = rendered {
                obj.entry("type".to_string())
                    .or_insert_with(|| Value::String(server.transport.clone()));
            }
        }

        Ok(rendered)
    }

    fn render_value(
        &self,
        template_value: &Value,
        context: &Value,
    ) -> ConfigResult<Value> {
        match template_value {
            Value::String(template_str) => {
                if template_str.contains("{{") {
                    let rendered = self.handlebars.render_template(template_str, context)?;
                    if let Ok(parsed) = serde_json::from_str(&rendered) {
                        Ok(parsed)
                    } else {
                        Ok(Value::String(rendered))
                    }
                } else {
                    Ok(Value::String(template_str.clone()))
                }
            }
            Value::Object(map) => {
                let mut rendered = Map::new();
                for (key, value) in map {
                    rendered.insert(key.clone(), self.render_value(value, context)?);
                }
                Ok(Value::Object(rendered))
            }
            Value::Array(items) => {
                let mut rendered = Vec::new();
                for item in items {
                    rendered.push(self.render_value(item, context)?);
                }
                Ok(Value::Array(rendered))
            }
            other => Ok(other.clone()),
        }
    }

    fn build_managed_server(
        &self,
        template: &ClientTemplate,
        profile_id: Option<&str>,
    ) -> ConfigResult<ServerTemplateInput> {
        // Derive supported transports directly from format_rules keys and
        // apply fixed global priority: streamable_http -> sse -> stdio.
        let candidates = derive_transports_by_priority(&template.config_mapping.format_rules);
        for transport in candidates {
            if let Some(server) = self.managed_runtime_for_transport(
                transport,
                transport,
                template.identifier.as_str(),
                profile_id,
            )? {
                return Ok(server);
            }
        }

        Err(ConfigError::TemplateParseError(format!(
            "Client {} managed mode missing available transport or format rule",
            template.identifier
        )))
    }

    fn managed_runtime_for_transport(
        &self,
        transport: &str,
        effective_transport: &str,
        client_id: &str,
        profile_id: Option<&str>,
    ) -> ConfigResult<Option<ServerTemplateInput>> {
        let runtime_config = get_runtime_port_config();

        match transport {
            "streamable_http" => Ok(Some(ServerTemplateInput {
                name: "mcpmate".to_string(),
                display_name: Some("MCPMate Proxy".to_string()),
                transport: effective_transport.to_string(),
                command: None,
                args: Vec::new(),
                env: HashMap::new(),
                url: Some(runtime_config.mcp_http_url()),
                headers: HashMap::new(),
                metadata: HashMap::new(),
            })),
            "sse" => Ok(Some(ServerTemplateInput {
                name: "mcpmate".to_string(),
                display_name: Some("MCPMate Proxy".to_string()),
                transport: effective_transport.to_string(),
                command: None,
                args: Vec::new(),
                env: HashMap::new(),
                url: Some(runtime_config.mcp_sse_url()),
                headers: HashMap::new(),
                metadata: HashMap::new(),
            })),
            "stdio" => {
                let bridge_path = get_bridge_path().map_err(|err| {
                    ConfigError::TemplateParseError(format!(
                        "Failed to locate MCP bridge executable for client {}: {}",
                        client_id, err
                    ))
                })?;

                let mut env = HashMap::new();
                env.insert("APPID".to_string(), client_id.to_string());
                if let Some(pid) = profile_id {
                    env.insert("PROFILE_ID".to_string(), pid.to_string());
                }

                let sse_url = format!("{}?client_id={}", runtime_config.mcp_sse_url(), client_id);

                let sanitized_client: String = client_id
                    .chars()
                    .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
                    .collect();
                let bridge_name = format!("mcpmate_bridge_{}", sanitized_client);

                Ok(Some(ServerTemplateInput {
                    name: bridge_name,
                    display_name: Some("MCPMate Bridge".to_string()),
                    transport: effective_transport.to_string(),
                    command: Some(bridge_path),
                    args: vec!["--sse-url".to_string(), sse_url],
                    env,
                    url: None,
                    headers: HashMap::new(),
                    metadata: HashMap::new(),
                }))
            }
            _ => Ok(None),
        }
    }
}

const TRANSPORT_PRIORITY: &[&str] = &["streamable_http", "sse", "stdio"];

fn derive_transports_by_priority(format_rules: &std::collections::HashMap<String, crate::clients::models::FormatRule>) -> Vec<&'static str> {
    let mut list = Vec::new();
    for t in TRANSPORT_PRIORITY {
        if format_rules.contains_key(*t) {
            list.push(*t);
        }
    }
    list
}

fn register_builtin_helpers(handlebars: &mut Handlebars<'static>) {
    handlebars.register_helper("json", Box::new(json_helper));
}

fn json_helper(
    helper: &Helper<'_>,
    _handlebars: &Handlebars<'_>,
    _context: &HbContext,
    _rc: &mut RenderContext<'_, '_>,
    out: &mut dyn Output,
) -> HelperResult {
    let param = helper.param(0).ok_or_else(|| {
        RenderError::from(RenderErrorReason::Other(
            "json helper requires one parameter".to_string(),
        ))
    })?;
    let rendered = serde_json::to_string(param.value()).map_err(|err| {
        RenderError::from(RenderErrorReason::Other(format!(
            "json helper serialization failed: {}",
            err
        )))
    })?;
    out.write(&rendered)
        .map_err(|err| RenderError::from(RenderErrorReason::Other(err.to_string())))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde_json::json;
    use std::collections::HashMap;

    use crate::clients::models::{
        ConfigMapping, ContainerType, FormatRule, ManagedEndpointConfig, MergeStrategy, StorageConfig, StorageKind,
    };

    struct MemorySource {
        template: ClientTemplate,
        config_path: String,
    }

    #[async_trait]
    impl ClientConfigSource for MemorySource {
        async fn list_client(&self) -> ConfigResult<Vec<ClientTemplate>> {
            Ok(vec![self.template.clone()])
        }

        async fn get_template(
            &self,
            client_id: &str,
            _platform: &str,
        ) -> ConfigResult<Option<ClientTemplate>> {
            if client_id == self.template.identifier {
                Ok(Some(self.template.clone()))
            } else {
                Ok(None)
            }
        }

        async fn get_config_path(
            &self,
            client_id: &str,
            _platform: &str,
        ) -> ConfigResult<Option<String>> {
            if client_id == self.template.identifier {
                Ok(Some(self.config_path.clone()))
            } else {
                Ok(None)
            }
        }

        async fn reload(&self) -> ConfigResult<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn managed_mode_falls_back_to_stdio_bridge() {
        let exe_dir = std::env::current_exe()
            .expect("locate current exe")
            .parent()
            .expect("exe dir")
            .to_path_buf();
        let bridge_stub = exe_dir.join(format!("bridge{}", std::env::consts::EXE_SUFFIX));
        std::fs::write(&bridge_stub, b"stub").expect("write bridge stub");

        let mut format_rules = HashMap::new();
        format_rules.insert(
            "stdio".to_string(),
            FormatRule {
                template: json!({
                    "type": "stdio",
                    "command": "{{command}}",
                    "args": "{{{json args}}}",
                    "env": "{{{json env}}}"
                }),
                requires_type_field: false,
            },
        );

        let template = ClientTemplate {
            identifier: "test-client".to_string(),
            format: TemplateFormat::Json,
            storage: StorageConfig {
                kind: StorageKind::File,
                path_strategy: Some("config_path".to_string()),
                adapter: None,
            },
            config_mapping: ConfigMapping {
                container_key: "mcpServers".to_string(),
                container_type: ContainerType::ObjectMap,
                merge_strategy: MergeStrategy::Replace,
                keep_original_config: false,
                managed_endpoint: Some(ManagedEndpointConfig { source: Some("profile".to_string()) }),
                managed_source: None,
                format_rules,
            },
            ..Default::default()
        };

        let source = MemorySource {
            template,
            config_path: "~/.config/test-client.json".to_string(),
        };

        let engine = TemplateEngine::with_defaults(Arc::new(source));

        let servers = vec![ServerTemplateInput {
            name: "server_a".to_string(),
            display_name: Some("Server A".to_string()),
            transport: "streamable_http".to_string(),
            command: Some("uvx".to_string()),
            args: vec!["run".to_string()],
            env: HashMap::new(),
            url: Some("https://example.com".to_string()),
            headers: HashMap::new(),
            metadata: HashMap::new(),
        }];

        let policy = BackupPolicySetting::default();
        let request = RenderRequest {
            client_id: "test-client",
            servers: &servers,
            mode: ConfigMode::Managed,
            profile_id: Some("profile-123"),
            dry_run: true,
            backup_policy: &policy,
        };

        let result = engine.render_config(request).await.expect("render");
        let content = match result {
            TemplateExecutionResult::DryRun { content, .. } => content,
            TemplateExecutionResult::Applied { content, .. } => content,
        };

        assert!(!content.contains("\\/"));
        assert!(!content.contains("\\/"));

        let json: Value = serde_json::from_str(&content).expect("json");
        let servers_obj = json
            .get("mcpServers")
            .and_then(|value| value.as_object())
            .expect("mcpServers object");

        let bridge_entry = servers_obj
            .values()
            .next()
            .expect("bridge entry")
            .as_object()
            .expect("bridge object");

        assert_eq!(
            bridge_entry.get("command").and_then(Value::as_str).unwrap(),
            bridge_stub.to_string_lossy()
        );

        let args = bridge_entry.get("args").and_then(Value::as_array).expect("args array");
        assert_eq!(args[0].as_str().unwrap(), "--sse-url");
        assert!(args[1].as_str().unwrap().contains("client_id=test-client"));

        let env = bridge_entry.get("env").and_then(Value::as_object).expect("env object");
        assert_eq!(env.get("APPID").and_then(Value::as_str).unwrap(), "test-client");

        std::fs::remove_file(&bridge_stub).expect("cleanup bridge stub");
    }

    #[tokio::test]
    async fn managed_mode_prefers_streamable_http_and_omits_metadata() {
        let mut format_rules = HashMap::new();
        format_rules.insert(
            "stdio".to_string(),
            FormatRule {
                template: json!({
                    "type": "stdio",
                    "command": "{{command}}",
                    "args": "{{{json args}}}"
                }),
                requires_type_field: false,
            },
        );
        format_rules.insert(
            "streamable_http".to_string(),
            FormatRule {
                template: json!({
                    "type": "streamable_http",
                    "url": "{{url}}"
                }),
                requires_type_field: false,
            },
        );

        let template = ClientTemplate {
            identifier: "test-client".to_string(),
            format: TemplateFormat::Json,
            storage: StorageConfig {
                kind: StorageKind::File,
                path_strategy: Some("config_path".to_string()),
                adapter: None,
            },
            config_mapping: ConfigMapping {
                container_key: "mcpServers".to_string(),
                container_type: ContainerType::ObjectMap,
                merge_strategy: MergeStrategy::Replace,
                keep_original_config: false,
                managed_endpoint: Some(ManagedEndpointConfig { source: Some("profile".to_string()) }),
                managed_source: None,
                format_rules,
            },
            ..Default::default()
        };

        let source = MemorySource {
            template,
            config_path: "~/.config/test-client.json".to_string(),
        };

        let engine = TemplateEngine::with_defaults(Arc::new(source));

        let servers = vec![ServerTemplateInput {
            name: "server_a".to_string(),
            display_name: Some("Server A".to_string()),
            transport: "streamable_http".to_string(),
            command: Some("uvx".to_string()),
            args: vec!["run".to_string()],
            env: HashMap::new(),
            url: Some("https://example.com".to_string()),
            headers: HashMap::new(),
            metadata: HashMap::new(),
        }];

        let policy = BackupPolicySetting::default();
        let request = RenderRequest {
            client_id: "test-client",
            servers: &servers,
            mode: ConfigMode::Managed,
            profile_id: None,
            dry_run: true,
            backup_policy: &policy,
        };

        let result = engine.render_config(request).await.expect("render");
        let content = match result {
            TemplateExecutionResult::DryRun { content, .. } => content,
            TemplateExecutionResult::Applied { content, .. } => content,
        };

        assert!(!content.contains("http:\\/:"));

        assert!(!content.contains("\\/"));

        let json: Value = serde_json::from_str(&content).expect("json");
        let servers_obj = json
            .get("mcpServers")
            .and_then(|value| value.as_object())
            .expect("mcpServers object");

        let entry = servers_obj
            .values()
            .next()
            .expect("managed entry")
            .as_object()
            .expect("entry object");

        let expected_url = get_runtime_port_config().mcp_http_url();
        assert_eq!(entry.get("type").and_then(Value::as_str), Some("streamable_http"));
        assert_eq!(entry.get("url").and_then(Value::as_str), Some(expected_url.as_str()));
        assert!(entry.get("metadata").is_none());
    }
}
