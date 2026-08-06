use std::collections::HashMap;

use mcpmate::core::capability::management::{
    ProfileActivationAction, ProfileRelationshipAction, ProfileSurfaceManagement, ServerSurfaceManagement,
};
use mcpmate::core::capability::materializer::{
    bootstrap_managed_surfaces, synchronize_builtin_catalog_and_bootstrap_managed_surfaces,
};
use mcpmate::mcper::UNIFY_BUILTIN_TOOL_NAMES;
use mcpmate_capability_store::{
    CapabilityCatalog, CapabilityKind, CapabilityObservation, CapabilityPayload, CatalogRecord, DeclarationState,
    InventoryState, KindObservation, SqliteCapabilityCatalog, SqliteSurfaceStore,
};
use rmcp::model::{InitializeResult, Tool};
use serde_json::json;
use sqlx::sqlite::SqlitePoolOptions;

async fn init_management_pool() -> sqlx::SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    database_support::prepare_config(&pool).await;
    mcpmate::config::server::init::initialize_server_tables(&pool)
        .await
        .unwrap();
    mcpmate::config::client::init::initialize_client_table(&pool)
        .await
        .unwrap();
    mcpmate::config::profile::init::initialize_profile_tables(&pool)
        .await
        .unwrap();
    SqliteCapabilityCatalog::new(pool.clone())
        .ensure_schema()
        .await
        .unwrap();
    pool
}

fn unify_builtin_catalog_records() -> Vec<CatalogRecord> {
    UNIFY_BUILTIN_TOOL_NAMES
        .iter()
        .copied()
        .map(|name| {
            CatalogRecord::materialize(
                mcpmate_capability_store::BUILTIN_CAPABILITY_SOURCE_ID,
                name,
                name,
                CapabilityPayload::Tool(Tool::new(
                    name,
                    format!("{name} description"),
                    std::sync::Arc::new(json!({"type": "object"}).as_object().unwrap().clone()),
                )),
            )
            .unwrap()
        })
        .collect()
}

fn sorted_unify_builtin_tool_names() -> Vec<String> {
    let mut names = UNIFY_BUILTIN_TOOL_NAMES
        .iter()
        .copied()
        .map(str::to_string)
        .collect::<Vec<_>>();
    names.sort();
    names
}

async fn fixture() -> (sqlx::SqlitePool, CatalogRecord) {
    let pool = init_management_pool().await;
    let catalog = SqliteCapabilityCatalog::new(pool.clone());

    sqlx::query(
        "INSERT INTO server_config (id, name, server_type, command, enabled) VALUES ('server-a', 'Server A', 'stdio', '', 1)",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO profile (id, name, description, type, role, is_active) VALUES ('profile-a', 'Profile A', '', 'shared', 'user', 1)",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        r#"
        INSERT INTO client (
            id, identifier, name, config_mode, approval_status,
            capability_source, selected_profile_ids
        )
        VALUES (
            'consumer-a', 'client-a', 'Client A', 'hosted', 'approved',
            'profiles', '["profile-a"]'
        )
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    let tool: Tool = serde_json::from_value(json!({
        "name": "analyze",
        "description": "Analyze input",
        "inputSchema": {"type": "object"}
    }))
    .unwrap();
    let record = CatalogRecord::materialize(
        "server-a",
        "analyze",
        "server_a__analyze",
        CapabilityPayload::Tool(tool),
    )
    .unwrap();
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
    sqlx::query("INSERT INTO profile_capability_refs (profile_id, ref_id, enabled) VALUES ('profile-a', ?, 1)")
        .bind(record.ref_id.as_str())
        .execute(&pool)
        .await
        .unwrap();
    (pool, record)
}

async fn add_sibling_tool(
    pool: &sqlx::SqlitePool,
    record: &CatalogRecord,
) -> CatalogRecord {
    let sibling_tool: Tool = serde_json::from_value(json!({
        "name": "summarize",
        "description": "Summarize input",
        "inputSchema": {"type": "object"}
    }))
    .unwrap();
    let sibling_record = CatalogRecord::materialize(
        "server-a",
        "summarize",
        "server_a__summarize",
        CapabilityPayload::Tool(sibling_tool),
    )
    .unwrap();
    let initialize: InitializeResult = serde_json::from_value(json!({
        "protocolVersion": "2025-11-25",
        "capabilities": {"tools": {"listChanged": true}},
        "serverInfo": {"name": "fixture", "version": "1.0.0"}
    }))
    .unwrap();
    SqliteCapabilityCatalog::new(pool.clone())
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
            vec![record.clone(), sibling_record.clone()],
        ))
        .await
        .unwrap();
    sibling_record
}

async fn commit_empty_server_observation(
    pool: &sqlx::SqlitePool,
    server_id: &str,
) {
    let initialize: InitializeResult = serde_json::from_value(json!({
        "protocolVersion": "2025-11-25",
        "capabilities": {},
        "serverInfo": {"name": server_id, "version": "1.0.0"}
    }))
    .unwrap();
    SqliteCapabilityCatalog::new(pool.clone())
        .commit_observation(CapabilityObservation::new(
            server_id,
            server_id,
            "config-v1",
            initialize,
            Vec::new(),
            Vec::new(),
        ))
        .await
        .unwrap();
}

#[tokio::test]
async fn hosted_profile_surface_ignores_preserved_unify_direct_exposure_intent() {
    let (pool, profile_record) = fixture().await;
    let direct_record = add_sibling_tool(&pool, &profile_record).await;
    sqlx::query("UPDATE server_config SET unify_direct_exposure_eligible = 1 WHERE id = 'server-a'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO direct_exposure_refs (consumer_id, ref_id, enabled) VALUES ('client-a', ?, 1)")
        .bind(direct_record.ref_id.as_str())
        .execute(&pool)
        .await
        .unwrap();

    bootstrap_managed_surfaces(&pool).await.unwrap();

    let published_ref_ids: Vec<String> = sqlx::query_scalar(
        r#"
        SELECT entry.ref_id
        FROM consumer_surface_bindings binding
        JOIN surface_publications publication
          ON publication.publication_id = binding.active_publication_id
        JOIN surface_manifest_entries entry ON entry.manifest_id = publication.manifest_id
        WHERE binding.consumer_id = 'client-a'
        ORDER BY entry.ref_id
        "#,
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(published_ref_ids, vec![profile_record.ref_id.to_string()]);
}

#[tokio::test]
async fn profile_activation_republishes_activated_consumers_in_the_same_operation() {
    let (pool, _) = fixture().await;
    sqlx::query(
        "UPDATE client SET capability_source = 'activated', selected_profile_ids = '[]' WHERE id = 'consumer-a'",
    )
    .execute(&pool)
    .await
    .unwrap();

    let activated = ProfileSurfaceManagement::set_profiles_active(
        &pool,
        &["profile-a".to_string()],
        ProfileActivationAction::Activate,
        HashMap::from([("profile-a".to_string(), 0)]),
        "test",
    )
    .await
    .unwrap();
    assert_eq!(activated.materializations.len(), 1);

    let store = SqliteSurfaceStore::new(pool.clone());
    let active_binding = store.load_binding("client-a").await.unwrap().unwrap();
    let active_entry_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM surface_publications publication
        JOIN surface_manifest_entries entry ON entry.manifest_id = publication.manifest_id
        WHERE publication.publication_id = ?
        "#,
    )
    .bind(&active_binding.active_publication_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(active_entry_count, 1);

    let deactivated = ProfileSurfaceManagement::set_profiles_active(
        &pool,
        &["profile-a".to_string()],
        ProfileActivationAction::Deactivate,
        HashMap::from([("profile-a".to_string(), 1)]),
        "test",
    )
    .await
    .unwrap();
    assert_eq!(deactivated.materializations.len(), 1);

    let inactive_binding = store.load_binding("client-a").await.unwrap().unwrap();
    let inactive_entry_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM surface_publications publication
        JOIN surface_manifest_entries entry ON entry.manifest_id = publication.manifest_id
        WHERE publication.publication_id = ?
        "#,
    )
    .bind(&inactive_binding.active_publication_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(inactive_entry_count, 0);
    assert!(inactive_binding.generation > active_binding.generation);
}

#[tokio::test]
async fn profile_capability_save_is_atomic_and_requires_the_displayed_revision_set() {
    let (pool, record) = fixture().await;

    let unknown_ref = "mcp-ref-v1:tools:unknown".to_string();
    let partial = ProfileSurfaceManagement::mutate_capabilities(
        &pool,
        "profile-a",
        &[record.ref_id.to_string(), unknown_ref],
        ProfileRelationshipAction::Disable,
        0,
        HashMap::from([("server-a".to_string(), 1)]),
        "test",
    )
    .await;
    assert!(partial.is_err());
    let enabled: bool =
        sqlx::query_scalar("SELECT enabled FROM profile_capability_refs WHERE profile_id = 'profile-a' AND ref_id = ?")
            .bind(record.ref_id.as_str())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(enabled);

    let stale = ProfileSurfaceManagement::mutate_capabilities(
        &pool,
        "profile-a",
        &[record.ref_id.to_string()],
        ProfileRelationshipAction::Disable,
        1,
        HashMap::from([("server-a".to_string(), 1)]),
        "test",
    )
    .await;
    assert!(matches!(
        stale,
        Err(mcpmate_capability_store::CatalogError::ConcurrencyConflict { .. })
    ));

    ProfileSurfaceManagement::mutate_capabilities(
        &pool,
        "profile-a",
        &[record.ref_id.to_string()],
        ProfileRelationshipAction::Disable,
        0,
        HashMap::from([("server-a".to_string(), 1)]),
        "test",
    )
    .await
    .unwrap();
    let binding_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM consumer_surface_bindings WHERE consumer_id = 'client-a'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(binding_count, 1);
}

#[tokio::test]
async fn profile_capability_override_can_disable_and_reenable_a_server_level_ref() {
    let (pool, record) = fixture().await;
    sqlx::query("DELETE FROM profile_capability_refs WHERE profile_id = 'profile-a'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO profile_server_relationships (profile_id, server_id, new_ref_policy) VALUES ('profile-a', 'server-a', 'follow')",
    )
    .execute(&pool)
    .await
    .unwrap();

    ProfileSurfaceManagement::mutate_capabilities(
        &pool,
        "profile-a",
        &[record.ref_id.to_string()],
        ProfileRelationshipAction::Disable,
        0,
        HashMap::from([("server-a".to_string(), 1)]),
        "test",
    )
    .await
    .expect("disable server-level capability through an explicit override");

    let disabled: bool =
        sqlx::query_scalar("SELECT enabled FROM profile_capability_refs WHERE profile_id = 'profile-a' AND ref_id = ?")
            .bind(record.ref_id.as_str())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(!disabled);
    let disabled_relationships = mcpmate::config::profile::capability_ref::load_profile_capability_refs(
        &pool,
        "profile-a",
        Some(CapabilityKind::Tools),
    )
    .await
    .unwrap();
    assert_eq!(disabled_relationships.len(), 1);
    assert!(!disabled_relationships[0].enabled);

    let store = SqliteSurfaceStore::new(pool.clone());
    let disabled_binding = store.load_binding("client-a").await.unwrap().unwrap();
    let disabled_entry_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM surface_publications publication
        JOIN surface_manifest_entries entry ON entry.manifest_id = publication.manifest_id
        WHERE publication.publication_id = ?
        "#,
    )
    .bind(&disabled_binding.active_publication_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(disabled_entry_count, 0);

    ProfileSurfaceManagement::mutate_capabilities(
        &pool,
        "profile-a",
        &[record.ref_id.to_string()],
        ProfileRelationshipAction::Enable,
        1,
        HashMap::from([("server-a".to_string(), 1)]),
        "test",
    )
    .await
    .expect("reenable server-level capability through the explicit override");

    let enabled: bool =
        sqlx::query_scalar("SELECT enabled FROM profile_capability_refs WHERE profile_id = 'profile-a' AND ref_id = ?")
            .bind(record.ref_id.as_str())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(enabled);
    let enabled_binding = store.load_binding("client-a").await.unwrap().unwrap();
    let enabled_entry_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM surface_publications publication
        JOIN surface_manifest_entries entry ON entry.manifest_id = publication.manifest_id
        WHERE publication.publication_id = ?
        "#,
    )
    .bind(&enabled_binding.active_publication_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(enabled_entry_count, 1);
}

#[tokio::test]
async fn generation_aware_profile_operation_associates_snapshot_ready_server_and_supports_toggles() {
    let (pool, record) = fixture().await;
    let association_materializations = ProfileSurfaceManagement::mutate_servers(
        &pool,
        "profile-a",
        &["server-a".to_string()],
        ProfileRelationshipAction::Enable,
        0,
        "test",
    )
    .await
    .unwrap();
    assert_eq!(association_materializations.len(), 1);

    ProfileSurfaceManagement::mutate_servers(
        &pool,
        "profile-a",
        &["server-a".to_string()],
        ProfileRelationshipAction::Disable,
        1,
        "test",
    )
    .await
    .unwrap();

    let disabled_servers = mcpmate::config::profile::get_profile_servers(&pool, "profile-a")
        .await
        .unwrap();
    assert_eq!(disabled_servers.len(), 1);
    assert!(!disabled_servers[0].enabled);
    let disabled_capability: bool =
        sqlx::query_scalar("SELECT enabled FROM profile_capability_refs WHERE profile_id = 'profile-a' AND ref_id = ?")
            .bind(record.ref_id.as_str())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(!disabled_capability);

    let store = SqliteSurfaceStore::new(pool.clone());
    let disabled_binding = store.load_binding("client-a").await.unwrap().unwrap();
    let disabled_entry_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM surface_publications publication
        JOIN surface_manifest_entries entry ON entry.manifest_id = publication.manifest_id
        WHERE publication.publication_id = ?
        "#,
    )
    .bind(&disabled_binding.active_publication_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(disabled_entry_count, 0);

    ProfileSurfaceManagement::mutate_servers(
        &pool,
        "profile-a",
        &["server-a".to_string()],
        ProfileRelationshipAction::Enable,
        2,
        "test",
    )
    .await
    .unwrap();

    let enabled_servers = mcpmate::config::profile::get_profile_servers(&pool, "profile-a")
        .await
        .unwrap();
    assert_eq!(enabled_servers.len(), 1);
    assert!(enabled_servers[0].enabled);
    let enabled_capability: bool =
        sqlx::query_scalar("SELECT enabled FROM profile_capability_refs WHERE profile_id = 'profile-a' AND ref_id = ?")
            .bind(record.ref_id.as_str())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(enabled_capability);
    let restored_binding = store.load_binding("client-a").await.unwrap().unwrap();
    let restored_entry_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM surface_publications publication
        JOIN surface_manifest_entries entry ON entry.manifest_id = publication.manifest_id
        WHERE publication.publication_id = ?
        "#,
    )
    .bind(&restored_binding.active_publication_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(restored_entry_count, 1);
}

#[tokio::test]
async fn enabling_a_profile_server_adds_and_enables_all_current_capability_refs() {
    let (pool, record) = fixture().await;
    sqlx::query("DELETE FROM profile_capability_refs WHERE profile_id = 'profile-a'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        r#"
        INSERT INTO profile_server_relationships (profile_id, server_id, enabled, new_ref_policy)
        VALUES ('profile-a', 'server-a', 0, 'follow')
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    ProfileSurfaceManagement::mutate_servers(
        &pool,
        "profile-a",
        &["server-a".to_string()],
        ProfileRelationshipAction::Enable,
        0,
        "test",
    )
    .await
    .unwrap();

    let enabled: bool =
        sqlx::query_scalar("SELECT enabled FROM profile_capability_refs WHERE profile_id = 'profile-a' AND ref_id = ?")
            .bind(record.ref_id.as_str())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(enabled);
}

#[tokio::test]
async fn disabling_the_last_profile_capability_disables_its_server() {
    let (pool, record) = fixture().await;
    ProfileSurfaceManagement::mutate_servers(
        &pool,
        "profile-a",
        &["server-a".to_string()],
        ProfileRelationshipAction::Enable,
        0,
        "test",
    )
    .await
    .unwrap();

    ProfileSurfaceManagement::mutate_capabilities(
        &pool,
        "profile-a",
        &[record.ref_id.to_string()],
        ProfileRelationshipAction::Disable,
        1,
        HashMap::from([("server-a".to_string(), 1)]),
        "test",
    )
    .await
    .unwrap();

    let server_enabled: bool = sqlx::query_scalar(
        "SELECT enabled FROM profile_server_relationships WHERE profile_id = 'profile-a' AND server_id = 'server-a'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!server_enabled);
}

#[tokio::test]
async fn enabling_a_profile_capability_enables_its_server_without_enabling_siblings() {
    let (pool, record) = fixture().await;
    let sibling_record = add_sibling_tool(&pool, &record).await;
    sqlx::query(
        r#"
        INSERT INTO profile_server_relationships (profile_id, server_id, enabled, new_ref_policy)
        VALUES ('profile-a', 'server-a', 0, 'follow')
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("UPDATE profile_capability_refs SET enabled = 0 WHERE profile_id = 'profile-a'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO profile_capability_refs (profile_id, ref_id, enabled) VALUES ('profile-a', ?, 0)")
        .bind(sibling_record.ref_id.as_str())
        .execute(&pool)
        .await
        .unwrap();

    ProfileSurfaceManagement::mutate_capabilities(
        &pool,
        "profile-a",
        &[record.ref_id.to_string()],
        ProfileRelationshipAction::Enable,
        0,
        HashMap::from([("server-a".to_string(), 2)]),
        "test",
    )
    .await
    .unwrap();

    let server_enabled: bool = sqlx::query_scalar(
        "SELECT enabled FROM profile_server_relationships WHERE profile_id = 'profile-a' AND server_id = 'server-a'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(server_enabled);
    let sibling_enabled: bool =
        sqlx::query_scalar("SELECT enabled FROM profile_capability_refs WHERE profile_id = 'profile-a' AND ref_id = ?")
            .bind(sibling_record.ref_id.as_str())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(!sibling_enabled);
}

#[tokio::test]
async fn reenabling_a_capability_on_an_enabled_server_preserves_derived_siblings() {
    let (pool, record) = fixture().await;
    let sibling_record = add_sibling_tool(&pool, &record).await;
    sqlx::query("DELETE FROM profile_capability_refs WHERE profile_id = 'profile-a'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        r#"
        INSERT INTO profile_server_relationships (profile_id, server_id, enabled, new_ref_policy)
        VALUES ('profile-a', 'server-a', 1, 'follow')
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    ProfileSurfaceManagement::mutate_capabilities(
        &pool,
        "profile-a",
        &[record.ref_id.to_string()],
        ProfileRelationshipAction::Disable,
        0,
        HashMap::from([("server-a".to_string(), 2)]),
        "test",
    )
    .await
    .unwrap();
    ProfileSurfaceManagement::mutate_capabilities(
        &pool,
        "profile-a",
        &[record.ref_id.to_string()],
        ProfileRelationshipAction::Enable,
        1,
        HashMap::from([("server-a".to_string(), 2)]),
        "test",
    )
    .await
    .unwrap();

    let relationships = mcpmate::config::profile::capability_ref::load_profile_capability_refs(
        &pool,
        "profile-a",
        Some(CapabilityKind::Tools),
    )
    .await
    .unwrap();
    let sibling = relationships
        .iter()
        .find(|relationship| relationship.ref_id == sibling_record.ref_id)
        .unwrap();
    assert!(sibling.enabled);
}

#[tokio::test]
async fn disabling_one_of_multiple_enabled_capabilities_keeps_its_server_enabled() {
    let (pool, record) = fixture().await;
    let sibling_record = add_sibling_tool(&pool, &record).await;
    sqlx::query(
        r#"
        INSERT INTO profile_server_relationships (profile_id, server_id, enabled, new_ref_policy)
        VALUES ('profile-a', 'server-a', 1, 'follow')
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO profile_capability_refs (profile_id, ref_id, enabled) VALUES ('profile-a', ?, 1)")
        .bind(sibling_record.ref_id.as_str())
        .execute(&pool)
        .await
        .unwrap();

    ProfileSurfaceManagement::mutate_capabilities(
        &pool,
        "profile-a",
        &[record.ref_id.to_string()],
        ProfileRelationshipAction::Disable,
        0,
        HashMap::from([("server-a".to_string(), 2)]),
        "test",
    )
    .await
    .unwrap();

    let server_enabled: bool = sqlx::query_scalar(
        "SELECT enabled FROM profile_server_relationships WHERE profile_id = 'profile-a' AND server_id = 'server-a'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(server_enabled);
    let sibling_enabled: bool =
        sqlx::query_scalar("SELECT enabled FROM profile_capability_refs WHERE profile_id = 'profile-a' AND ref_id = ?")
            .bind(sibling_record.ref_id.as_str())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(sibling_enabled);
}

#[tokio::test]
async fn profile_server_replace_preserves_retained_server_state_and_enables_new_members() {
    let (pool, _) = fixture().await;
    sqlx::query(
        "INSERT INTO server_config (id, name, server_type, command, enabled) VALUES ('server-b', 'Server B', 'stdio', '', 1)",
    )
    .execute(&pool)
    .await
    .unwrap();
    commit_empty_server_observation(&pool, "server-b").await;
    ProfileSurfaceManagement::mutate_servers(
        &pool,
        "profile-a",
        &["server-a".to_string()],
        ProfileRelationshipAction::Enable,
        0,
        "test",
    )
    .await
    .unwrap();
    ProfileSurfaceManagement::mutate_servers(
        &pool,
        "profile-a",
        &["server-a".to_string()],
        ProfileRelationshipAction::Disable,
        1,
        "test",
    )
    .await
    .unwrap();

    ProfileSurfaceManagement::replace_servers(
        &pool,
        "profile-a",
        &["server-a".to_string(), "server-b".to_string()],
        2,
        "test",
    )
    .await
    .unwrap();

    let servers = mcpmate::config::profile::get_profile_servers(&pool, "profile-a")
        .await
        .unwrap();
    assert_eq!(servers.len(), 2);
    assert_eq!(servers[0].server_id, "server-a");
    assert!(!servers[0].enabled);
    assert_eq!(servers[1].server_id, "server-b");
    assert!(servers[1].enabled);
}

#[tokio::test]
async fn profile_delete_republishes_affected_consumers_in_the_same_operation() {
    let (pool, record) = fixture().await;
    ProfileSurfaceManagement::mutate_capabilities(
        &pool,
        "profile-a",
        &[record.ref_id.to_string()],
        ProfileRelationshipAction::Enable,
        0,
        HashMap::from([("server-a".to_string(), 1)]),
        "test",
    )
    .await
    .unwrap();

    let deleted = ProfileSurfaceManagement::delete_profile(&pool, "profile-a", 1, "test")
        .await
        .unwrap();

    assert_eq!(deleted.profile_name, "Profile A");
    assert_eq!(deleted.materializations.len(), 1);
    let profile_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM profile WHERE id = 'profile-a'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(profile_count, 0);

    let store = SqliteSurfaceStore::new(pool.clone());
    let binding = store.load_binding("client-a").await.unwrap().unwrap();
    let entry_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM surface_publications publication
        JOIN surface_manifest_entries entry ON entry.manifest_id = publication.manifest_id
        WHERE publication.publication_id = ?
        "#,
    )
    .bind(binding.active_publication_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(entry_count, 0);
}

#[tokio::test]
async fn profile_server_replace_is_atomic() {
    let (pool, _) = fixture().await;
    sqlx::query(
        "INSERT INTO server_config (id, name, server_type, command, enabled) VALUES ('server-b', 'Server B', 'stdio', '', 1)",
    )
    .execute(&pool)
    .await
    .unwrap();
    commit_empty_server_observation(&pool, "server-b").await;
    sqlx::query(
        "INSERT INTO profile_server_relationships (profile_id, server_id, new_ref_policy) VALUES ('profile-a', 'server-a', 'follow')",
    )
    .execute(&pool)
    .await
    .unwrap();

    let partial = ProfileSurfaceManagement::replace_servers(
        &pool,
        "profile-a",
        &["server-b".to_string(), "missing".to_string()],
        0,
        "test",
    )
    .await;
    assert!(partial.is_err());
    let unchanged: Vec<String> = sqlx::query_scalar(
        "SELECT server_id FROM profile_server_relationships WHERE profile_id = 'profile-a' ORDER BY server_id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(unchanged, vec!["server-a"]);

    ProfileSurfaceManagement::replace_servers(&pool, "profile-a", &["server-b".to_string()], 0, "test")
        .await
        .unwrap();
    let replaced: Vec<String> = sqlx::query_scalar(
        "SELECT server_id FROM profile_server_relationships WHERE profile_id = 'profile-a' ORDER BY server_id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(replaced, vec!["server-b"]);
    let removed_capability_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM profile_capability_refs profile_ref
        JOIN capability_refs capability_ref ON capability_ref.ref_id = profile_ref.ref_id
        WHERE profile_ref.profile_id = 'profile-a'
          AND capability_ref.server_id = 'server-a'
        "#,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(removed_capability_count, 0);
}

#[tokio::test]
async fn server_operations_use_only_profile_authoring_generation() {
    let (pool, _) = fixture().await;

    ProfileSurfaceManagement::mutate_servers(
        &pool,
        "profile-a",
        &["server-a".to_string()],
        ProfileRelationshipAction::Enable,
        0,
        "test",
    )
    .await
    .unwrap();
    let generation: i64 = sqlx::query_scalar("SELECT authoring_generation FROM profile WHERE id = 'profile-a'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(generation, 1);

    let stale = ProfileSurfaceManagement::mutate_servers(
        &pool,
        "profile-a",
        &["server-a".to_string()],
        ProfileRelationshipAction::Disable,
        0,
        "test",
    )
    .await;
    assert!(matches!(
        stale,
        Err(mcpmate_capability_store::CatalogError::ConcurrencyConflict {
            entity: "profile authoring generation",
            ..
        })
    ));
}

#[tokio::test]
async fn capability_operations_require_exact_dependencies_and_profile_authoring_generation() {
    let (pool, record) = fixture().await;

    ProfileSurfaceManagement::mutate_capabilities(
        &pool,
        "profile-a",
        &[record.ref_id.to_string()],
        ProfileRelationshipAction::Disable,
        0,
        HashMap::from([("server-a".to_string(), 1)]),
        "test",
    )
    .await
    .unwrap();
    let generation: i64 = sqlx::query_scalar("SELECT authoring_generation FROM profile WHERE id = 'profile-a'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(generation, 1);

    for invalid_dependencies in [
        HashMap::new(),
        HashMap::from([("server-a".to_string(), 1), ("unrelated".to_string(), 1)]),
    ] {
        let invalid = ProfileSurfaceManagement::mutate_capabilities(
            &pool,
            "profile-a",
            &[record.ref_id.to_string()],
            ProfileRelationshipAction::Enable,
            1,
            invalid_dependencies,
            "test",
        )
        .await;
        assert!(matches!(
            invalid,
            Err(mcpmate_capability_store::CatalogError::InvalidSurfaceValue {
                field: "profile catalog dependency revisions",
                ..
            })
        ));
    }
    let generation_after_invalid: i64 =
        sqlx::query_scalar("SELECT authoring_generation FROM profile WHERE id = 'profile-a'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(generation_after_invalid, 1);

    add_sibling_tool(&pool, &record).await;
    let drift = ProfileSurfaceManagement::mutate_capabilities(
        &pool,
        "profile-a",
        &[record.ref_id.to_string()],
        ProfileRelationshipAction::Enable,
        1,
        HashMap::from([("server-a".to_string(), 1)]),
        "test",
    )
    .await;
    assert!(matches!(
        drift,
        Err(mcpmate_capability_store::CatalogError::ConcurrencyConflict {
            entity: "profile catalog dependency revisions",
            ..
        })
    ));
    let generation_after_drift: i64 =
        sqlx::query_scalar("SELECT authoring_generation FROM profile WHERE id = 'profile-a'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(generation_after_drift, 1);
}

#[tokio::test]
async fn delete_requires_current_profile_authoring_generation() {
    let (pool, _) = fixture().await;

    let stale = ProfileSurfaceManagement::delete_profile(&pool, "profile-a", 1, "test").await;
    assert!(matches!(
        stale,
        Err(mcpmate_capability_store::CatalogError::ConcurrencyConflict {
            entity: "profile authoring generation",
            ..
        })
    ));
    let retained: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM profile WHERE id = 'profile-a')")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(retained);

    ProfileSurfaceManagement::delete_profile(&pool, "profile-a", 0, "test")
        .await
        .unwrap();
    let deleted: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM profile WHERE id = 'profile-a')")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(!deleted);
}

#[tokio::test]
async fn activation_advances_target_and_automatically_deactivated_authoring_generations() {
    let (pool, _) = fixture().await;
    sqlx::query(
        "INSERT INTO profile (id, name, description, type, role, is_active, multi_select)
         VALUES ('profile-b', 'Profile B', '', 'shared', 'user', 0, 0)",
    )
    .execute(&pool)
    .await
    .unwrap();

    ProfileSurfaceManagement::set_profiles_active(
        &pool,
        &["profile-b".to_string()],
        ProfileActivationAction::Activate,
        HashMap::from([("profile-b".to_string(), 0)]),
        "test",
    )
    .await
    .unwrap();
    let generations: Vec<(String, bool, i64)> =
        sqlx::query_as("SELECT id, is_active, authoring_generation FROM profile ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        generations,
        vec![("profile-a".to_string(), false, 1), ("profile-b".to_string(), true, 1),]
    );

    ProfileSurfaceManagement::set_profiles_active(
        &pool,
        &["profile-b".to_string()],
        ProfileActivationAction::Deactivate,
        HashMap::from([("profile-b".to_string(), 1)]),
        "test",
    )
    .await
    .unwrap();
    let generation: (bool, i64) =
        sqlx::query_as("SELECT is_active, authoring_generation FROM profile WHERE id = 'profile-b'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(generation, (false, 2));
}

#[tokio::test]
async fn batch_activation_validates_once_and_advances_each_profile_once() {
    let (pool, _) = fixture().await;
    sqlx::query("UPDATE profile SET multi_select = 0 WHERE id = 'profile-a'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO profile (id, name, description, type, role, is_active, multi_select)
         VALUES ('profile-b', 'Profile B', '', 'shared', 'user', 1, 0)",
    )
    .execute(&pool)
    .await
    .unwrap();

    let result = ProfileSurfaceManagement::set_profiles_active(
        &pool,
        &["profile-a".to_string(), "profile-b".to_string()],
        ProfileActivationAction::Activate,
        HashMap::from([("profile-a".to_string(), 0), ("profile-b".to_string(), 0)]),
        "test",
    )
    .await
    .unwrap();

    assert_eq!(result.mutations.len(), 2);
    let profiles: Vec<(String, bool, i64)> =
        sqlx::query_as("SELECT id, is_active, authoring_generation FROM profile ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        profiles,
        vec![("profile-a".to_string(), false, 1), ("profile-b".to_string(), true, 1)]
    );
}

#[tokio::test]
async fn batch_activation_reports_the_non_first_stale_profile() {
    let (pool, _) = fixture().await;
    sqlx::query(
        "INSERT INTO profile (id, name, description, type, role, is_active, multi_select, authoring_generation)
         VALUES ('profile-b', 'Profile B', '', 'shared', 'user', 0, 1, 2)",
    )
    .execute(&pool)
    .await
    .unwrap();

    let result = ProfileSurfaceManagement::set_profiles_active(
        &pool,
        &["profile-a".to_string(), "profile-b".to_string()],
        ProfileActivationAction::Activate,
        HashMap::from([("profile-a".to_string(), 0), ("profile-b".to_string(), 1)]),
        "test",
    )
    .await;
    let error = match result {
        Ok(_) => panic!("stale non-first Profile must reject the whole batch"),
        Err(error) => error,
    };

    assert!(matches!(
        error,
        mcpmate_capability_store::CatalogError::ConcurrencyConflict {
            entity: "profile authoring generation",
            ref id,
        } if id == "profile-b"
    ));
    let unchanged: Vec<(String, bool, i64)> =
        sqlx::query_as("SELECT id, is_active, authoring_generation FROM profile ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        unchanged,
        vec![("profile-a".to_string(), true, 0), ("profile-b".to_string(), false, 2)]
    );
}

#[tokio::test]
async fn startup_bootstrap_creates_initial_publications_for_managed_consumers() {
    let (pool, _) = fixture().await;

    let commits = bootstrap_managed_surfaces(&pool).await.unwrap();

    assert_eq!(commits.len(), 1);
    let binding_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM consumer_surface_bindings WHERE consumer_id = 'client-a'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(binding_count, 1);
}

#[tokio::test]
async fn startup_bootstrap_persists_distinct_dependencies_for_each_consumer() {
    let pool = init_management_pool().await;
    for server_id in ["server-empty", "server-capability"] {
        sqlx::query(
            "INSERT INTO server_config (id, name, server_type, command, enabled, unify_direct_exposure_eligible) \
             VALUES (?, ?, 'stdio', '', 1, 1)",
        )
        .bind(server_id)
        .bind(server_id)
        .execute(&pool)
        .await
        .unwrap();
    }
    let capability = CatalogRecord::materialize(
        "server-capability",
        "analyze",
        "server_capability__analyze",
        CapabilityPayload::Tool(Tool::new(
            "analyze",
            "Analyze input",
            std::sync::Arc::new(json!({"type": "object"}).as_object().unwrap().clone()),
        )),
    )
    .unwrap();
    let initialize: InitializeResult = serde_json::from_value(json!({
        "protocolVersion": "2025-11-25",
        "capabilities": {"tools": {"listChanged": true}},
        "serverInfo": {"name": "fixture", "version": "1.0.0"}
    }))
    .unwrap();
    let catalog = SqliteCapabilityCatalog::new(pool.clone());
    catalog
        .commit_observation(CapabilityObservation::new(
            "server-empty",
            "Empty Server",
            "empty-v1",
            initialize.clone(),
            vec![KindObservation::new(
                CapabilityKind::Tools,
                DeclarationState::Supported,
                InventoryState::Complete,
            )],
            Vec::new(),
        ))
        .await
        .unwrap();
    catalog
        .commit_observation(CapabilityObservation::new(
            "server-capability",
            "Capability Server",
            "capability-v1",
            initialize,
            vec![KindObservation::new(
                CapabilityKind::Tools,
                DeclarationState::Supported,
                InventoryState::Complete,
            )],
            vec![capability.clone()],
        ))
        .await
        .unwrap();
    for (consumer_id, route_mode) in [
        ("client-server-intent", "server_level"),
        ("client-capability-intent", "capability_level"),
    ] {
        sqlx::query(
            r#"
            INSERT INTO client (
                id, identifier, name, config_mode, approval_status,
                capability_source, selected_profile_ids, unify_route_mode
            ) VALUES (?, ?, ?, 'unify', 'approved', 'activated', '[]', ?)
            "#,
        )
        .bind(consumer_id)
        .bind(consumer_id)
        .bind(consumer_id)
        .bind(route_mode)
        .execute(&pool)
        .await
        .unwrap();
    }
    sqlx::query(
        "INSERT INTO direct_exposure_servers (consumer_id, server_id, new_ref_policy) \
         VALUES ('client-server-intent', 'server-empty', 'follow')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO direct_exposure_refs (consumer_id, ref_id, enabled) \
         VALUES ('client-capability-intent', ?, 1)",
    )
    .bind(capability.ref_id.as_str())
    .execute(&pool)
    .await
    .unwrap();

    let commits = bootstrap_managed_surfaces(&pool).await.unwrap();

    assert_eq!(commits.len(), 2);
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT consumer_id, source_revision_set FROM surface_proposals \
         WHERE trigger_kind = 'startup_bootstrap' ORDER BY consumer_id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        rows.into_iter()
            .map(|(consumer_id, revisions)| (
                consumer_id,
                serde_json::from_str::<serde_json::Value>(&revisions).unwrap(),
            ))
            .collect::<Vec<_>>(),
        vec![
            ("client-capability-intent".to_string(), json!({"server-capability": 1}),),
            ("client-server-intent".to_string(), json!({"server-empty": 1}),),
        ]
    );
}

#[tokio::test]
async fn startup_bootstrap_publishes_an_empty_surface_for_inherited_managed_mode() {
    let pool = init_management_pool().await;
    sqlx::query(
        r#"
        INSERT INTO client (
            id, identifier, name, config_mode, approval_status,
            capability_source, selected_profile_ids
        )
        VALUES (
            'consumer-empty', 'client-empty', 'Empty Client', NULL, 'approved',
            'activated', '[]'
        )
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    let commits = bootstrap_managed_surfaces(&pool).await.unwrap();

    assert_eq!(commits.len(), 1);
    let binding = SqliteSurfaceStore::new(pool.clone())
        .load_binding("client-empty")
        .await
        .unwrap()
        .expect("inherited managed Consumer should have an active publication");
    let entry_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM surface_publications publication
        LEFT JOIN surface_manifest_entries entry ON entry.manifest_id = publication.manifest_id
        WHERE publication.publication_id = ? AND entry.manifest_id IS NOT NULL
        "#,
    )
    .bind(binding.active_publication_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(entry_count, 0);
}

#[tokio::test]
async fn builtin_catalog_sync_republishes_existing_unify_surface_with_only_ucan_tools() {
    let pool = init_management_pool().await;
    sqlx::query(
        r#"
        INSERT INTO client (
            id, identifier, name, config_mode, approval_status,
            capability_source, selected_profile_ids
        )
        VALUES (
            'consumer-unify', 'client-unify', 'Unify Client', 'unify', 'approved',
            'activated', '[]'
        )
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    bootstrap_managed_surfaces(&pool).await.unwrap();
    let store = SqliteSurfaceStore::new(pool.clone());
    let initial_binding = store
        .load_binding("client-unify")
        .await
        .unwrap()
        .expect("startup should publish the initial empty Surface");

    let builtin_records = unify_builtin_catalog_records();

    let (catalog_commit, materializations) =
        synchronize_builtin_catalog_and_bootstrap_managed_surfaces(&pool, builtin_records.clone())
            .await
            .unwrap();

    assert!(catalog_commit.changed);
    assert_eq!(materializations.len(), 1);
    let refreshed_binding = store
        .load_binding("client-unify")
        .await
        .unwrap()
        .expect("builtin Catalog change should republish the existing Surface");
    assert_eq!(refreshed_binding.generation, initial_binding.generation + 1);

    let exposed_names: Vec<String> = sqlx::query_scalar(
        r#"
        SELECT capability_ref.origin_key
        FROM consumer_surface_bindings binding
        JOIN surface_publications publication
          ON publication.publication_id = binding.active_publication_id
        JOIN surface_manifest_entries entry
          ON entry.manifest_id = publication.manifest_id
        JOIN capability_refs capability_ref
          ON capability_ref.ref_id = entry.ref_id
        WHERE binding.consumer_id = 'client-unify'
        ORDER BY capability_ref.origin_key
        "#,
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(exposed_names, sorted_unify_builtin_tool_names());

    let (unchanged_commit, unchanged_materializations) =
        synchronize_builtin_catalog_and_bootstrap_managed_surfaces(&pool, builtin_records)
            .await
            .unwrap();
    assert!(!unchanged_commit.changed);
    assert!(unchanged_materializations.is_empty());
    let unchanged_binding = store.load_binding("client-unify").await.unwrap().unwrap();
    assert_eq!(unchanged_binding.generation, refreshed_binding.generation);
}

#[tokio::test]
async fn global_server_disable_scopes_current_catalog_revisions_to_affected_surfaces() {
    let (pool, _) = fixture().await;
    sqlx::query(
        "INSERT INTO server_config (id, name, server_type, command, enabled) VALUES ('server-b', 'Server B', 'stdio', '', 1)",
    )
    .execute(&pool)
    .await
    .unwrap();
    let unrelated_initialize: InitializeResult = serde_json::from_value(json!({
        "protocolVersion": "2025-11-25",
        "capabilities": {},
        "serverInfo": {"name": "unrelated", "version": "1.0.0"}
    }))
    .unwrap();
    let catalog = SqliteCapabilityCatalog::new(pool.clone());
    catalog
        .commit_observation(CapabilityObservation::new(
            "server-b",
            "unrelated",
            "config-v1",
            unrelated_initialize.clone(),
            Vec::new(),
            Vec::new(),
        ))
        .await
        .unwrap();
    bootstrap_managed_surfaces(&pool).await.unwrap();
    catalog
        .commit_observation(CapabilityObservation::new(
            "server-b",
            "unrelated",
            "config-v2",
            unrelated_initialize,
            Vec::new(),
            Vec::new(),
        ))
        .await
        .unwrap();

    let result = ServerSurfaceManagement::set_server_enabled(&pool, "server-a", false, "test")
        .await
        .unwrap();

    assert!(!result.enabled);
    assert_eq!(result.materializations.len(), 1);
    let enabled: bool = sqlx::query_scalar("SELECT enabled FROM server_config WHERE id = 'server-a'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(!enabled);
    let active_entry_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM consumer_surface_bindings binding
        JOIN surface_publications publication
          ON publication.publication_id = binding.active_publication_id
        JOIN surface_manifest_entries entry ON entry.manifest_id = publication.manifest_id
        WHERE binding.consumer_id = 'client-a'
        "#,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(active_entry_count, 0);
    let source_revision_set: String = sqlx::query_scalar(
        "SELECT source_revision_set FROM surface_proposals WHERE trigger_kind = 'server_status_save' ORDER BY rowid DESC LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&source_revision_set).unwrap(),
        json!({"server-a": 1})
    );
}

#[tokio::test]
async fn direct_exposure_eligibility_and_affected_surfaces_commit_together() {
    let (pool, record) = fixture().await;
    sqlx::query("UPDATE profile_capability_refs SET enabled = 0 WHERE profile_id = 'profile-a'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE server_config SET unify_direct_exposure_eligible = 1 WHERE id = 'server-a'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO direct_exposure_refs (consumer_id, ref_id, enabled) VALUES ('client-a', ?, 1)")
        .bind(record.ref_id.as_str())
        .execute(&pool)
        .await
        .unwrap();
    bootstrap_managed_surfaces(&pool).await.unwrap();

    let result = ServerSurfaceManagement::set_direct_exposure_eligible(
        &pool,
        "server-a",
        false,
        HashMap::from([("server-a".to_string(), 1)]),
        "test",
    )
    .await
    .unwrap();

    assert!(!result.unify_direct_exposure_eligible);
    assert_eq!(result.materializations.len(), 1);
    let eligible: bool =
        sqlx::query_scalar("SELECT unify_direct_exposure_eligible FROM server_config WHERE id = 'server-a'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(!eligible);
    let intent_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM direct_exposure_refs WHERE consumer_id = 'client-a' AND ref_id = ?")
            .bind(record.ref_id.as_str())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(intent_count, 1);
    let active_entry_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM consumer_surface_bindings binding
        JOIN surface_publications publication
          ON publication.publication_id = binding.active_publication_id
        JOIN surface_manifest_entries entry ON entry.manifest_id = publication.manifest_id
        WHERE binding.consumer_id = 'client-a'
        "#,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(active_entry_count, 0);
}
#[path = "support/database.rs"]
mod database_support;
