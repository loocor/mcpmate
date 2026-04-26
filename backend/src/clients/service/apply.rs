use super::core::supported_transports_from_format_rules;
use super::core::{ApplyOutcome, ClientConfigService, ClientRenderOptions, ClientRenderResult, PreviewOutcome};
use crate::clients::TemplateExecutionResult;
use crate::clients::engine::RenderRequest;
use crate::clients::error::{ConfigError, ConfigResult};
use crate::clients::models::FormatRule;
use crate::clients::models::{ConfigMode, TemplateFormat};
use std::collections::HashMap;
use std::collections::HashSet;

fn normalize_transport(transport: &str) -> Option<&'static str> {
    match transport.trim().to_ascii_lowercase().as_str() {
        "streamable_http" | "streamablehttp" | "http" => Some("streamable_http"),
        "sse" => Some("sse"),
        "stdio" => Some("stdio"),
        _ => None,
    }
}

fn available_managed_transports(format_rules: Option<&HashMap<String, FormatRule>>) -> Vec<&'static str> {
    let Some(rules) = format_rules else {
        return Vec::new();
    };

    supported_transports_from_format_rules(rules)
        .into_iter()
        .filter_map(|transport| normalize_transport(&transport))
        .collect()
}

fn select_managed_transport(
    configured_transport: &str,
    supported_transports: &[&'static str],
) -> (Option<String>, bool) {
    if configured_transport != "auto" && supported_transports.contains(&configured_transport) {
        return (Some(configured_transport.to_string()), false);
    }

    let chosen_transport = supported_transports.first().map(|transport| (*transport).to_string());
    let auto_selected = configured_transport == "auto" && chosen_transport.is_some();

    (chosen_transport, auto_selected)
}

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

        if matches!(options.mode, ConfigMode::Managed | ConfigMode::Native)
            && self.verified_local_config_target(&options.client_id).await?.is_none()
        {
            return Err(ConfigError::DataAccessError(format!(
                "Client '{}' has no configured local config target; cannot render {:?} mode",
                options.client_id, options.mode
            )));
        }

        // Get client state for configuration metadata
        let state = self
            .fetch_state(&options.client_id)
            .await?
            .ok_or_else(|| ConfigError::DataAccessError(format!("Client state not found: {}", options.client_id)))?;
        let render_definition = Self::build_render_definition_from_state(&state)?;
        let format_rules = (!render_definition.config_mapping.format_rules.is_empty())
            .then(|| render_definition.config_mapping.format_rules.clone());

        // Validate format_rules exists for Managed mode
        if matches!(options.mode, ConfigMode::Managed) && format_rules.is_none() {
            return Err(ConfigError::DataAccessError(format!(
                "Client '{}' has no format_rules (likely legacy record); re-detect this client to populate transport configuration",
                options.client_id
            )));
        }

        // Select transport for managed mode if applicable
        let mut chosen_transport: Option<String> = None;
        let mut auto_selected = false;
        if matches!(options.mode, ConfigMode::Managed) {
            let supported_transports = available_managed_transports(format_rules.as_ref());

            if let Some((_, transport, _)) = self.get_client_settings(&options.client_id).await.unwrap_or(None) {
                (chosen_transport, auto_selected) = select_managed_transport(&transport, &supported_transports);
            } else {
                chosen_transport = supported_transports.first().map(|transport| (*transport).to_string());
                auto_selected = chosen_transport.is_some();
            }
        }
        let mut servers = self.prepare_servers(&options).await?;
        let backup_policy = self.get_backup_policy(&options.client_id).await?;

        if matches!(options.mode, ConfigMode::Native) {
            if let Some(ref rules) = format_rules {
                let supported: HashSet<String> = rules.keys().map(|transport| transport.to_string()).collect();
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
        }

        let mut warnings = Vec::new();
        let config_path = self.resolved_config_path(&options.client_id).await?.ok_or_else(|| {
            ConfigError::PathResolutionError(format!("No config_path for client {}", options.client_id))
        })?;
        let request = RenderRequest {
            client_id: &options.client_id,
            servers: &servers,
            mode: options.mode.clone(),
            definition: &render_definition,
            config_path: &config_path,
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
            let backup_policy = self.get_backup_policy(&options.client_id).await?;
            self.enforce_backup_retention(&options.client_id, &backup_policy)
                .await?;
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
            let state = self.fetch_state(&options.client_id).await.ok().flatten();
            if let Some(state) = state {
                let format = preview_outcome.preview.format;
                let container_keys = state.container_keys().unwrap_or_default();
                let container_type = state.container_type();
                // best-effort parse + summarize; don't fail the apply path on parse errors
                if let Some(summary) = Self::summarize_servers_for_probe(after, format, &container_keys, container_type)
                {
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
                    let backup_policy = self.get_backup_policy(&options.client_id).await?;
                    self.enforce_backup_retention(&options.client_id, &backup_policy)
                        .await?;
                    out.applied = true;
                    out.backup_path = backup_path;
                }
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
        container_keys: &[String],
        container_type: Option<&str>,
    ) -> Option<Vec<ProbeEntry>> {
        use serde_json::Value;
        let doc = Self::parse_by_format(format, raw)?;
        // Find container
        let mut container: Option<Value> = None;
        for key in container_keys {
            if let Some(v) = crate::clients::utils::get_nested_value(&doc, key) {
                container = Some(v.clone());
                break;
            }
        }
        let container = container?;

        let mut out: Vec<ProbeEntry> = Vec::new();
        let is_array = container_type == Some("array");
        match (is_array, container) {
            (false, Value::Object(map)) => {
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
            (true, Value::Array(items)) => {
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
