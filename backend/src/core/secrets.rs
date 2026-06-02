use std::collections::HashMap;

use mcpmate_secrets::{SecretError, SecretReference, SecretResolver, UnavailableSecretResolver, resolve_placeholders};

use crate::core::models::MCPServerConfig;
use store::{LocalSecretStore, SecretUsageLocationInput, SecretUsageUpsertInput};

pub mod store;

const SECRET_PREFIX: &str = "[[secret:";
const SECRET_SUFFIX: &str = "]]";

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

    store.replace_server_usages(server_id, usages).await
}

fn push_usages_from_value(
    usages: &mut Vec<SecretUsageUpsertInput>,
    server_id: &str,
    value: &str,
    location: SecretUsageLocationInput,
) -> anyhow::Result<()> {
    for alias in extract_secret_aliases(value)? {
        usages.push(SecretUsageUpsertInput {
            alias,
            server_id: server_id.to_string(),
            location: location.clone(),
        });
    }
    Ok(())
}

fn extract_secret_aliases(value: &str) -> anyhow::Result<Vec<String>> {
    let mut aliases = Vec::new();
    let mut rest = value;

    while let Some(start) = rest.find(SECRET_PREFIX) {
        let after_prefix = &rest[start + SECRET_PREFIX.len()..];
        let Some(end) = after_prefix.find(SECRET_SUFFIX) else {
            return Err(anyhow::anyhow!("unterminated secret placeholder"));
        };
        let alias = &after_prefix[..end];
        aliases.push(SecretReference::new(alias)?.alias().to_string());
        rest = &after_prefix[end + SECRET_SUFFIX.len()..];
    }

    Ok(aliases)
}
