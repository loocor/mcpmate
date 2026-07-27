use mcpmate::core::capability::surface_read::SurfaceReader;
use mcpmate_capability_store::{
    CapabilityCatalog, CapabilityKind, CapabilityObservation, CapabilityPayload, CatalogRecord, DeclarationState,
    InventoryState, KindObservation, SqliteCapabilityCatalog, SqliteSurfaceStore, SurfaceManifest,
    SurfaceManifestEntryInput, SurfacePublication,
};
use rmcp::model::{InitializeResult, Resource, ResourceTemplate, Tool};
use serde_json::json;
use sqlx::sqlite::SqlitePoolOptions;

#[tokio::test]
async fn list_and_call_authorization_read_the_same_active_publication_on_every_request() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    let catalog = SqliteCapabilityCatalog::new(pool.clone());
    catalog.ensure_schema().await.unwrap();
    let tool: Tool = serde_json::from_value(json!({
        "name": "analyze",
        "description": "fixture",
        "inputSchema": {"type": "object"}
    }))
    .unwrap();
    let record =
        CatalogRecord::materialize("server-a", "analyze", "fixture__analyze", CapabilityPayload::Tool(tool)).unwrap();
    let initialize: InitializeResult = serde_json::from_value(json!({
        "protocolVersion": "2025-11-25",
        "capabilities": {"tools": {"listChanged": true}},
        "serverInfo": {"name": "fixture", "version": "1.0.0"}
    }))
    .unwrap();
    catalog
        .commit_observation(CapabilityObservation::new(
            "server-a",
            "fixture",
            "config-v1",
            initialize,
            vec![KindObservation::new(
                CapabilityKind::Tools,
                DeclarationState::Supported,
                InventoryState::Complete,
            )],
            vec![record.clone()],
        ))
        .await
        .unwrap();
    let store = SqliteSurfaceStore::new(pool.clone());
    let manifest = SurfaceManifest::compile(
        "consumer-a",
        vec![SurfaceManifestEntryInput::new(
            record.ref_id.clone(),
            record.capability_id.clone(),
            record.kind(),
            record.external_key,
        )],
    )
    .unwrap();
    let empty = SurfaceManifest::compile("consumer-a", vec![]).unwrap();
    let mut transaction = pool.begin().await.unwrap();
    store
        .insert_manifest_in_transaction(&mut transaction, &manifest)
        .await
        .unwrap();
    store
        .insert_manifest_in_transaction(&mut transaction, &empty)
        .await
        .unwrap();
    store
        .publish_and_bind_in_transaction(
            &mut transaction,
            &SurfacePublication::new(
                "publication-1",
                "consumer-a",
                manifest.manifest_id,
                None,
                "initial",
                "system",
                None,
            ),
            None,
        )
        .await
        .unwrap();
    transaction.commit().await.unwrap();

    let reader = SurfaceReader::new(pool.clone());
    let first = reader.load("consumer-a").await.unwrap();
    assert_eq!(first.tools().len(), 1);
    let route = reader
        .require(CapabilityKind::Tools, "consumer-a", "fixture__analyze")
        .await
        .unwrap();
    assert_eq!(route.source_server_id, "server-a");
    assert_eq!(route.upstream_key, "analyze");

    let mut transaction = pool.begin().await.unwrap();
    store
        .publish_and_bind_in_transaction(
            &mut transaction,
            &SurfacePublication::new(
                "publication-2",
                "consumer-a",
                empty.manifest_id,
                None,
                "safe_contraction",
                "system",
                Some("publication-1".to_string()),
            ),
            Some(1),
        )
        .await
        .unwrap();
    transaction.commit().await.unwrap();

    assert!(reader.load("consumer-a").await.unwrap().tools().is_empty());
    assert!(
        reader
            .require(CapabilityKind::Tools, "consumer-a", "fixture__analyze")
            .await
            .is_err()
    );
    assert!(reader.load("unknown-consumer").await.is_err());
}

#[tokio::test]
async fn resource_reads_resolve_only_static_or_template_routes_pinned_by_the_active_publication() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    let catalog = SqliteCapabilityCatalog::new(pool.clone());
    catalog.ensure_schema().await.unwrap();
    let resource: Resource = serde_json::from_value(json!({
        "uri": "docs://static/guide",
        "name": "Guide"
    }))
    .unwrap();
    let template: ResourceTemplate = serde_json::from_value(json!({
        "uriTemplate": "docs://files/{path}",
        "name": "Files"
    }))
    .unwrap();
    let resource_record = CatalogRecord::materialize(
        "server-a",
        "docs://static/guide",
        "mcpmate://resources/docs/static/guide",
        CapabilityPayload::Resource(resource),
    )
    .unwrap();
    let template_record = CatalogRecord::materialize(
        "server-a",
        "docs://files/{path}",
        "mcpmate://resources/template/docs/files/{path}",
        CapabilityPayload::ResourceTemplate(template),
    )
    .unwrap();
    let initialize: InitializeResult = serde_json::from_value(json!({
        "protocolVersion": "2025-11-25",
        "capabilities": {"resources": {"listChanged": true}},
        "serverInfo": {"name": "fixture", "version": "1.0.0"}
    }))
    .unwrap();
    catalog
        .commit_observation(CapabilityObservation::new(
            "server-a",
            "fixture",
            "config-v1",
            initialize,
            vec![
                KindObservation::new(
                    CapabilityKind::Resources,
                    DeclarationState::Supported,
                    InventoryState::Complete,
                ),
                KindObservation::new(
                    CapabilityKind::ResourceTemplates,
                    DeclarationState::Supported,
                    InventoryState::Complete,
                ),
            ],
            vec![resource_record.clone(), template_record.clone()],
        ))
        .await
        .unwrap();
    let store = SqliteSurfaceStore::new(pool.clone());
    let manifest = SurfaceManifest::compile(
        "consumer-a",
        vec![
            SurfaceManifestEntryInput::new(
                resource_record.ref_id,
                resource_record.capability_id,
                CapabilityKind::Resources,
                resource_record.external_key,
            ),
            SurfaceManifestEntryInput::new(
                template_record.ref_id,
                template_record.capability_id,
                CapabilityKind::ResourceTemplates,
                template_record.external_key,
            ),
        ],
    )
    .unwrap();
    let mut transaction = pool.begin().await.unwrap();
    store
        .insert_manifest_in_transaction(&mut transaction, &manifest)
        .await
        .unwrap();
    store
        .publish_and_bind_in_transaction(
            &mut transaction,
            &SurfacePublication::new(
                "publication-1",
                "consumer-a",
                manifest.manifest_id,
                None,
                "initial",
                "system",
                None,
            ),
            None,
        )
        .await
        .unwrap();
    transaction.commit().await.unwrap();

    let reader = SurfaceReader::new(pool);
    let static_route = reader
        .resolve_resource_route("consumer-a", "mcpmate://resources/docs/static/guide")
        .await
        .unwrap();
    assert_eq!(static_route.source_server_id, "server-a");
    assert_eq!(static_route.upstream_uri, "docs://static/guide");
    assert!(static_route.template_arguments.is_none());

    let template_route = reader
        .resolve_resource_route(
            "consumer-a",
            "mcpmate://resources/template/docs/files/architecture%2Foverview.md",
        )
        .await
        .unwrap();
    assert_eq!(template_route.upstream_uri, "docs://files/architecture%2Foverview.md");
    assert_eq!(
        template_route.template_arguments.unwrap().get("path"),
        Some(&"architecture/overview.md".to_string())
    );
    assert!(
        reader
            .resolve_resource_route("consumer-a", "mcpmate://resources/template/other/files/architecture.md",)
            .await
            .is_err()
    );
}
