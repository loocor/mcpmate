use super::common::*;
use crate::{
    api::models::profile::{
        ProfileIdReq, WorkflowSpecificationDeleteData, WorkflowSpecificationDeleteReq, WorkflowSpecificationDeleteResp,
        WorkflowSpecificationPreviewData, WorkflowSpecificationPreviewResp, WorkflowSpecificationSaveData,
        WorkflowSpecificationSaveReq, WorkflowSpecificationSaveResp, WorkflowSpecificationViewData,
        WorkflowSpecificationViewResp,
    },
    core::profile::workflow::{
        WorkflowSpecificationError, WorkflowSpecificationSaveCommand, WorkflowSpecificationService,
    },
};

pub async fn workflow_specification_view(
    State(state): State<Arc<AppState>>,
    Query(request): Query<ProfileIdReq>,
) -> Result<Json<WorkflowSpecificationViewResp>, ApiError> {
    let db = get_database(&state).await?;
    let specification = WorkflowSpecificationService::new(db.pool.clone())
        .view(&request.id)
        .await
        .map_err(workflow_specification_error)?;
    Ok(Json(WorkflowSpecificationViewResp::success(
        WorkflowSpecificationViewData { specification },
    )))
}

pub async fn workflow_specification_save(
    State(state): State<Arc<AppState>>,
    Json(request): Json<WorkflowSpecificationSaveReq>,
) -> Result<Json<WorkflowSpecificationSaveResp>, ApiError> {
    let started_at = std::time::Instant::now();
    let profile_id = request.profile_id.clone();
    let db = get_database(&state).await?;
    let specification = WorkflowSpecificationService::new(db.pool.clone())
        .save(WorkflowSpecificationSaveCommand {
            profile_id: request.profile_id,
            expected_specification_revision: request.expected_specification_revision,
            validation_notes: request.validation_notes,
            avoid_rules: request.avoid_rules,
            steps: request.steps,
        })
        .await
        .map_err(workflow_specification_error)?;
    crate::audit::interceptor::emit_event(
        state.audit_service.as_ref(),
        crate::audit::interceptor::build_rest_event(
            crate::audit::AuditAction::ProfileUpdate,
            crate::audit::AuditStatus::Success,
            "POST",
            "/api/mcp/profile/workflow/specification/save",
            Some(started_at.elapsed().as_millis() as u64),
            None,
            Some(profile_id),
            None,
            None,
        ),
    )
    .await;
    Ok(Json(WorkflowSpecificationSaveResp::success(
        WorkflowSpecificationSaveData { specification },
    )))
}

pub async fn workflow_specification_delete(
    State(state): State<Arc<AppState>>,
    Json(request): Json<WorkflowSpecificationDeleteReq>,
) -> Result<Json<WorkflowSpecificationDeleteResp>, ApiError> {
    let started_at = std::time::Instant::now();
    let profile_id = request.profile_id.clone();
    let db = get_database(&state).await?;
    WorkflowSpecificationService::new(db.pool.clone())
        .delete(&request.profile_id, request.expected_specification_revision)
        .await
        .map_err(workflow_specification_error)?;
    crate::audit::interceptor::emit_event(
        state.audit_service.as_ref(),
        crate::audit::interceptor::build_rest_event(
            crate::audit::AuditAction::ProfileUpdate,
            crate::audit::AuditStatus::Success,
            "DELETE",
            "/api/mcp/profile/workflow/specification/delete",
            Some(started_at.elapsed().as_millis() as u64),
            None,
            Some(profile_id.clone()),
            None,
            None,
        ),
    )
    .await;
    Ok(Json(WorkflowSpecificationDeleteResp::success(
        WorkflowSpecificationDeleteData { profile_id },
    )))
}

pub async fn workflow_specification_preview(
    State(state): State<Arc<AppState>>,
    Query(request): Query<ProfileIdReq>,
) -> Result<Json<WorkflowSpecificationPreviewResp>, ApiError> {
    let db = get_database(&state).await?;
    let preview = WorkflowSpecificationService::new(db.pool.clone())
        .preview(&request.id)
        .await
        .map_err(workflow_specification_error)?;
    Ok(Json(WorkflowSpecificationPreviewResp::success(
        WorkflowSpecificationPreviewData { preview },
    )))
}

pub(super) fn workflow_specification_error(error: WorkflowSpecificationError) -> ApiError {
    match error {
        WorkflowSpecificationError::NotFound { profile_id } => {
            ApiError::NotFound(format!("Workflow Profile with ID '{profile_id}' not found"))
        }
        WorkflowSpecificationError::InvalidProfileMode { profile_id } => {
            ApiError::BadRequest(format!("Profile '{profile_id}' is not a workflow Profile"))
        }
        WorkflowSpecificationError::InvalidRequest(message) => ApiError::BadRequest(message),
        WorkflowSpecificationError::InvalidBinding { ref_id } => {
            ApiError::BadRequest(format!("Workflow capability binding '{ref_id}' is unavailable"))
        }
        WorkflowSpecificationError::SpecificationChanged {
            current_specification_revision,
        } => ApiError::Conflict(format!(
            "Workflow specification was changed by another author (current revision {current_specification_revision})"
        )),
        WorkflowSpecificationError::Database(_) | WorkflowSpecificationError::Capability(_) => {
            ApiError::InternalError("Workflow specification failed".to_string())
        }
    }
}
