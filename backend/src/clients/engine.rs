use crate::clients::error::{ConfigError, ConfigResult};
use crate::clients::models::{
    BackupPolicySetting, ClientTemplate, ConfigMode, ContainerType, ServerTemplateInput, StorageKind, TemplateFormat,
};
use crate::clients::renderer::{ConfigDiff, DynConfigRenderer};
use crate::clients::source::ClientConfigSource;
use crate::clients::storage::{DynConfigStorage, FileConfigStorage};
use crate::common::get_bridge_path;
use crate::system::config::get_runtime_port_config;
use handlebars::{
    Context as HbContext, Handlebars, Helper, HelperResult, Output, RenderContext, RenderError, RenderErrorReason,
};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::Arc;
use url::form_urlencoded;

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
    pub warnings: &'a mut Vec<String>,
    /// Preferred transport when rendering managed endpoint (None = engine priority)
    pub preferred_transport: Option<String>,
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
        engine.register_storage(Arc::new(FileConfigStorage::new()));
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
        match template.storage.kind {
            StorageKind::File => self
                .storages
                .get(&template.storage.kind)
                .cloned()
                .ok_or_else(|| ConfigError::StorageAdapterMissing("file".into())),
            StorageKind::Kv => {
                Err(ConfigError::StorageAdapterMissing(
                    "kv storage is no longer supported".into(),
                ))
            }
            StorageKind::Custom => {
                Err(ConfigError::StorageAdapterMissing(
                    "custom storage is no longer supported".into(),
                ))
            }
        }
    }

    /// Get storage adapter from persisted client state fields
    pub(crate) fn storage_for_client(
        &self,
        state: &crate::clients::service::core::ClientStateRow,
    ) -> ConfigResult<DynConfigStorage> {
        let storage_kind = match state.storage_kind() {
            Some(kind) => kind,
            None if state.has_local_config_target() => "file",
            None => {
                return Err(ConfigError::StorageAdapterMissing(
                    "storage_kind not set".into(),
                ));
            }
        };

        match storage_kind {
            "file" => self
                .storages
                .get(&StorageKind::File)
                .cloned()
                .ok_or_else(|| ConfigError::StorageAdapterMissing("file".into())),
            "kv" => {
                Err(ConfigError::StorageAdapterMissing(
                    "kv storage is no longer supported".into(),
                ))
            }
            "custom" => {
                Err(ConfigError::StorageAdapterMissing(
                    "custom storage is no longer supported".into(),
                ))
            }
            other => Err(ConfigError::StorageAdapterMissing(format!(
                "storage kind not supported: {}",
                other
            ))),
        }
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

        // Get config_path from config_source
        let platform = Self::current_platform();
        let config_path = self.config_source
            .get_config_path(request.client_id, platform)
            .await?
            .ok_or_else(|| ConfigError::PathResolutionError(
                format!("No config_path for client {}", request.client_id)
            ))?;

        let fragment = match request.mode {
            ConfigMode::Native => self.render_native_config(&template, request.servers, request.warnings)?,
            ConfigMode::Managed => self.render_managed_config(
                &template,
                request.profile_id,
                request.preferred_transport.as_deref(),
                request.warnings,
            )?,
        };

        let existing = storage.read(&config_path).await?.unwrap_or_default();
        let merged = renderer.merge(&existing, &fragment, &template)?;

        if request.dry_run {
            let diff = renderer.diff(&existing, &merged)?;
            Ok(TemplateExecutionResult::DryRun { diff, content: merged })
        } else {
            let backup_path = storage.write_atomic(request.client_id, &config_path, &merged, request.backup_policy).await?;
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
        warnings: &mut Vec<String>,
    ) -> ConfigResult<Value> {
        self.render_container(template, servers, warnings)
    }

    fn render_managed_config(
        &self,
        template: &ClientTemplate,
        profile_id: Option<&str>,
        preferred_transport: Option<&str>,
        warnings: &mut Vec<String>,
    ) -> ConfigResult<Value> {
        let managed = self.build_managed_server(template, profile_id, preferred_transport)?;
        self.render_container(template, &[managed], warnings)
    }

    fn render_container(
        &self,
        template: &ClientTemplate,
        servers: &[ServerTemplateInput],
        warnings: &mut Vec<String>,
    ) -> ConfigResult<Value> {
        match template.config_mapping.container_type {
            ContainerType::ObjectMap => self.render_object_map(template, servers, warnings),
            ContainerType::Array => self.render_array(template, servers, warnings),
        }
    }

    fn render_object_map(
        &self,
        template: &ClientTemplate,
        servers: &[ServerTemplateInput],
        warnings: &mut Vec<String>,
    ) -> ConfigResult<Value> {
        let mut container = Map::new();
        for server in servers {
            let server_config = self.render_server_config(template, server, warnings)?;
            container.insert(server.name.clone(), server_config);
        }
        Ok(Value::Object(container))
    }

    fn render_array(
        &self,
        template: &ClientTemplate,
        servers: &[ServerTemplateInput],
        warnings: &mut Vec<String>,
    ) -> ConfigResult<Value> {
        let mut items = Vec::new();
        for server in servers {
            let mut config = self.render_server_config(template, server, warnings)?;
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
        warnings: &mut Vec<String>,
    ) -> ConfigResult<Value> {
        let format_rules = &template.config_mapping.format_rules;
        let keymap = crate::clients::keymap::registry();
        let Some(rule_key) = keymap.resolve_rule_key(format_rules, &server.transport) else {
            return Err(ConfigError::TemplateParseError(format!(
                "Client {} missing format rule for transport {}",
                template.identifier, server.transport
            )));
        };
        let format_rule = format_rules
            .get(&rule_key)
            .ok_or_else(|| ConfigError::TemplateParseError("format rule key resolved but missing".into()))?;

        let context = serde_json::to_value(server)?;
        // Render with optional-key drop policy (format_rules scope)
        let mut rendered =
            self.render_object_with_policy(&format_rule.template, &context, &server.transport, warnings)?;

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

    // Render a top-level server object from format_rules with optional-key drop policy
    fn render_object_with_policy(
        &self,
        template_value: &Value,
        context: &Value,
        transport: &str,
        warnings: &mut Vec<String>,
    ) -> ConfigResult<Value> {
        use Value::*;
        // Helper: classify handlebars strict/unknown variable errors for optional-key pruning
        fn is_missing_var_error(e: &RenderError) -> bool {
            // Handlebars emits different texts across versions:
            // - "Unknown variable" / "not found"
            // - "Failed to access variable in strict mode Some(\"key\")"
            let msg = e.to_string().to_ascii_lowercase();
            msg.contains("unknown")
                || msg.contains("not found")
                || msg.contains("strict mode")
                || msg.contains("failed to access variable")
        }
        match template_value {
            Object(map) => {
                let mut out = serde_json::Map::new();
                for (key, value) in map {
                    // Special handling for metadata object: drop per-entry on unknowns
                    if key == "metadata" {
                        if let Object(meta) = value {
                            let mut rendered_meta = serde_json::Map::new();
                            for (mk, mv) in meta {
                                match self.render_value(mv, context) {
                                    Ok(rv) => {
                                        rendered_meta.insert(mk.clone(), rv);
                                    }
                                    Err(ConfigError::HandlebarsRenderError(e)) => {
                                        if is_missing_var_error(&e) {
                                            warnings.push(format!(
                                                "FR_OPTIONAL_DROPPED: metadata.{} dropped in '{}' (unknown variable)",
                                                mk, transport
                                            ));
                                            continue;
                                        }
                                        return Err(ConfigError::HandlebarsRenderError(e));
                                    }
                                    Err(other) => return Err(other),
                                }
                            }
                            if !rendered_meta.is_empty() {
                                out.insert(key.clone(), Object(rendered_meta));
                            } else {
                                warnings.push(format!(
                                    "FR_OPTIONAL_DROPPED: metadata dropped in '{}' (empty after pruning)",
                                    transport
                                ));
                            }
                            continue;
                        }
                    }

                    // stdio args optional default
                    let is_stdio_args = transport == "stdio" && key == "args";

                    match self.render_value(value, context) {
                        Ok(rv) => {
                            out.insert(key.clone(), rv);
                        }
                        Err(ConfigError::HandlebarsRenderError(e)) => {
                            if is_missing_var_error(&e) {
                                if is_stdio_args {
                                    // default by template position: string vs json helper
                                    let default_value = match value {
                                        Value::String(s) if s.contains("{{{json") => Value::Array(vec![]),
                                        _ => Value::String(std::string::String::new()),
                                    };
                                    out.insert(key.clone(), default_value);
                                    warnings
                                        .push(format!("FR_ARGS_DEFAULTED: defaulted empty 'args' in '{}'", transport));
                                } else if matches!(
                                    key.as_str(),
                                    "description" | "env" | "headers" | "longRunning" | "isActive"
                                ) {
                                    warnings.push(format!(
                                        "FR_OPTIONAL_DROPPED: '{}' dropped in '{}' (unknown variable)",
                                        key, transport
                                    ));
                                } else {
                                    // Required key missing -> propagate error
                                    return Err(ConfigError::HandlebarsRenderError(e));
                                }
                            } else {
                                return Err(ConfigError::HandlebarsRenderError(e));
                            }
                        }
                        Err(other) => return Err(other),
                    }
                }
                Ok(Value::Object(out))
            }
            // Non-object templates fall back to regular rendering
            other => self.render_value(other, context),
        }
    }

    fn build_managed_server(
        &self,
        template: &ClientTemplate,
        profile_id: Option<&str>,
        preferred_transport: Option<&str>,
    ) -> ConfigResult<ServerTemplateInput> {
        // Derive supported transports directly from format_rules keys and
        // apply fixed global priority: streamable_http -> stdio.
        let candidates = derive_transports_by_priority(&template.config_mapping.format_rules);
        if let Some(pref) = preferred_transport {
            if candidates.contains(&pref) {
                if let Some(server) =
                    self.managed_runtime_for_transport(pref, pref, template.identifier.as_str(), profile_id)?
                {
                    return Ok(server);
                }
            }
        }
        for transport in candidates {
            if let Some(server) =
                self.managed_runtime_for_transport(transport, transport, template.identifier.as_str(), profile_id)?
            {
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
            "streamable_http" => {
                let mut headers = HashMap::new();
                headers.insert("x-mcpmate-client-id".to_string(), client_id.to_string());
                if let Some(pid) = profile_id {
                    headers.insert("x-mcpmate-profile-id".to_string(), pid.to_string());
                }

                let mut mcp_url = runtime_config.mcp_http_url();
                let mut query = form_urlencoded::Serializer::new(String::new());
                query.append_pair("client_id", client_id);
                if let Some(pid) = profile_id {
                    query.append_pair("profile_id", pid);
                }
                let query = query.finish();
                if !query.is_empty() {
                    if mcp_url.contains('?') {
                        mcp_url.push('&');
                    } else {
                        mcp_url.push('?');
                    }
                    mcp_url.push_str(&query);
                }

                Ok(Some(ServerTemplateInput {
                    name: "MCPMate".to_string(),
                    display_name: Some("MCPMate Proxy".to_string()),
                    transport: effective_transport.to_string(),
                    command: None,
                    args: Vec::new(),
                    env: HashMap::new(),
                    url: Some(mcp_url),
                    headers,
                    metadata: HashMap::new(),
                }))
            }
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

                // Prefer streamable HTTP for bridge upstream; keep query params for context
                let mut mcp_url = runtime_config.mcp_http_url();
                let mut query = form_urlencoded::Serializer::new(String::new());
                query.append_pair("client_id", client_id);
                if let Some(pid) = profile_id {
                    query.append_pair("profile_id", pid);
                }
                let query = query.finish();
                if !query.is_empty() {
                    if mcp_url.contains('?') {
                        mcp_url.push('&');
                    } else {
                        mcp_url.push('?');
                    }
                    mcp_url.push_str(&query);
                }

                // Bridge CLI expects --upstream-url (alias --sse-url kept for legacy).
                let args = vec!["--upstream-url".to_string(), mcp_url];

                Ok(Some(ServerTemplateInput {
                    name: "MCPMate".to_string(),
                    display_name: Some("MCPMate Bridge".to_string()),
                    transport: effective_transport.to_string(),
                    command: Some(bridge_path),
                    args,
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

const TRANSPORT_PRIORITY: &[&str] = &["streamable_http", "stdio"];

fn derive_transports_by_priority(
    format_rules: &std::collections::HashMap<String, crate::clients::models::FormatRule>
) -> Vec<&'static str> {
    let map = crate::clients::keymap::registry();
    let mut list = Vec::new();
    for t in TRANSPORT_PRIORITY {
        if map.has_rule(format_rules, t) {
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
                container_keys: vec!["mcpServers".to_string()],
                container_type: ContainerType::ObjectMap,
                merge_strategy: MergeStrategy::Replace,
                keep_original_config: false,
                managed_endpoint: Some(ManagedEndpointConfig {
                    source: Some("profile".to_string()),
                }),
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
        let _policy = BackupPolicySetting::default();
        let mut warnings = Vec::new();
        let request = RenderRequest {
            client_id: "test-client",
            servers: &servers,
            mode: ConfigMode::Managed,
            profile_id: Some("profile-123"),
            dry_run: true,
            backup_policy: &policy,
            warnings: &mut warnings,
            preferred_transport: None,
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
        assert_eq!(args[0].as_str().unwrap(), "--upstream-url");
        assert!(args[1].as_str().unwrap().contains("client_id=test-client"));
        assert!(args[1].as_str().unwrap().contains("profile_id=profile-123"));

        let env = bridge_entry.get("env").and_then(Value::as_object).expect("env object");
        assert_eq!(env.get("APPID").and_then(Value::as_str).unwrap(), "test-client");
        assert_eq!(env.get("PROFILE_ID").and_then(Value::as_str).unwrap(), "profile-123");

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
                    "url": "{{{url}}}"
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
                container_keys: vec!["mcpServers".to_string()],
                container_type: ContainerType::ObjectMap,
                merge_strategy: MergeStrategy::Replace,
                keep_original_config: false,
                managed_endpoint: Some(ManagedEndpointConfig {
                    source: Some("profile".to_string()),
                }),
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
        let mut warnings = Vec::new();
        let request = RenderRequest {
            client_id: "test-client",
            servers: &servers,
            mode: ConfigMode::Managed,
            profile_id: None,
            dry_run: true,
            backup_policy: &policy,
            warnings: &mut warnings,
            preferred_transport: None,
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
        let rendered_url = entry.get("url").and_then(Value::as_str).expect("rendered url");
        assert_eq!(entry.get("type").and_then(Value::as_str), Some("streamable_http"));
        assert!(rendered_url.starts_with(expected_url.as_str()));
        assert!(entry.get("metadata").is_none());

        // Regression test: managed URL must NOT be HTML-escaped
        // Handlebars {{url}} would escape '=' to '&#x3D;' breaking client_id query param
        assert!(
            !rendered_url.contains("&#x3D;"),
            "URL must not contain HTML-escaped equals sign"
        );
        assert!(
            !rendered_url.contains("&#x26;"),
            "URL must not contain HTML-escaped ampersand"
        );
        assert!(
            rendered_url.contains("client_id=test-client"),
            "URL must contain literal client_id param"
        );
    }

    #[test]
    fn managed_streamable_http_injects_managed_side_band() {
        let source = MemorySource {
            template: ClientTemplate {
                identifier: "test-client".to_string(),
                ..Default::default()
            },
            config_path: "~/.config/test-client.json".to_string(),
        };
        let engine = TemplateEngine::with_defaults(Arc::new(source));

        let managed = engine
            .managed_runtime_for_transport("streamable_http", "streamable_http", "test-client", Some("profile-123"))
            .expect("managed server")
            .expect("streamable http server");

        assert_eq!(
            managed.headers.get("x-mcpmate-client-id").map(String::as_str),
            Some("test-client")
        );
        assert_eq!(
            managed.headers.get("x-mcpmate-profile-id").map(String::as_str),
            Some("profile-123")
        );
        let url = managed.url.expect("managed url");
        assert!(url.contains("client_id=test-client"));
        assert!(url.contains("profile_id=profile-123"));
    }
}
