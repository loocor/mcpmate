// Static file serving for the board frontend
// Serves the Vite React app from board/dist/

use std::{path::PathBuf, sync::Arc};

use axum::{
    Router,
    extract::Request,
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use tower_http::services::ServeDir;
use tracing;

use super::AppState;

/// Create static file serving routes
pub fn routes(_state: Arc<AppState>) -> Router {
    // Try multiple possible paths for the frontend files
    let possible_paths = vec![
        PathBuf::from("board/dist"),    // Development/source directory
        PathBuf::from("./board/dist"),  // Explicit relative path
        PathBuf::from("../board/dist"), // One level up
    ];

    let mut board_dist = None;
    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    tracing::info!("Current working directory: {}", current_dir.display());

    // Try to find the frontend files
    for path in &possible_paths {
        let full_path = if path.is_absolute() {
            path.clone()
        } else {
            current_dir.join(path)
        };

        tracing::debug!("Checking for frontend files at: {}", full_path.display());

        if full_path.join("index.html").exists() {
            tracing::info!("Found frontend files at: {}", full_path.display());
            board_dist = Some(path.clone());
            break;
        }
    }

    // If no frontend files found, return error handler
    let board_dist = match board_dist {
        Some(path) => path,
        None => {
            tracing::warn!("Frontend files not found in any of the expected locations:");
            for path in &possible_paths {
                let full_path = if path.is_absolute() {
                    path.clone()
                } else {
                    current_dir.join(path)
                };
                tracing::warn!("  - {}", full_path.display());
            }
            return Router::new().fallback(frontend_not_found_handler);
        }
    };

    tracing::info!("Serving frontend from: {}", board_dist.display());

    // Create the serve directory service with SPA fallback
    let serve_dir =
        ServeDir::new(&board_dist).not_found_service(tower::service_fn(spa_fallback_service));

    // Serve static files at root using fallback_service
    Router::new()
        .route("/", get(serve_index_html))
        .fallback_service(serve_dir)
}

/// Serve index.html for the root path
async fn serve_index_html() -> impl IntoResponse {
    // Try multiple possible paths for index.html
    let possible_paths = vec![
        PathBuf::from("board/dist/index.html"),
        PathBuf::from("./board/dist/index.html"),
        PathBuf::from("../board/dist/index.html"),
    ];

    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    for path in &possible_paths {
        let full_path = if path.is_absolute() {
            path.clone()
        } else {
            current_dir.join(path)
        };

        match tokio::fs::read_to_string(&full_path).await {
            Ok(content) => {
                tracing::debug!("Serving index.html from: {}", full_path.display());
                return (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                    content,
                );
            }
            Err(_) => {
                tracing::debug!("Failed to read index.html from: {}", full_path.display());
                continue;
            }
        }
    }

    // If all paths failed, return error
    tracing::warn!("Could not find index.html in any location");
    (
        StatusCode::NOT_FOUND,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        "Frontend not available".to_string(),
    )
}

/// SPA fallback service - serves index.html for client-side routing
async fn spa_fallback_service(
    req: Request
) -> Result<Response<axum::body::Body>, std::convert::Infallible> {
    let path = req.uri().path();

    // Don't handle API routes - let them return 404
    if path.starts_with("/api") {
        let response = Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(axum::body::Body::from("Not Found"))
            .unwrap();
        return Ok(response);
    }

    // For all other routes, serve index.html to support client-side routing
    // Try multiple possible paths for index.html
    let possible_paths = vec![
        PathBuf::from("board/dist/index.html"),
        PathBuf::from("./board/dist/index.html"),
        PathBuf::from("../board/dist/index.html"),
    ];

    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    for path in &possible_paths {
        let full_path = if path.is_absolute() {
            path.clone()
        } else {
            current_dir.join(path)
        };

        match tokio::fs::read_to_string(&full_path).await {
            Ok(content) => {
                tracing::debug!("Serving SPA fallback from: {}", full_path.display());
                let response = Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                    .body(axum::body::Body::from(content))
                    .unwrap();
                return Ok(response);
            }
            Err(_) => {
                tracing::debug!("Failed to read index.html from: {}", full_path.display());
                continue;
            }
        }
    }

    // If all paths failed
    tracing::warn!("SPA fallback failed - could not find index.html in any location");
    let response = Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(axum::body::Body::from("Frontend not available"))
        .unwrap();
    Ok(response)
}

/// Handler for when frontend is not found
async fn frontend_not_found_handler() -> impl IntoResponse {
    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let html = format!(
        r#"
<!DOCTYPE html>
<html>
<head>
    <title>MCPMate - Frontend Not Found</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 40px; text-align: center; }}
        .container {{ max-width: 600px; margin: 0 auto; }}
        .error {{ color: #d32f2f; }}
        .info {{ color: #1976d2; }}
        .debug {{ color: #666; font-size: 0.9em; margin-top: 20px; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>MCPMate Management Dashboard</h1>
        <p class="error">Frontend files not found.</p>
        <p class="info">Please ensure the board/dist/ directory exists with built frontend files.</p>
        <p>API endpoints are still available at: <a href="/api">/api</a></p>
        <div class="debug">
            <p>Current working directory: {}</p>
            <p>Expected frontend path: board/dist/index.html</p>
        </div>
    </div>
</body>
</html>
    "#,
        current_dir.display()
    );

    (StatusCode::NOT_FOUND, Html(html))
}
