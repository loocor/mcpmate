use std::num::NonZeroU32;
use std::sync::Arc;

use axum::{
    Json,
    extract::State,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use ring::pbkdf2;

use crate::{
    api::{
        handlers::ApiError,
        models::secrets::{
            PasswordChangeReq, PasswordClearReq, PasswordScopeUpdateReq, PasswordSetData, PasswordSetReq, PasswordSetResp,
            PasswordStatusData, PasswordStatusResp, PasswordVerifyData, PasswordVerifyReq, PasswordVerifyResp,
        },
        routes::AppState,
    },
};

const PASSWORD_HASH_ITERATIONS: NonZeroU32 = match NonZeroU32::new(600_000) {
    Some(n) => n,
    None => unreachable!(),
};
const PASSWORD_SALT_LEN: usize = 16;
const PBKDF2_ALG: pbkdf2::Algorithm = pbkdf2::PBKDF2_HMAC_SHA256;
const HASH_LEN: usize = 32;

fn derive_password_hash(password: &[u8], salt: &[u8], iterations: NonZeroU32) -> Vec<u8> {
    let mut hash = vec![0u8; HASH_LEN];
    pbkdf2::derive(PBKDF2_ALG, iterations, salt, password, &mut hash);
    hash
}

fn verify_password_hash(password: &[u8], salt: &[u8], iterations: NonZeroU32, expected: &[u8]) -> bool {
    pbkdf2::verify(PBKDF2_ALG, iterations, salt, password, expected).is_ok()
}

fn generate_salt() -> [u8; PASSWORD_SALT_LEN] {
    let rng = ring::rand::SystemRandom::new();
    let mut salt = [0u8; PASSWORD_SALT_LEN];
    ring::rand::SecureRandom::fill(&rng, &mut salt).expect("generate salt");
    salt
}

fn get_pool(state: &Arc<AppState>) -> Result<sqlx::Pool<sqlx::Sqlite>, ApiError> {
    state
        .database
        .as_ref()
        .map(|db| db.pool.clone())
        .ok_or_else(|| ApiError::ServiceUnavailable("No database configured".to_string()))
}

fn parse_scope(scope_str: &str) -> Vec<String> {
    serde_json::from_str(scope_str).unwrap_or_else(|_| vec!["startup".to_string()])
}

fn scope_to_json(scope: &[String]) -> String {
    serde_json::to_string(scope).unwrap_or_else(|_| r#"["startup"]"#.to_string())
}

fn default_scope() -> Vec<String> {
    vec!["startup".to_string()]
}

fn iterations_from_i64(n: i64) -> NonZeroU32 {
    NonZeroU32::new(n.max(1) as u32).unwrap_or(PASSWORD_HASH_ITERATIONS)
}

// ── Handlers ─────────────────────────────────────────────────

/// Load password config and verify the supplied password against it.
/// Returns `Ok(true)` if verified, `Ok(false)` if no password is set or verification fails,
/// or `Err` on internal errors (corrupt stored data).
async fn load_and_verify_password(
    pool: &sqlx::Pool<sqlx::Sqlite>,
    password: &str,
) -> Result<bool, ApiError> {
    let config = mcpmate_secrets::database::get_password_config(pool)
        .await
        .map_err(|err| ApiError::InternalError(format!("Failed to load password config: {err}")))?;

    let Some(cfg) = config else {
        return Ok(false);
    };

    let stored_hash = STANDARD
        .decode(&cfg.password_hash)
        .map_err(|_| ApiError::InternalError("Invalid stored hash".to_string()))?;
    let salt = STANDARD
        .decode(&cfg.hash_salt)
        .map_err(|_| ApiError::InternalError("Invalid stored salt".to_string()))?;

    Ok(verify_password_hash(
        password.as_bytes(),
        &salt,
        iterations_from_i64(cfg.hash_iterations),
        &stored_hash,
    ))
}

pub async fn get_password_status(
    State(state): State<Arc<AppState>>,
) -> Result<Json<PasswordStatusResp>, ApiError> {
    let pool = get_pool(&state)?;
    let config = mcpmate_secrets::database::get_password_config(&pool)
        .await
        .map_err(|err| ApiError::InternalError(format!("Failed to load password config: {err}")))?;

    match config {
        Some(cfg) => Ok(Json(PasswordStatusResp::success(PasswordStatusData {
            enabled: true,
            scope: parse_scope(&cfg.protection_scope),
            has_password: true,
        }))),
        None => Ok(Json(PasswordStatusResp::success(PasswordStatusData {
            enabled: false,
            scope: default_scope(),
            has_password: false,
        }))),
    }
}

pub async fn set_password(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PasswordSetReq>,
) -> Result<Json<PasswordSetResp>, ApiError> {
    if payload.password.is_empty() {
        return Err(ApiError::BadRequest("Password cannot be empty".to_string()));
    }
    if payload.password != payload.confirm {
        return Err(ApiError::BadRequest("Passwords do not match".to_string()));
    }
    if payload.password.len() < 4 {
        return Err(ApiError::BadRequest(
            "Password must be at least 4 characters".to_string(),
        ));
    }

    let pool = get_pool(&state)?;

    // Guard: if a password is already set, reject the overwrite.
    // Use the change_password endpoint to modify an existing password.
    let existing = mcpmate_secrets::database::get_password_config(&pool)
        .await
        .map_err(|err| ApiError::InternalError(format!("Failed to load password config: {err}")))?;
    if existing.is_some() {
        return Err(ApiError::Conflict(
            "A password is already set. Use the change-password endpoint to modify it.".to_string(),
        ));
    }

    let scope = payload.scope.unwrap_or_else(default_scope);
    let salt = generate_salt();
    let hash = derive_password_hash(payload.password.as_bytes(), &salt, PASSWORD_HASH_ITERATIONS);

    mcpmate_secrets::database::upsert_password_config(
        &pool,
        &STANDARD.encode(&hash),
        &STANDARD.encode(salt),
        PASSWORD_HASH_ITERATIONS.get() as i64,
        &scope_to_json(&scope),
    )
    .await
    .map_err(|err| ApiError::InternalError(format!("Failed to save password: {err}")))?;

    Ok(Json(PasswordSetResp::success(PasswordSetData {
        enabled: true,
        scope,
    })))
}

pub async fn verify_password_endpoint(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PasswordVerifyReq>,
) -> Result<Json<PasswordVerifyResp>, ApiError> {
    let pool = get_pool(&state)?;
    let valid = load_and_verify_password(&pool, &payload.password).await?;
    Ok(Json(PasswordVerifyResp::success(PasswordVerifyData { valid })))
}

pub async fn change_password(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PasswordChangeReq>,
) -> Result<Json<PasswordStatusResp>, ApiError> {
    if payload.new_password.is_empty() {
        return Err(ApiError::BadRequest("New password cannot be empty".to_string()));
    }
    if payload.new_password != payload.confirm {
        return Err(ApiError::BadRequest("Passwords do not match".to_string()));
    }
    if payload.new_password.len() < 4 {
        return Err(ApiError::BadRequest(
            "Password must be at least 4 characters".to_string(),
        ));
    }

    let pool = get_pool(&state)?;
    let config = mcpmate_secrets::database::get_password_config(&pool)
        .await
        .map_err(|err| ApiError::InternalError(format!("Failed to load password config: {err}")))?
        .ok_or_else(|| ApiError::BadRequest("No password is set".to_string()))?;

    // Verify old password.
    if !load_and_verify_password(&pool, &payload.old_password).await? {
        return Err(ApiError::BadRequest("Current password is incorrect".to_string()));
    }

    // Set new password.
    let new_salt = generate_salt();
    let new_hash = derive_password_hash(payload.new_password.as_bytes(), &new_salt, PASSWORD_HASH_ITERATIONS);
    let scope = parse_scope(&config.protection_scope);

    mcpmate_secrets::database::upsert_password_config(
        &pool,
        &STANDARD.encode(&new_hash),
        &STANDARD.encode(new_salt),
        PASSWORD_HASH_ITERATIONS.get() as i64,
        &config.protection_scope,
    )
    .await
    .map_err(|err| ApiError::InternalError(format!("Failed to save new password: {err}")))?;

    Ok(Json(PasswordStatusResp::success(PasswordStatusData {
        enabled: true,
        scope,
        has_password: true,
    })))
}

pub async fn clear_password(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PasswordClearReq>,
) -> Result<Json<PasswordStatusResp>, ApiError> {
    let pool = get_pool(&state)?;

    // Guard: no password is set.
    let has_config = mcpmate_secrets::database::get_password_config(&pool)
        .await
        .map_err(|err| ApiError::InternalError(format!("Failed to load password config: {err}")))?
        .is_some();
    if !has_config {
        return Err(ApiError::BadRequest("No password is set".to_string()));
    }

    // Verify password before clearing.
    if !load_and_verify_password(&pool, &payload.password).await? {
        return Err(ApiError::BadRequest("Password is incorrect".to_string()));
    }

    mcpmate_secrets::database::delete_password_config(&pool)
        .await
        .map_err(|err| ApiError::InternalError(format!("Failed to clear password: {err}")))?;

    Ok(Json(PasswordStatusResp::success(PasswordStatusData {
        enabled: false,
        scope: default_scope(),
        has_password: false,
    })))
}

pub async fn update_password_scope(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PasswordScopeUpdateReq>,
) -> Result<Json<PasswordStatusResp>, ApiError> {
    if payload.scope.is_empty() {
        return Err(ApiError::BadRequest("Scope cannot be empty".to_string()));
    }
    for entry in &payload.scope {
        if entry != "startup" && entry != "settings" {
            return Err(ApiError::BadRequest(format!(
                "Unsupported protection scope '{entry}'"
            )));
        }
    }

    let pool = get_pool(&state)?;
    let config = mcpmate_secrets::database::get_password_config(&pool)
        .await
        .map_err(|err| ApiError::InternalError(format!("Failed to load password config: {err}")))?
        .ok_or_else(|| ApiError::BadRequest("No password is set".to_string()))?;

    // Verify current password before allowing scope change.
    if !load_and_verify_password(&pool, &payload.current_password).await? {
        return Err(ApiError::BadRequest("Current password is incorrect".to_string()));
    }

    mcpmate_secrets::database::upsert_password_config(
        &pool,
        &config.password_hash,
        &config.hash_salt,
        config.hash_iterations,
        &scope_to_json(&payload.scope),
    )
    .await
    .map_err(|err| ApiError::InternalError(format!("Failed to update password scope: {err}")))?;

    Ok(Json(PasswordStatusResp::success(PasswordStatusData {
        enabled: true,
        scope: payload.scope,
        has_password: true,
    })))
}
