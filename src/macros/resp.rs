// API Response Macros
// Unified macro definitions for generating consistent API response structures

/// Macro to generate API response structures with consistent pattern
///
/// This macro generates response structures that maintain clean OpenAPI documentation
/// while eliminating code duplication. Each generated struct has its own name in
/// the OpenAPI schema, avoiding generic naming issues.
///
/// # Usage
/// ```rust
/// api_resp!(MyResp, MyData, "My API response description");
/// ```
///
/// # Generated Methods
/// - `success(data: T) -> Self` - Create successful response
/// - `error(error: ApiError) -> Self` - Create error response  
/// - `error_simple(code: &str, message: &str) -> Self` - Create simple error
/// - `error_details(code: &str, message: &str, details: Value) -> Self` - Create detailed error
macro_rules! api_resp {
    ($name:ident, $data_type:ty, $description:expr) => {
        #[derive(Debug, Serialize, JsonSchema)]
        #[schemars(description = $description)]
        pub struct $name {
            #[schemars(description = "Whether the operation was successful")]
            pub success: bool,
            #[schemars(description = "Response data when successful")]
            pub data: Option<$data_type>,
            #[schemars(description = "Error information when failed")]
            pub error: Option<crate::api::models::client::ApiError>,
        }

        impl $name {
            /// Create a successful response with data
            pub fn success(data: $data_type) -> Self {
                Self {
                    success: true,
                    data: Some(data),
                    error: None,
                }
            }

            /// Create an error response from ApiError object
            pub fn error(error: crate::api::models::client::ApiError) -> Self {
                Self {
                    success: false,
                    data: None,
                    error: Some(error),
                }
            }

            /// Create a simple error response with code and message
            pub fn error_simple(
                code: &str,
                message: &str,
            ) -> Self {
                Self::error(crate::api::models::client::ApiError {
                    code: code.to_string(),
                    message: message.to_string(),
                    details: None,
                })
            }

            /// Create an error response with additional details
            pub fn error_details(
                code: &str,
                message: &str,
                details: serde_json::Value,
            ) -> Self {
                Self::error(crate::api::models::client::ApiError {
                    code: code.to_string(),
                    message: message.to_string(),
                    details: Some(details),
                })
            }
        }
    };
}

// Export the macro for use in other modules
pub(crate) use api_resp;
