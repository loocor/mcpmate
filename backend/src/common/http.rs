use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use std::collections::HashMap;

/// Strip "Bearer " prefix (case-insensitive) from a string, returning the stripped content or None.
fn strip_bearer_case_insensitive(s: &str) -> Option<&str> {
    let trimmed = s.trim();
    trimmed
        .strip_prefix("Bearer ")
        .or_else(|| trimmed.strip_prefix("bearer "))
}

/// Extract bearer token value from `headers` map.
/// Expects `Authorization: Bearer <token>`; returns `<token>` without the prefix.
pub fn extract_bearer_token(headers: &Option<HashMap<String, String>>) -> Option<String> {
    let hdrs = headers.as_ref()?;
    // Case-insensitive key lookup for Authorization
    let mut auth_val: Option<&str> = None;
    for (k, v) in hdrs.iter() {
        if k.eq_ignore_ascii_case("authorization") {
            auth_val = Some(v.as_str());
            break;
        }
    }
    let val = auth_val?;
    strip_bearer_case_insensitive(val).map(|rest| rest.trim().to_string())
}

/// Remove a leading "Bearer " (case-insensitive) prefix from a token string.
pub fn trim_bearer_prefix<S: AsRef<str>>(s: S) -> String {
    let val = s.as_ref();
    strip_bearer_case_insensitive(val)
        .map(|rest| rest.trim().to_string())
        .unwrap_or_else(|| val.trim().to_string())
}

/// Build a Streamable HTTP client config using URL and optional headers.
/// If an Authorization bearer token is present, sets `auth_header` accordingly.
pub fn make_streamable_config(
    url: &str,
    headers: &Option<HashMap<String, String>>,
) -> StreamableHttpClientTransportConfig {
    let mut cfg = StreamableHttpClientTransportConfig::with_uri(url).reinit_on_expired_session(true);
    if let Some(token) = extract_bearer_token(headers) {
        cfg = cfg.auth_header(token);
    }
    cfg
}

/// Build a Streamable HTTP client config using URL and optional bearer token (with or without prefix).
pub fn make_streamable_config_with_bearer(
    url: &str,
    bearer: Option<&str>,
) -> StreamableHttpClientTransportConfig {
    let mut cfg = StreamableHttpClientTransportConfig::with_uri(url).reinit_on_expired_session(true);
    if let Some(token) = bearer {
        let trimmed = trim_bearer_prefix(token);
        if !trimmed.is_empty() {
            cfg = cfg.auth_header(trimmed);
        }
    }
    cfg
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_trim_bearer_prefix() {
        assert_eq!(trim_bearer_prefix("Bearer ABC"), "ABC");
        assert_eq!(trim_bearer_prefix("bearer XYZ "), "XYZ");
        assert_eq!(trim_bearer_prefix("NOPREFIX"), "NOPREFIX");
        assert_eq!(trim_bearer_prefix("  Bearer  TKN-123  "), "TKN-123");
    }

    #[test]
    fn test_extract_bearer_token() {
        let mut h = HashMap::new();
        h.insert("Authorization".to_string(), "Bearer tok-1".to_string());
        assert_eq!(extract_bearer_token(&Some(h)), Some("tok-1".to_string()));

        let mut h2 = HashMap::new();
        h2.insert("authorization".to_string(), "bearer tok-2".to_string());
        assert_eq!(extract_bearer_token(&Some(h2)), Some("tok-2".to_string()));

        let mut h3 = HashMap::new();
        h3.insert("AUTHORIZATION".to_string(), "Token something".to_string());
        assert_eq!(extract_bearer_token(&Some(h3)), None);
    }

    #[test]
    fn test_make_streamable_config_sets_auth() {
        let mut h = HashMap::new();
        h.insert("Authorization".to_string(), "Bearer tok-3".to_string());
        let cfg = make_streamable_config("http://x/mcp", &Some(h));
        assert_eq!(cfg.uri.as_ref(), "http://x/mcp");
        assert_eq!(cfg.auth_header.as_deref(), Some("tok-3"));
        assert!(cfg.reinit_on_expired_session);

        let cfg2 = make_streamable_config_with_bearer("http://y/mcp", Some("Bearer tok-4"));
        assert_eq!(cfg2.uri.as_ref(), "http://y/mcp");
        assert_eq!(cfg2.auth_header.as_deref(), Some("tok-4"));
        assert!(cfg2.reinit_on_expired_session);
    }
}
