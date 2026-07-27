use std::{collections::HashSet, fmt};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{
    CapabilityId, CapabilityKind, CapabilityRefId, CatalogError, Result, SURFACE_MANIFEST_FORMAT_V1, SurfaceManifestId,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SurfaceManifestEntryInput {
    pub ref_id: CapabilityRefId,
    pub capability_id: CapabilityId,
    pub kind: CapabilityKind,
    pub external_key: String,
}

impl SurfaceManifestEntryInput {
    pub fn new(
        ref_id: CapabilityRefId,
        capability_id: CapabilityId,
        kind: CapabilityKind,
        external_key: impl Into<String>,
    ) -> Self {
        Self {
            ref_id,
            capability_id,
            kind,
            external_key: external_key.into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceManifestEntry {
    pub ref_id: CapabilityRefId,
    pub capability_id: CapabilityId,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SurfaceManifestContentV1 {
    pub format: String,
    pub consumer_id: String,
    pub entries: Vec<SurfaceManifestEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SurfaceManifest {
    pub manifest_id: SurfaceManifestId,
    pub consumer_id: String,
    pub entries: Vec<SurfaceManifestEntry>,
    pub canonical_content: Vec<u8>,
}

impl SurfaceManifest {
    pub fn compile(
        consumer_id: impl Into<String>,
        mut entries: Vec<SurfaceManifestEntryInput>,
    ) -> Result<Self> {
        let consumer_id = consumer_id.into();
        if consumer_id.is_empty() {
            return Err(CatalogError::InvalidSurfaceValue {
                field: "consumer_id",
                value: consumer_id,
            });
        }
        entries.sort_by(|left, right| {
            left.kind
                .as_str()
                .cmp(right.kind.as_str())
                .then(left.external_key.cmp(&right.external_key))
                .then(left.capability_id.as_str().cmp(right.capability_id.as_str()))
                .then(left.ref_id.as_str().cmp(right.ref_id.as_str()))
        });
        let mut refs = HashSet::with_capacity(entries.len());
        for entry in &entries {
            if !refs.insert(entry.ref_id.clone()) {
                return Err(CatalogError::DuplicateManifestRef {
                    ref_id: entry.ref_id.to_string(),
                });
            }
        }
        let entries = entries
            .into_iter()
            .map(|entry| SurfaceManifestEntry {
                ref_id: entry.ref_id,
                capability_id: entry.capability_id,
            })
            .collect::<Vec<_>>();
        let content = SurfaceManifestContentV1 {
            format: SURFACE_MANIFEST_FORMAT_V1.to_string(),
            consumer_id: consumer_id.clone(),
            entries: entries.clone(),
        };
        let canonical_content = serde_json_canonicalizer::to_vec(&content)?;
        let manifest_id = SurfaceManifestId::derive(&content)?;
        Ok(Self {
            manifest_id,
            consumer_id,
            entries,
            canonical_content,
        })
    }

    pub(crate) fn content(&self) -> Result<SurfaceManifestContentV1> {
        let content: SurfaceManifestContentV1 = serde_json::from_slice(&self.canonical_content)?;
        if content.format != SURFACE_MANIFEST_FORMAT_V1
            || content.consumer_id != self.consumer_id
            || content.entries != self.entries
        {
            return Err(CatalogError::IntegrityMismatch {
                identity: self.manifest_id.to_string(),
            });
        }
        self.manifest_id.verify_content(&content)?;
        if serde_json_canonicalizer::to_vec(&content)? != self.canonical_content {
            return Err(CatalogError::IntegrityMismatch {
                identity: self.manifest_id.to_string(),
            });
        }
        Ok(content)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProposalLifecycle {
    Pending,
    Resolved,
    Superseded,
}

impl ProposalLifecycle {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Resolved => "resolved",
            Self::Superseded => "superseded",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceProposal {
    pub proposal_id: String,
    pub consumer_id: String,
    pub base_publication_id: Option<String>,
    pub proposed_manifest_id: SurfaceManifestId,
    pub trigger_kind: String,
    pub trigger_id: String,
    pub source_revision_set: Value,
    pub diff_summary: Value,
    pub lifecycle: ProposalLifecycle,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityChangeEvent {
    pub event_id: String,
    pub consumer_id: String,
    pub proposal_id: String,
    pub ref_id: CapabilityRefId,
    pub before_capability_id: Option<CapabilityId>,
    pub target_capability_id: Option<CapabilityId>,
    pub change_class: String,
    pub policy_action: String,
    pub actor: String,
    pub occurred_at: DateTime<Utc>,
}

impl CapabilityChangeEvent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        event_id: impl Into<String>,
        consumer_id: impl Into<String>,
        proposal_id: impl Into<String>,
        ref_id: CapabilityRefId,
        before_capability_id: Option<CapabilityId>,
        target_capability_id: Option<CapabilityId>,
        change_class: impl Into<String>,
        policy_action: impl Into<String>,
        actor: impl Into<String>,
    ) -> Self {
        Self {
            event_id: event_id.into(),
            consumer_id: consumer_id.into(),
            proposal_id: proposal_id.into(),
            ref_id,
            before_capability_id,
            target_capability_id,
            change_class: change_class.into(),
            policy_action: policy_action.into(),
            actor: actor.into(),
            occurred_at: Utc::now(),
        }
    }
}

impl SurfaceProposal {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        proposal_id: impl Into<String>,
        consumer_id: impl Into<String>,
        base_publication_id: Option<String>,
        proposed_manifest_id: SurfaceManifestId,
        trigger_kind: impl Into<String>,
        trigger_id: impl Into<String>,
        source_revision_set: Value,
        diff_summary: Value,
    ) -> Self {
        Self {
            proposal_id: proposal_id.into(),
            consumer_id: consumer_id.into(),
            base_publication_id,
            proposed_manifest_id,
            trigger_kind: trigger_kind.into(),
            trigger_id: trigger_id.into(),
            source_revision_set,
            diff_summary,
            lifecycle: ProposalLifecycle::Pending,
            created_at: Utc::now(),
            resolved_at: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReviewLifecycle {
    Pending,
    Resolved,
    Obsolete,
}

impl ReviewLifecycle {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Resolved => "resolved",
            Self::Obsolete => "obsolete",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "resolved" => Some(Self::Resolved),
            "obsolete" => Some(Self::Obsolete),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ReviewTargetKey(String);

impl ReviewTargetKey {
    pub fn capability(capability_id: &CapabilityId) -> Self {
        Self(format!("capability:{capability_id}"))
    }

    pub fn missing(state_generation: i64) -> Self {
        Self(format!("missing:{state_generation}"))
    }

    pub fn reappeared(
        capability_id: &CapabilityId,
        state_generation: i64,
    ) -> Self {
        Self(format!("reappeared:{capability_id}:{state_generation}"))
    }

    pub fn evidence(evidence_id: &str) -> Self {
        Self(format!("evidence:{evidence_id}"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn parse(value: String) -> Result<Self> {
        if ["capability:", "missing:", "reappeared:", "evidence:"]
            .iter()
            .any(|prefix| value.starts_with(prefix) && value.len() > prefix.len())
        {
            Ok(Self(value))
        } else {
            Err(CatalogError::InvalidSurfaceValue {
                field: "target_key",
                value,
            })
        }
    }
}

impl fmt::Display for ReviewTargetKey {
    fn fmt(
        &self,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::str::FromStr for ReviewTargetKey {
    type Err = CatalogError;

    fn from_str(value: &str) -> Result<Self> {
        Self::parse(value.to_string())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceReviewItemDraft {
    pub review_item_id: String,
    pub created_by_proposal_id: String,
    pub consumer_id: String,
    pub ref_id: CapabilityRefId,
    pub before_capability_id: Option<CapabilityId>,
    pub target_capability_id: Option<CapabilityId>,
    pub target_key: ReviewTargetKey,
    pub change_class: String,
    pub policy_action: String,
}

impl SurfaceReviewItemDraft {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        review_item_id: impl Into<String>,
        created_by_proposal_id: impl Into<String>,
        consumer_id: impl Into<String>,
        ref_id: CapabilityRefId,
        before_capability_id: Option<CapabilityId>,
        target_capability_id: Option<CapabilityId>,
        target_key: ReviewTargetKey,
        change_class: impl Into<String>,
        policy_action: impl Into<String>,
    ) -> Self {
        Self {
            review_item_id: review_item_id.into(),
            created_by_proposal_id: created_by_proposal_id.into(),
            consumer_id: consumer_id.into(),
            ref_id,
            before_capability_id,
            target_capability_id,
            target_key,
            change_class: change_class.into(),
            policy_action: policy_action.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceReviewItem {
    pub review_item_id: String,
    pub created_by_proposal_id: String,
    pub consumer_id: String,
    pub ref_id: CapabilityRefId,
    pub before_capability_id: Option<CapabilityId>,
    pub target_capability_id: Option<CapabilityId>,
    pub target_key: ReviewTargetKey,
    pub change_class: String,
    pub policy_action: String,
    pub lifecycle: ReviewLifecycle,
    pub current_decision_id: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReviewResolutionAction {
    ApproveTarget,
    RejectTarget,
    KeepIntent,
    RemoveIntent,
    RebindRef,
}

impl ReviewResolutionAction {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::ApproveTarget => "approve_target",
            Self::RejectTarget => "reject_target",
            Self::KeepIntent => "keep_intent",
            Self::RemoveIntent => "remove_intent",
            Self::RebindRef => "rebind_ref",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "approve_target" => Some(Self::ApproveTarget),
            "reject_target" => Some(Self::RejectTarget),
            "keep_intent" => Some(Self::KeepIntent),
            "remove_intent" => Some(Self::RemoveIntent),
            "rebind_ref" => Some(Self::RebindRef),
            _ => None,
        }
    }
}

impl std::str::FromStr for ReviewResolutionAction {
    type Err = CatalogError;

    fn from_str(value: &str) -> Result<Self> {
        Self::parse(value).ok_or_else(|| CatalogError::InvalidSurfaceValue {
            field: "review resolution action",
            value: value.to_string(),
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceReviewDecision {
    pub decision_id: String,
    pub review_item_id: String,
    pub resolution_action: ReviewResolutionAction,
    pub resolution_payload: Option<Value>,
    pub actor: String,
    pub decided_at: DateTime<Utc>,
    pub supersedes_decision_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceReviewDecisionDraft {
    pub decision_id: String,
    pub review_item_id: String,
    pub resolution_action: ReviewResolutionAction,
    pub resolution_payload: Option<Value>,
    pub actor: String,
    pub decided_at: DateTime<Utc>,
}

impl SurfaceReviewDecisionDraft {
    pub fn new(
        decision_id: impl Into<String>,
        review_item_id: impl Into<String>,
        resolution_action: ReviewResolutionAction,
        resolution_payload: Option<Value>,
        actor: impl Into<String>,
    ) -> Self {
        Self {
            decision_id: decision_id.into(),
            review_item_id: review_item_id.into(),
            resolution_action,
            resolution_payload,
            actor: actor.into(),
            decided_at: Utc::now(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ReviewOwnerType {
    StandardProfile,
    CustomProfile,
    ConsumerDirectExposure,
    ProfileServerExposure,
    ConsumerServerExposure,
    ModeRule,
}

impl ReviewOwnerType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::StandardProfile => "standard_profile",
            Self::CustomProfile => "custom_profile",
            Self::ConsumerDirectExposure => "consumer_direct_exposure",
            Self::ProfileServerExposure => "profile_server_exposure",
            Self::ConsumerServerExposure => "consumer_server_exposure",
            Self::ModeRule => "mode_rule",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "standard_profile" => Some(Self::StandardProfile),
            "custom_profile" => Some(Self::CustomProfile),
            "consumer_direct_exposure" => Some(Self::ConsumerDirectExposure),
            "profile_server_exposure" => Some(Self::ProfileServerExposure),
            "consumer_server_exposure" => Some(Self::ConsumerServerExposure),
            "mode_rule" => Some(Self::ModeRule),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SurfaceReviewOwner {
    pub owner_type: ReviewOwnerType,
    pub owner_id: String,
}

impl SurfaceReviewOwner {
    pub fn new(
        owner_type: ReviewOwnerType,
        owner_id: impl Into<String>,
    ) -> Self {
        Self {
            owner_type,
            owner_id: owner_id.into(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SurfaceReviewFilter {
    pub consumer_id: Option<String>,
    pub owner_type: Option<ReviewOwnerType>,
    pub owner_id: Option<String>,
    pub lifecycle: Option<ReviewLifecycle>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceReviewRecord {
    pub item: SurfaceReviewItem,
    pub owners: Vec<SurfaceReviewOwner>,
    pub current_decision: Option<SurfaceReviewDecision>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfacePublication {
    pub publication_id: String,
    pub consumer_id: String,
    pub manifest_id: SurfaceManifestId,
    pub proposal_id: Option<String>,
    pub reason: String,
    pub published_by: String,
    pub published_at: DateTime<Utc>,
    pub supersedes_publication_id: Option<String>,
}

impl SurfacePublication {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        publication_id: impl Into<String>,
        consumer_id: impl Into<String>,
        manifest_id: SurfaceManifestId,
        proposal_id: Option<String>,
        reason: impl Into<String>,
        published_by: impl Into<String>,
        supersedes_publication_id: Option<String>,
    ) -> Self {
        Self {
            publication_id: publication_id.into(),
            consumer_id: consumer_id.into(),
            manifest_id,
            proposal_id,
            reason: reason.into(),
            published_by: published_by.into(),
            published_at: Utc::now(),
            supersedes_publication_id,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConsumerSurfaceBinding {
    pub consumer_id: String,
    pub active_publication_id: String,
    pub generation: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RollbackBlock {
    pub ref_id: CapabilityRefId,
    pub pinned_capability_id: CapabilityId,
    pub current_capability_id: Option<CapabilityId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReconciliationJobStatus {
    Pending,
    Leased,
    Succeeded,
    Failed,
}

impl ReconciliationJobStatus {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Leased => "leased",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "leased" => Some(Self::Leased),
            "succeeded" => Some(Self::Succeeded),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceReconciliationJob {
    pub idempotency_key: String,
    pub cause_kind: String,
    pub cause_id: String,
    pub consumer_id: String,
    pub target_revision_set: Value,
    pub expected_binding_generation: i64,
    pub status: ReconciliationJobStatus,
    pub attempt_count: i64,
    pub leased_by: Option<String>,
    pub lease_expires_at: Option<DateTime<Utc>>,
    pub next_attempt_at: DateTime<Utc>,
    pub last_error: Option<String>,
    pub success_receipt: Option<Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SurfaceReconciliationJob {
    pub fn new(
        cause_kind: impl Into<String>,
        cause_id: impl Into<String>,
        consumer_id: impl Into<String>,
        target_revision_set: Value,
        expected_binding_generation: i64,
    ) -> Result<Self> {
        let cause_kind = cause_kind.into();
        let cause_id = cause_id.into();
        let consumer_id = consumer_id.into();
        let canonical_key = serde_json_canonicalizer::to_vec(&serde_json::json!({
            "causeKind": &cause_kind,
            "causeId": &cause_id,
            "consumerId": &consumer_id,
            "targetRevisionSet": &target_revision_set,
            "expectedBindingGeneration": expected_binding_generation,
        }))?;
        let now = Utc::now();
        Ok(Self {
            idempotency_key: format!("reconcile_sha256:{:x}", Sha256::digest(canonical_key)),
            cause_kind,
            cause_id,
            consumer_id,
            target_revision_set,
            expected_binding_generation,
            status: ReconciliationJobStatus::Pending,
            attempt_count: 0,
            leased_by: None,
            lease_expires_at: None,
            next_attempt_at: now,
            last_error: None,
            success_receipt: None,
            created_at: now,
            updated_at: now,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceOutboxEvent {
    pub event_id: String,
    pub event_kind: String,
    pub aggregate_id: String,
    pub payload: Value,
    pub created_at: DateTime<Utc>,
    pub delivered_at: Option<DateTime<Utc>>,
}

impl SurfaceOutboxEvent {
    pub fn new(
        event_id: impl Into<String>,
        event_kind: impl Into<String>,
        aggregate_id: impl Into<String>,
        payload: Value,
    ) -> Self {
        Self {
            event_id: event_id.into(),
            event_kind: event_kind.into(),
            aggregate_id: aggregate_id.into(),
            payload,
            created_at: Utc::now(),
            delivered_at: None,
        }
    }
}
