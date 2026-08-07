use super::common::*;
use crate::api::handlers::ProfileConflict;
use crate::api::models::profile::{
    ProfileAuthoringSaveData, ProfileAuthoringSaveReq, ProfileAuthoringSaveResp, ProfileAuthoringViewData,
    ProfileAuthoringViewResp, ProfileIdReq,
};
use crate::core::profile::authoring::{ProfileAuthoringCommand, ProfileAuthoringError, ProfileAuthoringService};

pub async fn profile_authoring_view(
    State(state): State<Arc<AppState>>,
    Query(request): Query<ProfileIdReq>,
) -> Result<Json<ProfileAuthoringViewResp>, ApiError> {
    let db = get_database(&state).await?;
    let view = ProfileAuthoringService::new(db.pool.clone())
        .view(&request.id)
        .await
        .map_err(profile_authoring_error)?;
    Ok(Json(ProfileAuthoringViewResp::success(ProfileAuthoringViewData {
        profile: profile_to_response(&view.profile),
        server_ids: view.server_ids,
    })))
}

pub async fn profile_authoring_save(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ProfileAuthoringSaveReq>,
) -> Result<Json<ProfileAuthoringSaveResp>, ApiError> {
    let started_at = std::time::Instant::now();
    let db = get_database(&state).await?;
    let is_create = request.id.is_none();
    let saved = ProfileAuthoringService::new(db.pool.clone())
        .save(
            ProfileAuthoringCommand {
                id: request.id,
                expected_authoring_generation: request.expected_authoring_generation,
                name: request.name,
                description: request.description,
                profile_type: request.profile_type,
                multi_select: request.multi_select,
                priority: request.priority,
                is_active: request.is_active,
                is_default: request.is_default,
                server_ids: request.server_ids,
                clone_from_id: request.clone_from_id,
            },
            "profile_management",
        )
        .await
        .map_err(profile_authoring_error)?;

    publish_post_commit_runtime_effects(&saved);
    if let Some(profile_service) = &state.profile_merge_service {
        profile_service.invalidate_cache().await;
    }
    let profile_id = saved.profile.id.clone().unwrap_or_default();
    let response = Json(ProfileAuthoringSaveResp::success(ProfileAuthoringSaveData {
        profile: profile_to_response(&saved.profile),
    }));
    crate::audit::interceptor::emit_event(
        state.audit_service.as_ref(),
        crate::audit::interceptor::build_rest_event(
            if is_create {
                crate::audit::AuditAction::ProfileCreate
            } else {
                crate::audit::AuditAction::ProfileUpdate
            },
            crate::audit::AuditStatus::Success,
            "POST",
            "/api/mcp/profile/authoring/save",
            Some(started_at.elapsed().as_millis() as u64),
            None,
            Some(profile_id.clone()),
            None,
            None,
        ),
    )
    .await;
    super::emit_surface_publication_audits(
        &state,
        "profile_management",
        Some(&profile_id),
        "/api/mcp/profile/authoring/save",
        saved.materializations,
    )
    .await;
    Ok(response)
}

fn publish_post_commit_runtime_effects(saved: &crate::core::profile::authoring::ProfileAuthoringSaveResult) {
    let profile_id = saved.profile.id.as_deref().unwrap_or_default();
    if let Some(enabled) = saved.activation_delta {
        crate::core::events::EventBus::global().publish(crate::core::events::Event::ProfileStatusChanged {
            profile_id: profile_id.to_string(),
            enabled,
        });
    }
    for deactivated_profile_id in &saved.automatically_deactivated_profile_ids {
        crate::core::events::EventBus::global().publish(crate::core::events::Event::ProfileStatusChanged {
            profile_id: deactivated_profile_id.clone(),
            enabled: false,
        });
    }
    for delta in &saved.server_relationship_deltas {
        crate::core::events::EventBus::global().publish(crate::core::events::Event::ServerEnabledInProfileChanged {
            server_id: delta.server_id.clone(),
            server_name: delta.server_name.clone(),
            profile_id: profile_id.to_string(),
            enabled: delta.enabled,
        });
    }
}

fn profile_authoring_error(error: ProfileAuthoringError) -> ApiError {
    match error {
        ProfileAuthoringError::InvalidRequest(message) => ApiError::BadRequest(message),
        ProfileAuthoringError::InvalidTarget { dependency_server_ids } => {
            ApiError::InvalidProfileTarget(dependency_server_ids)
        }
        ProfileAuthoringError::NotFound { profile_id } => {
            ApiError::NotFound(format!("Profile with ID '{profile_id}' not found"))
        }
        ProfileAuthoringError::ProfileAuthoringChanged {
            current_authoring_generation,
        } => ApiError::ProfileConflict(ProfileConflict::profile_authoring_changed(current_authoring_generation)),
        ProfileAuthoringError::ConsumerBindingChanged { dependency_server_ids } => {
            ApiError::ProfileConflict(ProfileConflict::consumer_binding_changed(dependency_server_ids))
        }
        ProfileAuthoringError::Persistence(mcpmate_capability_store::CatalogError::ConcurrencyConflict {
            entity: "capability catalog revision set",
            id,
        }) => ApiError::ProfileConflict(ProfileConflict::catalog_dependency_changed(
            id.split(',')
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .collect(),
        )),
        ProfileAuthoringError::Persistence(_) | ProfileAuthoringError::Database(_) => {
            ApiError::InternalError("Profile authoring failed".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::common::profile::ProfileType;
    use crate::config::models::Profile;
    use crate::core::events::{Event, EventBus};
    use crate::core::profile::authoring::{ProfileAuthoringSaveResult, ProfileServerRelationshipDelta};

    #[test]
    fn post_commit_runtime_effects_are_published_once_from_committed_deltas() {
        let mut receiver = EventBus::global().subscribe_async();
        let mut profile = Profile::new("Profile B".to_string(), ProfileType::Shared);
        profile.id = Some("effect-profile-b".to_string());
        let saved = ProfileAuthoringSaveResult {
            profile,
            server_ids: vec!["effect-server-b".to_string()],
            materializations: Vec::new(),
            activation_delta: Some(true),
            automatically_deactivated_profile_ids: vec!["effect-profile-a".to_string()],
            server_relationship_deltas: vec![
                ProfileServerRelationshipDelta {
                    server_id: "effect-server-a".to_string(),
                    server_name: "Server A".to_string(),
                    enabled: false,
                },
                ProfileServerRelationshipDelta {
                    server_id: "effect-server-b".to_string(),
                    server_name: "Server B".to_string(),
                    enabled: true,
                },
            ],
        };

        super::publish_post_commit_runtime_effects(&saved);

        let mut effects = Vec::new();
        while let Ok(event) = receiver.try_recv() {
            match event {
                Event::ProfileStatusChanged { profile_id, enabled } if profile_id.starts_with("effect-") => {
                    effects.push(format!("profile:{profile_id}:{enabled}"));
                }
                Event::ServerEnabledInProfileChanged {
                    server_id,
                    profile_id,
                    enabled,
                    ..
                } if profile_id.starts_with("effect-") => {
                    effects.push(format!("server:{profile_id}:{server_id}:{enabled}"));
                }
                _ => {}
            }
        }
        assert_eq!(
            effects.len(),
            4,
            "each committed delta must publish exactly one runtime event"
        );
        assert_eq!(
            effects,
            vec![
                "profile:effect-profile-b:true".to_string(),
                "profile:effect-profile-a:false".to_string(),
                "server:effect-profile-b:effect-server-a:false".to_string(),
                "server:effect-profile-b:effect-server-b:true".to_string(),
            ]
        );
    }
}
