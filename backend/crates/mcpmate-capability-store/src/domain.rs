use chrono::{DateTime, Utc};
use rmcp::model::{InitializeResult, Prompt, Resource, ResourceTemplate, Tool};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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

    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Tools => "tools",
            Self::Prompts => "prompts",
            Self::Resources => "resources",
            Self::ResourceTemplates => "resource_templates",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
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
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CatalogRecord {
    pub stable_id: String,
    pub upstream_key: String,
    pub external_key: String,
    pub payload: CapabilityPayload,
}

impl CatalogRecord {
    pub fn new(
        stable_id: impl Into<String>,
        upstream_key: impl Into<String>,
        external_key: impl Into<String>,
        payload: CapabilityPayload,
    ) -> Self {
        Self {
            stable_id: stable_id.into(),
            upstream_key: upstream_key.into(),
            external_key: external_key.into(),
            payload,
        }
    }

    pub const fn kind(&self) -> CapabilityKind {
        self.payload.kind()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct KindObservation {
    pub kind: CapabilityKind,
    pub declaration: DeclarationState,
    pub inventory: InventoryState,
    pub error: Option<String>,
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
        }
    }

    pub fn with_error(
        mut self,
        error: impl Into<String>,
    ) -> Self {
        self.error = Some(error.into());
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CatalogCommit {
    pub server_id: String,
    pub revision: i64,
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
    pub kind: CapabilityKind,
    pub reason: String,
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
        Self {
            server_id: server_id.into(),
            server_name: server_name.into(),
            config_fingerprint: config_fingerprint.into(),
            kind,
            reason: reason.into(),
            observed_at: Utc::now(),
        }
    }
}
