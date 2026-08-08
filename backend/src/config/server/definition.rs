use std::collections::HashMap;

use anyhow::{Context, Result, anyhow};
use sqlx::{Pool, Sqlite};

use crate::{
    common::server::ServerType,
    config::{
        models::{Server, ServerTransportDraft, ValidatedTransport},
        server::{
            args::replace_server_args_tx, crud::upsert_server_tx, env::replace_server_env_tx,
            headers::replace_server_headers_tx, transport::upsert_server_transport_draft_tx,
        },
    },
};

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
