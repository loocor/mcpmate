// MCPMate Proxy API handlers for Config Suit basic operations
// Contains handler functions for listing and getting Config Suits

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;

use crate::api::{
    routes::AppState,
    models::suits::{
        SuitsListResp, ConfigSuitResp, SuitsListApiResp, ConfigSuitApiResp
    }
};
use crate::config::suit::{get_all_config_suits, get_config_suit};
use chrono;

/// Get database pool from app state
async fn get_database(state: &Arc<AppState>) -> Result<Arc<crate::config::database::Database>, StatusCode> {
    match &state.database {
        Some(db) => Ok(db.clone()),
        None => Err(StatusCode::SERVICE_UNAVAILABLE),
    }
}

/// Convert suit database model to response format
fn suit_to_response(suit: &crate::config::models::suit::ConfigSuit) -> ConfigSuitResp {
    ConfigSuitResp {
        id: suit.id.clone().unwrap_or_default(),
        name: suit.name.clone(),
        description: suit.description.clone(),
        suit_type: suit.suit_type.to_string(),
        multi_select: suit.multi_select,
        priority: suit.priority,
        is_active: suit.is_active,
        is_default: suit.is_default,
        allowed_operations: vec!["update".to_string(), "delete".to_string()],
    }
}

/// List all configuration suits
pub async fn list_suits(
    State(state): State<Arc<AppState>>
) -> Result<Json<SuitsListApiResp>, StatusCode> {
    // Get database reference
    let db = get_database(&state).await?;

    // Get all configuration suits
    let suits = get_all_config_suits(&db.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Convert to response format
    let suit_responses = suits.iter().map(suit_to_response).collect();

    let list_resp = SuitsListResp {
        suits: suit_responses,
        total: suits.len(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    // Return response
    Ok(Json(SuitsListApiResp::success(list_resp)))
}

/// Get a specific configuration suit
pub async fn get_suit(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ConfigSuitApiResp>, StatusCode> {
    // Get database reference
    let db = get_database(&state).await?;

    // Get the configuration suit
    let suit = get_config_suit(&db.pool, &id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Check if the suit exists
    let suit = match suit {
        Some(s) => s,
        None => {
            return Ok(Json(ConfigSuitApiResp::error(
                "NOT_FOUND",
                &format!("Configuration suit with ID '{id}' not found")
            )));
        }
    };

    // Convert to response format
    let response = suit_to_response(&suit);

    // Return response
    Ok(Json(ConfigSuitApiResp::success(response)))
}
