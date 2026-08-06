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
    let (pool, _, _, _) = published_tool_surface().await;
    let store = SqliteSurfaceStore::new(pool.clone());
    let empty = SurfaceManifest::compile("consumer-a", vec![]).unwrap();
    let mut transaction = pool.begin().await.unwrap();
    store
        .insert_manifest_in_transaction(&mut transaction, &empty)
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
                Some("publication-stale".to_string()),
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

async fn published_tool_surface() -> (
    sqlx::SqlitePool,
    SqliteCapabilityCatalog,
    CatalogRecord,
    InitializeResult,
) {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    database_support::prepare_config(&pool).await;
    let catalog = SqliteCapabilityCatalog::new(pool.clone());
    catalog.ensure_schema().await.unwrap();
    let initialize: InitializeResult = serde_json::from_value(json!({
        "protocolVersion": "2025-11-25",
        "capabilities": {"tools": {"listChanged": true}},
        "serverInfo": {"name": "fixture", "version": "1.0.0"}
    }))
    .unwrap();
    let first_tool: Tool = serde_json::from_value(json!({
        "name": "analyze",
        "description": "version one",
        "inputSchema": {"type": "object"}
    }))
    .unwrap();
    let first = CatalogRecord::materialize(
        "server-a",
        "analyze",
        "fixture__analyze",
        CapabilityPayload::Tool(first_tool),
    )
    .unwrap();
    catalog
        .commit_observation(CapabilityObservation::new(
            "server-a",
            "fixture",
            "config-v1",
            initialize.clone(),
            vec![KindObservation::new(
                CapabilityKind::Tools,
                DeclarationState::Supported,
                InventoryState::Complete,
            )],
            vec![first.clone()],
        ))
        .await
        .unwrap();

    let manifest = SurfaceManifest::compile(
        "consumer-a",
        vec![SurfaceManifestEntryInput::new(
            first.ref_id.clone(),
            first.capability_id.clone(),
            first.kind(),
            first.external_key.clone(),
        )],
    )
    .unwrap();
    let store = SqliteSurfaceStore::new(pool.clone());
    let mut transaction = pool.begin().await.unwrap();
    store
        .insert_manifest_in_transaction(&mut transaction, &manifest)
        .await
        .unwrap();
    store
        .publish_and_bind_in_transaction(
            &mut transaction,
            &SurfacePublication::new(
                "publication-stale",
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
    (pool, catalog, first, initialize)
}

#[tokio::test]
async fn active_surface_keeps_its_pinned_version_after_the_catalog_current_pointer_advances() {
    let (pool, catalog, first, initialize) = published_tool_surface().await;

    let second_tool: Tool = serde_json::from_value(json!({
        "name": "analyze",
        "description": "version two",
        "inputSchema": {"type": "object"}
    }))
    .unwrap();
    let second = CatalogRecord::materialize(
        "server-a",
        "analyze",
        "fixture__analyze",
        CapabilityPayload::Tool(second_tool),
    )
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
            vec![second],
        ))
        .await
        .unwrap();

    let surface = SurfaceReader::new(pool)
        .load("consumer-a")
        .await
        .expect("the committed publication remains active until it is replaced");
    assert_eq!(surface.entries.len(), 1);
    assert_eq!(surface.entries[0].capability_id, first.capability_id);
    assert_eq!(surface.tools()[0].description.as_deref(), Some("version one"));
}

#[tokio::test]
async fn active_surface_rejects_manifest_content_and_entry_row_divergence() {
    let (pool, _, _, _) = published_tool_surface().await;
    sqlx::query("DELETE FROM surface_manifest_entries")
        .execute(&pool)
        .await
        .unwrap();

    let error = SurfaceReader::new(pool)
        .load("consumer-a")
        .await
        .expect_err("manifest entry divergence must fail closed");
    assert!(error.to_string().contains("surf_sha256:"));
}

#[tokio::test]
async fn resource_reads_resolve_only_static_or_template_routes_pinned_by_the_active_publication() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    database_support::prepare_config(&pool).await;
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
#[path = "support/database.rs"]
mod database_support;
