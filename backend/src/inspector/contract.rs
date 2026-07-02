use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Operating mode for Inspector endpoints.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum InspectorMode {
    /// Aggregate/managed view: unique naming, profile-aware (recommended)
    #[default]
    Proxy,
    /// Direct upstream view: single server/instance, no unique naming
    Native,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum InspectorProxyMode {
    #[default]
    Hosted,
    Unify,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InspectorProxyScope {
    Isolated,
    ActiveCatalog,
}
