use std::collections::{HashMap, HashSet};

use mcpmate_secrets::{
    SecretError, SecretResolver, UnavailableSecretResolver, extract_secret_references, resolve_placeholders,
};

use crate::core::models::MCPServerConfig;
use store::{LocalSecretStore, SecretUsageLocationInput, SecretUsageUpsertInput, SecretUsageView};

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

    dedup_secret_usages(&mut usages);
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
        SecretUsageLocationInput::StdioArgument { index } => config
            .args
            .as_ref()
            .and_then(|args| args.get(*index).map(|s| s.as_str())),
        SecretUsageLocationInput::StdioEnv { name } => config
            .env
            .as_ref()
            .and_then(|env| env.get(name.as_str()).map(|s| s.as_str())),
        SecretUsageLocationInput::StreamableHttpUrl => config.url.as_deref(),
        SecretUsageLocationInput::StreamableHttpHeader { name } => config
            .headers
            .as_ref()
            .and_then(|h| h.get(name.as_str()).map(|s| s.as_str())),
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

fn dedup_secret_usages(usages: &mut Vec<SecretUsageUpsertInput>) {
    let mut seen = HashSet::new();
    usages.retain(|usage| seen.insert((usage.alias.clone(), usage.server_id.clone(), usage.location.clone())));
}

pub fn usage_binding_key(
    server_id: &str,
    location: &SecretUsageLocationInput,
) -> String {
    location.binding_key(server_id)
}

pub async fn mcp_config_from_server(
    pool: &sqlx::SqlitePool,
    server_id: &str,
    server: &crate::config::models::server::Server,
) -> anyhow::Result<MCPServerConfig> {
    use crate::config::server::{get_server_args, get_server_env, get_server_headers};

    let args = get_server_args(pool, server_id)
        .await?
        .into_iter()
        .map(|arg| arg.arg_value)
        .collect::<Vec<_>>();
    let env = get_server_env(pool, server_id).await?;
    let headers = get_server_headers(pool, server_id).await?;

    Ok(MCPServerConfig {
        kind: server.server_type,
        command: server.command.clone(),
        args: if args.is_empty() { None } else { Some(args) },
        url: server.url.clone(),
        env: if env.is_empty() { None } else { Some(env) },
        headers: if headers.is_empty() { None } else { Some(headers) },
    })
}

pub async fn load_mcp_server_config(
    pool: &sqlx::SqlitePool,
    server_id: &str,
) -> anyhow::Result<Option<MCPServerConfig>> {
    use crate::config::server::get_server_by_id;

    let Some(server) = get_server_by_id(pool, server_id).await? else {
        return Ok(None);
    };

    Ok(Some(mcp_config_from_server(pool, server_id, &server).await?))
}

/// Scan persisted server configs for secret placeholder references.
///
/// This is the source of truth for config-owned active usage counts. The
/// `secure_store_usages` index can lag behind config when sync did not run
/// (imports, legacy data, etc.).
pub async fn discover_config_usages(pool: &sqlx::SqlitePool) -> anyhow::Result<Vec<SecretUsageView>> {
    discover_config_usages_filtered(pool, None).await
}

/// Scan all owner records that can actively hold secret placeholders.
pub async fn discover_active_secret_usages(pool: &sqlx::SqlitePool) -> anyhow::Result<Vec<SecretUsageView>> {
    discover_active_secret_usages_filtered(pool, None).await
}

pub async fn discover_config_usages_for_alias(
    pool: &sqlx::SqlitePool,
    alias: &str,
) -> anyhow::Result<Vec<SecretUsageView>> {
    discover_config_usages_filtered(pool, Some(alias)).await
}

pub async fn discover_active_secret_usages_for_alias(
    pool: &sqlx::SqlitePool,
    alias: &str,
) -> anyhow::Result<Vec<SecretUsageView>> {
    discover_active_secret_usages_filtered(pool, Some(alias)).await
}

async fn discover_active_secret_usages_filtered(
    pool: &sqlx::SqlitePool,
    alias_filter: Option<&str>,
) -> anyhow::Result<Vec<SecretUsageView>> {
    use crate::config::server::get_all_servers;

    let servers = get_all_servers(pool).await?;
    let mut usages = discover_config_usages_with_servers(pool, &servers, alias_filter).await?;
    usages.extend(discover_oauth_usages_with_servers(pool, &servers, alias_filter).await?);
    dedup_secret_usage_views(&mut usages);
    Ok(usages)
}

async fn discover_config_usages_filtered(
    pool: &sqlx::SqlitePool,
    alias_filter: Option<&str>,
) -> anyhow::Result<Vec<SecretUsageView>> {
    use crate::config::server::get_all_servers;

    let servers = get_all_servers(pool).await?;
    discover_config_usages_with_servers(pool, &servers, alias_filter).await
}

async fn discover_config_usages_with_servers(
    pool: &sqlx::SqlitePool,
    servers: &[crate::config::models::server::Server],
    alias_filter: Option<&str>,
) -> anyhow::Result<Vec<SecretUsageView>> {
    let mut usages = Vec::new();
    for server in servers {
        let Some(server_id) = server.id.clone() else {
            continue;
        };
        let config = mcp_config_from_server(pool, &server_id, server).await?;
        for entry in collect_secret_usages(&server_id, &config)? {
            if alias_filter.is_some_and(|alias| entry.alias != alias) {
                continue;
            }
            usages.push(entry.into());
        }
    }
    Ok(usages)
}

async fn discover_oauth_usages_with_servers(
    pool: &sqlx::SqlitePool,
    servers: &[crate::config::models::server::Server],
    alias_filter: Option<&str>,
) -> anyhow::Result<Vec<SecretUsageView>> {
    use crate::config::server::{get_all_oauth_configs, get_all_oauth_tokens};

    let oauth_configs = get_all_oauth_configs(pool).await?;
    let oauth_tokens = get_all_oauth_tokens(pool).await?;

    let config_by_server: std::collections::HashMap<String, _> =
        oauth_configs.into_iter().map(|c| (c.server_id.clone(), c)).collect();
    let token_by_server: std::collections::HashMap<String, _> =
        oauth_tokens.into_iter().map(|t| (t.server_id.clone(), t)).collect();

    let mut usages = Vec::new();
    for server_model in servers {
        let Some(server_id) = server_model.id.as_deref() else {
            continue;
        };

        if let Some(config) = config_by_server.get(server_id) {
            if let Some(client_secret) = config.client_secret.as_deref() {
                push_oauth_usages_from_value(&mut usages, alias_filter, server_id, client_secret)?;
            }
        }

        if let Some(token) = token_by_server.get(server_id) {
            push_oauth_usages_from_value(&mut usages, alias_filter, server_id, &token.access_token)?;
            if let Some(refresh_token) = token.refresh_token.as_deref() {
                push_oauth_usages_from_value(&mut usages, alias_filter, server_id, refresh_token)?;
            }
        }
    }
    Ok(usages)
}

fn push_oauth_usages_from_value(
    usages: &mut Vec<SecretUsageView>,
    alias_filter: Option<&str>,
    server_id: &str,
    value: &str,
) -> anyhow::Result<()> {
    for reference in extract_secret_references(value)? {
        if alias_filter.is_some_and(|alias| reference.alias() != alias) {
            continue;
        }
        usages.push(SecretUsageView {
            alias: reference.alias().to_string(),
            server_id: server_id.to_string(),
            location: SecretUsageLocationInput::OAuthToken,
        });
    }
    Ok(())
}

fn dedup_secret_usage_views(usages: &mut Vec<SecretUsageView>) {
    let mut seen = HashSet::new();
    usages.retain(|usage| seen.insert((usage.alias.clone(), usage.server_id.clone(), usage.location.clone())));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::server::ServerType;

    #[test]
    fn discover_collects_stdio_argument_and_http_header_for_same_alias() {
        let server_id = "serv-context7";
        let config = MCPServerConfig {
            kind: ServerType::StreamableHttp,
            command: None,
            args: None,
            url: Some("https://example.com/mcp".to_string()),
            env: None,
            headers: Some(HashMap::from([(
                "context7_api_key".to_string(),
                "[[secret:server-context7-token]]".to_string(),
            )])),
        };
        let http_usages = collect_secret_usages(server_id, &config).expect("http usages");
        assert_eq!(http_usages.len(), 1);
        assert_eq!(http_usages[0].alias, "server-context7-token");

        let stdio_config = MCPServerConfig {
            kind: ServerType::Stdio,
            command: Some("npx".to_string()),
            args: Some(vec![
                "-y".to_string(),
                "@upstash/context7-mcp".to_string(),
                "--api-key".to_string(),
                "[[secret:server-context7-token]]".to_string(),
            ]),
            url: None,
            env: None,
            headers: None,
        };
        let stdio_usages = collect_secret_usages("serv-stdio", &stdio_config).expect("stdio usages");
        assert_eq!(stdio_usages.len(), 1);
        assert_eq!(stdio_usages[0].alias, "server-context7-token");
        assert!(matches!(
            stdio_usages[0].location,
            SecretUsageLocationInput::StdioArgument { index: 3 }
        ));
    }

    #[test]
    fn collect_deduplicates_same_alias_in_same_location() {
        let config = MCPServerConfig {
            kind: ServerType::StreamableHttp,
            command: None,
            args: None,
            url: Some("https://example.com/[[secret:token]]/[[secret:token]]".to_string()),
            env: None,
            headers: None,
        };

        let usages = collect_secret_usages("serv-http", &config).expect("secret usages");

        assert_eq!(usages.len(), 1);
        assert_eq!(usages[0].alias, "token");
        assert!(matches!(
            usages[0].location,
            SecretUsageLocationInput::StreamableHttpUrl
        ));
    }
}
