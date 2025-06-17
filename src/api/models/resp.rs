/// Unified response converter service to eliminate model conversion duplication
pub struct ResponseConverter;

/// Generic success response
#[derive(serde::Serialize)]
pub struct SuccessResponse {
    /// Success message
    pub message: String,
}

/// Generic error response
#[derive(serde::Serialize)]
pub struct ErrorResponse {
    /// Error details
    pub error: ErrorDetails,
}

/// Error details
#[derive(serde::Serialize)]
pub struct ErrorDetails {
    /// Error message
    pub message: String,
    /// HTTP status code
    pub status: u16,
}

impl ResponseConverter {
    /// Convert ConfigSuit to ConfigSuitResponse with consistent logic
    pub fn suit_to_response(
        suit: &crate::config::models::ConfigSuit
    ) -> crate::api::models::suits::ConfigSuitResponse {
        let mut allowed_operations = Vec::new();

        // Add allowed operations based on current state
        if suit.is_active {
            allowed_operations.push("deactivate".to_string());
        } else {
            allowed_operations.push("activate".to_string());
        }

        // Always allow update and delete
        allowed_operations.push("update".to_string());
        allowed_operations.push("delete".to_string());

        crate::api::models::suits::ConfigSuitResponse {
            id: suit.id.clone().unwrap_or_default(),
            name: suit.name.clone(),
            description: suit.description.clone(),
            suit_type: suit.suit_type_string(),
            multi_select: suit.multi_select,
            priority: suit.priority,
            is_active: suit.is_active,
            is_default: suit.is_default,
            allowed_operations,
        }
    }

    /// Convert Server to ConfigSuitServerResponse with consistent logic
    pub fn server_to_suit_response(
        server: &crate::config::models::Server,
        enabled: bool,
    ) -> crate::api::models::suits::ConfigSuitServerResponse {
        let mut allowed_operations = Vec::new();
        if enabled {
            allowed_operations.push("disable".to_string());
        } else {
            allowed_operations.push("enable".to_string());
        }

        crate::api::models::suits::ConfigSuitServerResponse {
            id: server.id.clone().unwrap_or_default(),
            name: server.name.clone(),
            enabled,
            allowed_operations,
        }
    }

    /// Format timestamps consistently across all API responses
    pub fn format_timestamp(timestamp: Option<chrono::DateTime<chrono::Utc>>) -> Option<String> {
        timestamp.map(|dt| dt.to_rfc3339())
    }

    /// Create consistent error response
    pub fn create_error_response(
        message: &str,
        status: u16,
    ) -> ErrorResponse {
        ErrorResponse {
            error: ErrorDetails {
                message: message.to_string(),
                status,
            },
        }
    }
}
