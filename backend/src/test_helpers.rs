use mcpmate_secrets::store::SecretOriginInput;

/// Build a `SecretOriginInput` for an OAuth-managed secret slot.
pub fn oauth_secret_origin(
    server_id: &str,
    server_name: &str,
    field_key: &str,
) -> SecretOriginInput {
    SecretOriginInput {
        server_id: Some(server_id.to_string()),
        server_name: Some(server_name.to_string()),
        server_kind: Some("streamable_http".to_string()),
        source: Some("oauth".to_string()),
        field_group: Some("oauth".to_string()),
        field_key: Some(field_key.to_string()),
        field_index: None,
        field_path: Some(format!("oauth.{field_key}")),
    }
}
