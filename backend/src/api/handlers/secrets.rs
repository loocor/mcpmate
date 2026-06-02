use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};

use crate::{
    api::{
        handlers::ApiError,
        models::secrets::{
            SecretCreateReq, SecretDeleteData, SecretDeleteReq, SecretDeleteResp, SecretDetailsReq, SecretKindPayload,
            SecretListData, SecretListResp, SecretMetadataData, SecretMetadataResp, SecretUpdateReq, SecretUsageData,
            SecretUsageListData, SecretUsageListResp, SecretUsageLocationData, SecretUsageReq,
        },
        routes::AppState,
    },
    core::secrets::store::{
        SecretCreateInput, SecretKindInput, SecretMetadataView, SecretUpdateInput, SecretUsageLocationInput,
        SecretUsageView,
    },
};

pub async fn list_secrets(State(state): State<Arc<AppState>>) -> Result<Json<SecretListResp>, ApiError> {
    let store = get_secret_store(&state)?;
    let secrets = store
        .list_secret_metadata()
        .await
        .map_err(|err| ApiError::InternalError(err.to_string()))?
        .into_iter()
        .map(secret_metadata_data)
        .collect();
    Ok(Json(SecretListResp::success(SecretListData { secrets })))
}

pub async fn create_secret(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SecretCreateReq>,
) -> Result<Json<SecretMetadataResp>, ApiError> {
    let store = get_secret_store(&state)?;
    let metadata = store
        .create_secret(SecretCreateInput {
            alias: payload.alias,
            kind: secret_kind_input(payload.kind),
            value: payload.value,
            label: payload.label,
        })
        .await
        .map_err(map_secret_store_error)?;
    Ok(Json(SecretMetadataResp::success(secret_metadata_data(metadata))))
}

pub async fn update_secret(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SecretUpdateReq>,
) -> Result<Json<SecretMetadataResp>, ApiError> {
    let store = get_secret_store(&state)?;
    let metadata = store
        .update_secret(SecretUpdateInput {
            alias: payload.alias,
            kind: payload.kind.map(secret_kind_input),
            value: payload.value,
            label: payload.label,
        })
        .await
        .map_err(map_secret_store_error)?;
    Ok(Json(SecretMetadataResp::success(secret_metadata_data(metadata))))
}

pub async fn get_secret_details(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SecretDetailsReq>,
) -> Result<Json<SecretMetadataResp>, ApiError> {
    let store = get_secret_store(&state)?;
    let metadata = store
        .get_secret_metadata(&query.alias)
        .await
        .map_err(map_secret_store_error)?;
    Ok(Json(SecretMetadataResp::success(secret_metadata_data(metadata))))
}

pub async fn delete_secret(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SecretDeleteReq>,
) -> Result<Json<SecretDeleteResp>, ApiError> {
    let store = get_secret_store(&state)?;
    store
        .delete_secret(&payload.alias, payload.force)
        .await
        .map_err(map_secret_store_error)?;
    Ok(Json(SecretDeleteResp::success(SecretDeleteData {
        alias: payload.alias,
        deleted: true,
    })))
}

pub async fn list_secret_usages(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SecretUsageReq>,
) -> Result<Json<SecretUsageListResp>, ApiError> {
    let store = get_secret_store(&state)?;
    let usages = store
        .list_usages(&query.alias)
        .await
        .map_err(map_secret_store_error)?
        .into_iter()
        .map(secret_usage_data)
        .collect();
    Ok(Json(SecretUsageListResp::success(SecretUsageListData { usages })))
}

fn get_secret_store(state: &Arc<AppState>) -> Result<Arc<crate::core::secrets::store::LocalSecretStore>, ApiError> {
    state
        .secret_store
        .clone()
        .ok_or_else(|| ApiError::InternalError("Secret store is unavailable".to_string()))
}

fn map_secret_store_error(error: anyhow::Error) -> ApiError {
    let message = error.to_string();
    if message.contains("was not found") {
        ApiError::NotFound(message)
    } else if message.contains("in use") {
        ApiError::Conflict(message)
    } else {
        ApiError::BadRequest(message)
    }
}

fn secret_kind_input(kind: SecretKindPayload) -> SecretKindInput {
    match kind {
        SecretKindPayload::Generic => SecretKindInput::Generic,
        SecretKindPayload::Token => SecretKindInput::Token,
        SecretKindPayload::ApiKey => SecretKindInput::ApiKey,
        SecretKindPayload::Password => SecretKindInput::Password,
        SecretKindPayload::OAuthAccessToken => SecretKindInput::OAuthAccessToken,
        SecretKindPayload::OAuthRefreshToken => SecretKindInput::OAuthRefreshToken,
        SecretKindPayload::UrlCredential => SecretKindInput::UrlCredential,
        SecretKindPayload::HeaderValue => SecretKindInput::HeaderValue,
    }
}

fn secret_metadata_data(metadata: SecretMetadataView) -> SecretMetadataData {
    SecretMetadataData {
        alias: metadata.alias,
        placeholder: metadata.placeholder,
        kind: metadata.kind,
        label: metadata.label,
        provider_id: metadata.provider_id,
        provider_kind: metadata.provider_kind,
        version: metadata.version,
        used_by_count: metadata.used_by_count,
        created_at: metadata.created_at,
        updated_at: metadata.updated_at,
    }
}

fn secret_usage_data(usage: SecretUsageView) -> SecretUsageData {
    SecretUsageData {
        alias: usage.alias,
        server_id: usage.server_id,
        location: secret_usage_location_data(usage.location),
    }
}

fn secret_usage_location_data(location: SecretUsageLocationInput) -> SecretUsageLocationData {
    match location {
        SecretUsageLocationInput::StdioCommand => SecretUsageLocationData::StdioCommand,
        SecretUsageLocationInput::StdioArgument { index } => SecretUsageLocationData::StdioArgument { index },
        SecretUsageLocationInput::StdioEnv { name } => SecretUsageLocationData::StdioEnv { name },
        SecretUsageLocationInput::StreamableHttpUrl => SecretUsageLocationData::StreamableHttpUrl,
        SecretUsageLocationInput::StreamableHttpHeader { name } => {
            SecretUsageLocationData::StreamableHttpHeader { name }
        }
        SecretUsageLocationInput::OAuthToken => SecretUsageLocationData::OAuthToken,
    }
}
