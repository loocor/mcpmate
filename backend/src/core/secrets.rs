use std::collections::HashMap;

use mcpmate_secrets::{
    SecretError, SecretResolver, UnavailableSecretResolver, extract_secret_references, resolve_placeholders,
};

use crate::core::models::MCPServerConfig;
use store::{LocalSecretStore, SecretUsageLocationInput, SecretUsageUpsertInput};

pub mod store;

pub fn resolve_runtime_server_config(
    config: &MCPServerConfig,
    resolver: &(impl SecretResolver + ?Sized),
) -> Result<MCPServerConfig, SecretError> {
    Ok(MCPServerConfig {
        kind: config.kind,
        command: resolve_optional_string(config.command.as_ref(), resolver)?,
        args: resolve_optional_vec(config.args.as_ref(), resolver)?,
        url: resolve_optional_string(config.url.as_ref(), resolver)?,
        env: resolve_optional_map(config.env.as_ref(), resolver)?,
        headers: resolve_optional_map(config.headers.as_ref(), resolver)?,
    })
}

pub fn resolve_runtime_server_config_with_optional_resolver(
    config: &MCPServerConfig,
    resolver: Option<&dyn SecretResolver>,
) -> Result<MCPServerConfig, SecretError> {
    match resolver {
        Some(resolver) => resolve_runtime_server_config(config, resolver),
        None => resolve_runtime_server_config(config, &UnavailableSecretResolver),
    }
}

fn resolve_optional_string(
    value: Option<&String>,
    resolver: &(impl SecretResolver + ?Sized),
) -> Result<Option<String>, SecretError> {
    value.map(|item| resolve_placeholders(item, resolver)).transpose()
}

fn resolve_optional_vec(
    values: Option<&Vec<String>>,
    resolver: &(impl SecretResolver + ?Sized),
) -> Result<Option<Vec<String>>, SecretError> {
    values
        .map(|items| {
            items
                .iter()
                .map(|item| resolve_placeholders(item, resolver))
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()
}

fn resolve_optional_map(
    values: Option<&HashMap<String, String>>,
    resolver: &(impl SecretResolver + ?Sized),
) -> Result<Option<HashMap<String, String>>, SecretError> {
    values
        .map(|items| {
            items
                .iter()
                .map(|(key, value)| resolve_placeholders(value, resolver).map(|resolved| (key.clone(), resolved)))
                .collect::<Result<HashMap<_, _>, _>>()
        })
        .transpose()
}

pub async fn sync_server_secret_usages(
    store: &LocalSecretStore,
    server_id: &str,
    config: &MCPServerConfig,
) -> anyhow::Result<()> {
    let usages = collect_secret_usages(server_id, config)?;
    store.replace_server_usages(server_id, usages).await
}

pub fn collect_secret_usages(
    server_id: &str,
    config: &MCPServerConfig,
) -> anyhow::Result<Vec<SecretUsageUpsertInput>> {
    let mut usages = Vec::new();

    if let Some(command) = config.command.as_ref() {
        push_usages_from_value(&mut usages, server_id, command, SecretUsageLocationInput::StdioCommand)?;
    }

    if let Some(args) = config.args.as_ref() {
        for (index, value) in args.iter().enumerate() {
            push_usages_from_value(
                &mut usages,
                server_id,
                value,
                SecretUsageLocationInput::StdioArgument { index },
            )?;
        }
    }

    if let Some(env) = config.env.as_ref() {
        for (name, value) in env {
            push_usages_from_value(
                &mut usages,
                server_id,
                value,
                SecretUsageLocationInput::StdioEnv { name: name.clone() },
            )?;
        }
    }

    if let Some(url) = config.url.as_ref() {
        push_usages_from_value(&mut usages, server_id, url, SecretUsageLocationInput::StreamableHttpUrl)?;
    }

    if let Some(headers) = config.headers.as_ref() {
        for (name, value) in headers {
            push_usages_from_value(
                &mut usages,
                server_id,
                value,
                SecretUsageLocationInput::StreamableHttpHeader { name: name.clone() },
            )?;
        }
    }

    Ok(usages)
}

pub fn is_usage_active_in_config(
    alias: &str,
    _server_id: &str,
    location: &SecretUsageLocationInput,
    config: &MCPServerConfig,
) -> anyhow::Result<bool> {
    // Check only the specific config field for this location, O(1) per usage.
    let value = match location {
        SecretUsageLocationInput::StdioCommand => config.command.as_deref(),
        SecretUsageLocationInput::StdioArgument { index } => {
            config.args.as_ref().and_then(|args| args.get(*index).map(|s| s.as_str()))
        }
        SecretUsageLocationInput::StdioEnv { name } => {
            config.env.as_ref().and_then(|env| env.get(name.as_str()).map(|s| s.as_str()))
        }
        SecretUsageLocationInput::StreamableHttpUrl => config.url.as_deref(),
        SecretUsageLocationInput::StreamableHttpHeader { name } => {
            config.headers.as_ref().and_then(|h| h.get(name.as_str()).map(|s| s.as_str()))
        }
        SecretUsageLocationInput::OAuthToken => return Ok(false),
    };

    let Some(value) = value else {
        return Ok(false);
    };

    // Check if the value still contains a reference to this secret alias.
    for reference in extract_secret_references(value)? {
        if reference.alias() == alias {
            return Ok(true);
        }
    }
    Ok(false)
}

fn push_usages_from_value(
    usages: &mut Vec<SecretUsageUpsertInput>,
    server_id: &str,
    value: &str,
    location: SecretUsageLocationInput,
) -> anyhow::Result<()> {
    for reference in extract_secret_references(value)? {
        usages.push(SecretUsageUpsertInput {
            alias: reference.alias().to_string(),
            server_id: server_id.to_string(),
            location: location.clone(),
        });
    }
    Ok(())
}
