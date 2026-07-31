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
        HashMap::from([("server-a".to_string(), 1)]),
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
        HashMap::from([("server-a".to_string(), 1)]),
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
        HashMap::new(),
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
async fn profile_server_toggle_updates_all_capabilities_without_removing_membership() {
    let (pool, record) = fixture().await;
    ProfileSurfaceManagement::mutate_servers(
        &pool,
        "profile-a",
        &["server-a".to_string()],
        ProfileRelationshipAction::Enable,
        HashMap::from([("server-a".to_string(), 1)]),
        "test",
    )
    .await
    .unwrap();

    ProfileSurfaceManagement::mutate_servers(
        &pool,
        "profile-a",
        &["server-a".to_string()],
        ProfileRelationshipAction::Disable,
        HashMap::from([("server-a".to_string(), 1)]),
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
        HashMap::from([("server-a".to_string(), 1)]),
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
        HashMap::from([("server-a".to_string(), 1)]),
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
        HashMap::from([("server-a".to_string(), 1)]),
        "test",
    )
    .await
    .unwrap();

    ProfileSurfaceManagement::mutate_capabilities(
        &pool,
        "profile-a",
        &[record.ref_id.to_string()],
        ProfileRelationshipAction::Disable,
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
    ProfileSurfaceManagement::mutate_servers(
        &pool,
        "profile-a",
        &["server-a".to_string()],
        ProfileRelationshipAction::Enable,
        HashMap::from([("server-a".to_string(), 1)]),
        "test",
    )
    .await
    .unwrap();
    ProfileSurfaceManagement::mutate_servers(
        &pool,
        "profile-a",
        &["server-a".to_string()],
        ProfileRelationshipAction::Disable,
        HashMap::from([("server-a".to_string(), 1)]),
        "test",
    )
    .await
    .unwrap();

    ProfileSurfaceManagement::replace_servers(
        &pool,
        "profile-a",
        &["server-a".to_string(), "server-b".to_string()],
        HashMap::from([("server-a".to_string(), 1)]),
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
        HashMap::from([("server-a".to_string(), 1)]),
        "test",
    )
    .await
    .unwrap();

    let deleted = ProfileSurfaceManagement::delete_profile(
        &pool,
        "profile-a",
        HashMap::from([("server-a".to_string(), 1)]),
        "test",
    )
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
        HashMap::from([("server-a".to_string(), 1)]),
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

    ProfileSurfaceManagement::replace_servers(
        &pool,
        "profile-a",
        &["server-b".to_string()],
        HashMap::from([("server-a".to_string(), 1)]),
        "test",
    )
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
async fn global_server_status_and_affected_surfaces_commit_together() {
    let (pool, _) = fixture().await;
    bootstrap_managed_surfaces(&pool).await.unwrap();

    let result = ServerSurfaceManagement::set_server_enabled(
        &pool,
        "server-a",
        false,
        HashMap::from([("server-a".to_string(), 1)]),
        "test",
    )
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
