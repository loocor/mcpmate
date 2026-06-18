use std::sync::Arc;

use axum::extract::State;
use axum::Json;

use crate::api::handlers::ApiError;
use crate::api::models::llm::*;
use crate::api::routes::AppState;
use crate::config::llm::{crud as llm_crud, models::LlmProviderDefaultParams};
use crate::core::llm::factory;
use crate::core::llm::templates::TemplateEngine;
use crate::core::llm::test_gen::ToolInfo;

fn get_pool(
    state: &AppState,
) -> Result<&sqlx::Pool<sqlx::Sqlite>, ApiError> {
    state
        .database
        .as_ref()
        .map(|db| &db.pool)
        .ok_or_else(|| ApiError::ServiceUnavailable("Database not available".into()))
}

async fn resolve_api_key(state: &AppState, alias: &str) -> Result<String, ApiError> {
    let store_guard = state.secret_store.read().await;
    let store = store_guard
        .as_ref()
        .ok_or_else(|| ApiError::ServiceUnavailable("Secret store not available".into()))?;

    use mcpmate_secrets::SecretResolver;
    use mcpmate_secrets::SecretReference;
    let reference = SecretReference::new(alias)
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    let secret_value = store
        .resolve_secret(&reference)
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok(secret_value.expose().to_string())
}

fn validate_base_url(url: &str) -> Result<(), ApiError> {
    let parsed = url::Url::parse(url)
        .map_err(|_| ApiError::BadRequest("Invalid base URL".into()))?;

    // Reject URLs with userinfo (potential credential injection)
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(ApiError::BadRequest(
            "Base URL must not contain credentials".into(),
        ));
    }

    let host = parsed.host_str().unwrap_or("");
    let is_localhost = matches!(host, "localhost" | "127.0.0.1" | "::1" | "[::1]");

    match parsed.scheme() {
        "https" => {}
        "http" => {
            if !is_localhost {
                return Err(ApiError::BadRequest(
                    "HTTP is only allowed for localhost".into(),
                ));
            }
        }
        _ => {
            return Err(ApiError::BadRequest(
                "Base URL must use http or https scheme".into(),
            ));
        }
    }

    // Block private/link-local IPs for non-localhost hosts
    if !is_localhost {
        if let Ok(ip) = host.parse::<std::net::IpAddr>() {
            if is_private_ip(ip) {
                return Err(ApiError::BadRequest(
                    "Private/link-local IP addresses are not allowed".into(),
                ));
            }
        }
    }

    Ok(())
}

fn is_private_ip(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => {
            v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_broadcast()
                || v4.is_documentation()
        }
        std::net::IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unicast_link_local()
                || is_ipv6_unique_local(v6)
        }
    }
}

fn is_ipv6_unique_local(ip: std::net::Ipv6Addr) -> bool {
    // fc00::/7 — Unique Local Addresses
    (ip.octets()[0] & 0xfe) == 0xfc
}

fn to_provider_data(p: &crate::config::llm::models::LlmProviderConfig) -> LlmProviderData {
    let params = LlmProviderDefaultParams::from_json(&p.default_params_json);
    LlmProviderData {
        id: p.id.clone().unwrap_or_default(),
        name: p.name.clone(),
        provider_type: p.provider_type.clone(),
        base_url: p.base_url.clone(),
        model_id: p.model_id.clone(),
        has_api_key: p.secret_alias.is_some(),
        is_default: p.is_default,
        default_params: LlmDefaultParamsData {
            temperature: params.temperature,
            max_tokens: params.max_tokens,
        },
        created_at: p.created_at.map(|t| t.to_rfc3339()),
        updated_at: p.updated_at.map(|t| t.to_rfc3339()),
    }
}

pub async fn list_providers(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pool = get_pool(&state)?;

    let providers = llm_crud::get_all_providers(pool)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let data: Vec<LlmProviderData> = providers.iter().map(to_provider_data).collect();

    Ok(Json(serde_json::json!({ "success": true, "data": data })))
}

pub async fn create_provider(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LlmProviderCreateReq>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pool = get_pool(&state)?;

    // Validate provider_type
    let _: crate::config::llm::models::LlmProviderType = payload
        .provider_type
        .parse()
        .map_err(|_| ApiError::BadRequest("Invalid provider type".into()))?;

    // Validate base_url
    validate_base_url(&payload.base_url)?;

    let params_json = payload.default_params.map(|p| {
        let params = LlmProviderDefaultParams {
            temperature: p.temperature.unwrap_or(0.7),
            max_tokens: p.max_tokens.unwrap_or(4096),
        };
        serde_json::to_string(&params).unwrap_or_default()
    });

    let secret_alias = if let Some(ref api_key) = payload.api_key {
        if !api_key.is_empty() {
            let store_guard = state.secret_store.read().await;
            let store = store_guard
                .as_ref()
                .ok_or_else(|| ApiError::ServiceUnavailable("Secret store not available".into()))?;

            let alias = format!("llm_provider_{}", uuid::Uuid::new_v4().as_simple());
            store
                .create_secret(crate::core::secrets::store::SecretCreateInput {
                    alias: alias.clone(),
                    kind: crate::core::secrets::store::SecretKindInput::ApiKey,
                    value: api_key.clone(),
                    label: Some(format!("LLM Provider: {}", payload.name)),
                    origin: None,
                })
                .await
                .map_err(|e| ApiError::InternalError(e.to_string()))?;

            Some(alias)
        } else {
            None
        }
    } else {
        None
    };

    let provider = llm_crud::create_provider(
        pool,
        &payload.name,
        &payload.provider_type,
        &payload.base_url,
        &payload.model_id,
        secret_alias.as_deref(),
        params_json.as_deref(),
    )
    .await
    .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let data = to_provider_data(&provider);
    Ok(Json(serde_json::json!({ "success": true, "data": data })))
}

pub async fn update_provider(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LlmProviderUpdateReq>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pool = get_pool(&state)?;
    let id = payload.id;

    // Validate provider_type if provided
    if let Some(ref pt) = payload.provider_type {
        let _: crate::config::llm::models::LlmProviderType = pt
            .parse()
            .map_err(|_| ApiError::BadRequest("Invalid provider type".into()))?;
    }

    // Validate base_url if provided
    if let Some(ref url) = payload.base_url {
        validate_base_url(url)?;
    }

    let existing = llm_crud::get_provider_by_id(pool, &id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Provider not found".into()))?;

    // Handle secret rotation: create new secret first, then delete old
    let mut new_secret_alias: Option<String> = None;
    let mut old_alias_to_delete: Option<String> = None;

    if let Some(ref api_key_opt) = payload.api_key {
        if let Some(api_key) = api_key_opt {
            if !api_key.is_empty() {
                let store_guard = state.secret_store.read().await;
                let store = store_guard
                    .as_ref()
                    .ok_or_else(|| ApiError::ServiceUnavailable("Secret store not available".into()))?;

                // Create new secret first
                let alias = format!("llm_provider_{}", uuid::Uuid::new_v4().as_simple());
                store
                    .create_secret(crate::core::secrets::store::SecretCreateInput {
                        alias: alias.clone(),
                        kind: crate::core::secrets::store::SecretKindInput::ApiKey,
                        value: api_key.clone(),
                        label: Some(format!("LLM Provider: {}", existing.name)),
                        origin: None,
                    })
                    .await
                    .map_err(|e| ApiError::InternalError(e.to_string()))?;

                new_secret_alias = Some(alias);
                old_alias_to_delete = existing.secret_alias.clone();
            }
        } else {
            // Explicit null = remove the key
            old_alias_to_delete = existing.secret_alias.clone();
        }
    }

    let params_json = payload.default_params.map(|p| {
        let existing_params = LlmProviderDefaultParams::from_json(&existing.default_params_json);
        let params = LlmProviderDefaultParams {
            temperature: p.temperature.unwrap_or(existing_params.temperature),
            max_tokens: p.max_tokens.unwrap_or(existing_params.max_tokens),
        };
        serde_json::to_string(&params).unwrap_or_default()
    });

    let secret_alias_for_update = new_secret_alias.as_deref().or(existing.secret_alias.as_deref());

    let provider = llm_crud::update_provider(
        pool,
        &id,
        payload.name.as_deref(),
        payload.provider_type.as_deref(),
        payload.base_url.as_deref(),
        payload.model_id.as_deref(),
        Some(secret_alias_for_update),
        Some(params_json.as_deref()),
    )
    .await
    .map_err(|e| ApiError::InternalError(e.to_string()))?
    .ok_or_else(|| ApiError::NotFound("Provider not found".into()))?;

    // Delete old secret only after successful DB update
    if let Some(old_alias) = old_alias_to_delete {
        let store_guard = state.secret_store.read().await;
        if let Some(store) = store_guard.as_ref() {
            let _ = store.delete_secret(&old_alias, false).await;
        }
    }

    let data = to_provider_data(&provider);
    Ok(Json(serde_json::json!({ "success": true, "data": data })))
}

pub async fn delete_provider(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LlmProviderIdReq>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pool = get_pool(&state)?;

    let existing = llm_crud::get_provider_by_id(pool, &payload.id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Provider not found".into()))?;

    if let Some(ref alias) = existing.secret_alias {
        let store_guard = state.secret_store.read().await;
        if let Some(store) = store_guard.as_ref() {
            let _ = store.delete_secret(alias, false).await;
        }
    }

    llm_crud::delete_provider(pool, &payload.id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(Json(serde_json::json!({ "success": true, "data": null })))
}

pub async fn test_provider(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LlmProviderIdReq>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pool = get_pool(&state)?;

    let config = llm_crud::get_provider_by_id(pool, &payload.id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Provider not found".into()))?;

    let api_key = if let Some(ref alias) = config.secret_alias {
        resolve_api_key(&state, alias).await?
    } else {
        String::new()
    };

    let provider =
        factory::create_provider(&config, &api_key).map_err(|e| ApiError::InternalError(e.to_string()))?;

    let result = provider
        .test_connectivity()
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let data = LlmConnectivityResult {
        success: result.success,
        latency_ms: result.latency_ms,
        model: result.model,
        error: result.error,
    };

    Ok(Json(serde_json::json!({ "success": true, "data": data })))
}

pub async fn list_models(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LlmProviderIdReq>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pool = get_pool(&state)?;

    let config = llm_crud::get_provider_by_id(pool, &payload.id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Provider not found".into()))?;

    let api_key = if let Some(ref alias) = config.secret_alias {
        resolve_api_key(&state, alias).await?
    } else {
        String::new()
    };

    let provider =
        factory::create_provider(&config, &api_key).map_err(|e| ApiError::InternalError(e.to_string()))?;

    let models = provider
        .list_models()
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(Json(
        serde_json::json!({ "success": true, "data": { "models": models } }),
    ))
}

pub async fn generate_tests(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LlmTestGenerateReq>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pool = get_pool(&state)?;

    let provider_config = llm_crud::get_provider_by_id(pool, &payload.provider_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Provider not found".into()))?;

    let api_key = if let Some(ref alias) = provider_config.secret_alias {
        resolve_api_key(&state, alias).await?
    } else {
        String::new()
    };

    let provider = factory::create_provider(&provider_config, &api_key)
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let tool_info = get_tool_info_from_inspector(&state, &payload.server_id, &payload.tool_name)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let template_dir = find_template_dir();

    let templates = TemplateEngine::new(template_dir);

    let count = payload.count.min(20);

    let cases = crate::core::llm::test_gen::generate_test_cases(
        provider.as_ref(),
        &templates,
        &tool_info,
        payload.template_name.as_deref(),
        payload.custom_scenario.as_deref(),
        count,
    )
    .await
    .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let data: Vec<LlmTestCaseData> = cases
        .into_iter()
        .map(|c| LlmTestCaseData {
            id: c.id,
            params: c.params,
            description: c.description,
            test_type: serde_json::to_string(&c.test_type)
                .unwrap_or_default()
                .trim_matches('"')
                .to_string(),
            expected_behavior: c.expected_behavior,
        })
        .collect();

    Ok(Json(serde_json::json!({ "success": true, "data": data })))
}

pub async fn run_tests(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LlmTestRunReq>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let run_id = uuid::Uuid::new_v4().to_string();
    let total = payload.cases.len() as u32;

    let mut results = Vec::new();

    for case in &payload.cases {
        let arguments = match &case.params {
            serde_json::Value::Object(map) => Some(map.clone()),
            _ => None,
        };

        let call_req = crate::api::models::inspector::InspectorToolCallReq {
            tool: payload.tool_name.clone(),
            server_id: Some(payload.server_id.clone()),
            server_name: None,
            arguments,
            mode: crate::api::models::inspector::InspectorMode::Native,
            timeout_ms: Some(30000),
            session_id: None,
        };

        match crate::inspector::service::call_tool(&state, &call_req).await {
            Ok(outcome) => {
                let status = if outcome.message.is_some() {
                    "Error"
                } else {
                    "Passed"
                };
                results.push(LlmTestResultData {
                    case_id: case.id.clone(),
                    params: case.params.clone(),
                    actual_response: outcome.result,
                    latency_ms: outcome.elapsed_ms,
                    status: status.to_string(),
                    error_message: outcome.message,
                });
            }
            Err(e) => {
                results.push(LlmTestResultData {
                    case_id: case.id.clone(),
                    params: case.params.clone(),
                    actual_response: None,
                    latency_ms: 0,
                    status: "Error".to_string(),
                    error_message: Some(e.to_string()),
                });
            }
        }
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "data": {
            "run_id": run_id,
            "total_cases": total,
            "results": results
        }
    })))
}

pub async fn set_default_provider(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LlmProviderIdReq>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pool = get_pool(&state)?;

    // Verify provider exists
    llm_crud::get_provider_by_id(pool, &payload.id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Provider not found".into()))?;

    llm_crud::set_default_provider(pool, &payload.id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(Json(serde_json::json!({ "success": true, "data": null })))
}

pub async fn get_default_provider(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pool = get_pool(&state)?;

    let provider = llm_crud::get_default_provider(pool)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let data = provider.map(|p| to_provider_data(&p));
    Ok(Json(serde_json::json!({ "success": true, "data": data })))
}

fn find_template_dir() -> std::path::PathBuf {
    // Try relative to binary location first (production)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let candidate = exe_dir.join("templates").join("llm");
            if candidate.exists() {
                return candidate;
            }
        }
    }

    // Try CARGO_MANIFEST_DIR (development)
    let manifest_dir = option_env!("CARGO_MANIFEST_DIR").unwrap_or(".");
    let candidate = std::path::PathBuf::from(manifest_dir).join("templates").join("llm");
    if candidate.exists() {
        return candidate;
    }

    // Fallback
    std::path::PathBuf::from("backend/templates/llm")
}

async fn get_tool_info_from_inspector(
    state: &Arc<AppState>,
    server_id: &str,
    tool_name: &str,
) -> anyhow::Result<ToolInfo> {
    let query = crate::api::models::inspector::InspectorListQuery {
        server_id: Some(server_id.to_string()),
        server_name: None,
        session_id: None,
        mode: crate::api::models::inspector::InspectorMode::Native,
        refresh: false,
    };

    let tools_json = crate::inspector::service::list_tools(state, &query)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to list tools: {:?}", e))?;

    let tools_array = tools_json
        .get("tools")
        .and_then(|t| t.as_array())
        .ok_or_else(|| anyhow::anyhow!("Invalid tools response format"))?;

    let tool_value = tools_array
        .iter()
        .find(|t| t.get("name").and_then(|n| n.as_str()) == Some(tool_name))
        .ok_or_else(|| anyhow::anyhow!("Tool '{}' not found on server '{}'", tool_name, server_id))?;

    Ok(ToolInfo {
        name: tool_name.to_string(),
        description: tool_value
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_string(),
        input_schema: tool_value
            .get("inputSchema")
            .cloned()
            .unwrap_or(serde_json::Value::Object(Default::default())),
        output_schema: tool_value.get("outputSchema").cloned(),
        annotations: tool_value.get("annotations").cloned(),
    })
}
