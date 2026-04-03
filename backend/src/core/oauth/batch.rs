use sqlx::SqlitePool;
use std::collections::HashMap;
use crate::core::oauth::types::OAuthConnectionState;
use crate::common::errors::Result;

pub async fn load_oauth_states(pool: &SqlitePool, server_ids: &[String]) -> Result<HashMap<String, OAuthConnectionState>> {
    if server_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let query = format!(
        "SELECT c.server_id, c.authorization_endpoint, t.access_token, t.expires_at 
         FROM server_oauth_config c 
         LEFT JOIN server_oauth_token t ON c.server_id = t.server_id 
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
        use sqlx::Row;
        let server_id: String = row.try_get("server_id").unwrap_or_default();
        let access_token: Option<String> = row.try_get("access_token").unwrap_or_default();
        let expires_at: Option<String> = row.try_get("expires_at").unwrap_or_default();

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

        map.insert(server_id, state);
    }

    Ok(map)
}
