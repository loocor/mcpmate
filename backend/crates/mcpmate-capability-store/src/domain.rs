use chrono::{DateTime, Utc};
use rmcp::model::{InitializeResult, Prompt, Resource, ResourceTemplate, Tool};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{CapabilityId, CapabilityRefId, CapabilitySourceIdentity, EffectiveCapabilityRecordV1, Result};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityKind {
    Tools,
    Prompts,
    Resources,
    ResourceTemplates,
}

impl CapabilityKind {
    pub const ALL: [Self; 4] = [Self::Tools, Self::Prompts, Self::Resources, Self::ResourceTemplates];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Tools => "tools",
            Self::Prompts => "prompts",
            Self::Resources => "resources",
            Self::ResourceTemplates => "resource_templates",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "tools" => Some(Self::Tools),
            "prompts" => Some(Self::Prompts),
            "resources" => Some(Self::Resources),
            "resource_templates" => Some(Self::ResourceTemplates),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeclarationState {
    Unknown,
    Unsupported,
    Supported,
}

impl DeclarationState {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Unsupported => "unsupported",
            Self::Supported => "supported",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "unknown" => Some(Self::Unknown),
            "unsupported" => Some(Self::Unsupported),
            "supported" => Some(Self::Supported),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InventoryState {
    Unknown,
    Complete,
    Failed,
}

impl InventoryState {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Complete => "complete",
            Self::Failed => "failed",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "unknown" => Some(Self::Unknown),
            "complete" => Some(Self::Complete),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotState {
    Ready,
    Invalidated,
    Unavailable,
}

impl SnapshotState {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Invalidated => "invalidated",
            Self::Unavailable => "unavailable",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "ready" => Some(Self::Ready),
            "invalidated" => Some(Self::Invalidated),
            "unavailable" => Some(Self::Unavailable),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
pub enum CapabilityPayload {
    Tool(Tool),
    Prompt(Prompt),
    Resource(Resource),
    ResourceTemplate(ResourceTemplate),
}

impl CapabilityPayload {
    pub const fn kind(&self) -> CapabilityKind {
        match self {
            Self::Tool(_) => CapabilityKind::Tools,
            Self::Prompt(_) => CapabilityKind::Prompts,
            Self::Resource(_) => CapabilityKind::Resources,
            Self::ResourceTemplate(_) => CapabilityKind::ResourceTemplates,
        }
    }

    pub fn origin_key(&self) -> &str {
        match self {
            Self::Tool(value) => value.name.as_ref(),
            Self::Prompt(value) => &value.name,
            Self::Resource(value) => &value.uri,
            Self::ResourceTemplate(value) => &value.uri_template,
        }
    }

    fn with_identity_key(
        mut self,
        identity_key: &str,
    ) -> Self {
        match &mut self {
            Self::Tool(value) => value.name = identity_key.to_string().into(),
            Self::Prompt(value) => value.name = identity_key.to_string(),
            Self::Resource(value) => value.uri = identity_key.to_string(),
            Self::ResourceTemplate(value) => value.uri_template = identity_key.to_string(),
        }
        self
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CatalogRecord {
    pub ref_id: CapabilityRefId,
    pub capability_id: CapabilityId,
    pub upstream_key: String,
    pub external_key: String,
    pub canonical_record: Vec<u8>,
    pub source_payload: CapabilityPayload,
    pub effective_payload: CapabilityPayload,
}

impl CatalogRecord {
    pub fn materialize(
        server_id: impl Into<String>,
        upstream_key: impl Into<String>,
        external_key: impl Into<String>,
        payload: CapabilityPayload,
    ) -> Result<Self> {
        let server_id = server_id.into();
        let upstream_key = upstream_key.into();
        let external_key = external_key.into();
        if payload.origin_key() != upstream_key {
            return Err(crate::CatalogError::IntegrityMismatch {
                identity: format!(
                    "capability origin key '{}' does not match payload origin key '{}'",
                    upstream_key,
                    payload.origin_key()
                ),
            });
        }
        let source = CapabilitySourceIdentity::new(&server_id, payload.kind(), &upstream_key);
        let ref_id = CapabilityRefId::derive(&source)?;
        let effective_payload = payload.clone().with_identity_key(&external_key);
        let effective_record = EffectiveCapabilityRecordV1::new(ref_id.clone(), source, effective_payload.clone())?;
        let capability_id = CapabilityId::derive(&effective_record)?;
        let canonical_record = effective_record.canonical_bytes()?;
        Ok(Self {
            ref_id,
            capability_id,
            upstream_key,
            external_key,
            canonical_record,
            source_payload: payload,
            effective_payload,
        })
    }

    pub(crate) fn from_persisted_record(
        capability_id: CapabilityId,
        canonical_record: Vec<u8>,
        source_payload: CapabilityPayload,
        effective_payload: CapabilityPayload,
        effective_record: EffectiveCapabilityRecordV1,
    ) -> Result<Self> {
        effective_record.validate()?;
        capability_id.verify_canonical_content(&canonical_record, &canonical_record)?;
        let upstream_key = effective_record.source.origin_key.clone();
        let external_key = effective_record.definition.external_key();
        Self::validate_persisted_payloads(&capability_id, &source_payload, &effective_payload, &effective_record)?;
        Ok(Self {
            ref_id: effective_record.ref_id,
            capability_id,
            upstream_key,
            external_key,
            canonical_record,
            source_payload,
            effective_payload,
        })
    }

    pub(crate) fn validate_persisted_payloads(
        capability_id: &CapabilityId,
        source_payload: &CapabilityPayload,
        effective_payload: &CapabilityPayload,
        effective_record: &EffectiveCapabilityRecordV1,
    ) -> Result<()> {
        let canonical_effective_payload = effective_record.definition.clone().into_payload();
        let normalized_source_payload = source_payload
            .clone()
            .with_identity_key(&effective_record.definition.external_key());
        if source_payload.kind() != effective_record.source.kind
            || source_payload.origin_key() != effective_record.source.origin_key
            || normalized_source_payload != *effective_payload
            || *effective_payload != canonical_effective_payload
        {
            return Err(crate::CatalogError::IntegrityMismatch {
                identity: capability_id.to_string(),
            });
        }
        Ok(())
    }

    pub const fn kind(&self) -> CapabilityKind {
        self.source_payload.kind()
    }

    /// Exact payload observed from the upstream source.
    pub fn upstream_payload(&self) -> CapabilityPayload {
        self.source_payload.clone()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityRefState {
    Active,
    Unresolved,
    Retired,
}

impl CapabilityRefState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Unresolved => "unresolved",
            Self::Retired => "retired",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "active" => Some(Self::Active),
            "unresolved" => Some(Self::Unresolved),
            "retired" => Some(Self::Retired),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityRefRecord {
    pub ref_id: CapabilityRefId,
    pub server_id: String,
    pub kind: CapabilityKind,
    pub origin_key: String,
    pub state: CapabilityRefState,
    pub state_generation: i64,
    pub first_observed_revision: i64,
    pub last_observed_revision: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityVersionRecord {
    pub capability_id: CapabilityId,
    pub ref_id: CapabilityRefId,
    pub canonical_record: Vec<u8>,
    pub record_format: String,
    pub first_observed_revision: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityVersionChange {
    pub ref_id: CapabilityRefId,
    pub before_capability_id: CapabilityId,
    pub target_capability_id: CapabilityId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KindCompleteness {
    pub kind: CapabilityKind,
    pub inventory: InventoryState,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CatalogDelta {
    pub added_refs: Vec<CapabilityRefId>,
    pub changed_versions: Vec<CapabilityVersionChange>,
    pub unresolved_refs: Vec<CapabilityRefId>,
    pub reappeared_refs: Vec<CapabilityRefId>,
    pub unchanged_refs: Vec<CapabilityRefId>,
    pub kind_completeness: Vec<KindCompleteness>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CatalogReconciliation {
    pub commit: CatalogCommit,
    pub delta: CatalogDelta,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KindFailureKind {
    Timeout,
    SessionGone,
    TransportClosed,
    StaleGeneration,
    Authentication,
    AuthRequired,
    Unauthorized,
    Forbidden,
    InsufficientScope,
    Protocol,
    Application,
    Other,
}

impl KindFailureKind {
    pub(crate) const fn as_str(&self) -> &'static str {
        match self {
            Self::Timeout => "timeout",
            Self::SessionGone => "session_gone",
            Self::TransportClosed => "transport_closed",
            Self::StaleGeneration => "stale_generation",
            Self::Authentication => "authentication",
            Self::AuthRequired => "auth_required",
            Self::Unauthorized => "unauthorized",
            Self::Forbidden => "forbidden",
            Self::InsufficientScope => "insufficient_scope",
            Self::Protocol => "protocol",
            Self::Application => "application",
            Self::Other => "other",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "timeout" => Some(Self::Timeout),
            "session_gone" => Some(Self::SessionGone),
            "transport_closed" => Some(Self::TransportClosed),
            "stale_generation" => Some(Self::StaleGeneration),
            "authentication" => Some(Self::Authentication),
            "auth_required" => Some(Self::AuthRequired),
            "unauthorized" => Some(Self::Unauthorized),
            "forbidden" => Some(Self::Forbidden),
            "insufficient_scope" => Some(Self::InsufficientScope),
            "protocol" => Some(Self::Protocol),
            "application" => Some(Self::Application),
            "other" => Some(Self::Other),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct KindObservation {
    pub kind: CapabilityKind,
    pub declaration: DeclarationState,
    pub inventory: InventoryState,
    pub error: Option<String>,
    pub failure_kind: Option<KindFailureKind>,
    pub timeout_ms: Option<u64>,
}

impl KindObservation {
    pub fn new(
        kind: CapabilityKind,
        declaration: DeclarationState,
        inventory: InventoryState,
    ) -> Self {
        Self {
            kind,
            declaration,
            inventory,
            error: None,
            failure_kind: None,
            timeout_ms: None,
        }
    }

    pub fn with_error(
        mut self,
        error: impl Into<String>,
    ) -> Self {
        self.error = Some(error.into());
        self
    }

    pub fn with_failure(
        mut self,
        failure_kind: KindFailureKind,
        error: impl Into<String>,
        timeout_ms: Option<u64>,
    ) -> Self {
        self.failure_kind = Some(failure_kind);
        self.error = Some(error.into());
        self.timeout_ms = timeout_ms;
        self
    }
}

#[derive(Clone, Debug)]
pub struct CapabilityObservation {
    pub server_id: String,
    pub server_name: String,
    pub config_fingerprint: String,
    pub initialize: InitializeResult,
    pub kind_states: Vec<KindObservation>,
    pub records: Vec<CatalogRecord>,
    pub observed_at: DateTime<Utc>,
    pub state: SnapshotState,
    pub last_error: Option<String>,
    pub observed_kinds: Vec<CapabilityKind>,
}

impl CapabilityObservation {
    pub fn new(
        server_id: impl Into<String>,
        server_name: impl Into<String>,
        config_fingerprint: impl Into<String>,
        initialize: InitializeResult,
        kind_states: Vec<KindObservation>,
        records: Vec<CatalogRecord>,
    ) -> Self {
        let observed_kinds = kind_states.iter().map(|state| state.kind).collect();
        Self {
            server_id: server_id.into(),
            server_name: server_name.into(),
            config_fingerprint: config_fingerprint.into(),
            initialize,
            kind_states,
            records,
            observed_at: Utc::now(),
            state: SnapshotState::Ready,
            last_error: None,
            observed_kinds,
        }
    }

    pub fn with_state(
        mut self,
        state: SnapshotState,
        last_error: Option<String>,
    ) -> Self {
        self.state = state;
        self.last_error = last_error;
        self
    }

    pub fn with_observed_kinds(
        mut self,
        observed_kinds: impl IntoIterator<Item = CapabilityKind>,
    ) -> Self {
        self.observed_kinds = observed_kinds.into_iter().collect();
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CatalogCommit {
    pub server_id: String,
    pub revision: i64,
    pub changed: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CatalogStats {
    pub snapshots: i64,
    pub ready_snapshots: i64,
    pub invalidated_snapshots: i64,
    pub unavailable_snapshots: i64,
    pub records: i64,
    pub tools: i64,
    pub prompts: i64,
    pub resources: i64,
    pub resource_templates: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CatalogInvalidation {
    pub server_id: String,
    pub server_name: String,
    pub revision: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CatalogSnapshot {
    pub server_id: String,
    pub server_name: String,
    pub config_fingerprint: String,
    pub revision: i64,
    pub state: SnapshotState,
    pub initialize: Option<InitializeResult>,
    pub kind_states: Vec<KindObservation>,
    pub records: Vec<CatalogRecord>,
    pub observed_at: DateTime<Utc>,
    pub committed_at: DateTime<Utc>,
    pub last_error: Option<String>,
}

#[derive(Clone, Debug)]
pub struct CapabilityFailureObservation {
    pub server_id: String,
    pub server_name: String,
    pub config_fingerprint: String,
    pub kinds: Vec<CapabilityKind>,
    pub reason: String,
    pub failure_kind: Option<KindFailureKind>,
    pub timeout_ms: Option<u64>,
    pub observed_at: DateTime<Utc>,
}

impl CapabilityFailureObservation {
    pub fn new(
        server_id: impl Into<String>,
        server_name: impl Into<String>,
        config_fingerprint: impl Into<String>,
        kind: CapabilityKind,
        reason: impl Into<String>,
    ) -> Self {
        Self::for_kinds(server_id, server_name, config_fingerprint, [kind], reason)
    }

    pub fn for_kinds(
        server_id: impl Into<String>,
        server_name: impl Into<String>,
        config_fingerprint: impl Into<String>,
        kinds: impl IntoIterator<Item = CapabilityKind>,
        reason: impl Into<String>,
    ) -> Self {
        let kinds = kinds.into_iter().collect::<Vec<_>>();
        assert!(
            !kinds.is_empty(),
            "failure observation requires at least one capability kind"
        );
        Self {
            server_id: server_id.into(),
            server_name: server_name.into(),
            config_fingerprint: config_fingerprint.into(),
            kinds,
            reason: reason.into(),
            failure_kind: None,
            timeout_ms: None,
            observed_at: Utc::now(),
        }
    }

    pub fn with_failure(
        mut self,
        failure_kind: KindFailureKind,
        timeout_ms: Option<u64>,
    ) -> Self {
        self.failure_kind = Some(failure_kind);
        self.timeout_ms = timeout_ms;
        self
    }
}
