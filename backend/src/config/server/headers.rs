// Server HTTP headers persistence helpers
// Provides CRUD utilities for default headers used by HTTP-based transports

use anyhow::Result;
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;

const TABLE: &str = "server_headers";
const REDACTED_FULL: &str = "***REDACTED***";

/// Returns true when a value is an API redaction mask and must not be persisted.
/// IMPORTANT: keep in sync with board/src/lib/secure-field.ts isRedactedMask.
pub fn is_redacted_display_value(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed == REDACTED_FULL {
        return true;
    }

    // Partial mask pattern: 6 ASCII chars + "***" + 2 ASCII chars (e.g. "Bearer***ue").
    // Redaction masks are always ASCII, so byte-length checks are safe and faster
    // than chars().count().
    if let Some(idx) = trimmed.find("***") {
        let head = &trimmed[..idx];
        let tail = &trimmed[idx + 3..];
        if head.is_ascii() && tail.is_ascii() && head.len() == 6 && tail.len() == 2 {
            return true;
        }
    }

    false
}

/// Merge incoming header updates with stored values, preserving secrets for redacted masks.
pub fn merge_headers_for_update(
    incoming: &HashMap<String, String>,
    existing: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut merged = HashMap::new();

    for (raw_key, raw_value) in incoming {
        let key = raw_key.trim().to_ascii_lowercase();
        if key.is_empty() {
            continue;
        }

        if is_redacted_display_value(raw_value) {
            if let Some(existing_value) = existing.get(&key) {
                merged.insert(key, existing_value.clone());
            }
            continue;
        }

        merged.insert(key, raw_value.clone());
    }

    merged
}

/// Merge incoming env updates with stored values, preserving secrets for redacted masks.
/// Env keys are case-sensitive (unlike HTTP headers).
pub fn merge_env_for_update(
    incoming: &HashMap<String, String>,
    existing: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut merged = HashMap::new();

    for (raw_key, raw_value) in incoming {
        let key = raw_key.trim();
        if key.is_empty() {
            continue;
        }

        if is_redacted_display_value(raw_value) {
            if let Some(existing_value) = existing.get(key) {
                merged.insert(key.to_string(), existing_value.clone());
            }
            continue;
        }

        merged.insert(key.to_string(), raw_value.clone());
    }

    merged
}

/// Create or replace all headers for a server (idempotent upsert per key)
pub async fn upsert_server_headers(
    pool: &Pool<Sqlite>,
    server_id: &str,
    headers: &HashMap<String, String>,
) -> Result<()> {
    let mut tx = pool.begin().await?;

    // Upsert each header (normalized key to lowercase)
    for (k, v) in headers.iter() {
        let key = k.trim().to_ascii_lowercase();
        if key.is_empty() {
            continue;
        }
        sqlx::query(&format!(
            r#"
            INSERT INTO {TABLE} (server_id, header_key, header_value)
            VALUES (?, ?, ?)
            ON CONFLICT(server_id, header_key) DO UPDATE SET header_value = excluded.header_value
            "#,
        ))
        .bind(server_id)
        .bind(&key)
        .bind(v)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

/// Replace headers set: remove keys not present in provided map, upsert provided ones
pub async fn replace_server_headers(
    pool: &Pool<Sqlite>,
    server_id: &str,
    headers: &HashMap<String, String>,
) -> Result<()> {
    let mut tx = pool.begin().await?;

    // Fetch existing keys
    let rows: Vec<(String,)> = sqlx::query_as(&format!("SELECT header_key FROM {TABLE} WHERE server_id = ?"))
        .bind(server_id)
        .fetch_all(&mut *tx)
        .await?;
    let existing: std::collections::HashSet<String> = rows.into_iter().map(|(k,)| k).collect();

    // Upsert provided
    for (k, v) in headers.iter() {
        let key = k.trim().to_ascii_lowercase();
        if key.is_empty() {
            continue;
        }
        sqlx::query(&format!(
            r#"INSERT INTO {TABLE} (server_id, header_key, header_value)
                VALUES (?, ?, ?)
                ON CONFLICT(server_id, header_key) DO UPDATE SET header_value = excluded.header_value"#
        ))
        .bind(server_id)
        .bind(&key)
        .bind(v)
        .execute(&mut *tx)
        .await?;
    }

    // Delete removed keys
    for key in existing {
        if !headers.contains_key(&key) && !headers.contains_key(&key.to_ascii_uppercase()) {
            sqlx::query(&format!("DELETE FROM {TABLE} WHERE server_id = ? AND header_key = ?"))
                .bind(server_id)
                .bind(&key)
                .execute(&mut *tx)
                .await?;
        }
    }

    tx.commit().await?;
    Ok(())
}

/// Get all headers for a server as a map (keys normalized to lowercase)
pub async fn get_server_headers(
    pool: &Pool<Sqlite>,
    server_id: &str,
) -> Result<HashMap<String, String>> {
    let rows = sqlx::query_as::<_, (String, String)>(&format!(
        "SELECT header_key, header_value FROM {TABLE} WHERE server_id = ? ORDER BY header_key"
    ))
    .bind(server_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::{
        is_redacted_display_value, merge_env_for_update, merge_headers_for_update,
    };
    use std::collections::HashMap;

    #[test]
    fn detects_redacted_display_values() {
        assert!(is_redacted_display_value("***REDACTED***"));
        assert!(is_redacted_display_value("Bearer***ue"));
        assert!(!is_redacted_display_value("Bearer [[secret:token]]"));
    }

    #[test]
    fn merge_headers_preserves_existing_secrets_for_redacted_masks() {
        let existing = HashMap::from([
            ("authorization".to_string(), "Bearer real-token".to_string()),
            ("x-custom".to_string(), "visible".to_string()),
        ]);
        let incoming = HashMap::from([
            ("authorization".to_string(), "Bearer***ue".to_string()),
            ("x-custom".to_string(), "updated".to_string()),
        ]);

        let merged = merge_headers_for_update(&incoming, &existing);
        assert_eq!(
            merged.get("authorization").map(String::as_str),
            Some("Bearer real-token")
        );
        assert_eq!(merged.get("x-custom").map(String::as_str), Some("updated"));
    }

    #[test]
    fn merge_env_preserves_existing_secrets_for_redacted_masks() {
        let existing = HashMap::from([
            ("API_KEY".to_string(), "real-secret".to_string()),
            ("PUBLIC".to_string(), "visible".to_string()),
        ]);
        let incoming = HashMap::from([
            ("API_KEY".to_string(), "***REDACTED***".to_string()),
            ("PUBLIC".to_string(), "updated".to_string()),
        ]);

        let merged = merge_env_for_update(&incoming, &existing);
        assert_eq!(merged.get("API_KEY").map(String::as_str), Some("real-secret"));
        assert_eq!(merged.get("PUBLIC").map(String::as_str), Some("updated"));
    }

    #[test]
    fn merge_env_preserves_case_sensitive_keys() {
        let existing = HashMap::from([("Path".to_string(), "/usr/bin".to_string())]);
        let incoming = HashMap::from([("Path".to_string(), "updated".to_string())]);

        let merged = merge_env_for_update(&incoming, &existing);
        assert_eq!(merged.get("Path").map(String::as_str), Some("updated"));
        assert!(!merged.contains_key("path"));
    }
}
