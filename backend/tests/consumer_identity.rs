use axum::http::{Request, header::HeaderValue};
use mcpmate::core::proxy::server::{
    ClientIdentitySource, ManagedEndpointTrust, VerifiedConsumerCredential, resolve_initialize_context_parts,
};
use rmcp::model::{ClientCapabilities, Implementation, InitializeRequestParams, ProtocolVersion};

fn initialize(name: &str) -> InitializeRequestParams {
    InitializeRequestParams::new(ClientCapabilities::default(), Implementation::new(name, "1.0.0"))
        .with_protocol_version(ProtocolVersion::LATEST)
}

#[test]
fn local_endpoint_trusts_legacy_sideband_but_never_client_info() {
    let mut request = Request::builder().uri("/mcp?client_id=consumer-a").body(()).unwrap();
    request.extensions_mut().insert(ManagedEndpointTrust::LocalOnly);
    let parts = request.into_parts().0;
    let context = resolve_initialize_context_parts(Some(&parts), &initialize("impersonated-name")).unwrap();
    assert_eq!(context.client_id, "consumer-a");
    assert_eq!(context.source, ClientIdentitySource::ManagedQuery);
}

#[test]
fn remote_endpoint_requires_verified_credential_and_rejects_conflicting_declaration() {
    let mut request = Request::builder().uri("/mcp?client_id=consumer-b").body(()).unwrap();
    request
        .extensions_mut()
        .insert(ManagedEndpointTrust::VerifiedCredentialRequired);
    request.extensions_mut().insert(VerifiedConsumerCredential {
        consumer_id: "consumer-a".to_string(),
    });
    request
        .headers_mut()
        .insert("x-mcpmate-client-id", HeaderValue::from_static("consumer-b"));
    let parts = request.into_parts().0;
    assert!(resolve_initialize_context_parts(Some(&parts), &initialize("consumer-a")).is_err());

    let mut no_credential = Request::builder().uri("/mcp?client_id=consumer-a").body(()).unwrap();
    no_credential
        .extensions_mut()
        .insert(ManagedEndpointTrust::VerifiedCredentialRequired);
    assert!(resolve_initialize_context_parts(Some(&no_credential.into_parts().0), &initialize("consumer-a")).is_err());
}
