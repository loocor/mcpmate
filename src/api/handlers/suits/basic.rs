// MCPMate Proxy API handlers for Config Suit basic operations
// Contains handler functions for listing and getting Config Suits

use super::common::*;

/// List all configuration suits
pub async fn list_suits(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ConfigSuitListResponse>, ApiError> {
    // Get database reference
    let db = get_database(&state).await?;

    // Get all configuration suits
    let suits = crate::conf::operations::suit::get_all_config_suits(&db.pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get configuration suits: {}", e)))?;

    // Convert to response format
    let suit_responses = suits.iter().map(suit_to_response).collect();

    // Return response
    Ok(Json(ConfigSuitListResponse {
        suits: suit_responses,
    }))
}

/// Get a specific configuration suit
pub async fn get_suit(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ConfigSuitResponse>, ApiError> {
    // Get database reference
    let db = get_database(&state).await?;

    // Get the configuration suit
    let suit = crate::conf::operations::suit::get_config_suit(&db.pool, &id)
        .await
        .map_err(|e| {
            ApiError::InternalError(format!("Failed to get configuration suit: {}", e))
        })?;

    // Check if the suit exists
    let suit = match suit {
        Some(s) => s,
        None => {
            return Err(ApiError::NotFound(format!(
                "Configuration suit with ID '{}' not found",
                id
            )));
        }
    };

    // Convert to response format
    let response = suit_to_response(&suit);

    // Return response
    Ok(Json(response))
}
