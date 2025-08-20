use aide::{
    openapi::{OpenApi, Server, Tag},
    transform::TransformOpenApi,
};
use axum::{Json, Router, response::Html, routing::get};

/// Create OpenAPI documentation routes
pub fn openapi_routes(api: OpenApi) -> Router {
    Router::new()
        .route("/docs", get(serve_docs))
        .route("/openapi.json", get(move || async { Json(api) }))
}

/// Configure OpenAPI documentation
pub fn api_docs(api: TransformOpenApi) -> TransformOpenApi {
    api.title("MCPMate Management API")
        .description("API for managing MCP servers and instances")
        .version("0.1.0")
        .server(Server {
            url: "/api".into(),
            description: Some("MCPMate API Server".into()),
            ..Default::default()
        })
        .tag(Tag {
            name: "system".into(),
            description: Some("System management endpoints".into()),
            ..Default::default()
        })
        .tag(Tag {
            name: "server".into(),
            description: Some("Server management endpoints".into()),
            ..Default::default()
        })
        .tag(Tag {
            name: "instance".into(),
            description: Some("Instance management endpoints".into()),
            ..Default::default()
        })
        .tag(Tag {
            name: "cache".into(),
            description: Some("Cache management endpoints".into()),
            ..Default::default()
        })
        .tag(Tag {
            name: "suits".into(),
            description: Some("Configuration suit management endpoints".into()),
            ..Default::default()
        })
        .tag(Tag {
            name: "runtime".into(),
            description: Some("Runtime management endpoints".into()),
            ..Default::default()
        })
        .tag(Tag {
            name: "clients".into(),
            description: Some("Client management endpoints".into()),
            ..Default::default()
        })
        .tag(Tag {
            name: "notifs".into(),
            description: Some("Notification management endpoints".into()),
            ..Default::default()
        })
}

/// Serve API documentation using Scalar UI
pub async fn serve_docs() -> Html<&'static str> {
    Html(
        r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>MCPMate API Documentation</title>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
            </head>
            <body>
                <script id="api-reference" data-url="/openapi.json"></script>
                <script src="https://cdn.jsdelivr.net/npm/@scalar/api-reference"></script>
            </body>
            </html>
        "#,
    )
}

/// Macro to generate aide-compatible wrapper and documentation functions
///
/// This macro automatically generates both the `_aide` wrapper function and `_docs`
/// documentation function for legacy handlers that return `Result<Json<T>, ApiError>`.
///
/// # Usage
/// ```rust
/// aide_wrapper!(
///     system::get_status,   // Full handler path (enables VS Code jump + auto tag)
///     StatusResponse,       // Response type
///     "Get system status"  // Description (tag auto-extracted from module name)
/// );
/// ```
///
/// This generates:
/// - `get_status_aide` function compatible with aide
/// - `get_status_docs` function for OpenAPI documentation
///
/// Then use in routes:
/// ```rust
/// .api_route("/system/status", get_with(get_status_aide, get_status_docs))
/// ```
#[macro_export]
macro_rules! aide_wrapper {
    ($module:ident :: $handler:ident, $response_type:ty, $description:expr) => {
        paste::paste! {
            /// Aide-compatible wrapper function
            pub async fn [<$handler _aide>](
                axum::extract::State(state): axum::extract::State<std::sync::Arc<crate::api::routes::AppState>>
            ) -> impl aide::axum::IntoApiResponse {
                use axum::response::IntoResponse;
                match $module::$handler(axum::extract::State(state)).await {
                    Ok(json_response) => json_response.into_response(),
                    Err(api_error) => api_error.into_response(),
                }
            }

            /// Documentation function for OpenAPI generation
            pub fn [<$handler _docs>](
                op: aide::transform::TransformOperation
            ) -> aide::transform::TransformOperation {
                op.description($description)
                    .tag(stringify!($module))  // auto-extract tag from module name
                    .response::<200, axum::Json<$response_type>>()
                    .response::<500, ()>()
            }
        }
    };
}
