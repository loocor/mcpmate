use url::Url;

/// Sanitize URL for logging by removing credentials, hostnames, ports, paths,
/// and query parameters.
///
/// This prevents sensitive information from being written to log files while
/// still preserving the scheme for basic diagnostics.
///
/// # Examples
///
/// ```
/// let sanitized = sanitize_url_for_logging("https://user:token@api.example.com:8080/v1?secret=xxx");
/// assert_eq!(sanitized, "https://[redacted]");
/// ```
pub fn sanitize_url_for_logging(url: &str) -> String {
    match Url::parse(url) {
        Ok(parsed) => format!("{}://[redacted]", parsed.scheme()),
        Err(_) => "[invalid-url]".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_url_with_credentials() {
        let url = "https://user:password@api.example.com:8080/v1?token=secret";
        let sanitized = sanitize_url_for_logging(url);
        assert_eq!(sanitized, "https://[redacted]");
    }

    #[test]
    fn test_sanitize_url_without_credentials() {
        let url = "https://api.example.com:8080/v1";
        let sanitized = sanitize_url_for_logging(url);
        assert_eq!(sanitized, "https://[redacted]");
    }

    #[test]
    fn test_sanitize_url_with_query_params() {
        let url = "https://api.example.com/v1?token=secret&key=123";
        let sanitized = sanitize_url_for_logging(url);
        assert_eq!(sanitized, "https://[redacted]");
    }

    #[test]
    fn test_sanitize_internal_hostname() {
        let url = "http://corp-api.internal:8443/private";
        let sanitized = sanitize_url_for_logging(url);
        assert_eq!(sanitized, "http://[redacted]");
    }

    #[test]
    fn test_sanitize_invalid_url() {
        let url = "not-a-url";
        let sanitized = sanitize_url_for_logging(url);
        assert_eq!(sanitized, "[invalid-url]");
    }
}
