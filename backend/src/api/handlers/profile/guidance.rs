// Profile-scoped Skills-style guidance handlers.

use super::{common::*, helpers::get_profile_or_error};
use crate::api::models::profile::{
    ProfileGuidanceData, ProfileGuidanceDeleteReq, ProfileGuidanceListData, ProfileGuidanceListReq,
    ProfileGuidanceListResp, ProfileGuidanceResp, ProfileGuidanceRespData, ProfileGuidanceUpsertReq, ProfileManageData,
    ProfileManageResp, ProfileOperationResult,
};
use chrono::Utc;

fn guidance_to_response(record: crate::config::models::ProfileGuidance) -> ProfileGuidanceData {
    ProfileGuidanceData {
        id: record.id,
        profile_id: record.profile_id,
        slug: record.slug,
        title: record.title,
        summary: record.summary,
        scenario: record.scenario,
        activation: record.activation,
        capability_refs: record.capability_refs,
        validation_notes: record.validation_notes,
        avoid: record.avoid,
        content_markdown: record.content_markdown,
        source_uri: record.source_uri,
        enabled: record.enabled,
    }
}

fn validate_guidance_text(
    field: &str,
    value: &str,
) -> Result<(), ApiError> {
    if value.trim().is_empty() {
        return Err(ApiError::BadRequest(format!("{field} must not be empty")));
    }
    Ok(())
}

fn validate_guidance_slug(slug: &str) -> Result<(), ApiError> {
    validate_guidance_text("slug", slug)?;
    let valid = slug
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_');
    if !valid {
        return Err(ApiError::BadRequest(
            "slug may only contain ASCII letters, numbers, hyphen, or underscore".to_string(),
        ));
    }
    Ok(())
}

/// List Skills-style guidance for a profile.
pub async fn guidance_list(
    State(state): State<Arc<AppState>>,
    Query(request): Query<ProfileGuidanceListReq>,
) -> Result<Json<ProfileGuidanceListResp>, ApiError> {
    let db = get_database(&state).await?;
    get_profile_or_error(&db, &request.profile_id).await?;

    let records = if request.enabled_only == Some(true) {
        crate::config::profile::list_enabled_profile_guidance(&db.pool, &request.profile_id).await
    } else {
        crate::config::profile::list_profile_guidance(&db.pool, &request.profile_id).await
    }
    .map_err(|e| ApiError::InternalError(format!("Failed to list profile guidance: {e}")))?;

    let guidance = records.into_iter().map(guidance_to_response).collect::<Vec<_>>();
    let response = ProfileGuidanceListData {
        profile_id: request.profile_id,
        total: guidance.len(),
        guidance,
    };

    Ok(Json(ProfileGuidanceListResp::success(response)))
}

/// Create or update one Skills-style guidance record for a profile.
pub async fn guidance_upsert(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ProfileGuidanceUpsertReq>,
) -> Result<Json<ProfileGuidanceResp>, ApiError> {
    let db = get_database(&state).await?;
    get_profile_or_error(&db, &request.profile_id).await?;

    validate_guidance_slug(&request.slug)?;
    validate_guidance_text("title", &request.title)?;
    validate_guidance_text("content_markdown", &request.content_markdown)?;

    let slug = request.slug;
    let draft = crate::config::profile::ProfileGuidanceDraft {
        id: request.id,
        profile_id: request.profile_id.clone(),
        slug: slug.clone(),
        title: request.title,
        summary: request.summary,
        scenario: request.scenario,
        activation: request.activation,
        capability_refs: request.capability_refs,
        validation_notes: request.validation_notes,
        avoid: request.avoid,
        content_markdown: request.content_markdown,
        source_uri: request.source_uri,
        enabled: request.enabled,
    };

    crate::config::profile::upsert_profile_guidance(&db.pool, draft)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to save profile guidance: {e}")))?;

    let record = crate::config::profile::get_profile_guidance(&db.pool, &request.profile_id, &slug)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to load saved profile guidance: {e}")))?
        .ok_or_else(|| ApiError::InternalError("Saved profile guidance was not found".to_string()))?;

    Ok(Json(ProfileGuidanceResp::success(ProfileGuidanceRespData {
        guidance: guidance_to_response(record),
    })))
}

/// Delete one Skills-style guidance record from a profile.
pub async fn guidance_delete(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ProfileGuidanceDeleteReq>,
) -> Result<Json<ProfileManageResp>, ApiError> {
    let db = get_database(&state).await?;
    let profile = get_profile_or_error(&db, &request.profile_id).await?;
    validate_guidance_slug(&request.slug)?;

    let deleted = crate::config::profile::delete_profile_guidance(&db.pool, &request.profile_id, &request.slug)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to delete profile guidance: {e}")))?;
    if !deleted {
        return Err(ApiError::NotFound(format!(
            "Profile guidance '{}' not found for profile '{}'",
            request.slug, request.profile_id
        )));
    }

    let response = ProfileManageData {
        success_count: 1,
        failed_count: 0,
        results: vec![ProfileOperationResult {
            id: request.slug,
            name: profile.name,
            result: "deleted".to_string(),
            status: "inactive".to_string(),
            error: None,
        }],
        timestamp: Utc::now().to_rfc3339(),
    };

    Ok(Json(ProfileManageResp::success(response)))
}
