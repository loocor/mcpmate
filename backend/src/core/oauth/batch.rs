use crate::core::oauth::types::{OAuthConnectionState, OAuthCustodyState, OAuthStatusIssue};
use anyhow::Result;
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct OAuthServerSummary {
    pub state: OAuthConnectionState,
    pub custody_state: OAuthCustodyState,
    pub requires_reconnect: bool,
    pub issue: Option<OAuthStatusIssue>,
}

pub async fn load_oauth_states(
    pool: &SqlitePool,
    server_ids: &[String],
    secure_store_available: bool,
) -> Result<HashMap<String, OAuthServerSummary>> {
    if server_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let query = format!(
        "SELECT c.server_id, c.client_secret, t.access_token, t.refresh_token, t.expires_at
         FROM server_oauth_config c 
         LEFT JOIN server_oauth_tokens t ON c.server_id = t.server_id 
         WHERE c.server_id IN ({})",
        server_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ")
    );

    let mut q = sqlx::query(&query);
    for id in server_ids {
        q = q.bind(id);
    }

    let rows = q.fetch_all(pool).await?;
    let mut map = HashMap::new();

    for row in rows {
        let server_id: String = row.try_get("server_id")?;
        let client_secret: Option<String> = row.try_get("client_secret")?;
        let access_token: Option<String> = row.try_get("access_token")?;
        let refresh_token: Option<String> = row.try_get("refresh_token")?;
        let expires_at: Option<String> = row.try_get("expires_at")?;

        let state = if access_token.is_none() {
            OAuthConnectionState::Disconnected
        } else {
            let is_expired = expires_at
                .and_then(|value| chrono::DateTime::parse_from_rfc3339(&value).ok())
                .map(|expires| expires.with_timezone(&chrono::Utc) <= chrono::Utc::now())
                .unwrap_or(false);

            if is_expired {
                OAuthConnectionState::Expired
            } else {
                OAuthConnectionState::Connected
            }
        };
        let (custody_state, requires_reconnect, issue) = match oauth_summary_custody(
            secure_store_available,
            client_secret.as_deref(),
            access_token.as_deref(),
            refresh_token.as_deref(),
        ) {
            Ok(result) => result,
            Err(error) => {
                tracing::warn!(
                    server_id = %server_id,
                    error = %error,
                    "Custody classification failed for server; defaulting to LegacyPlaintext"
                );
                (
                    OAuthCustodyState::LegacyPlaintext,
                    true,
                    Some(OAuthStatusIssue {
                        code: "custody_classification_error".to_string(),
                        message: format!("Custody check failed: {error}"),
                    }),
                )
            }
        };

        map.insert(
            server_id,
            OAuthServerSummary {
                state,
                custody_state,
                requires_reconnect,
                issue,
            },
        );
    }

    Ok(map)
}

fn oauth_summary_custody(
    secure_store_available: bool,
    client_secret: Option<&str>,
    access_token: Option<&str>,
    refresh_token: Option<&str>,
) -> Result<(OAuthCustodyState, bool, Option<OAuthStatusIssue>)> {
    let values: Vec<&str> = [client_secret, access_token, refresh_token]
        .into_iter()
        .flatten()
        .collect();
    super::types::classify_custody(secure_store_available, &values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custody_marks_plaintext_values_as_legacy_when_store_is_available() {
        let (custody_state, requires_reconnect, issue) =
            oauth_summary_custody(true, None, Some("legacy-access-token"), None).expect("custody summary");

        assert!(matches!(custody_state, OAuthCustodyState::LegacyPlaintext));
        assert!(requires_reconnect);
        assert_eq!(
            issue.as_ref().map(|issue| issue.code.as_str()),
            Some("legacy_plaintext_oauth_credentials")
        );
    }

    #[test]
    fn custody_marks_configured_oauth_unavailable_without_store() {
        let (custody_state, requires_reconnect, issue) =
            oauth_summary_custody(false, None, Some("[[secret:oauth/test/access-token]]"), None)
                .expect("custody summary");

        assert!(matches!(custody_state, OAuthCustodyState::Unavailable));
        assert!(requires_reconnect);
        assert_eq!(
            issue.as_ref().map(|issue| issue.code.as_str()),
            Some("secure_store_unavailable")
        );
    }
}
