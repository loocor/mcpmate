use std::collections::HashMap;

use mcpmate::{
    common::server::ServerType,
    core::{models::MCPServerConfig, secrets::resolve_runtime_server_config},
};
use mcpmate_secrets::testing::InMemorySecretResolver;
use mcpmate_secrets::{SecretError, UnavailableSecretResolver};

#[test]
fn resolves_stdio_runtime_args_and_env_placeholders() {
    let resolver = InMemorySecretResolver::from_pairs([
        ("github_pat", "ghp_runtime_token"),
        ("workspace_token", "workspace-secret"),
    ]);
    let config = MCPServerConfig {
        kind: ServerType::Stdio,
        command: Some("node".to_string()),
        args: Some(vec![
            "server.js".to_string(),
            "--token".to_string(),
            "[[secret:github_pat]]".to_string(),
            "--workspace=[[secret:workspace_token]]".to_string(),
        ]),
        url: None,
        env: Some(HashMap::from([
            ("GITHUB_TOKEN".to_string(), "[[secret:github_pat]]".to_string()),
            (
                "WORKSPACE_AUTH".to_string(),
                "Bearer [[secret:workspace_token]]".to_string(),
            ),
        ])),
        headers: None,
    };

    let resolved = resolve_runtime_server_config(&config, &resolver).expect("runtime config resolves");

    assert_eq!(
        resolved.args,
        Some(vec![
            "server.js".to_string(),
            "--token".to_string(),
            "ghp_runtime_token".to_string(),
            "--workspace=workspace-secret".to_string(),
        ])
    );
    let env = resolved.env.expect("resolved env");
    assert_eq!(env.get("GITHUB_TOKEN").map(String::as_str), Some("ghp_runtime_token"));
    assert_eq!(
        env.get("WORKSPACE_AUTH").map(String::as_str),
        Some("Bearer workspace-secret")
    );
}

#[test]
fn resolves_streamable_http_runtime_url_and_headers() {
    let resolver =
        InMemorySecretResolver::from_pairs([("http_token", "http-runtime-token"), ("api_key", "runtime-api-key")]);
    let config = MCPServerConfig {
        kind: ServerType::StreamableHttp,
        command: None,
        args: None,
        url: Some("https://mcp.example.test/mcp?token=[[secret:http_token]]".to_string()),
        env: None,
        headers: Some(HashMap::from([
            ("Authorization".to_string(), "Bearer [[secret:http_token]]".to_string()),
            ("X-Api-Key".to_string(), "[[secret:api_key]]".to_string()),
        ])),
    };

    let resolved = resolve_runtime_server_config(&config, &resolver).expect("runtime config resolves");

    assert_eq!(
        resolved.url.as_deref(),
        Some("https://mcp.example.test/mcp?token=http-runtime-token")
    );
    let headers = resolved.headers.expect("resolved headers");
    assert_eq!(
        headers.get("Authorization").map(String::as_str),
        Some("Bearer http-runtime-token")
    );
    assert_eq!(headers.get("X-Api-Key").map(String::as_str), Some("runtime-api-key"));
}

#[test]
fn unavailable_resolver_allows_plain_config_and_rejects_secret_placeholders() {
    let resolver = UnavailableSecretResolver;
    let plain = MCPServerConfig {
        kind: ServerType::StreamableHttp,
        command: None,
        args: None,
        url: Some("https://mcp.example.test/mcp".to_string()),
        env: None,
        headers: Some(HashMap::from([("X-Mode".to_string(), "plain".to_string())])),
    };

    let resolved_plain = resolve_runtime_server_config(&plain, &resolver).expect("plain config resolves");

    assert_eq!(resolved_plain.url.as_deref(), Some("https://mcp.example.test/mcp"));

    let secret_config = MCPServerConfig {
        kind: ServerType::StreamableHttp,
        command: None,
        args: None,
        url: Some("https://mcp.example.test/mcp?token=[[secret:http_token]]".to_string()),
        env: None,
        headers: None,
    };

    let err =
        resolve_runtime_server_config(&secret_config, &resolver).expect_err("placeholder requires a secret provider");

    assert_eq!(err, SecretError::ProviderUnavailable);
}
