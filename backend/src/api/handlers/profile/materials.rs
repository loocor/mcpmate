use std::sync::Arc;

use axum::extract::Multipart;

use super::common::*;
use crate::{
    api::models::profile::{
        ProfileIdReq, WorkflowMaterialDeleteData, WorkflowMaterialDeleteReq, WorkflowMaterialDeleteResp,
        WorkflowMaterialPreviewData, WorkflowMaterialPreviewResp, WorkflowMaterialSaveData, WorkflowMaterialSaveReq,
        WorkflowMaterialSaveResp, WorkflowMaterialsReorderData, WorkflowMaterialsReorderReq,
        WorkflowMaterialsReorderResp, WorkflowMaterialsViewData, WorkflowMaterialsViewResp,
        WorkflowStepMaterialsSaveData, WorkflowStepMaterialsSaveReq, WorkflowStepMaterialsSaveResp,
    },
    core::profile::materials::{
        MAX_UPLOAD_BYTES, WorkflowMaterialSaveCommand, WorkflowMaterialsError, WorkflowMaterialsService,
    },
};

pub async fn workflow_materials_view(
    State(state): State<Arc<AppState>>,
    Query(request): Query<ProfileIdReq>,
) -> Result<Json<WorkflowMaterialsViewResp>, ApiError> {
    let materials = materials_service(&state)
        .await?
        .view(&request.id)
        .await
        .map_err(workflow_materials_error)?;
    Ok(Json(WorkflowMaterialsViewResp::success(WorkflowMaterialsViewData {
        materials,
    })))
}

pub async fn workflow_material_save(
    State(state): State<Arc<AppState>>,
    Json(request): Json<WorkflowMaterialSaveReq>,
) -> Result<Json<WorkflowMaterialSaveResp>, ApiError> {
    let profile_id = request.profile_id.clone();
    let material = materials_service(&state)
        .await?
        .save(WorkflowMaterialSaveCommand {
            profile_id: request.profile_id,
            material_id: request.material_id,
            expected_material_revision: request.expected_material_revision,
            expected_materials_revision: request.expected_materials_revision,
            title: request.title,
            kind: request.kind,
            external_url: request.external_url,
            markdown_content: request.markdown_content,
        })
        .await
        .map_err(workflow_materials_error)?;
    audit_material_change(&state, "POST", "/api/mcp/profile/workflow/materials/save", profile_id).await;
    Ok(Json(WorkflowMaterialSaveResp::success(WorkflowMaterialSaveData {
        material,
    })))
}

pub async fn workflow_material_upload(
    State(state): State<Arc<AppState>>,
    multipart: Multipart,
) -> Result<Json<WorkflowMaterialSaveResp>, ApiError> {
    let upload = parse_upload(multipart).await?;
    let profile_id = upload.profile_id.clone();
    let material = materials_service(&state)
        .await?
        .upload(
            &upload.profile_id,
            upload.title,
            upload.filename,
            upload.bytes,
            None,
            None,
            upload.expected_materials_revision,
        )
        .await
        .map_err(workflow_materials_error)?;
    audit_material_change(&state, "POST", "/api/mcp/profile/workflow/materials/upload", profile_id).await;
    Ok(Json(WorkflowMaterialSaveResp::success(WorkflowMaterialSaveData {
        material,
    })))
}

pub async fn workflow_material_replace(
    State(state): State<Arc<AppState>>,
    multipart: Multipart,
) -> Result<Json<WorkflowMaterialSaveResp>, ApiError> {
    let upload = parse_upload(multipart).await?;
    let material_id = upload
        .material_id
        .ok_or_else(|| ApiError::BadRequest("material_id is required for replace".to_string()))?;
    let expected_revision = upload
        .expected_material_revision
        .ok_or_else(|| ApiError::BadRequest("expected_material_revision is required for replace".to_string()))?;
    let profile_id = upload.profile_id.clone();
    let material = materials_service(&state)
        .await?
        .upload(
            &upload.profile_id,
            upload.title,
            upload.filename,
            upload.bytes,
            Some(&material_id),
            Some(expected_revision),
            upload.expected_materials_revision,
        )
        .await
        .map_err(workflow_materials_error)?;
    audit_material_change(
        &state,
        "POST",
        "/api/mcp/profile/workflow/materials/replace",
        profile_id,
    )
    .await;
    Ok(Json(WorkflowMaterialSaveResp::success(WorkflowMaterialSaveData {
        material,
    })))
}

pub async fn workflow_material_delete(
    State(state): State<Arc<AppState>>,
    Json(request): Json<WorkflowMaterialDeleteReq>,
) -> Result<Json<WorkflowMaterialDeleteResp>, ApiError> {
    let profile_id = request.profile_id.clone();
    materials_service(&state)
        .await?
        .delete(
            &request.profile_id,
            &request.material_id,
            request.expected_material_revision,
            request.expected_materials_revision,
        )
        .await
        .map_err(workflow_materials_error)?;
    audit_material_change(
        &state,
        "DELETE",
        "/api/mcp/profile/workflow/materials/delete",
        profile_id,
    )
    .await;
    Ok(Json(WorkflowMaterialDeleteResp::success(WorkflowMaterialDeleteData {
        material_id: request.material_id,
    })))
}

pub async fn workflow_step_materials_save(
    State(state): State<Arc<AppState>>,
    Json(request): Json<WorkflowStepMaterialsSaveReq>,
) -> Result<Json<WorkflowStepMaterialsSaveResp>, ApiError> {
    let profile_id = request.profile_id.clone();
    let material_ids = materials_service(&state)
        .await?
        .save_step_materials(request.into())
        .await
        .map_err(workflow_materials_error)?;
    audit_material_change(
        &state,
        "POST",
        "/api/mcp/profile/workflow/step-materials/save",
        profile_id,
    )
    .await;
    Ok(Json(WorkflowStepMaterialsSaveResp::success(
        WorkflowStepMaterialsSaveData { material_ids },
    )))
}

pub async fn workflow_materials_reorder(
    State(state): State<Arc<AppState>>,
    Json(request): Json<WorkflowMaterialsReorderReq>,
) -> Result<Json<WorkflowMaterialsReorderResp>, ApiError> {
    let profile_id = request.profile_id.clone();
    let material_ids = materials_service(&state)
        .await?
        .reorder(request.into())
        .await
        .map_err(workflow_materials_error)?;
    audit_material_change(
        &state,
        "POST",
        "/api/mcp/profile/workflow/materials/reorder",
        profile_id,
    )
    .await;
    Ok(Json(WorkflowMaterialsReorderResp::success(
        WorkflowMaterialsReorderData { material_ids },
    )))
}

pub async fn workflow_material_preview(
    State(state): State<Arc<AppState>>,
    Query(request): Query<MaterialPreviewReq>,
) -> Result<Json<WorkflowMaterialPreviewResp>, ApiError> {
    let content = materials_service(&state)
        .await?
        .read_preview(&request.profile_id, &request.material_id)
        .await
        .map_err(workflow_materials_error)?;
    Ok(Json(WorkflowMaterialPreviewResp::success(
        WorkflowMaterialPreviewData { content },
    )))
}

#[derive(schemars::JsonSchema, serde::Deserialize)]
pub struct MaterialPreviewReq {
    pub profile_id: String,
    pub material_id: String,
}

struct MaterialUpload {
    profile_id: String,
    material_id: Option<String>,
    expected_material_revision: Option<i64>,
    expected_materials_revision: i64,
    title: String,
    filename: String,
    bytes: Vec<u8>,
}

async fn parse_upload(mut multipart: Multipart) -> Result<MaterialUpload, ApiError> {
    let (mut profile_id, mut material_id, mut revision, mut materials_revision, mut title, mut filename, mut bytes) =
        (None, None, None, None, None, None, None);
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|error| ApiError::BadRequest(format!("invalid multipart body: {error}")))?
    {
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            "file" => {
                filename = field.file_name().map(ToOwned::to_owned);
                let data = field
                    .bytes()
                    .await
                    .map_err(|error| ApiError::BadRequest(format!("failed to read upload: {error}")))?;
                if data.len() > MAX_UPLOAD_BYTES {
                    return Err(ApiError::BadRequest("uploaded file exceeds 5 MiB".to_string()));
                }
                bytes = Some(data.to_vec());
            }
            "profile_id" => {
                profile_id = Some(
                    field
                        .text()
                        .await
                        .map_err(|error| ApiError::BadRequest(format!("invalid profile_id: {error}")))?,
                )
            }
            "material_id" => {
                material_id = Some(
                    field
                        .text()
                        .await
                        .map_err(|error| ApiError::BadRequest(format!("invalid material_id: {error}")))?,
                )
            }
            "expected_material_revision" => {
                revision = Some(
                    field
                        .text()
                        .await
                        .map_err(|error| ApiError::BadRequest(format!("invalid material revision: {error}")))?
                        .parse()
                        .map_err(|_| {
                            ApiError::BadRequest("expected_material_revision must be an integer".to_string())
                        })?,
                )
            }
            "expected_materials_revision" => {
                materials_revision = Some(
                    field
                        .text()
                        .await
                        .map_err(|error| ApiError::BadRequest(format!("invalid Materials revision: {error}")))?
                        .parse()
                        .map_err(|_| {
                            ApiError::BadRequest("expected_materials_revision must be an integer".to_string())
                        })?,
                )
            }
            "title" => {
                title = Some(
                    field
                        .text()
                        .await
                        .map_err(|error| ApiError::BadRequest(format!("invalid title: {error}")))?,
                )
            }
            _ => return Err(ApiError::BadRequest("unexpected multipart field".to_string())),
        }
    }
    Ok(MaterialUpload {
        profile_id: profile_id
            .filter(|value: &String| !value.trim().is_empty())
            .ok_or_else(|| ApiError::BadRequest("profile_id is required".to_string()))?,
        material_id,
        expected_material_revision: revision,
        expected_materials_revision: materials_revision
            .ok_or_else(|| ApiError::BadRequest("expected_materials_revision is required".to_string()))?,
        title: title
            .filter(|value: &String| !value.trim().is_empty())
            .ok_or_else(|| ApiError::BadRequest("title is required".to_string()))?,
        filename: filename.ok_or_else(|| ApiError::BadRequest("file is required".to_string()))?,
        bytes: bytes.ok_or_else(|| ApiError::BadRequest("file is required".to_string()))?,
    })
}

async fn materials_service(state: &Arc<AppState>) -> Result<WorkflowMaterialsService, ApiError> {
    let db = get_database(state).await?;
    Ok(WorkflowMaterialsService::new(
        db.pool.clone(),
        db.path.parent().unwrap_or(std::path::Path::new(".")).join("skills"),
    ))
}

async fn audit_material_change(
    state: &Arc<AppState>,
    method: &str,
    route: &str,
    profile_id: String,
) {
    crate::audit::interceptor::emit_event(
        state.audit_service.as_ref(),
        crate::audit::interceptor::build_rest_event(
            crate::audit::AuditAction::ProfileUpdate,
            crate::audit::AuditStatus::Success,
            method,
            route,
            None,
            None,
            Some(profile_id),
            None,
            None,
        ),
    )
    .await;
}

pub(super) fn workflow_materials_error(error: WorkflowMaterialsError) -> ApiError {
    match error {
        WorkflowMaterialsError::Workflow(error) => super::workflow::workflow_specification_error(error),
        WorkflowMaterialsError::InvalidRequest(message) => ApiError::BadRequest(message),
        WorkflowMaterialsError::MaterialChanged {
            current_material_revision,
        } => ApiError::Conflict(format!(
            "Workflow Material was changed by another author (current revision {current_material_revision})"
        )),
        WorkflowMaterialsError::MaterialsChanged {
            current_materials_revision,
        } => ApiError::Conflict(format!(
            "Workflow Materials library was changed by another author (current revision {current_materials_revision})"
        )),
        WorkflowMaterialsError::MaterialNotFound => ApiError::NotFound("Workflow Material was not found".to_string()),
        WorkflowMaterialsError::Database(_)
        | WorkflowMaterialsError::File(_)
        | WorkflowMaterialsError::SkillDirectoryRecovery { .. } => {
            ApiError::InternalError("Workflow Material operation failed".to_string())
        }
    }
}
