use anyhow::{Context, Result, anyhow, bail};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{Duration, Utc};
use reqwest::Url;
use reqwest::header::WWW_AUTHENTICATE;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

use crate::{
    common::server::ServerType,
    config::{models::{ServerOAuthConfig, ServerOAuthToken}, server},
};

use super::types::{OAuthConfigInput, OAuthConnectionState, OAuthInitiateResult, OAuthPrepareInput, OAuthStatus};

#[derive(Debug, Clone)]
struct PendingOAuthFlow {
    server_id: String,
    code_verifier: String,
    created_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct OAuthTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    token_type: Option<String>,
    expires_in: Option<i64>,
    scope: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProtectedResourceMetadata {
    authorization_servers: Option<Vec<String>>,
    scopes_supported: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct AuthorizationServerMetadata {
    authorization_endpoint: String,
    token_endpoint: String,
    registration_endpoint: Option<String>,
    scopes_supported: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct DynamicClientRegistrationResponse {
    client_id: String,
    client_secret: Option<String>,
    scope: Option<String>,
}

#[derive(Clone)]
pub struct OAuthManager {
    pool: SqlitePool,
    pending_flows: Arc<Mutex<HashMap<String, PendingOAuthFlow>>>,
    http_client: reqwest::Client,
}

impl OAuthManager {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            pending_flows: Arc::new(Mutex::new(HashMap::new())),
            http_client: reqwest::Client::new(),
        }
    }

    pub async fn upsert_config(&self, server_id: &str, input: OAuthConfigInput) -> Result<OAuthStatus> {
        let server_model = server::get_server_by_id(&self.pool, server_id)
            .await?
            .ok_or_else(|| anyhow!("Server '{}' not found", server_id))?;
        if server_model.server_type != ServerType::StreamableHttp {
            bail!("OAuth is only supported for streamable_http servers");
        }

        let existing = server::get_server_oauth_config(&self.pool, server_id).await?;
        let config = ServerOAuthConfig {
            id: existing.as_ref().and_then(|item| item.id.clone()),
            server_id: server_id.to_string(),
            authorization_endpoint: input.authorization_endpoint.trim().to_string(),
            token_endpoint: input.token_endpoint.trim().to_string(),
            client_id: input.client_id.trim().to_string(),
            client_secret: match input.client_secret {
                Some(secret) if !secret.trim().is_empty() => Some(secret.trim().to_string()),
                _ => existing.and_then(|item| item.client_secret),
            },
            scopes: input.scopes.and_then(|scopes| {
                let trimmed = scopes.trim().to_string();
                (!trimmed.is_empty()).then_some(trimmed)
            }),
            redirect_uri: input.redirect_uri.trim().to_string(),
            created_at: None,
            updated_at: None,
        };
        server::upsert_server_oauth_config(&self.pool, &config).await?;
        self.get_status(server_id).await
    }

    pub async fn prepare(&self, server_id: &str, input: OAuthPrepareInput) -> Result<OAuthStatus> {
        let server_model = server::get_server_by_id(&self.pool, server_id)
            .await?
            .ok_or_else(|| anyhow!("Server '{}' not found", server_id))?;
        if server_model.server_type != ServerType::StreamableHttp {
            bail!("OAuth is only supported for streamable_http servers");
        }

        let server_url = server_model
            .url
            .as_deref()
            .ok_or_else(|| anyhow!("Streamable HTTP server is missing a URL"))?;
        let resource_url = Url::parse(server_url)
            .with_context(|| format!("Invalid streamable HTTP server URL '{}'", server_url))?;

        let existing = server::get_server_oauth_config(&self.pool, server_id).await?;
        if let Some(existing_config) = existing.as_ref() {
            if !existing_config.authorization_endpoint.trim().is_empty()
                && !existing_config.token_endpoint.trim().is_empty()
                && !existing_config.client_id.trim().is_empty()
            {
                return self
                    .upsert_config(
                        server_id,
                        OAuthConfigInput {
                            authorization_endpoint: existing_config.authorization_endpoint.clone(),
                            token_endpoint: existing_config.token_endpoint.clone(),
                            client_id: existing_config.client_id.clone(),
                            client_secret: existing_config.client_secret.clone(),
                            scopes: input.scopes.or_else(|| existing_config.scopes.clone()),
                            redirect_uri: input.redirect_uri,
                        },
                    )
                    .await;
            }
        }

        let protected_metadata = self.discover_protected_resource_metadata(&resource_url).await?;
        let authorization_server = protected_metadata
            .authorization_servers
            .as_ref()
            .and_then(|servers| servers.first())
            .cloned()
            .unwrap_or_else(|| resource_origin(&resource_url));
        let authorization_metadata = self
            .discover_authorization_server_metadata(&authorization_server)
            .await?;

        let registration = match authorization_metadata.registration_endpoint.as_deref() {
            Some(endpoint) => Some(self.register_dynamic_client(endpoint, &input.redirect_uri).await?),
            None => None,
        };

        let client_id = registration
            .as_ref()
            .map(|item| item.client_id.clone())
            .ok_or_else(|| anyhow!("OAuth discovery succeeded, but the authorization server does not support automatic client registration yet"))?;
        let scopes = input
            .scopes
            .or_else(|| registration.as_ref().and_then(|item| item.scope.clone()))
            .or_else(|| protected_metadata.scopes_supported.as_ref().map(|scopes| scopes.join(" ")))
            .or_else(|| authorization_metadata.scopes_supported.as_ref().map(|scopes| scopes.join(" ")));

        self.upsert_config(
            server_id,
            OAuthConfigInput {
                authorization_endpoint: authorization_metadata.authorization_endpoint,
                token_endpoint: authorization_metadata.token_endpoint,
                client_id,
                client_secret: registration.and_then(|item| item.client_secret),
                scopes,
                redirect_uri: input.redirect_uri,
            },
        )
        .await
    }

    pub async fn initiate(&self, server_id: &str) -> Result<OAuthInitiateResult> {
        let server_model = server::get_server_by_id(&self.pool, server_id)
            .await?
            .ok_or_else(|| anyhow!("Server '{}' not found", server_id))?;
        if server_model.server_type != ServerType::StreamableHttp {
            bail!("OAuth is only supported for streamable_http servers");
        }
        let config = server::get_server_oauth_config(&self.pool, server_id)
            .await?
            .ok_or_else(|| anyhow!("OAuth is not configured for server '{}'", server_id))?;
        let resource = oauth_resource_from_server(&server_model)?;

        let state = generate_oauth_random(32);
        let code_verifier = generate_oauth_random(64);
        let code_challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(code_verifier.as_bytes()));

        let mut url = Url::parse(&config.authorization_endpoint)
            .with_context(|| format!("Invalid authorization endpoint '{}'", config.authorization_endpoint))?;
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("response_type", "code");
            query.append_pair("client_id", &config.client_id);
            query.append_pair("redirect_uri", &config.redirect_uri);
            query.append_pair("resource", &resource);
            query.append_pair("state", &state);
            query.append_pair("code_challenge", &code_challenge);
            query.append_pair("code_challenge_method", "S256");
            if let Some(scopes) = config.scopes.as_ref().filter(|value| !value.trim().is_empty()) {
                query.append_pair("scope", scopes);
            }
        }

        let mut flows = self.pending_flows.lock().await;
        flows.retain(|_, flow| Utc::now() - flow.created_at < Duration::minutes(10));
        flows.insert(
            state.clone(),
            PendingOAuthFlow {
                server_id: server_id.to_string(),
                code_verifier,
                created_at: Utc::now(),
            },
        );

        Ok(OAuthInitiateResult {
            server_id: server_id.to_string(),
            authorization_url: url.to_string(),
            state,
        })
    }

    pub async fn exchange_code(&self, state: &str, code: &str) -> Result<OAuthStatus> {
        let pending = {
            let mut flows = self.pending_flows.lock().await;
            flows.remove(state)
        }
        .ok_or_else(|| anyhow!("Invalid or expired OAuth state"))?;

        let config = server::get_server_oauth_config(&self.pool, &pending.server_id)
            .await?
            .ok_or_else(|| anyhow!("OAuth config missing for server '{}'", pending.server_id))?;
        let server_model = server::get_server_by_id(&self.pool, &pending.server_id)
            .await?
            .ok_or_else(|| anyhow!("Server '{}' not found", pending.server_id))?;
        if server_model.server_type != ServerType::StreamableHttp {
            bail!("OAuth is only supported for streamable_http servers");
        }
        let resource = oauth_resource_from_server(&server_model)?;

        let mut form = vec![
            ("grant_type", "authorization_code".to_string()),
            ("code", code.to_string()),
            ("redirect_uri", config.redirect_uri.clone()),
            ("client_id", config.client_id.clone()),
            ("code_verifier", pending.code_verifier.clone()),
            ("resource", resource),
        ];
        if let Some(secret) = config.client_secret.as_ref().filter(|value| !value.trim().is_empty()) {
            form.push(("client_secret", secret.clone()));
        }

        let encoded_body = {
            let mut serializer = url::form_urlencoded::Serializer::new(String::new());
            for (key, value) in &form {
                serializer.append_pair(key, value);
            }
            serializer.finish()
        };

        let response = self
            .http_client
            .post(&config.token_endpoint)
            .header(reqwest::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(encoded_body)
            .send()
            .await
            .context("Failed to call OAuth token endpoint")?;

        if !response.status().is_success() {
            let status = response.status();
            bail!("OAuth token endpoint returned error status: {status}");
        }

        let token_response = response
            .json::<OAuthTokenResponse>()
            .await
            .context("Failed to parse OAuth token response")?;

        let expires_at = token_response
            .expires_in
            .map(|seconds| (Utc::now() + Duration::seconds(seconds)).to_rfc3339());

        server::upsert_server_oauth_token(
            &self.pool,
            &ServerOAuthToken {
                id: None,
                server_id: pending.server_id.clone(),
                access_token: token_response.access_token,
                refresh_token: token_response.refresh_token,
                token_type: token_response.token_type.unwrap_or_else(|| "bearer".to_string()),
                expires_at,
                scope: token_response.scope,
                created_at: None,
                updated_at: None,
            },
        )
        .await?;

        self.get_status(&pending.server_id).await
    }

    pub async fn get_status(&self, server_id: &str) -> Result<OAuthStatus> {
        let config = server::get_server_oauth_config(&self.pool, server_id).await?;
        let token = server::get_server_oauth_token(&self.pool, server_id).await?;
        let manual_headers = server::get_server_headers(&self.pool, server_id).await.ok();
        let manual_authorization_override = server::has_manual_authorization_header(&manual_headers);

        let state = match (&config, &token) {
            (None, _) => OAuthConnectionState::NotConfigured,
            (Some(_), None) => OAuthConnectionState::Disconnected,
            (Some(_), Some(token_row)) => {
                if is_token_expired(token_row) {
                    OAuthConnectionState::Expired
                } else {
                    OAuthConnectionState::Connected
                }
            }
        };

        Ok(OAuthStatus {
            server_id: server_id.to_string(),
            configured: config.is_some(),
            state,
            authorization_endpoint: config.as_ref().map(|item| item.authorization_endpoint.clone()),
            token_endpoint: config.as_ref().map(|item| item.token_endpoint.clone()),
            client_id: config.as_ref().map(|item| item.client_id.clone()),
            scopes: config.as_ref().and_then(|item| item.scopes.clone()),
            redirect_uri: config.as_ref().map(|item| item.redirect_uri.clone()),
            has_client_secret: config
                .as_ref()
                .and_then(|item| item.client_secret.as_ref())
                .map(|value| !value.trim().is_empty())
                .unwrap_or(false),
            manual_authorization_override,
            expires_at: token.and_then(|item| item.expires_at),
        })
    }

    pub async fn revoke(&self, server_id: &str) -> Result<OAuthStatus> {
        server::delete_server_oauth_token(&self.pool, server_id).await?;
        self.get_status(server_id).await
    }
}

impl OAuthManager {
    async fn discover_protected_resource_metadata(&self, resource_url: &Url) -> Result<ProtectedResourceMetadata> {
        if let Some(metadata) = self.discover_protected_metadata_from_challenge(resource_url).await? {
            return Ok(metadata);
        }

        let candidates = protected_resource_candidates(resource_url)?;
        for candidate in candidates {
            if let Ok(metadata) = self.fetch_json::<ProtectedResourceMetadata>(&candidate).await {
                return Ok(metadata);
            }
        }

        Ok(ProtectedResourceMetadata {
            authorization_servers: Some(vec![resource_origin(resource_url)]),
            scopes_supported: None,
        })
    }

    async fn discover_protected_metadata_from_challenge(
        &self,
        resource_url: &Url,
    ) -> Result<Option<ProtectedResourceMetadata>> {
        let response = self.http_client.get(resource_url.clone()).send().await;
        let response = match response {
            Ok(response) => response,
            Err(_) => return Ok(None),
        };
        if response.status() != reqwest::StatusCode::UNAUTHORIZED {
            return Ok(None);
        }

        let mut resource_metadata_url: Option<String> = None;
        let mut authorization_endpoint: Option<String> = None;
        let mut token_endpoint: Option<String> = None;
        let mut scopes: Option<String> = None;

        for value in response.headers().get_all(WWW_AUTHENTICATE) {
            if let Ok(raw) = value.to_str() {
                let params = parse_www_authenticate(raw);
                resource_metadata_url = resource_metadata_url.or_else(|| params.get("resource_metadata").cloned());
                authorization_endpoint = authorization_endpoint.or_else(|| params.get("authorization_uri").cloned());
                token_endpoint = token_endpoint.or_else(|| params.get("token_uri").cloned());
                scopes = scopes.or_else(|| params.get("scope").cloned());
            }
        }

        if let Some(resource_metadata_url) = resource_metadata_url {
            if let Ok(metadata) = self.fetch_json::<ProtectedResourceMetadata>(&resource_metadata_url).await {
                return Ok(Some(metadata));
            }
        }

        if let (Some(authorization_endpoint), Some(_token_endpoint)) = (authorization_endpoint, token_endpoint) {
            return Ok(Some(ProtectedResourceMetadata {
                authorization_servers: Some(vec![issuer_from_endpoint(&authorization_endpoint)]),
                scopes_supported: scopes.map(|value| value.split_whitespace().map(ToOwned::to_owned).collect()),
            }));
        }

        Ok(None)
    }

    async fn discover_authorization_server_metadata(
        &self,
        issuer_or_base: &str,
    ) -> Result<AuthorizationServerMetadata> {
        let mut candidates = Vec::new();
        let base = issuer_or_base.trim_end_matches('/');
        candidates.push(format!("{base}/.well-known/oauth-authorization-server"));
        candidates.push(format!("{base}/.well-known/openid-configuration"));

        for candidate in candidates {
            if let Ok(metadata) = self.fetch_json::<AuthorizationServerMetadata>(&candidate).await {
                return Ok(metadata);
            }
        }

        bail!("Failed to discover OAuth authorization server metadata")
    }

    async fn register_dynamic_client(
        &self,
        registration_endpoint: &str,
        redirect_uri: &str,
    ) -> Result<DynamicClientRegistrationResponse> {
        let payload = serde_json::json!({
            "client_name": "MCPMate",
            "application_type": "native",
            "grant_types": ["authorization_code", "refresh_token"],
            "response_types": ["code"],
            "redirect_uris": [redirect_uri],
            "token_endpoint_auth_method": "none"
        });

        self.http_client
            .post(registration_endpoint)
            .json(&payload)
            .send()
            .await
            .context("Failed to call OAuth dynamic client registration endpoint")?
            .error_for_status()
            .context("OAuth dynamic client registration returned error status")?
            .json::<DynamicClientRegistrationResponse>()
            .await
            .context("Failed to parse OAuth dynamic client registration response")
    }

    async fn fetch_json<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T> {
        self.http_client
            .get(url)
            .send()
            .await
            .with_context(|| format!("Failed to fetch OAuth metadata from '{url}'"))?
            .error_for_status()
            .with_context(|| format!("OAuth metadata endpoint '{url}' returned error status"))?
            .json::<T>()
            .await
            .with_context(|| format!("Failed to parse OAuth metadata from '{url}'"))
    }
}

fn resource_origin(resource_url: &Url) -> String {
    let mut origin = resource_url.clone();
    origin.set_path("");
    origin.set_query(None);
    origin.set_fragment(None);
    origin.to_string().trim_end_matches('/').to_string()
}

fn issuer_from_endpoint(endpoint: &str) -> String {
    Url::parse(endpoint)
        .ok()
        .map(|url| resource_origin(&url))
        .unwrap_or_else(|| endpoint.trim_end_matches('/').to_string())
}

fn oauth_resource_from_server(server_model: &crate::config::models::Server) -> Result<String> {
    let server_url = server_model
        .url
        .as_deref()
        .ok_or_else(|| anyhow!("Server is missing a URL"))?;
    let mut resource = Url::parse(server_url)
        .with_context(|| format!("Invalid server URL '{}'", server_url))?;
    resource.set_query(None);
    resource.set_fragment(None);
    Ok(resource.to_string())
}

fn protected_resource_candidates(resource_url: &Url) -> Result<Vec<String>> {
    let origin = resource_origin(resource_url);
    let mut candidates = vec![format!("{origin}/.well-known/oauth-protected-resource")];
    let path = resource_url.path().trim_start_matches('/');
    if !path.is_empty() {
        candidates.push(format!("{origin}/.well-known/oauth-protected-resource/{path}"));
    }
    Ok(candidates)
}

fn parse_www_authenticate(header_value: &str) -> HashMap<String, String> {
    let trimmed = header_value.trim();
    let payload = trimmed
        .strip_prefix("Bearer ")
        .or_else(|| trimmed.strip_prefix("bearer "))
        .unwrap_or(trimmed);

    payload
        .split(',')
        .filter_map(|pair| {
            let (key, value) = pair.split_once('=')?;
            let parsed = value.trim().trim_matches('"').to_string();
            Some((key.trim().to_string(), parsed))
        })
        .collect()
}

fn generate_oauth_random(size: usize) -> String {
    nanoid::nanoid!(size, &nanoid::alphabet::SAFE)
}

fn is_token_expired(token: &ServerOAuthToken) -> bool {
    token
        .expires_at
        .as_ref()
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .map(|expires_at| expires_at.with_timezone(&Utc) <= Utc::now())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        common::{server::ServerType, status::EnabledStatus},
        config::{models::Server, server::{crud::upsert_server, init::initialize_server_tables}},
    };
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{body_string_contains, method, path},
    };

    async fn setup_manager() -> OAuthManager {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("connect in-memory sqlite");
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .expect("enable foreign keys");
        initialize_server_tables(&pool).await.expect("init tables");
        OAuthManager::new(pool)
    }

    async fn insert_server(pool: &SqlitePool, id: &str) {
        let server = Server {
            id: Some(id.to_string()),
            name: format!("server-{id}"),
            server_type: ServerType::StreamableHttp,
            command: None,
            url: Some("https://example.com/mcp".to_string()),
            registry_server_id: None,
            capabilities: None,
            enabled: EnabledStatus::Enabled,
            pending_import: false,
            created_at: None,
            updated_at: None,
        };
        upsert_server(pool, &server).await.expect("insert server");
    }

    #[tokio::test]
    async fn initiate_returns_valid_authorization_url() {
        let manager = setup_manager().await;
        insert_server(&manager.pool, "serv_initiate").await;
        manager
            .upsert_config(
                "serv_initiate",
                OAuthConfigInput {
                    authorization_endpoint: "https://issuer.example.com/authorize".to_string(),
                    token_endpoint: "https://issuer.example.com/token".to_string(),
                    client_id: "client-1".to_string(),
                    client_secret: None,
                    scopes: Some("read write".to_string()),
                    redirect_uri: "http://localhost:5173/oauth/callback".to_string(),
                },
            )
            .await
            .expect("save oauth config");

        let initiate = manager.initiate("serv_initiate").await.expect("initiate oauth");
        assert!(initiate.authorization_url.contains("response_type=code"));
        assert!(initiate.authorization_url.contains("code_challenge="));
        assert!(initiate.authorization_url.contains("code_challenge_method=S256"));
        assert!(initiate.authorization_url.contains("client_id=client-1"));
        assert!(initiate.authorization_url.contains("resource=https%3A%2F%2Fexample.com%2Fmcp"));
    }

    #[tokio::test]
    async fn exchange_code_stores_tokens() {
        let manager = setup_manager().await;
        insert_server(&manager.pool, "serv_exchange").await;
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/token"))
            .and(body_string_contains("resource=https%3A%2F%2Fexample.com%2Fmcp"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "access_token": "access-123",
                    "refresh_token": "refresh-123",
                    "token_type": "bearer",
                    "expires_in": 3600,
                    "scope": "read write"
                })),
            )
            .mount(&mock_server)
            .await;

        manager
            .upsert_config(
                "serv_exchange",
                OAuthConfigInput {
                    authorization_endpoint: format!("{}/authorize", mock_server.uri()),
                    token_endpoint: format!("{}/token", mock_server.uri()),
                    client_id: "client-1".to_string(),
                    client_secret: None,
                    scopes: Some("read write".to_string()),
                    redirect_uri: "http://localhost:5173/oauth/callback".to_string(),
                },
            )
            .await
            .expect("save oauth config");

        let initiate = manager.initiate("serv_exchange").await.expect("initiate oauth");
        let status = manager
            .exchange_code(&initiate.state, "code-123")
            .await
            .expect("exchange code");

        assert!(matches!(status.state, OAuthConnectionState::Connected));
        let stored = server::get_server_oauth_token(&manager.pool, "serv_exchange")
            .await
            .expect("load stored token")
            .expect("stored token exists");
        assert_eq!(stored.access_token, "access-123");
    }

    #[tokio::test]
    async fn exchange_code_rejects_invalid_state() {
        let manager = setup_manager().await;
        let error = manager.exchange_code("missing-state", "code").await.expect_err("invalid state should fail");
        assert!(error.to_string().contains("Invalid or expired OAuth state"));
    }
}
