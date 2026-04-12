use std::sync::Arc;

use axum::{
    extract::{Json, Query, State},
    http::StatusCode,
};

use crate::api::handlers::client::handlers::get_client_service;
use crate::api::models::client::{
    ClientBackupActionData, ClientBackupActionResp, ClientBackupListData, ClientBackupListReq, ClientBackupListResp,
    ClientBackupOperateReq, ClientBackupPolicyData, ClientBackupPolicyPayload, ClientBackupPolicyReq,
    ClientBackupPolicyResp, ClientBackupPolicySetReq,
};
use crate::api::routes::AppState;
use crate::audit::{AuditAction, AuditEvent, AuditStatus};
use crate::clients::ConfigError;
use crate::clients::models::{BackupPolicy, BackupPolicySetting};

pub async fn list_backups(
    State(app_state): State<Arc<AppState>>,
    Query(request): Query<ClientBackupListReq>,
) -> Result<Json<ClientBackupListResp>, StatusCode> {
    let service = get_client_service(&app_state)?;
    let records = service
        .list_backups(request.identifier.as_deref())
        .await
        .map_err(|err| map_storage_error(err, request.identifier.as_deref()))?;

    let backups = records
        .into_iter()
        .map(|record| crate::api::models::client::ClientBackupEntry {
            identifier: record.identifier,
            backup: record.backup,
            path: record.path,
            size: record.size,
            created_at: record.created_at.map(|dt| dt.to_rfc3339()),
        })
        .collect();

    Ok(Json(ClientBackupListResp::success(ClientBackupListData { backups })))
}

pub async fn delete_backup(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<ClientBackupOperateReq>,
) -> Result<Json<ClientBackupActionResp>, StatusCode> {
    let service = get_client_service(&app_state)?;
    service
        .delete_backup(&request.identifier, &request.backup)
        .await
        .map_err(|err| map_restore_error(err, &request.identifier, &request.backup))?;

    crate::audit::interceptor::emit_event(
        app_state.audit_service.as_ref(),
        AuditEvent::new(AuditAction::ClientBackupDelete, AuditStatus::Success)
            .with_http_route("POST", "/api/client/backups/delete")
            .with_client_id(request.identifier.clone())
            .with_target(request.backup.clone())
            .with_data(serde_json::json!({
                "identifier": request.identifier,
                "backup": request.backup,
            }))
            .build(),
    )
    .await;

    let data = ClientBackupActionData {
        identifier: request.identifier,
        backup: request.backup,
        message: "Backup removed".to_string(),
    };

    Ok(Json(ClientBackupActionResp::success(data)))
}

pub async fn get_backup_policy(
    State(app_state): State<Arc<AppState>>,
    Query(request): Query<ClientBackupPolicyReq>,
) -> Result<Json<ClientBackupPolicyResp>, StatusCode> {
    let service = get_client_service(&app_state)?;
    let policy = service
        .get_backup_policy(&request.identifier)
        .await
        .map_err(|err| map_policy_error(err, &request.identifier))?;

    Ok(Json(ClientBackupPolicyResp::success(ClientBackupPolicyData {
        identifier: request.identifier,
        policy: policy.policy.as_str().to_string(),
        limit: policy.limit,
    })))
}

pub async fn set_backup_policy(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<ClientBackupPolicySetReq>,
) -> Result<Json<ClientBackupPolicyResp>, StatusCode> {
    let service = get_client_service(&app_state)?;
    let policy = parse_policy(&request.policy)?;
    let updated = service
        .set_backup_policy(&request.identifier, policy)
        .await
        .map_err(|err| map_policy_error(err, &request.identifier))?;

    crate::audit::interceptor::emit_event(
        app_state.audit_service.as_ref(),
        AuditEvent::new(AuditAction::ClientBackupPolicyUpdate, AuditStatus::Success)
            .with_http_route("POST", "/api/client/backups/policy")
            .with_client_id(request.identifier.clone())
            .with_target(request.identifier.clone())
            .with_data(serde_json::json!({
                "policy": updated.policy.as_str(),
                "limit": updated.limit,
            }))
            .build(),
    )
    .await;

    Ok(Json(ClientBackupPolicyResp::success(ClientBackupPolicyData {
        identifier: request.identifier,
        policy: updated.policy.as_str().to_string(),
        limit: updated.limit,
    })))
}

fn parse_policy(payload: &ClientBackupPolicyPayload) -> Result<BackupPolicySetting, StatusCode> {
    let policy = payload.policy.trim().to_lowercase();
    match policy.as_str() {
        "keep_last" => Ok(BackupPolicySetting {
            policy: BackupPolicy::KeepLast,
            limit: None,
        }),
        "off" => Ok(BackupPolicySetting {
            policy: BackupPolicy::Off,
            limit: None,
        }),
        "keep_n" => {
            let limit = payload.limit.unwrap_or(5);
            if limit == 0 {
                return Err(StatusCode::BAD_REQUEST);
            }
            Ok(BackupPolicySetting {
                policy: BackupPolicy::KeepN,
                limit: Some(limit),
            })
        }
        other => {
            if let Some(number_suffix) = other.strip_prefix("keep_") {
                let parsed_limit: u32 = number_suffix.parse().map_err(|_| StatusCode::BAD_REQUEST)?;
                if parsed_limit == 0 {
                    return Err(StatusCode::BAD_REQUEST);
                }
                return Ok(BackupPolicySetting {
                    policy: BackupPolicy::KeepN,
                    limit: Some(parsed_limit),
                });
            }

            Err(StatusCode::BAD_REQUEST)
        }
    }
}

fn map_storage_error(
    err: ConfigError,
    identifier: Option<&str>,
) -> StatusCode {
    match err {
        ConfigError::TemplateIndexError(_) => {
            if let Some(id) = identifier {
                tracing::warn!("Client template {} not found while listing backups", id);
            }
            StatusCode::NOT_FOUND
        }
        other => {
            tracing::error!("Failed to list client backups: {}", other);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

fn map_restore_error(
    err: ConfigError,
    identifier: &str,
    backup: &str,
) -> StatusCode {
    match err {
        ConfigError::TemplateIndexError(_) => {
            tracing::warn!(
                "Client template {} not found while operating on backup {}",
                identifier,
                backup
            );
            StatusCode::NOT_FOUND
        }
        ConfigError::FileOperationError(_) => {
            tracing::warn!("Backup {} for client {} missing or unreadable", backup, identifier);
            StatusCode::NOT_FOUND
        }
        other => {
            tracing::error!("Backup operation failed for {} / {}: {}", identifier, backup, other);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

fn map_policy_error(
    err: ConfigError,
    identifier: &str,
) -> StatusCode {
    match err {
        ConfigError::TemplateIndexError(_) => {
            tracing::warn!("Client template {} not found while accessing policy", identifier);
            StatusCode::NOT_FOUND
        }
        other => {
            tracing::error!("Policy operation failed for {}: {}", identifier, other);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_keep_last_and_off() {
        let last = parse_policy(&ClientBackupPolicyPayload {
            policy: "keep_last".into(),
            limit: None,
        })
        .expect("keep_last");
        assert_eq!(last.policy, BackupPolicy::KeepLast);

        let off = parse_policy(&ClientBackupPolicyPayload {
            policy: "off".into(),
            limit: Some(3),
        })
        .expect("off");
        assert_eq!(off.policy, BackupPolicy::Off);
        assert!(off.limit.is_none());
    }

    #[test]
    fn parses_keep_n_variants() {
        let explicit = parse_policy(&ClientBackupPolicyPayload {
            policy: "keep_n".into(),
            limit: Some(7),
        })
        .expect("keep_n");
        assert_eq!(explicit.policy, BackupPolicy::KeepN);
        assert_eq!(explicit.limit, Some(7));

        let shorthand = parse_policy(&ClientBackupPolicyPayload {
            policy: "keep_5".into(),
            limit: None,
        })
        .expect("keep_5");
        assert_eq!(shorthand.policy, BackupPolicy::KeepN);
        assert_eq!(shorthand.limit, Some(5));
    }

    #[test]
    fn rejects_invalid_keep_alias() {
        let err = parse_policy(&ClientBackupPolicyPayload {
            policy: "keep_0".into(),
            limit: None,
        })
        .expect_err("invalid suffix");
        assert_eq!(err, StatusCode::BAD_REQUEST);

        let err = parse_policy(&ClientBackupPolicyPayload {
            policy: "keep_invalid".into(),
            limit: None,
        })
        .expect_err("invalid token");
        assert_eq!(err, StatusCode::BAD_REQUEST);
    }
}
