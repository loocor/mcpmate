use std::fmt;

use serde_json::{Map, Value};

use crate::inspector::contract::{InspectorMode, InspectorProxyMode, InspectorProxyScope};
use crate::inspector::runtime::ProxyRuntimeSurface;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InspectorTarget {
    Native(InspectorNativeTarget),
    Proxy(InspectorProxyTarget),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InspectorNativeTarget {
    Managed { server_id: String },
    Scratch { record_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InspectorProxyTarget {
    HostedIsolated { server_ids: Vec<String> },
    HostedActiveCatalog { requested_server_ids: Option<Vec<String>> },
    UnifyIsolated { server_ids: Vec<String> },
    UnifyActiveCatalog { requested_server_ids: Option<Vec<String>> },
}

#[derive(Debug, Clone)]
pub struct InspectorTargetRequest {
    pub mode: InspectorMode,
    pub server_id: Option<String>,
    pub server_name: Option<String>,
    pub scratch_id: Option<String>,
    pub proxy_mode: Option<InspectorProxyMode>,
    pub proxy_scope: Option<InspectorProxyScope>,
}

#[derive(Debug, Clone)]
pub struct InspectorCapabilityListRequest {
    pub target: InspectorTargetRequest,
    pub session_id: Option<String>,
    pub refresh: bool,
}

#[derive(Debug, Clone)]
pub struct InspectorSnapshotRequest {
    pub target: InspectorTargetRequest,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct InspectorToolCallRequest {
    pub target: InspectorTargetRequest,
    pub session_id: Option<String>,
    pub tool: String,
    pub arguments: Option<Map<String, Value>>,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct InspectorPromptGetRequest {
    pub target: InspectorTargetRequest,
    pub session_id: Option<String>,
    pub name: String,
    pub arguments: Option<Map<String, Value>>,
}

#[derive(Debug, Clone)]
pub struct InspectorResourceReadRequest {
    pub target: InspectorTargetRequest,
    pub session_id: Option<String>,
    pub uri: String,
}

#[derive(Debug, Clone)]
pub struct InspectorCapabilityPatchRequest {
    pub target: InspectorTargetRequest,
    pub capability_kind: String,
    pub capability_key: String,
    pub patch: Map<String, Value>,
}

#[derive(Debug, Clone)]
pub struct InspectorLlmEvaluationRequest {
    pub target: InspectorTargetRequest,
    pub session_id: Option<String>,
    pub provider_id: Option<String>,
    pub scenario: String,
    pub max_tools: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InspectorServerReference {
    Id(String),
    Name(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InspectorTargetError {
    ExpectedNativeMode,
    ModeMismatch,
    MissingIsolatedServer,
    MissingServerReference,
    SessionOptionsChanged,
    SessionTargetMismatch,
    ScratchOnlyNativeMode,
    ScratchWithManagedReference,
}

impl fmt::Display for InspectorTargetError {
    fn fmt(
        &self,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            InspectorTargetError::ExpectedNativeMode => formatter.write_str("Expected native Inspector mode"),
            InspectorTargetError::ModeMismatch => formatter.write_str("Inspector session is bound to a different mode"),
            InspectorTargetError::MissingIsolatedServer => {
                formatter.write_str("proxy_scope=isolated requires server_id or server_name")
            }
            InspectorTargetError::MissingServerReference => formatter.write_str("server_id or server_name is required"),
            InspectorTargetError::SessionOptionsChanged => {
                formatter.write_str("proxy_mode and proxy_scope cannot be changed for an existing Inspector session")
            }
            InspectorTargetError::SessionTargetMismatch => {
                formatter.write_str("Inspector session is bound to a different server")
            }
            InspectorTargetError::ScratchOnlyNativeMode => {
                formatter.write_str("scratch_id is only supported in native Inspector mode")
            }
            InspectorTargetError::ScratchWithManagedReference => {
                formatter.write_str("scratch_id cannot be combined with server_id or server_name")
            }
        }
    }
}

impl InspectorTarget {
    pub fn native(server_id: String) -> Self {
        Self::Native(InspectorNativeTarget::managed(server_id))
    }

    pub fn native_scratch(record_id: String) -> Self {
        Self::Native(InspectorNativeTarget::scratch(record_id))
    }

    pub fn proxy(proxy_target: InspectorProxyTarget) -> Self {
        Self::Proxy(proxy_target)
    }

    pub fn mode(&self) -> InspectorMode {
        match self {
            Self::Native(_) => InspectorMode::Native,
            Self::Proxy(_) => InspectorMode::Proxy,
        }
    }

    pub fn server_id(&self) -> Option<&str> {
        match self {
            Self::Native(native_target) => native_target.server_id(),
            Self::Proxy(proxy_target) => proxy_target.server_id(),
        }
    }

    pub fn scratch_id(&self) -> Option<&str> {
        match self {
            Self::Native(native_target) => native_target.scratch_id(),
            Self::Proxy(_) => None,
        }
    }

    pub fn native_reference_id(&self) -> Option<&str> {
        match self {
            Self::Native(native_target) => Some(native_target.reference_id()),
            Self::Proxy(_) => None,
        }
    }

    pub fn proxy_mode(&self) -> Option<InspectorProxyMode> {
        match self {
            Self::Native(_) => None,
            Self::Proxy(proxy_target) => Some(proxy_target.proxy_mode()),
        }
    }

    pub fn proxy_scope(&self) -> Option<InspectorProxyScope> {
        match self {
            Self::Native(_) => None,
            Self::Proxy(proxy_target) => Some(proxy_target.proxy_scope()),
        }
    }

    pub fn as_native_server_id(&self) -> Option<&str> {
        match self {
            Self::Native(native_target) => native_target.server_id(),
            Self::Proxy(_) => None,
        }
    }

    pub fn as_native(&self) -> Option<&InspectorNativeTarget> {
        match self {
            Self::Native(native_target) => Some(native_target),
            Self::Proxy(_) => None,
        }
    }

    pub fn as_proxy(&self) -> Option<&InspectorProxyTarget> {
        match self {
            Self::Native(_) => None,
            Self::Proxy(proxy_target) => Some(proxy_target),
        }
    }

    pub fn binding_id_for_mode(
        &self,
        mode: InspectorMode,
    ) -> Option<&str> {
        match mode {
            InspectorMode::Native => self.native_reference_id(),
            InspectorMode::Proxy => self.server_id(),
        }
    }

    pub fn ensure_session_binding(
        &self,
        mode: InspectorMode,
        target_id: &str,
    ) -> Result<(), InspectorTargetError> {
        self.ensure_mode(mode)?;

        if self
            .binding_id_for_mode(mode)
            .is_some_and(|session_id| session_id != target_id)
        {
            return Err(InspectorTargetError::SessionTargetMismatch);
        }

        Ok(())
    }

    pub fn ensure_mode(
        &self,
        mode: InspectorMode,
    ) -> Result<(), InspectorTargetError> {
        if self.mode() == mode {
            Ok(())
        } else {
            Err(InspectorTargetError::ModeMismatch)
        }
    }
}

impl InspectorNativeTarget {
    pub fn managed(server_id: String) -> Self {
        Self::Managed { server_id }
    }

    pub fn scratch(record_id: String) -> Self {
        Self::Scratch { record_id }
    }

    pub fn reference_id(&self) -> &str {
        match self {
            Self::Managed { server_id } => server_id,
            Self::Scratch { record_id } => record_id,
        }
    }

    pub fn server_id(&self) -> Option<&str> {
        match self {
            Self::Managed { server_id } => Some(server_id),
            Self::Scratch { .. } => None,
        }
    }

    pub fn scratch_id(&self) -> Option<&str> {
        match self {
            Self::Managed { .. } => None,
            Self::Scratch { record_id } => Some(record_id),
        }
    }
}

impl InspectorProxyTarget {
    pub fn from_tool_call_reference(
        proxy_mode: Option<InspectorProxyMode>,
        proxy_scope: Option<InspectorProxyScope>,
        server_id: String,
    ) -> Result<Self, InspectorTargetError> {
        let target_server_ids = if matches!(proxy_scope, Some(InspectorProxyScope::ActiveCatalog)) {
            None
        } else {
            Some(vec![server_id])
        };
        Self::from_parts(proxy_mode, proxy_scope, target_server_ids)
    }

    pub fn from_parts(
        proxy_mode: Option<InspectorProxyMode>,
        proxy_scope: Option<InspectorProxyScope>,
        target_server_ids: Option<Vec<String>>,
    ) -> Result<Self, InspectorTargetError> {
        let proxy_mode = proxy_mode.unwrap_or_default();
        let proxy_scope = proxy_scope.unwrap_or_else(|| match proxy_mode {
            InspectorProxyMode::Hosted => {
                if target_server_ids
                    .as_ref()
                    .is_some_and(|server_ids| !server_ids.is_empty())
                {
                    InspectorProxyScope::Isolated
                } else {
                    InspectorProxyScope::ActiveCatalog
                }
            }
            InspectorProxyMode::Unify => InspectorProxyScope::ActiveCatalog,
        });

        let target_server_ids = target_server_ids.filter(|server_ids| !server_ids.is_empty());

        Ok(match (proxy_mode, proxy_scope, target_server_ids) {
            (InspectorProxyMode::Hosted, InspectorProxyScope::Isolated, Some(server_ids)) => {
                Self::HostedIsolated { server_ids }
            }
            (InspectorProxyMode::Hosted, InspectorProxyScope::ActiveCatalog, target_server_ids) => {
                Self::HostedActiveCatalog {
                    requested_server_ids: target_server_ids,
                }
            }
            (InspectorProxyMode::Unify, InspectorProxyScope::Isolated, Some(server_ids)) => {
                Self::UnifyIsolated { server_ids }
            }
            (InspectorProxyMode::Unify, InspectorProxyScope::ActiveCatalog, target_server_ids) => {
                Self::UnifyActiveCatalog {
                    requested_server_ids: target_server_ids,
                }
            }
            (_, InspectorProxyScope::Isolated, None) => return Err(InspectorTargetError::MissingIsolatedServer),
        })
    }

    pub fn proxy_mode(&self) -> InspectorProxyMode {
        match self {
            Self::HostedIsolated { .. } | Self::HostedActiveCatalog { .. } => InspectorProxyMode::Hosted,
            Self::UnifyIsolated { .. } | Self::UnifyActiveCatalog { .. } => InspectorProxyMode::Unify,
        }
    }

    pub fn proxy_scope(&self) -> InspectorProxyScope {
        match self {
            Self::HostedIsolated { .. } | Self::UnifyIsolated { .. } => InspectorProxyScope::Isolated,
            Self::HostedActiveCatalog { .. } | Self::UnifyActiveCatalog { .. } => InspectorProxyScope::ActiveCatalog,
        }
    }

    pub fn target_server_ids(&self) -> Option<&[String]> {
        match self {
            Self::HostedIsolated { server_ids } | Self::UnifyIsolated { server_ids } => Some(server_ids),
            Self::HostedActiveCatalog { requested_server_ids } | Self::UnifyActiveCatalog { requested_server_ids } => {
                requested_server_ids.as_deref()
            }
        }
    }

    pub fn runtime_server_ids(&self) -> Option<&[String]> {
        match self {
            Self::HostedIsolated { server_ids } | Self::UnifyIsolated { server_ids } => Some(server_ids),
            Self::HostedActiveCatalog { .. } | Self::UnifyActiveCatalog { .. } => None,
        }
    }

    pub fn server_id(&self) -> Option<&str> {
        self.target_server_ids()
            .and_then(|server_ids| server_ids.first())
            .map(String::as_str)
    }

    pub fn to_surface(&self) -> ProxyRuntimeSurface {
        ProxyRuntimeSurface {
            proxy_mode: self.proxy_mode(),
            proxy_scope: self.proxy_scope(),
            target_server_ids: self.runtime_server_ids().map(<[_]>::to_vec),
        }
    }
}

impl InspectorTargetRequest {
    pub fn ensure_proxy_mode(&self) -> Result<(), InspectorTargetError> {
        if self.scratch_id.is_some() {
            return Err(InspectorTargetError::ScratchOnlyNativeMode);
        }
        Ok(())
    }

    pub fn ensure_session_options_unchanged(
        &self,
        has_session: bool,
    ) -> Result<(), InspectorTargetError> {
        if has_session && (self.proxy_mode.is_some() || self.proxy_scope.is_some()) {
            Err(InspectorTargetError::SessionOptionsChanged)
        } else {
            Ok(())
        }
    }

    pub fn server_reference(&self) -> Result<Option<InspectorServerReference>, InspectorTargetError> {
        if self.scratch_id.is_some() {
            if matches!(self.mode, InspectorMode::Proxy) {
                return Err(InspectorTargetError::ScratchOnlyNativeMode);
            }
            if self.server_id.is_some() || self.server_name.is_some() {
                return Err(InspectorTargetError::ScratchWithManagedReference);
            }
            return Ok(None);
        }

        Ok(Self::managed_server_reference(&self.server_id, &self.server_name))
    }

    pub fn managed_server_reference(
        server_id: &Option<String>,
        server_name: &Option<String>,
    ) -> Option<InspectorServerReference> {
        if let Some(server_id) = server_id.clone() {
            return Some(InspectorServerReference::Id(server_id));
        }

        server_name.clone().map(InspectorServerReference::Name)
    }

    pub fn into_target(
        self,
        resolved_server_id: Option<String>,
    ) -> Result<InspectorTarget, InspectorTargetError> {
        match self.mode {
            InspectorMode::Native => self.into_native_target(resolved_server_id).map(InspectorTarget::Native),
            InspectorMode::Proxy => {
                if self.scratch_id.is_some() {
                    return Err(InspectorTargetError::ScratchOnlyNativeMode);
                }
                let target_server_ids = resolved_server_id.map(|server_id| vec![server_id]);
                InspectorProxyTarget::from_parts(self.proxy_mode, self.proxy_scope, target_server_ids)
                    .map(InspectorTarget::proxy)
            }
        }
    }

    pub fn into_native_target(
        self,
        resolved_server_id: Option<String>,
    ) -> Result<InspectorNativeTarget, InspectorTargetError> {
        if !matches!(self.mode, InspectorMode::Native) {
            return Err(InspectorTargetError::ExpectedNativeMode);
        }

        if let Some(record_id) = self.scratch_id {
            if self.server_id.is_some() || self.server_name.is_some() {
                return Err(InspectorTargetError::ScratchWithManagedReference);
            }
            return Ok(InspectorNativeTarget::scratch(record_id));
        }

        resolved_server_id
            .map(InspectorNativeTarget::managed)
            .ok_or(InspectorTargetError::MissingServerReference)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosted_proxy_with_target_defaults_to_isolated_scope() {
        let target = InspectorProxyTarget::from_parts(
            Some(InspectorProxyMode::Hosted),
            None,
            Some(vec!["server-a".to_string()]),
        )
        .expect("hosted target");

        assert_eq!(target.proxy_mode(), InspectorProxyMode::Hosted);
        assert_eq!(target.proxy_scope(), InspectorProxyScope::Isolated);
        assert_eq!(target.server_id(), Some("server-a"));
    }

    #[test]
    fn unify_proxy_with_target_defaults_to_active_catalog_scope() {
        let target = InspectorProxyTarget::from_parts(
            Some(InspectorProxyMode::Unify),
            None,
            Some(vec!["server-a".to_string()]),
        )
        .expect("unify target");

        assert_eq!(target.proxy_mode(), InspectorProxyMode::Unify);
        assert_eq!(target.proxy_scope(), InspectorProxyScope::ActiveCatalog);
        assert_eq!(target.server_id(), Some("server-a"));
    }

    #[test]
    fn active_catalog_proxy_keeps_request_context_out_of_runtime_filter() {
        let target = InspectorProxyTarget::from_parts(
            Some(InspectorProxyMode::Unify),
            Some(InspectorProxyScope::ActiveCatalog),
            Some(vec!["server-a".to_string()]),
        )
        .expect("active catalog target");

        assert_eq!(target.server_id(), Some("server-a"));
        assert_eq!(target.runtime_server_ids(), None);

        let surface = target.to_surface();
        assert_eq!(surface.proxy_mode, InspectorProxyMode::Unify);
        assert_eq!(surface.proxy_scope, InspectorProxyScope::ActiveCatalog);
        assert_eq!(surface.target_server_ids, None);
    }

    #[test]
    fn tool_call_reference_keeps_explicit_active_catalog_unfiltered() {
        let target = InspectorProxyTarget::from_tool_call_reference(
            Some(InspectorProxyMode::Unify),
            Some(InspectorProxyScope::ActiveCatalog),
            "server-a".to_string(),
        )
        .expect("tool call target");

        assert_eq!(target.proxy_mode(), InspectorProxyMode::Unify);
        assert_eq!(target.proxy_scope(), InspectorProxyScope::ActiveCatalog);
        assert_eq!(target.server_id(), None);
        assert_eq!(target.to_surface().target_server_ids, None);
    }

    #[test]
    fn isolated_proxy_requires_a_target_server() {
        let error = InspectorProxyTarget::from_parts(None, Some(InspectorProxyScope::Isolated), None)
            .expect_err("missing isolated target should fail");

        assert_eq!(error, InspectorTargetError::MissingIsolatedServer);
    }

    #[test]
    fn scratch_native_target_exposes_scratch_reference_only() {
        let target = InspectorTarget::native_scratch("inspector-record".to_string());

        assert_eq!(target.mode(), InspectorMode::Native);
        assert_eq!(target.server_id(), None);
        assert_eq!(target.scratch_id(), Some("inspector-record"));
        assert_eq!(target.native_reference_id(), Some("inspector-record"));
    }

    #[test]
    fn target_request_rejects_scratch_with_managed_reference_before_resolution() {
        let request = InspectorTargetRequest {
            mode: InspectorMode::Native,
            server_id: Some("server-a".to_string()),
            server_name: None,
            scratch_id: Some("scratch-a".to_string()),
            proxy_mode: None,
            proxy_scope: None,
        };

        assert_eq!(
            request.server_reference(),
            Err(InspectorTargetError::ScratchWithManagedReference)
        );
    }

    #[test]
    fn target_request_rejects_scratch_in_proxy_mode() {
        let request = InspectorTargetRequest {
            mode: InspectorMode::Proxy,
            server_id: None,
            server_name: None,
            scratch_id: Some("scratch-a".to_string()),
            proxy_mode: Some(InspectorProxyMode::Hosted),
            proxy_scope: None,
        };

        assert_eq!(
            request.server_reference(),
            Err(InspectorTargetError::ScratchOnlyNativeMode)
        );
        assert_eq!(
            request.ensure_proxy_mode(),
            Err(InspectorTargetError::ScratchOnlyNativeMode)
        );
    }

    #[test]
    fn target_request_builds_scratch_native_target() {
        let request = InspectorTargetRequest {
            mode: InspectorMode::Native,
            server_id: None,
            server_name: None,
            scratch_id: Some("scratch-a".to_string()),
            proxy_mode: None,
            proxy_scope: None,
        };

        let target = request.into_target(None).expect("scratch target");

        assert_eq!(target.mode(), InspectorMode::Native);
        assert_eq!(target.server_id(), None);
        assert_eq!(target.scratch_id(), Some("scratch-a"));
        assert_eq!(target.native_reference_id(), Some("scratch-a"));
    }

    #[test]
    fn managed_server_reference_prefers_id_over_name() {
        let server_id = Some("server-a".to_string());
        let server_name = Some("Fetch".to_string());

        assert_eq!(
            InspectorTargetRequest::managed_server_reference(&server_id, &server_name),
            Some(InspectorServerReference::Id("server-a".to_string()))
        );
    }

    #[test]
    fn target_request_builds_active_catalog_proxy_without_server() {
        let request = InspectorTargetRequest {
            mode: InspectorMode::Proxy,
            server_id: None,
            server_name: None,
            scratch_id: None,
            proxy_mode: Some(InspectorProxyMode::Hosted),
            proxy_scope: None,
        };

        let target = request.into_target(None).expect("active catalog target");
        let proxy_target = target.as_proxy().expect("proxy target");
        assert_eq!(proxy_target.proxy_scope(), InspectorProxyScope::ActiveCatalog);
        assert_eq!(proxy_target.target_server_ids(), None);
    }

    #[test]
    fn target_request_rejects_proxy_options_for_existing_session() {
        let request = InspectorTargetRequest {
            mode: InspectorMode::Proxy,
            server_id: Some("server-a".to_string()),
            server_name: None,
            scratch_id: None,
            proxy_mode: Some(InspectorProxyMode::Hosted),
            proxy_scope: None,
        };

        assert_eq!(
            request.ensure_session_options_unchanged(true),
            Err(InspectorTargetError::SessionOptionsChanged)
        );
        assert_eq!(request.ensure_session_options_unchanged(false), Ok(()));
    }

    #[test]
    fn session_binding_matches_native_scratch_reference() {
        let target = InspectorTarget::native_scratch("scratch-a".to_string());

        assert_eq!(
            target.ensure_session_binding(InspectorMode::Native, "scratch-a"),
            Ok(())
        );
        assert_eq!(
            target.ensure_session_binding(InspectorMode::Native, "scratch-b"),
            Err(InspectorTargetError::SessionTargetMismatch)
        );
    }

    #[test]
    fn session_binding_rejects_mode_mismatch() {
        let target = InspectorTarget::native("server-a".to_string());

        assert_eq!(
            target.ensure_session_binding(InspectorMode::Proxy, "server-a"),
            Err(InspectorTargetError::ModeMismatch)
        );
        assert_eq!(
            target.ensure_mode(InspectorMode::Proxy),
            Err(InspectorTargetError::ModeMismatch)
        );
    }
}
