use std::{sync::Arc, time::Duration};

use axum::{Json, extract::Query, extract::State};
use serde::Deserialize;
use serde_json::Value;

use super::ApiError;
use crate::api::routes::AppState;

#[derive(Debug, Deserialize, Clone)]
pub struct RegistryServersQuery {
    pub limit: Option<u32>,
    pub cursor: Option<String>,
    pub search: Option<String>,
    pub version: Option<String>,
}

pub async fn list_servers(
    State(_state): State<Arc<AppState>>,
    Query(query): Query<RegistryServersQuery>,
) -> Result<Json<Value>, ApiError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("MCPMate/0.1.0 (+https://mcpmate.io)")
        .build()
        .map_err(|err| ApiError::InternalError(format!("Failed to init HTTP client: {err}")))?;

    let mut params: Vec<(String, String)> = Vec::new();

    let limit = query.limit.unwrap_or(30).clamp(1, 100);
    params.push(("limit".to_string(), limit.to_string()));

    let version = query
        .version
        .clone()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "latest".to_string());
    params.push(("version".to_string(), version));

    if let Some(cursor) = query.cursor.filter(|v| !v.trim().is_empty()) {
        params.push(("cursor".to_string(), cursor));
    }

    if let Some(search) = query.search.filter(|v| !v.trim().is_empty()) {
        params.push(("search".to_string(), search));
    }

    let response = client
        .get("https://registry.modelcontextprotocol.io/v0/servers")
        .query(&params)
        .send()
        .await
        .map_err(|err| ApiError::InternalError(format!("Registry request failed: {err}")))?;

    if !response.status().is_success() {
        return Err(ApiError::InternalError(format!(
            "Registry responded with status {}",
            response.status()
        )));
    }

    let payload: Value = response
        .json()
        .await
        .map_err(|err| ApiError::InternalError(format!("Failed to decode registry payload: {err}")))?;

    Ok(Json(payload))
}
