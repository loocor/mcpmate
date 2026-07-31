use std::collections::BTreeSet;

use serde_json::Value;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChangeClass {
    Unchanged,
    ObservationMetadata,
    ModelVisible,
    InvocationContract,
    SecurityExecution,
    BuiltinDefinition,
    OriginKey,
    Missing,
    Reappeared,
    NewRef,
    BackendEvidence,
    Authoring,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PolicyAction {
    Record,
    Follow,
    Review,
    ManualRebind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RelationshipLevel {
    Capability,
    Server,
    Builtin,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NewRefPolicy {
    Follow,
    Review,
}

impl ChangeClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unchanged => "unchanged",
            Self::ObservationMetadata => "observation_metadata",
            Self::ModelVisible => "model_visible",
            Self::InvocationContract => "invocation_contract",
            Self::SecurityExecution => "security_execution",
            Self::BuiltinDefinition => "builtin_definition",
            Self::OriginKey => "origin_key",
            Self::Missing => "missing",
            Self::Reappeared => "reappeared",
            Self::NewRef => "new_ref",
            Self::BackendEvidence => "backend_evidence",
            Self::Authoring => "authoring",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "unchanged" => Some(Self::Unchanged),
            "observation_metadata" => Some(Self::ObservationMetadata),
            "model_visible" => Some(Self::ModelVisible),
            "invocation_contract" => Some(Self::InvocationContract),
            "security_execution" => Some(Self::SecurityExecution),
            "builtin_definition" => Some(Self::BuiltinDefinition),
            "origin_key" => Some(Self::OriginKey),
            "missing" => Some(Self::Missing),
            "reappeared" => Some(Self::Reappeared),
            "new_ref" => Some(Self::NewRef),
            "backend_evidence" => Some(Self::BackendEvidence),
            "authoring" => Some(Self::Authoring),
            _ => None,
        }
    }
}

impl PolicyAction {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Record => "record",
            Self::Follow => "follow",
            Self::Review => "review",
            Self::ManualRebind => "manual_rebind",
        }
    }
}

pub fn policy_action(
    change: ChangeClass,
    level: RelationshipLevel,
    new_ref_policy: NewRefPolicy,
) -> PolicyAction {
    match change {
        ChangeClass::Unchanged | ChangeClass::ObservationMetadata | ChangeClass::BackendEvidence => {
            PolicyAction::Record
        }
        ChangeClass::Missing | ChangeClass::Reappeared if level == RelationshipLevel::Builtin => PolicyAction::Follow,
        ChangeClass::ModelVisible
        | ChangeClass::InvocationContract
        | ChangeClass::SecurityExecution
        | ChangeClass::Missing
        | ChangeClass::Authoring => PolicyAction::Review,
        ChangeClass::BuiltinDefinition if level == RelationshipLevel::Builtin => PolicyAction::Follow,
        ChangeClass::BuiltinDefinition => PolicyAction::Review,
        ChangeClass::OriginKey => PolicyAction::ManualRebind,
        ChangeClass::Reappeared
            if level == RelationshipLevel::Server && matches!(new_ref_policy, NewRefPolicy::Follow) =>
        {
            PolicyAction::Follow
        }
        ChangeClass::Reappeared => PolicyAction::Review,
        ChangeClass::NewRef if level == RelationshipLevel::Builtin => PolicyAction::Follow,
        ChangeClass::NewRef if level == RelationshipLevel::Server && matches!(new_ref_policy, NewRefPolicy::Follow) => {
            PolicyAction::Follow
        }
        ChangeClass::NewRef if level == RelationshipLevel::Server => PolicyAction::Review,
        ChangeClass::NewRef => PolicyAction::Record,
    }
}

pub fn classify_effective_definition_change(
    baseline: &Value,
    target: &Value,
) -> ChangeClass {
    if baseline == target {
        return ChangeClass::Unchanged;
    }

    let mut strongest = ChangeClass::ModelVisible;
    collect_change_class(baseline, target, &mut Vec::new(), &mut strongest);
    strongest
}

fn collect_change_class(
    baseline: &Value,
    target: &Value,
    path: &mut Vec<String>,
    strongest: &mut ChangeClass,
) {
    if baseline == target {
        return;
    }
    match (baseline, target) {
        (Value::Object(left), Value::Object(right)) => {
            let keys = left.keys().chain(right.keys()).cloned().collect::<BTreeSet<_>>();
            for key in keys {
                path.push(key.clone());
                collect_change_class(
                    left.get(&key).unwrap_or(&Value::Null),
                    right.get(&key).unwrap_or(&Value::Null),
                    path,
                    strongest,
                );
                path.pop();
            }
        }
        _ => {
            let candidate = classify_changed_path(path);
            if change_class_priority(candidate) > change_class_priority(*strongest) {
                *strongest = candidate;
            }
        }
    }
}

fn classify_changed_path(path: &[String]) -> ChangeClass {
    if path.iter().any(|segment| {
        matches!(
            segment.as_str(),
            "annotations" | "_meta" | "execution" | "securitySchemes"
        )
    }) {
        return ChangeClass::SecurityExecution;
    }
    if path.iter().any(|segment| {
        matches!(
            segment.as_str(),
            "inputSchema" | "outputSchema" | "arguments" | "mimeType" | "size"
        )
    }) {
        return ChangeClass::InvocationContract;
    }
    ChangeClass::ModelVisible
}

const fn change_class_priority(change: ChangeClass) -> u8 {
    match change {
        ChangeClass::SecurityExecution => 3,
        ChangeClass::InvocationContract => 2,
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ChangeClass, NewRefPolicy, PolicyAction, RelationshipLevel, classify_effective_definition_change, policy_action,
    };
    use serde_json::json;

    #[test]
    fn security_changes_cannot_be_weakened_by_server_follow() {
        assert_eq!(
            policy_action(
                ChangeClass::SecurityExecution,
                RelationshipLevel::Server,
                NewRefPolicy::Follow
            ),
            PolicyAction::Review
        );
    }

    #[test]
    fn mode_rule_builtin_refs_follow_the_versioned_allowlist() {
        assert_eq!(
            policy_action(ChangeClass::NewRef, RelationshipLevel::Builtin, NewRefPolicy::Follow,),
            PolicyAction::Follow
        );
        assert_eq!(
            policy_action(
                ChangeClass::BuiltinDefinition,
                RelationshipLevel::Builtin,
                NewRefPolicy::Follow,
            ),
            PolicyAction::Follow
        );
        assert_eq!(
            policy_action(ChangeClass::Missing, RelationshipLevel::Builtin, NewRefPolicy::Follow,),
            PolicyAction::Follow
        );
        assert_eq!(
            policy_action(
                ChangeClass::Reappeared,
                RelationshipLevel::Builtin,
                NewRefPolicy::Follow,
            ),
            PolicyAction::Follow
        );
    }

    #[test]
    fn classifies_model_visible_invocation_and_security_changes() {
        let baseline = json!({
            "name": "analyze",
            "description": "Analyze input",
            "inputSchema": {"type": "object"},
            "annotations": {"readOnlyHint": true}
        });

        assert_eq!(
            classify_effective_definition_change(
                &baseline,
                &json!({
                    "name": "analyze",
                    "description": "Analyze documents",
                    "inputSchema": {"type": "object"},
                    "annotations": {"readOnlyHint": true}
                }),
            ),
            ChangeClass::ModelVisible
        );
        assert_eq!(
            classify_effective_definition_change(
                &baseline,
                &json!({
                    "name": "analyze",
                    "description": "Analyze input",
                    "inputSchema": {"type": "object", "required": ["text"]},
                    "annotations": {"readOnlyHint": true}
                }),
            ),
            ChangeClass::InvocationContract
        );
        assert_eq!(
            classify_effective_definition_change(
                &baseline,
                &json!({
                    "name": "analyze",
                    "description": "Analyze documents",
                    "inputSchema": {"type": "object", "required": ["text"]},
                    "annotations": {"readOnlyHint": false}
                }),
            ),
            ChangeClass::SecurityExecution
        );
    }

    #[test]
    fn external_projection_changes_are_not_origin_key_changes() {
        assert_eq!(
            classify_effective_definition_change(
                &json!({"name": "server_a__analyze", "inputSchema": {"type": "object"}}),
                &json!({"name": "server_b__analyze", "inputSchema": {"type": "object"}}),
            ),
            ChangeClass::ModelVisible
        );
    }
}
