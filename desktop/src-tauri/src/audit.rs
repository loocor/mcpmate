use anyhow::Result;
use mcpmate::{
    audit::{AuditAction, AuditEvent, AuditStatus, AuditStore},
    config::audit_database::AuditDatabase,
};
use serde_json::Value;

pub async fn emit_desktop_audit_event(
    action: AuditAction,
    status: AuditStatus,
    target: Option<String>,
    detail: Option<String>,
    data: Option<Value>,
    error_message: Option<String>,
) -> Result<()> {
    let database = AuditDatabase::new().await?;
    let store = AuditStore::from_database(&database);
    store.initialize().await?;

    let mut event = AuditEvent::new(action, status).with_actor("desktop");

    if let Some(target) = target {
        event = event.with_target(target);
    }
    if let Some(detail) = detail {
        event = event.with_detail(detail);
    }
    if let Some(data) = data {
        event = event.with_data(data);
    }
    if let Some(error_message) = error_message {
        event = event.with_error(None::<String>, error_message);
    }

    store.insert(&event.build()).await?;
    database.close().await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn writes_desktop_audit_event() {
        let dir = std::env::temp_dir().join(format!(
            "mcpmate-desktop-audit-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        unsafe {
            std::env::set_var("MCPMATE_DATA_DIR", &dir);
        }

        emit_desktop_audit_event(
            AuditAction::CoreSourceApply,
            AuditStatus::Success,
            Some("localhost".to_string()),
            Some("Applied desktop core source configuration".to_string()),
            Some(serde_json::json!({ "selected_source": "localhost" })),
            None,
        )
        .await
        .expect("emit event");

        let database = AuditDatabase::new().await.expect("audit db");
        let store = AuditStore::from_database(&database);
        let page = store
            .list(&mcpmate::audit::AuditFilter::default(), None, Some(10))
            .await
            .expect("list events");

        assert_eq!(page.events.len(), 1);
        assert_eq!(page.events[0].action, AuditAction::CoreSourceApply);
        assert_eq!(page.events[0].actor.as_deref(), Some("desktop"));
        assert_eq!(page.events[0].target.as_deref(), Some("localhost"));

        database.close().await;
        let _ = std::fs::remove_dir_all(&dir);
    }
}
