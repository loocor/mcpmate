use super::core::{ApplyOutcome, ClientConfigService, ClientRenderOptions, ClientRenderResult, PreviewOutcome};
use crate::clients::TemplateExecutionResult;
use crate::clients::engine::RenderRequest;
use crate::clients::error::{ConfigError, ConfigResult};
use crate::clients::models::{ClientTemplate, ConfigMode, TemplateFormat};
use std::collections::HashSet;

impl ClientConfigService {
    /// Execute configuration generation (dry-run or apply)
    pub async fn execute_render(
        &self,
        options: ClientRenderOptions,
    ) -> ConfigResult<ClientRenderResult> {
        if !self.is_client_managed(&options.client_id).await? {
            return Err(ConfigError::ClientDisabled {
                identifier: options.client_id.clone(),
            });
        }

        let template = self.get_client_template(&options.client_id).await?;
        // Select transport for managed mode if applicable
        let mut chosen_transport: Option<String> = None;
        let mut auto_selected = false;
        if matches!(options.mode, ConfigMode::Managed) {
            // load client settings
            if let Some((_, transport, ver_opt)) = self.get_client_settings(&options.client_id).await.unwrap_or(None) {
                let supported = {
                    let keymap = crate::clients::keymap::registry();
                    let mut list: Vec<&'static str> = Vec::new();
                    for t in ["streamable_http", "stdio"] {
                        if keymap.has_rule(&template.config_mapping.format_rules, t) {
                            list.push(t);
                        }
                    }
                    list
                };
                let allows = |_t: &str| -> bool {
                    let _ver = ver_opt.as_deref();
                    // TODO: version gating; currently allow all
                    let _ = _ver;
                    true
                };
                // Check if transport is explicitly set (not "auto")
                if transport != "auto" && supported.contains(&transport.as_str()) && allows(transport.as_str()) {
                    chosen_transport = Some(transport.clone());
                }
                // If still not chosen (either "auto" or unsupported), auto-select
                if chosen_transport.is_none() {
                    for t in supported {
                        if allows(t) {
                            chosen_transport = Some(t.to_string());
                            auto_selected = transport == "auto";
                            break;
                        }
                    }
                }
            } else {
                // No settings row yet: pick by priority
                let keymap = crate::clients::keymap::registry();
                for t in ["streamable_http", "stdio"] {
                    if keymap.has_rule(&template.config_mapping.format_rules, t) {
                        chosen_transport = Some(t.to_string());
                        auto_selected = true;
                        break;
                    }
                }
            }
        }
        let mut servers = self.prepare_servers(&options).await?;
        let backup_policy = self.get_backup_policy(&options.client_id).await?;

        if matches!(options.mode, ConfigMode::Native) {
            let supported: HashSet<String> = template.config_mapping.format_rules.keys().cloned().collect();
            let before = servers.len();
            servers.retain(|server| supported.contains(&server.transport));
            if before != servers.len() {
                tracing::debug!(
                    "Filtered unsupported servers for client {} (native mode): {} -> {}",
                    options.client_id,
                    before,
                    servers.len()
                );
            }
        }

        let mut warnings = Vec::new();
        let request = RenderRequest {
            client_id: &options.client_id,
            servers: &servers,
            mode: options.mode.clone(),
            profile_id: options.profile_id.as_deref(),
            dry_run: options.dry_run,
            backup_policy: &backup_policy,
            warnings: &mut warnings,
            preferred_transport: chosen_transport.clone(),
        };

        let execution = self.template_engine.render_config(request).await?;
        let target_path = self.resolved_config_path(&options.client_id).await?;

        Ok(ClientRenderResult {
            execution,
            target_path,
            servers,
            warnings,
            chosen_transport,
            auto_selected,
        })
    }

    fn preview_from_execution(exec: &TemplateExecutionResult) -> PreviewOutcome {
        match exec {
            TemplateExecutionResult::DryRun { diff, .. } => PreviewOutcome {
                format: diff.format,
                before: diff.before.clone(),
                after: diff.after.clone(),
                summary: diff.summary.clone(),
            },
            _ => PreviewOutcome {
                format: TemplateFormat::Json,
                before: None,
                after: None,
                summary: None,
            },
        }
    }

    pub async fn apply_or_preview(
        &self,
        options: ClientRenderOptions,
    ) -> ConfigResult<ApplyOutcome> {
        let result = self.execute_render(options.clone()).await?;
        let mut outcome = ApplyOutcome {
            preview: Self::preview_from_execution(&result.execution),
            warnings: result.warnings.clone(),
            ..Default::default()
        };
        if let TemplateExecutionResult::Applied { backup_path, .. } = result.execution {
            outcome.applied = true;
            outcome.backup_path = backup_path;
        }
        Ok(outcome)
    }

    pub async fn apply_with_deferred(
        &self,
        options: ClientRenderOptions,
    ) -> ConfigResult<ApplyOutcome> {
        // Always compute a preview via dry-run first for stable diff/preview fields.
        let mut preview_opts = options.clone();
        preview_opts.dry_run = true;
        let preview_outcome = self.apply_or_preview(preview_opts).await?;

        // Temporary: write-probe logging before actual apply
        // Helps diagnose which transports are being generated and whether 'args' exist
        if let Some(ref after) = preview_outcome.preview.after {
            if let Ok(template) = self.get_client_template(&options.client_id).await {
                let format = preview_outcome.preview.format;
                // best-effort parse + summarize; don't fail the apply path on parse errors
                if let Some(summary) = Self::summarize_servers_for_probe(after, format, &template) {
                    for entry in summary.into_iter().take(5) {
                        tracing::debug!(
                            target: "mcpmate::client::apply_probe",
                            client = %options.client_id,
                            name = %entry.name,
                            transport = %entry.transport,
                            has_args = entry.has_args,
                            args_len = entry.args_len,
                            has_url = entry.has_url,
                            has_command = entry.has_command,
                            "write-probe: server entry"
                        );
                    }
                } else {
                    tracing::debug!(
                        target: "mcpmate::client::apply_probe",
                        client = %options.client_id,
                        "write-probe: unable to summarize servers (parse skipped)"
                    );
                }
            }
        }

        // If the original request is a preview, return directly
        if options.dry_run {
            return Ok(preview_outcome);
        }

        // If not a preview: try to write
        match self.execute_render(options.clone()).await {
            Ok(exec) => {
                let mut out = preview_outcome;
                if let TemplateExecutionResult::Applied { backup_path, .. } = exec.execution {
                    out.applied = true;
                    out.backup_path = backup_path;
                }
                Ok(out)
            }
            Err(ConfigError::FileOperationError(msg)) if msg.to_ascii_lowercase().contains("locked") => {
                // Only Cherry triggers delayed write
                let template = self.get_client_template(&options.client_id).await?;
                let is_cherry = template.storage.kind == crate::clients::models::StorageKind::Kv
                    && template.storage.adapter.as_deref() == Some("cherry_kv");
                if !is_cherry {
                    return Err(ConfigError::FileOperationError(msg));
                }

                let merged_after = preview_outcome.preview.after.clone().unwrap_or_default();
                let policy = self.get_backup_policy(&options.client_id).await?;
                self.schedule_write_after_unlock(template, merged_after, policy)?;

                let mut out = preview_outcome;
                out.scheduled = true;
                out.scheduled_reason = Some("db_locked".into());
                Ok(out)
            }
            Err(e) => Err(e),
        }
    }

    fn parse_by_format(
        format: TemplateFormat,
        raw: &str,
    ) -> Option<serde_json::Value> {
        match format {
            TemplateFormat::Json => serde_json::from_str(raw).ok(),
            TemplateFormat::Json5 => json5::from_str(raw).ok(),
            TemplateFormat::Toml => toml::from_str::<toml::Value>(raw)
                .ok()
                .and_then(|v| serde_json::to_value(v).ok()),
            TemplateFormat::Yaml => serde_yaml::from_str(raw).ok(),
        }
    }

    fn summarize_servers_for_probe(
        raw: &str,
        format: TemplateFormat,
        template: &ClientTemplate,
    ) -> Option<Vec<ProbeEntry>> {
        use serde_json::Value;
        let doc = Self::parse_by_format(format, raw)?;
        // Find container
        let mut container: Option<Value> = None;
        for key in &template.config_mapping.container_keys {
            if let Some(v) = crate::clients::utils::get_nested_value(&doc, key) {
                container = Some(v.clone());
                break;
            }
        }
        let container = container?;

        let mut out: Vec<ProbeEntry> = Vec::new();
        match (template.config_mapping.container_type, container) {
            (crate::clients::models::ContainerType::ObjectMap, Value::Object(map)) => {
                for (name, v) in map {
                    let transport = v.get("type").and_then(|x| x.as_str()).unwrap_or("").to_string();
                    let has_args = v.get("args").is_some();
                    let args_len = v
                        .get("args")
                        .and_then(|a| a.as_array())
                        .map(|a| a.len() as u32)
                        .unwrap_or(0);
                    let has_url = v.get("url").is_some() || v.get("baseUrl").is_some();
                    let has_command = v.get("command").is_some();
                    out.push(ProbeEntry {
                        name,
                        transport,
                        has_args,
                        args_len,
                        has_url,
                        has_command,
                    });
                }
            }
            (crate::clients::models::ContainerType::Array, Value::Array(items)) => {
                for v in items {
                    let name = v.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string();
                    let transport = v.get("type").and_then(|x| x.as_str()).unwrap_or("").to_string();
                    let has_args = v.get("args").is_some();
                    let args_len = v
                        .get("args")
                        .and_then(|a| a.as_array())
                        .map(|a| a.len() as u32)
                        .unwrap_or(0);
                    let has_url = v.get("url").is_some() || v.get("baseUrl").is_some();
                    let has_command = v.get("command").is_some();
                    out.push(ProbeEntry {
                        name,
                        transport,
                        has_args,
                        args_len,
                        has_url,
                        has_command,
                    });
                }
            }
            _ => {}
        }

        if out.is_empty() { None } else { Some(out) }
    }
}

#[derive(Debug)]
struct ProbeEntry {
    name: String,
    transport: String,
    has_args: bool,
    args_len: u32,
    has_url: bool,
    has_command: bool,
}
