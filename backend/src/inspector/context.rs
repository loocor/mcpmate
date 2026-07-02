use crate::api::routes::AppState;
use crate::config::database::Database;
use crate::inspector::calls::InspectorCallRegistry;
use crate::inspector::runtime::{InspectorRuntimeAdapter, InspectorRuntimeEnvironment};
use crate::inspector::sessions::InspectorSessionManager;
use crate::inspector::workspace::InspectorWorkspace;

pub struct InspectorServiceContext<'a> {
    runtime_adapter: InspectorRuntimeAdapter<'a>,
    sessions: &'a InspectorSessionManager,
    calls: &'a InspectorCallRegistry,
}

impl<'a> InspectorServiceContext<'a> {
    pub fn from_app_state(state: &'a AppState) -> Self {
        Self {
            runtime_adapter: InspectorRuntimeAdapter {
                database: state.database.as_deref(),
                proxy_surface_available: state.http_proxy.is_some(),
                inspector_workspace: &state.inspector_workspace,
                secret_store: &state.secret_store,
            },
            sessions: &state.inspector_sessions,
            calls: &state.inspector_calls,
        }
    }

    pub fn database(&self) -> Option<&Database> {
        self.runtime_adapter.database
    }

    pub fn runtime_environment(&self) -> InspectorRuntimeEnvironment<'_> {
        InspectorRuntimeEnvironment::new(self.runtime_adapter)
    }

    pub fn sessions(&self) -> &InspectorSessionManager {
        self.sessions
    }

    pub fn workspace(&self) -> &InspectorWorkspace {
        self.runtime_adapter.inspector_workspace
    }

    pub fn calls(&self) -> &InspectorCallRegistry {
        self.calls
    }
}
