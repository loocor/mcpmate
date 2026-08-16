use std::{collections::HashMap, path::PathBuf, sync::Arc};

use axum::{
    Json,
    extract::{Query, State},
};
use mcpmate::{
    api::{
        handlers::profile::workflow_specification_preview,
        models::{profile::ProfileIdReq, resp::ResponseConverter},
        routes::{AppState, unavailable_secret_store_readiness},
    },
    config::{database::Database, models::ProfileMode},
    core::{
        capability::management::{ProfileActivationAction, ProfileSurfaceManagement},
        pool::UpstreamConnectionPool,
        profile::{
            authoring::{ProfileAuthoringCommand, ProfileAuthoringService},
            workflow::{
                WorkflowBindingCommand, WorkflowBindingPolicy, WorkflowBindingValidation, WorkflowSpecificationError,
                WorkflowSpecificationSaveCommand, WorkflowSpecificationService, WorkflowStepCommand,
            },
        },
        proxy::ProxyServer,
    },
    inspector::{calls::InspectorCallRegistry, service as inspector_service, sessions::InspectorSessionManager},
    system::metrics::MetricsCollector,
};
use mcpmate_capability_store::{
    CapabilityCatalog, CapabilityKind, CapabilityObservation, CapabilityPayload, CatalogError, CatalogRecord,
    DeclarationState, InventoryState, KindObservation, SqliteCapabilityCatalog,
};
use rmcp::model::{InitializeResult, Tool};
use serde_json::json;
use sqlx::sqlite::SqlitePoolOptions;
use tokio::sync::{Mutex, RwLock};

#[path = "support/database.rs"]
mod database_support;

async fn pool() -> sqlx::SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("connect test database");
    database_support::prepare_config(&pool).await;
    pool
}

fn tool_record(
    server_id: &str,
    name: &str,
    description: &str,
) -> CatalogRecord {
    CatalogRecord::materialize(
        server_id,
        name,
        format!("{}__{name}", server_id.replace('-', "_")),
        CapabilityPayload::Tool(Tool::new(
            name.to_string(),
            description.to_string(),
            Arc::new(json!({"type": "object"}).as_object().unwrap().clone()),
        )),
    )
    .expect("materialize fixture tool")
}

async fn observe_server(
    pool: &sqlx::SqlitePool,
    server_id: &str,
    server_name: &str,
    description: &str,
) -> Vec<String> {
    let initialize: InitializeResult = serde_json::from_value(json!({
        "protocolVersion": "2025-11-25",
        "capabilities": {"tools": {"listChanged": true}},
        "serverInfo": {"name": server_name, "version": "1.0.0"}
    }))
    .expect("fixture initialize result");
    let records = vec![
        tool_record(server_id, "lookup", description),
        tool_record(server_id, "search", "search supporting evidence"),
    ];
    let ref_ids = records.iter().map(|record| record.ref_id.to_string()).collect();
    SqliteCapabilityCatalog::new(pool.clone())
        .commit_observation(CapabilityObservation::new(
            server_id,
            server_name,
            "config-v1",
            initialize,
            vec![KindObservation::new(
                CapabilityKind::Tools,
                DeclarationState::Supported,
                InventoryState::Complete,
            )],
            records,
        ))
        .await
        .expect("commit fixture capability observation");
    ref_ids
}

async fn add_server(
    pool: &sqlx::SqlitePool,
    server_id: &str,
    server_name: &str,
    enabled: i64,
) -> Vec<String> {
    sqlx::query(
        "INSERT INTO server_config (id, name, server_type, command, enabled)
         VALUES (?, ?, 'stdio', '', ?)",
    )
    .bind(server_id)
    .bind(server_name)
    .bind(enabled)
    .execute(pool)
    .await
    .expect("insert fixture server");
    observe_server(pool, server_id, server_name, "initial lookup").await
}

fn workflow_profile_command() -> ProfileAuthoringCommand {
    ProfileAuthoringCommand {
        id: None,
        expected_authoring_generation: None,
        name: "Investigate incident".to_string(),
        description: Some("Workflow-only authoring profile".to_string()),
        profile_type: "scenario".to_string(),
        priority: 0,
        is_active: false,
        is_default: false,
        server_ids: vec!["server-a".to_string()],
        clone_from_id: None,
        profile_mode: Some(ProfileMode::Workflow),
    }
}

fn workflow_steps(ref_ids: &[String]) -> Vec<WorkflowStepCommand> {
    vec![
        WorkflowStepCommand {
            title: "Discover".to_string(),
            description: Some("Inspect available evidence".to_string()),
            bindings: vec![
                WorkflowBindingCommand {
                    ref_id: ref_ids[0].clone(),
                    binding_policy: WorkflowBindingPolicy::default(),
                },
                WorkflowBindingCommand {
                    ref_id: ref_ids[1].clone(),
                    binding_policy: WorkflowBindingPolicy::Direct,
                },
            ],
        },
        WorkflowStepCommand {
            title: "Resolve".to_string(),
            description: None,
            bindings: vec![WorkflowBindingCommand {
                ref_id: ref_ids[0].clone(),
                binding_policy: WorkflowBindingPolicy::Direct,
            }],
        },
    ]
}

fn app_state(pool: sqlx::SqlitePool) -> Arc<AppState> {
    let database = Arc::new(Database {
        pool,
        path: PathBuf::new(),
        capability_cache: Arc::new(mcpmate_capability_store::DerivedCapabilityCache::default()),
    });
    let inspector_calls = Arc::new(InspectorCallRegistry::new());
    inspector_service::set_call_registry(inspector_calls.clone());
    let config = Arc::new(mcpmate::core::models::Config::default());
    let connection_pool = Arc::new(Mutex::new(UpstreamConnectionPool::new(
        config.clone(),
        Some(database.clone()),
    )));
    let mut proxy = ProxyServer::new(config);
    proxy.connection_pool = connection_pool.clone();
    proxy.database = Some(database.clone());
    Arc::new(AppState {
        connection_pool,
        metrics_collector: Arc::new(MetricsCollector::new(std::time::Duration::from_secs(1))),
        http_proxy: Some(Arc::new(proxy)),
        profile_merge_service: None,
        database: Some(database),
        audit_database: None,
        audit_service: None,
        config_application_state: Arc::new(mcpmate::core::profile::ConfigApplicationStateManager::new()),
        client_service: None,
        inspector_calls,
        inspector_sessions: Arc::new(InspectorSessionManager::new()),
        oauth_manager: RwLock::new(None),
        secret_store: RwLock::new(None),
        secret_store_readiness: RwLock::new(unavailable_secret_store_readiness("test_unavailable")),
    })
}

#[tokio::test]
async fn workflow_specification_is_ordered_cas_aware_and_never_publishes_a_surface() {
    let pool = pool().await;
    let ref_ids = add_server(&pool, "server-a", "Server A", 1).await;
    let profile = ProfileAuthoringService::new(pool.clone())
        .save(workflow_profile_command(), "test")
        .await
        .expect("create workflow Profile");
    assert_eq!(profile.profile_mode, ProfileMode::Workflow);
    assert!(profile.materializations.is_empty());
    let profile_response = ResponseConverter::profile_to_response(&profile.profile);
    assert_eq!(profile_response.profile_mode, ProfileMode::Workflow);
    assert!(
        !profile_response
            .allowed_operations
            .iter()
            .any(|operation| operation == "activate")
    );
    let profile_id = profile.profile.id.clone().expect("created workflow Profile ID");
    let activation = ProfileSurfaceManagement::set_profiles_active(
        &pool,
        std::slice::from_ref(&profile_id),
        ProfileActivationAction::Activate,
        HashMap::from([(profile_id.clone(), profile.profile.authoring_generation)]),
        "test",
    )
    .await;
    assert!(matches!(activation, Err(CatalogError::InvalidSurfaceValue { .. })));
    let baseline_publications: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM surface_publications")
        .fetch_one(&pool)
        .await
        .expect("count surface publications");

    let service = WorkflowSpecificationService::new(pool.clone());
    let empty = service
        .view(&profile_id)
        .await
        .expect("load empty workflow specification");
    assert_eq!(empty.specification_revision, None);
    assert_eq!(empty.validation_notes, None);
    assert_eq!(empty.avoid_rules, None);
    assert_eq!(empty.tool_binding_count, 0);
    assert!(empty.steps.is_empty());
    let saved = service
        .save(WorkflowSpecificationSaveCommand {
            profile_id: profile_id.clone(),
            expected_specification_revision: None,
            validation_notes: Some("Confirm the evidence before resolving the incident".to_string()),
            avoid_rules: Some("Do not change production state during investigation".to_string()),
            steps: workflow_steps(&ref_ids),
        })
        .await
        .expect("create workflow specification");
    assert_eq!(saved.specification_revision, Some(0));
    assert_eq!(
        saved.validation_notes.as_deref(),
        Some("Confirm the evidence before resolving the incident")
    );
    assert_eq!(
        saved.avoid_rules.as_deref(),
        Some("Do not change production state during investigation")
    );
    assert_eq!(
        saved.steps.iter().map(|step| step.title.as_str()).collect::<Vec<_>>(),
        ["Discover", "Resolve"]
    );
    assert_eq!(
        saved.steps[0].bindings[0].binding_policy,
        WorkflowBindingPolicy::MetaOnDemand
    );
    assert_eq!(saved.steps[0].bindings[1].ref_id, ref_ids[1]);
    assert_eq!(saved.steps[0].bindings[1].binding_policy, WorkflowBindingPolicy::Direct);
    assert_eq!(saved.steps[1].bindings[0].binding_policy, WorkflowBindingPolicy::Direct);
    assert_eq!(saved.tool_binding_count, 3);
    let persisted_policies: Vec<String> = sqlx::query_scalar(
        "SELECT binding_policy FROM workflow_profile_step_bindings WHERE profile_id = ? ORDER BY step_index, binding_index",
    )
    .bind(&profile_id)
    .fetch_all(&pool)
    .await
    .expect("load persisted binding order");
    assert_eq!(persisted_policies, ["meta_on_demand", "direct", "direct"]);
    let persisted_guidance: (Option<String>, Option<String>) = sqlx::query_as(
        "SELECT validation_notes, avoid_rules FROM workflow_profile_specifications WHERE profile_id = ?",
    )
    .bind(&profile_id)
    .fetch_one(&pool)
    .await
    .expect("load persisted workflow guidance");
    assert_eq!(persisted_guidance.0, saved.validation_notes);
    assert_eq!(persisted_guidance.1, saved.avoid_rules);
    let publications_after_save: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM surface_publications")
        .fetch_one(&pool)
        .await
        .expect("recount surface publications");
    assert_eq!(publications_after_save, baseline_publications);

    let updated = service
        .save(WorkflowSpecificationSaveCommand {
            profile_id: profile_id.clone(),
            expected_specification_revision: Some(0),
            validation_notes: Some("Re-check the final incident evidence".to_string()),
            avoid_rules: Some("Avoid unreviewed production changes".to_string()),
            steps: workflow_steps(&ref_ids),
        })
        .await
        .expect("update workflow specification");
    assert_eq!(updated.specification_revision, Some(1));
    assert_eq!(
        updated.validation_notes.as_deref(),
        Some("Re-check the final incident evidence")
    );
    assert_eq!(
        updated.avoid_rules.as_deref(),
        Some("Avoid unreviewed production changes")
    );
    let stale = service
        .save(WorkflowSpecificationSaveCommand {
            profile_id: profile_id.clone(),
            expected_specification_revision: Some(0),
            validation_notes: Some("stale guidance".to_string()),
            avoid_rules: Some("stale avoidance".to_string()),
            steps: workflow_steps(&ref_ids),
        })
        .await
        .expect_err("stale workflow specification update must fail");
    assert!(matches!(
        stale,
        WorkflowSpecificationError::SpecificationChanged {
            current_specification_revision: 1
        }
    ));

    observe_server(&pool, "server-a", "Server A", "changed lookup").await;
    let Json(preview) = workflow_specification_preview(
        State(app_state(pool.clone())),
        Query(ProfileIdReq { id: profile_id.clone() }),
    )
    .await
    .expect("preview through Workflow API handler");
    let preview = preview.data.expect("Workflow API preview data").preview;
    assert!(!preview.valid);
    assert_eq!(preview.specification_revision, Some(1));
    assert_eq!(preview.validation_notes, updated.validation_notes);
    assert_eq!(preview.avoid_rules, updated.avoid_rules);
    assert_eq!(
        preview.steps[0].bindings[0].validation,
        WorkflowBindingValidation::Drifted
    );

    let invalid = service
        .save(WorkflowSpecificationSaveCommand {
            profile_id: profile_id.clone(),
            expected_specification_revision: Some(1),
            validation_notes: updated.validation_notes.clone(),
            avoid_rules: updated.avoid_rules.clone(),
            steps: vec![WorkflowStepCommand {
                title: "Invalid".to_string(),
                description: None,
                bindings: vec![WorkflowBindingCommand {
                    ref_id: "tool:server-a:missing".to_string(),
                    binding_policy: WorkflowBindingPolicy::MetaOnDemand,
                }],
            }],
        })
        .await
        .expect_err("binding outside the Profile capability boundary must fail");
    assert!(matches!(invalid, WorkflowSpecificationError::InvalidBinding { .. }));
    assert_eq!(service.view(&profile_id).await.unwrap().specification_revision, Some(1));
    sqlx::query("UPDATE capability_refs SET kind = 'resources' WHERE ref_id = ?")
        .bind(&ref_ids[1])
        .execute(&pool)
        .await
        .expect("change one fixture capability to a resource");
    let viewed = service
        .view(&profile_id)
        .await
        .expect("load workflow specification with mixed capability kinds");
    assert_eq!(viewed.tool_binding_count, 2);

    service
        .delete(&profile_id, 1)
        .await
        .expect("delete workflow specification with current revision");
    let empty_after_delete = service
        .view(&profile_id)
        .await
        .expect("reload empty specification after delete");
    assert_eq!(empty_after_delete.specification_revision, None);
    assert!(empty_after_delete.steps.is_empty());
}

#[tokio::test]
async fn workflow_authoring_and_specification_save_rolls_back_together() {
    let pool = pool().await;
    let ref_ids = add_server(&pool, "server-a", "Server A", 1).await;
    let authoring = ProfileAuthoringService::new(pool.clone());
    let created = authoring
        .save(workflow_profile_command(), "test")
        .await
        .expect("create workflow Profile");
    let profile_id = created.profile.id.clone().expect("created workflow Profile ID");
    let workflow = WorkflowSpecificationService::new(pool.clone())
        .save(WorkflowSpecificationSaveCommand {
            profile_id: profile_id.clone(),
            expected_specification_revision: None,
            validation_notes: Some("Initial validation notes".to_string()),
            avoid_rules: None,
            steps: workflow_steps(&ref_ids),
        })
        .await
        .expect("create workflow specification");

    let mut command = workflow_profile_command();
    command.id = Some(profile_id.clone());
    command.expected_authoring_generation = Some(created.profile.authoring_generation);
    command.name = "Changed workflow name".to_string();
    let error = authoring
        .save_with_workflow_specification(
            command,
            WorkflowSpecificationSaveCommand {
                profile_id: profile_id.clone(),
                expected_specification_revision: Some(9),
                validation_notes: Some("Stale validation notes".to_string()),
                avoid_rules: None,
                steps: workflow_steps(&ref_ids),
            },
        )
        .await
        .expect_err("stale workflow specification must roll back authoring changes");
    assert!(matches!(
        error,
        mcpmate::core::profile::authoring::WorkflowProfileAuthoringError::Workflow(
            WorkflowSpecificationError::SpecificationChanged {
                current_specification_revision: 0
            }
        )
    ));

    let persisted_name: String = sqlx::query_scalar("SELECT name FROM profile WHERE id = ?")
        .bind(&profile_id)
        .fetch_one(&pool)
        .await
        .expect("load persisted Profile name");
    assert_eq!(persisted_name, "Investigate incident");
    let persisted = WorkflowSpecificationService::new(pool.clone())
        .view(&profile_id)
        .await
        .expect("load persisted workflow specification");
    assert_eq!(persisted.specification_revision, workflow.specification_revision);
    assert_eq!(persisted.validation_notes, workflow.validation_notes);
}

#[tokio::test]
async fn workflow_binding_accepts_globally_enabled_servers_outside_the_profile() {
    let pool = pool().await;
    add_server(&pool, "server-a", "Server A", 1).await;
    let external_refs = add_server(&pool, "server-b", "Server B", 1).await;
    let profile = ProfileAuthoringService::new(pool.clone())
        .save(workflow_profile_command(), "test")
        .await
        .expect("create workflow Profile");
    let profile_id = profile.profile.id.clone().expect("created workflow Profile ID");

    let service = WorkflowSpecificationService::new(pool.clone());
    let saved = service
        .save(WorkflowSpecificationSaveCommand {
            profile_id: profile_id.clone(),
            expected_specification_revision: None,
            validation_notes: None,
            avoid_rules: None,
            steps: vec![WorkflowStepCommand {
                title: "External evidence".to_string(),
                description: None,
                bindings: vec![WorkflowBindingCommand {
                    ref_id: external_refs[0].clone(),
                    binding_policy: WorkflowBindingPolicy::MetaOnDemand,
                }],
            }],
        })
        .await
        .expect("workflow may bind a globally enabled server capability outside the Profile");
    assert_eq!(saved.steps[0].bindings[0].ref_id, external_refs[0]);

    let Json(preview) = workflow_specification_preview(
        State(app_state(pool.clone())),
        Query(ProfileIdReq { id: profile_id.clone() }),
    )
    .await
    .expect("preview after global binding");
    assert!(preview.data.expect("preview data").preview.valid);

    observe_server(&pool, "server-b", "Server B", "changed external").await;
    let Json(preview) = workflow_specification_preview(
        State(app_state(pool.clone())),
        Query(ProfileIdReq { id: profile_id.clone() }),
    )
    .await
    .expect("preview after global capability drift");
    let preview = preview.data.expect("preview data").preview;
    assert!(!preview.valid);
    assert_eq!(
        preview.steps[0].bindings[0].validation,
        WorkflowBindingValidation::Drifted
    );
}

#[tokio::test]
async fn workflow_binding_rejects_globally_disabled_servers() {
    let pool = pool().await;
    add_server(&pool, "server-a", "Server A", 1).await;
    let disabled_refs = add_server(&pool, "server-c", "Server C", 0).await;
    let profile = ProfileAuthoringService::new(pool.clone())
        .save(workflow_profile_command(), "test")
        .await
        .expect("create workflow Profile");
    let profile_id = profile.profile.id.clone().expect("created workflow Profile ID");

    let invalid = WorkflowSpecificationService::new(pool.clone())
        .save(WorkflowSpecificationSaveCommand {
            profile_id,
            expected_specification_revision: None,
            validation_notes: None,
            avoid_rules: None,
            steps: vec![WorkflowStepCommand {
                title: "Disabled".to_string(),
                description: None,
                bindings: vec![WorkflowBindingCommand {
                    ref_id: disabled_refs[0].clone(),
                    binding_policy: WorkflowBindingPolicy::MetaOnDemand,
                }],
            }],
        })
        .await
        .expect_err("globally disabled server capability must not be bindable");
    assert!(matches!(invalid, WorkflowSpecificationError::InvalidBinding { .. }));
}
