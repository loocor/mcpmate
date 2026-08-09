use std::collections::HashMap;

use anyhow::{Context, Result, anyhow};
use sqlx::{Pool, Sqlite};

use crate::{
    common::server::ServerType,
    config::{
        models::{Server, ServerTransportDraft, ValidatedTransport},
        server::{
            args::replace_server_args_tx,
            crud::upsert_server_tx,
            env::replace_server_env_tx,
            headers::replace_server_headers_tx,
            transport::{get_server_transport_draft, upsert_server_transport_draft_tx},
        },
    },
};

/// Loads and validates the persisted transport definition required for runtime use.
///
/// Runtime consumers must not reconstruct transport settings from legacy projections.
/// A missing, undecodable, or invalid draft is an error so callers can fail closed.
pub async fn load_validated_server_transport(
    pool: &Pool<Sqlite>,
    server_id: &str,
) -> Result<ValidatedTransport> {
    let draft = get_server_transport_draft(pool, server_id)
        .await
        .map_err(|error| anyhow!("persisted ServerTransportDraft for server '{server_id}' could not be read: {error}"))?
        .ok_or_else(|| anyhow!("persisted ServerTransportDraft is missing for server '{server_id}'"))?;

    draft.validate().map_err(|diagnostics| {
        anyhow!("persisted ServerTransportDraft is invalid for server '{server_id}': {diagnostics:?}")
    })
}

/// Verifies that an OAuth exchange can atomically clear manual Authorization
/// headers from the persisted HTTP definition after storing its token.
pub async fn ensure_persisted_http_authorization_headers_clearable(
    pool: &Pool<Sqlite>,
    server_id: &str,
) -> Result<()> {
    let transport = load_validated_server_transport(pool, server_id).await?;
    if !matches!(
        transport,
        ValidatedTransport::Sse { .. } | ValidatedTransport::StreamableHttp { .. }
    ) {
        anyhow::bail!("persisted ServerTransportDraft is not HTTP for server '{server_id}'");
    }

    let projection_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM server_config WHERE id = ?)")
        .bind(server_id)
        .fetch_one(pool)
        .await
        .with_context(|| format!("load server projection for server '{server_id}'"))?;
    if !projection_exists {
        anyhow::bail!("server projection is missing for server '{server_id}'");
    }

    Ok(())
}

/// Removes manual Authorization headers from a persisted HTTP transport definition.
///
/// The typed draft is the source of truth. This updates its validated legacy
/// projections in the same transaction and never falls back to legacy headers.
pub async fn clear_persisted_http_authorization_headers(
    pool: &Pool<Sqlite>,
    server_id: &str,
) -> Result<()> {
    let mut transaction = pool
        .begin()
        .await
        .context("begin persisted HTTP authorization header transaction")?;
    let draft_json: Option<String> = sqlx::query_scalar("SELECT draft_json FROM server_transport WHERE server_id = ?")
        .bind(server_id)
        .fetch_optional(&mut *transaction)
        .await
        .with_context(|| format!("load persisted ServerTransportDraft for server '{server_id}'"))?;
    let draft_json =
        draft_json.ok_or_else(|| anyhow!("persisted ServerTransportDraft is missing for server '{server_id}'"))?;
    let mut draft: ServerTransportDraft = serde_json::from_str(&draft_json)
        .with_context(|| format!("decode persisted ServerTransportDraft for server '{server_id}'"))?;

    let ServerTransportDraft::Http { headers, .. } = &mut draft else {
        anyhow::bail!("persisted ServerTransportDraft is not HTTP for server '{server_id}'");
    };
    headers.retain(|key, _| !super::headers::is_authorization_header_key(key));

    let validated = draft.validate().map_err(|diagnostics| {
        anyhow!("persisted ServerTransportDraft is invalid for server '{server_id}': {diagnostics:?}")
    })?;
    let mut projected_server: Server = sqlx::query_as("SELECT * FROM server_config WHERE id = ?")
        .bind(server_id)
        .fetch_optional(&mut *transaction)
        .await
        .with_context(|| format!("load server projection for server '{server_id}'"))?
        .ok_or_else(|| anyhow!("server projection is missing for server '{server_id}'"))?;
    let (args, env, headers) = project_legacy_transport(&mut projected_server, &validated);

    upsert_server_tx(&mut transaction, &projected_server).await?;
    replace_server_args_tx(&mut transaction, server_id, &args).await?;
    replace_server_env_tx(&mut transaction, server_id, &env).await?;
    replace_server_headers_tx(&mut transaction, server_id, &headers).await?;
    upsert_server_transport_draft_tx(&mut transaction, server_id, &draft).await?;
    transaction
        .commit()
        .await
        .context("commit persisted HTTP authorization header transaction")?;

    Ok(())
}

/// Stores one valid server transport draft and its legacy projections in one transaction.
///
/// The legacy rows remain the compatibility source for consumers not yet moved to the
/// structured definition. New writes must enter through this function so they cannot
/// leave those projections out of sync with the typed draft.
pub async fn upsert_server_definition(
    pool: &Pool<Sqlite>,
    server: &Server,
    draft: &ServerTransportDraft,
) -> Result<String> {
    let validated = draft
        .validate()
        .map_err(|diagnostics| anyhow!("server transport draft is invalid: {diagnostics:?}"))?;
    let mut projected_server = server.clone();
    let (args, env, headers) = project_legacy_transport(&mut projected_server, &validated);

    let mut transaction = pool.begin().await.context("begin server definition transaction")?;
    let server_id = upsert_server_tx(&mut transaction, &projected_server).await?;
    replace_server_args_tx(&mut transaction, &server_id, &args).await?;
    replace_server_env_tx(&mut transaction, &server_id, &env).await?;
    replace_server_headers_tx(&mut transaction, &server_id, &headers).await?;
    upsert_server_transport_draft_tx(&mut transaction, &server_id, draft).await?;
    transaction
        .commit()
        .await
        .context("commit server definition transaction")?;

    Ok(server_id)
}

fn project_legacy_transport(
    server: &mut Server,
    transport: &ValidatedTransport,
) -> (Vec<String>, HashMap<String, String>, HashMap<String, String>) {
    match transport {
        ValidatedTransport::Stdio { command, args, env } => {
            server.server_type = ServerType::Stdio;
            server.command = Some(command.clone());
            server.url = None;
            (
                args.clone(),
                env.iter()
                    .map(|(key, value)| (key.clone(), value.runtime_value()))
                    .collect(),
                HashMap::new(),
            )
        }
        ValidatedTransport::Sse { endpoint, headers } => {
            server.server_type = ServerType::Sse;
            server.command = None;
            server.url = Some(endpoint.to_string());
            (
                Vec::new(),
                HashMap::new(),
                headers
                    .iter()
                    .map(|(key, value)| (key.clone(), value.runtime_value()))
                    .collect(),
            )
        }
        ValidatedTransport::StreamableHttp { endpoint, headers } => {
            server.server_type = ServerType::StreamableHttp;
            server.command = None;
            server.url = Some(endpoint.to_string());
            (
                Vec::new(),
                HashMap::new(),
                headers
                    .iter()
                    .map(|(key, value)| (key.clone(), value.runtime_value()))
                    .collect(),
            )
        }
    }
}
