use super::shared::*;
use crate::api::{
    handlers::ApiError,
    models::oauth::{
        OAuthInitiateResp, OAuthStatusResp, ServerOAuthCallbackReq, ServerOAuthConfigReq,
        ServerOAuthInitiateReq, ServerOAuthRevokeReq, ServerOAuthStatusReq,
    },
};
use crate::core::oauth::OAuthConfigInput;

fn get_oauth_manager(
    state: &Arc<AppState>,
) -> Result<Arc<crate::core::oauth::OAuthManager>, ApiError> {
    state
        .oauth_manager
        .clone()
        .ok_or_else(|| ApiError::InternalError("OAuth manager unavailable".to_string()))
}

pub async fn configure_oauth(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ServerOAuthConfigReq>,
) -> Result<Json<OAuthStatusResp>, ApiError> {
    let manager = get_oauth_manager(&state)?;
    let status = manager
        .upsert_config(
            &payload.server_id,
            OAuthConfigInput {
                authorization_endpoint: payload.authorization_endpoint,
                token_endpoint: payload.token_endpoint,
                client_id: payload.client_id,
                client_secret: payload.client_secret,
                scopes: payload.scopes,
                redirect_uri: payload.redirect_uri,
            },
        )
        .await
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;
    Ok(Json(OAuthStatusResp::success(status)))
}

pub async fn start_oauth(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ServerOAuthInitiateReq>,
) -> Result<Json<OAuthInitiateResp>, ApiError> {
    let manager = get_oauth_manager(&state)?;
    let result = manager
        .initiate(&payload.server_id)
        .await
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;
    Ok(Json(OAuthInitiateResp::success(result)))
}

pub async fn complete_oauth(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ServerOAuthCallbackReq>,
) -> Result<Json<OAuthStatusResp>, ApiError> {
    let manager = get_oauth_manager(&state)?;
    let status = manager
        .exchange_code(&payload.state, &payload.code)
        .await
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;
    Ok(Json(OAuthStatusResp::success(status)))
}

pub async fn oauth_status(
    State(state): State<Arc<AppState>>,
    Query(request): Query<ServerOAuthStatusReq>,
) -> Result<Json<OAuthStatusResp>, ApiError> {
    let manager = get_oauth_manager(&state)?;
    let status = manager
        .get_status(&request.id)
        .await
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;
    Ok(Json(OAuthStatusResp::success(status)))
}

pub async fn revoke_oauth(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ServerOAuthRevokeReq>,
) -> Result<Json<OAuthStatusResp>, ApiError> {
    let manager = get_oauth_manager(&state)?;
    let status = manager
        .revoke(&payload.server_id)
        .await
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;
    Ok(Json(OAuthStatusResp::success(status)))
}
