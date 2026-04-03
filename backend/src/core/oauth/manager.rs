use anyhow::{Context, Result, anyhow, bail};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{Duration, Utc};
use reqwest::Url;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

use crate::{
    common::server::ServerType,
    config::{models::{ServerOAuthConfig, ServerOAuthToken}, server},
};

use super::types::{OAuthConfigInput, OAuthConnectionState, OAuthInitiateResult, OAuthStatus};

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

    pub async fn initiate(&self, server_id: &str) -> Result<OAuthInitiateResult> {
        let config = server::get_server_oauth_config(&self.pool, server_id)
            .await?
            .ok_or_else(|| anyhow!("OAuth is not configured for server '{}'", server_id))?;

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

        let mut form = vec![
            ("grant_type", "authorization_code".to_string()),
            ("code", code.to_string()),
            ("redirect_uri", config.redirect_uri.clone()),
            ("client_id", config.client_id.clone()),
            ("code_verifier", pending.code_verifier.clone()),
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

        let token_response = self
            .http_client
            .post(&config.token_endpoint)
            .header(reqwest::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(encoded_body)
            .send()
            .await
            .context("Failed to call OAuth token endpoint")?
            .error_for_status()
            .context("OAuth token endpoint returned error status")?
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
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::{method, path}};

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
    }

    #[tokio::test]
    async fn exchange_code_stores_tokens() {
        let manager = setup_manager().await;
        insert_server(&manager.pool, "serv_exchange").await;
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/token"))
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
