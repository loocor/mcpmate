use std::{str::FromStr, time::Duration};

use chrono::{DateTime, TimeDelta, Utc};
use sqlx::{Pool, Row, Sqlite, SqliteConnection, Transaction};

use crate::{
    CapabilityChangeEvent, CapabilityId, CapabilityRefId, CatalogError, ConsumerSurfaceBinding, ProposalLifecycle,
    ReconciliationJobStatus, Result, ReviewLifecycle, ReviewOwnerType, ReviewResolutionAction, ReviewTargetKey,
    RollbackBlock, SurfaceManifest, SurfaceManifestId, SurfaceOutboxEvent, SurfaceProposal, SurfacePublication,
    SurfaceReconciliationJob, SurfaceReviewDecision, SurfaceReviewDecisionDraft, SurfaceReviewFilter,
    SurfaceReviewItem, SurfaceReviewItemDraft, SurfaceReviewOwner, SurfaceReviewRecord,
};

#[derive(Clone)]
pub struct SqliteSurfaceStore {
    pool: Pool<Sqlite>,
}

impl SqliteSurfaceStore {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn insert_capability_change_event_in_transaction(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        event: &CapabilityChangeEvent,
    ) -> Result<()> {
        let inserted = sqlx::query(
            r#"
            INSERT OR IGNORE INTO capability_change_events (
                event_id, consumer_id, proposal_id, ref_id,
                before_capability_id, target_capability_id,
                change_class, policy_action, actor, occurred_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&event.event_id)
        .bind(&event.consumer_id)
        .bind(&event.proposal_id)
        .bind(event.ref_id.as_str())
        .bind(event.before_capability_id.as_ref().map(CapabilityId::as_str))
        .bind(event.target_capability_id.as_ref().map(CapabilityId::as_str))
        .bind(&event.change_class)
        .bind(&event.policy_action)
        .bind(&event.actor)
        .bind(event.occurred_at.to_rfc3339())
        .execute(&mut **transaction)
        .await?
        .rows_affected();
        if inserted == 0 {
            let existing = sqlx::query(
                r#"
                SELECT consumer_id, ref_id, before_capability_id, target_capability_id,
                       change_class, policy_action
                FROM capability_change_events
                WHERE event_id = ?
                "#,
            )
            .bind(&event.event_id)
            .fetch_one(&mut **transaction)
            .await?;
            if existing.try_get::<String, _>("consumer_id")? != event.consumer_id
                || existing.try_get::<String, _>("ref_id")? != event.ref_id.as_str()
                || existing.try_get::<Option<String>, _>("target_capability_id")?
                    != event.target_capability_id.as_ref().map(ToString::to_string)
                || existing.try_get::<String, _>("change_class")? != event.change_class
                || existing.try_get::<String, _>("policy_action")? != event.policy_action
            {
                return Err(CatalogError::IntegrityMismatch {
                    identity: event.event_id.clone(),
                });
            }
        }
        Ok(())
    }

    pub async fn insert_manifest_in_transaction(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        manifest: &SurfaceManifest,
    ) -> Result<()> {
        manifest.content()?;
        for entry in &manifest.entries {
            let exists: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM capability_versions WHERE ref_id = ? AND capability_id = ?")
                    .bind(entry.ref_id.as_str())
                    .bind(entry.capability_id.as_str())
                    .fetch_one(&mut **transaction)
                    .await?;
            if exists != 1 {
                return Err(CatalogError::SurfaceNotFound {
                    entity: "capability version",
                    id: format!("{}/{}", entry.ref_id, entry.capability_id),
                });
            }
        }

        let inserted = sqlx::query(
            r#"
            INSERT OR IGNORE INTO surface_manifests (
                manifest_id, consumer_id, canonical_content, created_at
            ) VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(manifest.manifest_id.as_str())
        .bind(&manifest.consumer_id)
        .bind(&manifest.canonical_content)
        .bind(Utc::now().to_rfc3339())
        .execute(&mut **transaction)
        .await?
        .rows_affected();

        if inserted == 0 {
            self.verify_manifest_in_transaction(transaction, manifest).await?;
            return Ok(());
        }

        for (position, entry) in manifest.entries.iter().enumerate() {
            sqlx::query(
                r#"
                INSERT INTO surface_manifest_entries (
                    manifest_id, position, ref_id, capability_id
                ) VALUES (?, ?, ?, ?)
                "#,
            )
            .bind(manifest.manifest_id.as_str())
            .bind(position as i64)
            .bind(entry.ref_id.as_str())
            .bind(entry.capability_id.as_str())
            .execute(&mut **transaction)
            .await?;
        }
        Ok(())
    }

    async fn verify_manifest_in_transaction(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        manifest: &SurfaceManifest,
    ) -> Result<()> {
        let row = sqlx::query("SELECT consumer_id, canonical_content FROM surface_manifests WHERE manifest_id = ?")
            .bind(manifest.manifest_id.as_str())
            .fetch_optional(&mut **transaction)
            .await?
            .ok_or_else(|| CatalogError::SurfaceNotFound {
                entity: "surface manifest",
                id: manifest.manifest_id.to_string(),
            })?;
        let consumer_id: String = row.try_get("consumer_id")?;
        let canonical_content: Vec<u8> = row.try_get("canonical_content")?;
        if consumer_id != manifest.consumer_id || canonical_content != manifest.canonical_content {
            return Err(CatalogError::IntegrityMismatch {
                identity: manifest.manifest_id.to_string(),
            });
        }
        let entries = sqlx::query(
            r#"
            SELECT ref_id, capability_id
            FROM surface_manifest_entries
            WHERE manifest_id = ?
            ORDER BY position
            "#,
        )
        .bind(manifest.manifest_id.as_str())
        .fetch_all(&mut **transaction)
        .await?;
        if entries.len() != manifest.entries.len() {
            return Err(CatalogError::IntegrityMismatch {
                identity: manifest.manifest_id.to_string(),
            });
        }
        for (row, expected) in entries.iter().zip(&manifest.entries) {
            let ref_id: String = row.try_get("ref_id")?;
            let capability_id: String = row.try_get("capability_id")?;
            if ref_id != expected.ref_id.as_str() || capability_id != expected.capability_id.as_str() {
                return Err(CatalogError::IntegrityMismatch {
                    identity: manifest.manifest_id.to_string(),
                });
            }
        }
        Ok(())
    }

    pub async fn insert_proposal_in_transaction(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        proposal: &SurfaceProposal,
    ) -> Result<()> {
        let manifest_consumer: Option<String> =
            sqlx::query_scalar("SELECT consumer_id FROM surface_manifests WHERE manifest_id = ?")
                .bind(proposal.proposed_manifest_id.as_str())
                .fetch_optional(&mut **transaction)
                .await?;
        if manifest_consumer.as_deref() != Some(proposal.consumer_id.as_str()) {
            return Err(CatalogError::InvalidSurfaceValue {
                field: "proposal.consumer_id",
                value: proposal.consumer_id.clone(),
            });
        }
        let source_revision_set = serde_json::to_string(&proposal.source_revision_set)?;
        let diff_summary = serde_json::to_string(&proposal.diff_summary)?;
        let inserted = sqlx::query(
            r#"
            INSERT OR IGNORE INTO surface_proposals (
                proposal_id, consumer_id, base_publication_id, proposed_manifest_id,
                trigger_kind, trigger_id, source_revision_set, diff_summary,
                lifecycle, created_at, resolved_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&proposal.proposal_id)
        .bind(&proposal.consumer_id)
        .bind(&proposal.base_publication_id)
        .bind(proposal.proposed_manifest_id.as_str())
        .bind(&proposal.trigger_kind)
        .bind(&proposal.trigger_id)
        .bind(&source_revision_set)
        .bind(&diff_summary)
        .bind(proposal.lifecycle.as_str())
        .bind(proposal.created_at.to_rfc3339())
        .bind(proposal.resolved_at.map(|value| value.to_rfc3339()))
        .execute(&mut **transaction)
        .await?
        .rows_affected();
        if inserted == 0 {
            let row = sqlx::query(
                r#"
                SELECT consumer_id, base_publication_id, proposed_manifest_id, trigger_kind,
                       trigger_id, source_revision_set, diff_summary, lifecycle
                FROM surface_proposals WHERE proposal_id = ?
                "#,
            )
            .bind(&proposal.proposal_id)
            .fetch_one(&mut **transaction)
            .await?;
            let matches = row.try_get::<String, _>("consumer_id")? == proposal.consumer_id
                && row.try_get::<String, _>("proposed_manifest_id")? == proposal.proposed_manifest_id.as_str()
                && row.try_get::<String, _>("trigger_kind")? == proposal.trigger_kind
                && row.try_get::<String, _>("trigger_id")? == proposal.trigger_id
                && row.try_get::<String, _>("source_revision_set")? == source_revision_set
                && row.try_get::<String, _>("diff_summary")? == diff_summary;
            if !matches {
                return Err(CatalogError::IntegrityMismatch {
                    identity: proposal.proposal_id.clone(),
                });
            }
        }
        Ok(())
    }

    pub async fn transition_proposal_in_transaction(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        proposal_id: &str,
        expected: ProposalLifecycle,
        target: ProposalLifecycle,
    ) -> Result<()> {
        if expected != ProposalLifecycle::Pending || target == ProposalLifecycle::Pending {
            return Err(CatalogError::InvalidSurfaceValue {
                field: "proposal lifecycle transition",
                value: format!("{} -> {}", expected.as_str(), target.as_str()),
            });
        }
        let result = sqlx::query(
            r#"
            UPDATE surface_proposals
            SET lifecycle = ?, resolved_at = ?
            WHERE proposal_id = ? AND lifecycle = ?
            "#,
        )
        .bind(target.as_str())
        .bind(Utc::now().to_rfc3339())
        .bind(proposal_id)
        .bind(expected.as_str())
        .execute(&mut **transaction)
        .await?;
        if result.rows_affected() != 1 {
            return Err(CatalogError::ConcurrencyConflict {
                entity: "surface proposal",
                id: proposal_id.to_string(),
            });
        }
        Ok(())
    }

    pub async fn create_or_reuse_review_item_in_transaction(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        draft: &SurfaceReviewItemDraft,
        owners: &[SurfaceReviewOwner],
    ) -> Result<SurfaceReviewItem> {
        let proposal_consumer: Option<String> =
            sqlx::query_scalar("SELECT consumer_id FROM surface_proposals WHERE proposal_id = ?")
                .bind(&draft.created_by_proposal_id)
                .fetch_optional(&mut **transaction)
                .await?;
        if proposal_consumer.as_deref() != Some(draft.consumer_id.as_str()) {
            return Err(CatalogError::InvalidSurfaceValue {
                field: "review_item.consumer_id",
                value: draft.consumer_id.clone(),
            });
        }

        sqlx::query(
            r#"
            UPDATE surface_review_items
            SET lifecycle = 'obsolete', updated_at = ?
            WHERE consumer_id = ? AND ref_id = ? AND target_key <> ? AND lifecycle <> 'obsolete'
            "#,
        )
        .bind(Utc::now().to_rfc3339())
        .bind(&draft.consumer_id)
        .bind(draft.ref_id.as_str())
        .bind(draft.target_key.as_str())
        .execute(&mut **transaction)
        .await?;

        let now = Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO surface_review_items (
                review_item_id, created_by_proposal_id, consumer_id, ref_id,
                before_capability_id, target_capability_id, target_key,
                change_class, policy_action, lifecycle, current_decision_id,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'pending', NULL, ?, ?)
            "#,
        )
        .bind(&draft.review_item_id)
        .bind(&draft.created_by_proposal_id)
        .bind(&draft.consumer_id)
        .bind(draft.ref_id.as_str())
        .bind(draft.before_capability_id.as_ref().map(CapabilityId::as_str))
        .bind(draft.target_capability_id.as_ref().map(CapabilityId::as_str))
        .bind(draft.target_key.as_str())
        .bind(&draft.change_class)
        .bind(&draft.policy_action)
        .bind(&now)
        .bind(&now)
        .execute(&mut **transaction)
        .await?;

        let item = load_review_item_with_executor(
            &mut **transaction,
            &draft.consumer_id,
            &draft.ref_id,
            draft.target_key.as_str(),
        )
        .await?
        .ok_or_else(|| CatalogError::SurfaceNotFound {
            entity: "surface review item",
            id: draft.review_item_id.clone(),
        })?;
        if item.ref_id != draft.ref_id
            || item.before_capability_id != draft.before_capability_id
            || item.target_capability_id != draft.target_capability_id
            || item.change_class != draft.change_class
            || item.policy_action != draft.policy_action
        {
            return Err(CatalogError::IntegrityMismatch {
                identity: item.review_item_id,
            });
        }
        sqlx::query("INSERT OR IGNORE INTO surface_proposal_review_items (proposal_id, review_item_id) VALUES (?, ?)")
            .bind(&draft.created_by_proposal_id)
            .bind(&item.review_item_id)
            .execute(&mut **transaction)
            .await?;
        self.sync_review_item_owners_in_transaction(
            transaction,
            &item.review_item_id,
            &draft.created_by_proposal_id,
            owners,
        )
        .await?;
        self.load_review_item_in_transaction(transaction, &item.review_item_id)
            .await?
            .ok_or_else(|| CatalogError::SurfaceNotFound {
                entity: "surface review item",
                id: item.review_item_id,
            })
    }

    pub async fn sync_review_item_owners_in_transaction(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        review_item_id: &str,
        proposal_id: &str,
        owners: &[SurfaceReviewOwner],
    ) -> Result<()> {
        let linked: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM surface_proposal_review_items
            WHERE proposal_id = ? AND review_item_id = ?
            "#,
        )
        .bind(proposal_id)
        .bind(review_item_id)
        .fetch_one(&mut **transaction)
        .await?;
        if linked != 1 {
            return Err(CatalogError::InvalidSurfaceValue {
                field: "review owner proposal",
                value: format!("{proposal_id}/{review_item_id}"),
            });
        }
        sqlx::query("UPDATE surface_review_owners SET active = 0 WHERE review_item_id = ?")
            .bind(review_item_id)
            .execute(&mut **transaction)
            .await?;
        for owner in owners {
            sqlx::query(
                r#"
                INSERT INTO surface_review_owners (
                    review_item_id, owner_type, owner_id, active, first_proposal_id, last_proposal_id
                ) VALUES (?, ?, ?, 1, ?, ?)
                ON CONFLICT (review_item_id, owner_type, owner_id)
                DO UPDATE SET active = 1, last_proposal_id = excluded.last_proposal_id
                "#,
            )
            .bind(review_item_id)
            .bind(owner.owner_type.as_str())
            .bind(&owner.owner_id)
            .bind(proposal_id)
            .bind(proposal_id)
            .execute(&mut **transaction)
            .await?;
        }
        let active_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM surface_review_owners WHERE review_item_id = ? AND active = 1")
                .bind(review_item_id)
                .fetch_one(&mut **transaction)
                .await?;
        let lifecycle = if active_count == 0 {
            ReviewLifecycle::Obsolete
        } else {
            let current_decision: Option<String> =
                sqlx::query_scalar("SELECT current_decision_id FROM surface_review_items WHERE review_item_id = ?")
                    .bind(review_item_id)
                    .fetch_optional(&mut **transaction)
                    .await?
                    .flatten();
            if current_decision.is_some() {
                ReviewLifecycle::Resolved
            } else {
                ReviewLifecycle::Pending
            }
        };
        let updated =
            sqlx::query("UPDATE surface_review_items SET lifecycle = ?, updated_at = ? WHERE review_item_id = ?")
                .bind(lifecycle.as_str())
                .bind(Utc::now().to_rfc3339())
                .bind(review_item_id)
                .execute(&mut **transaction)
                .await?;
        if updated.rows_affected() != 1 {
            return Err(CatalogError::SurfaceNotFound {
                entity: "surface review item",
                id: review_item_id.to_string(),
            });
        }
        Ok(())
    }

    pub async fn obsolete_unrepresented_review_items_in_transaction(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        consumer_id: &str,
        proposal_id: &str,
        represented_review_item_ids: &[String],
    ) -> Result<()> {
        let existing_ids = sqlx::query_scalar::<_, String>(
            r#"
            SELECT review_item_id
            FROM surface_review_items
            WHERE consumer_id = ? AND lifecycle <> 'obsolete'
            ORDER BY review_item_id
            "#,
        )
        .bind(consumer_id)
        .fetch_all(&mut **transaction)
        .await?;
        for review_item_id in existing_ids {
            if represented_review_item_ids.contains(&review_item_id) {
                continue;
            }
            sqlx::query(
                "INSERT OR IGNORE INTO surface_proposal_review_items (proposal_id, review_item_id) VALUES (?, ?)",
            )
            .bind(proposal_id)
            .bind(&review_item_id)
            .execute(&mut **transaction)
            .await?;
            self.sync_review_item_owners_in_transaction(transaction, &review_item_id, proposal_id, &[])
                .await?;
        }
        Ok(())
    }

    pub async fn obsolete_consumer_review_items_in_transaction(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        consumer_id: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE surface_review_owners
            SET active = 0
            WHERE active = 1
              AND review_item_id IN (
                  SELECT review_item_id
                  FROM surface_review_items
                  WHERE consumer_id = ?
              )
            "#,
        )
        .bind(consumer_id)
        .execute(&mut **transaction)
        .await?;
        sqlx::query(
            r#"
            UPDATE surface_review_items
            SET lifecycle = 'obsolete', updated_at = ?
            WHERE consumer_id = ? AND lifecycle <> 'obsolete'
            "#,
        )
        .bind(Utc::now().to_rfc3339())
        .bind(consumer_id)
        .execute(&mut **transaction)
        .await?;
        Ok(())
    }

    pub async fn sync_existing_review_item_owners_in_transaction(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        consumer_id: &str,
        ref_id: &CapabilityRefId,
        target_key: &ReviewTargetKey,
        proposal_id: &str,
        owners: &[SurfaceReviewOwner],
    ) -> Result<Option<String>> {
        let Some(review_item_id) = sqlx::query_scalar::<_, String>(
            r#"
            SELECT review_item_id
            FROM surface_review_items
            WHERE consumer_id = ? AND ref_id = ? AND target_key = ? AND lifecycle <> 'obsolete'
            "#,
        )
        .bind(consumer_id)
        .bind(ref_id.as_str())
        .bind(target_key.as_str())
        .fetch_optional(&mut **transaction)
        .await?
        else {
            return Ok(None);
        };
        sqlx::query("INSERT OR IGNORE INTO surface_proposal_review_items (proposal_id, review_item_id) VALUES (?, ?)")
            .bind(proposal_id)
            .bind(&review_item_id)
            .execute(&mut **transaction)
            .await?;
        self.sync_review_item_owners_in_transaction(transaction, &review_item_id, proposal_id, owners)
            .await?;
        Ok(Some(review_item_id))
    }

    pub async fn reconcile_proposal_lifecycles_in_transaction(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        consumer_id: &str,
        current_proposal_id: &str,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            UPDATE surface_proposals
            SET lifecycle = 'superseded', resolved_at = ?
            WHERE consumer_id = ? AND proposal_id <> ? AND lifecycle = 'pending'
            "#,
        )
        .bind(&now)
        .bind(consumer_id)
        .bind(current_proposal_id)
        .execute(&mut **transaction)
        .await?;
        let pending_items: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM surface_proposal_review_items link
            JOIN surface_review_items item ON item.review_item_id = link.review_item_id
            WHERE link.proposal_id = ? AND item.lifecycle = 'pending'
            "#,
        )
        .bind(current_proposal_id)
        .fetch_one(&mut **transaction)
        .await?;
        if pending_items == 0 {
            let lifecycle: String = sqlx::query_scalar("SELECT lifecycle FROM surface_proposals WHERE proposal_id = ?")
                .bind(current_proposal_id)
                .fetch_one(&mut **transaction)
                .await?;
            match lifecycle.as_str() {
                "pending" => {
                    self.transition_proposal_in_transaction(
                        transaction,
                        current_proposal_id,
                        ProposalLifecycle::Pending,
                        ProposalLifecycle::Resolved,
                    )
                    .await?;
                }
                "resolved" => {}
                _ => {
                    return Err(CatalogError::ConcurrencyConflict {
                        entity: "surface proposal lifecycle",
                        id: current_proposal_id.to_string(),
                    });
                }
            }
        }
        Ok(())
    }

    pub async fn append_review_decision_in_transaction(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        decision: &SurfaceReviewDecisionDraft,
        expected_current_decision_id: Option<&str>,
    ) -> Result<()> {
        let payload_is_valid = match decision.resolution_action {
            ReviewResolutionAction::RemoveIntent => decision
                .resolution_payload
                .as_ref()
                .is_some_and(review_payload_has_owner),
            ReviewResolutionAction::RebindRef => decision.resolution_payload.as_ref().is_some_and(|payload| {
                review_payload_has_owner(payload)
                    && payload
                        .get("new_ref_id")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|ref_id| !ref_id.is_empty())
            }),
            _ => decision.resolution_payload.is_none(),
        };
        if !payload_is_valid {
            return Err(CatalogError::InvalidSurfaceValue {
                field: "review decision payload",
                value: decision.resolution_action.as_str().to_string(),
            });
        }
        let payload = decision
            .resolution_payload
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        sqlx::query(
            r#"
            INSERT INTO surface_review_decisions (
                decision_id, review_item_id, resolution_action, resolution_payload,
                actor, decided_at, supersedes_decision_id
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&decision.decision_id)
        .bind(&decision.review_item_id)
        .bind(decision.resolution_action.as_str())
        .bind(payload)
        .bind(&decision.actor)
        .bind(decision.decided_at.to_rfc3339())
        .bind(expected_current_decision_id)
        .execute(&mut **transaction)
        .await?;

        let updated = match expected_current_decision_id {
            Some(expected) => {
                sqlx::query(
                    r#"
                    UPDATE surface_review_items
                    SET current_decision_id = ?, lifecycle = 'resolved', updated_at = ?
                    WHERE review_item_id = ? AND current_decision_id = ?
                    "#,
                )
                .bind(&decision.decision_id)
                .bind(Utc::now().to_rfc3339())
                .bind(&decision.review_item_id)
                .bind(expected)
                .execute(&mut **transaction)
                .await?
            }
            None => {
                sqlx::query(
                    r#"
                    UPDATE surface_review_items
                    SET current_decision_id = ?, lifecycle = 'resolved', updated_at = ?
                    WHERE review_item_id = ? AND current_decision_id IS NULL
                    "#,
                )
                .bind(&decision.decision_id)
                .bind(Utc::now().to_rfc3339())
                .bind(&decision.review_item_id)
                .execute(&mut **transaction)
                .await?
            }
        };
        if updated.rows_affected() != 1 {
            sqlx::query("DELETE FROM surface_review_decisions WHERE decision_id = ?")
                .bind(&decision.decision_id)
                .execute(&mut **transaction)
                .await?;
            return Err(CatalogError::ConcurrencyConflict {
                entity: "surface review item",
                id: decision.review_item_id.clone(),
            });
        }
        Ok(())
    }

    pub async fn deactivate_review_owner_in_transaction(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        review_item_id: &str,
        owner: &SurfaceReviewOwner,
    ) -> Result<()> {
        let updated = sqlx::query(
            r#"
            UPDATE surface_review_owners
            SET active = 0
            WHERE review_item_id = ? AND owner_type = ? AND owner_id = ? AND active = 1
            "#,
        )
        .bind(review_item_id)
        .bind(owner.owner_type.as_str())
        .bind(&owner.owner_id)
        .execute(&mut **transaction)
        .await?;
        if updated.rows_affected() != 1 {
            return Err(CatalogError::ConcurrencyConflict {
                entity: "surface review owner",
                id: format!("{review_item_id}/{}/{}", owner.owner_type.as_str(), owner.owner_id),
            });
        }
        let active_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM surface_review_owners WHERE review_item_id = ? AND active = 1")
                .bind(review_item_id)
                .fetch_one(&mut **transaction)
                .await?;
        if active_count == 0 {
            sqlx::query(
                "UPDATE surface_review_items SET lifecycle = 'obsolete', updated_at = ? WHERE review_item_id = ?",
            )
            .bind(Utc::now().to_rfc3339())
            .bind(review_item_id)
            .execute(&mut **transaction)
            .await?;
        } else {
            sqlx::query(
                r#"
                UPDATE surface_review_items
                SET lifecycle = 'pending', current_decision_id = NULL, updated_at = ?
                WHERE review_item_id = ?
                "#,
            )
            .bind(Utc::now().to_rfc3339())
            .bind(review_item_id)
            .execute(&mut **transaction)
            .await?;
        }
        Ok(())
    }

    pub async fn load_review_item(
        &self,
        review_item_id: &str,
    ) -> Result<Option<SurfaceReviewItem>> {
        load_review_item_by_id(&self.pool, review_item_id).await
    }

    pub async fn list_review_items(
        &self,
        filter: &SurfaceReviewFilter,
    ) -> Result<Vec<SurfaceReviewRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT review_item_id
            FROM surface_review_items item
            WHERE (? IS NULL OR item.consumer_id = ?)
              AND (? IS NULL OR item.lifecycle = ?)
              AND (
                (? IS NULL AND ? IS NULL)
                OR EXISTS (
                    SELECT 1
                    FROM surface_review_owners owner
                    WHERE owner.review_item_id = item.review_item_id
                      AND owner.active = 1
                      AND (? IS NULL OR owner.owner_type = ?)
                      AND (? IS NULL OR owner.owner_id = ?)
                )
              )
            ORDER BY item.created_at, item.review_item_id
            "#,
        )
        .bind(filter.consumer_id.as_deref())
        .bind(filter.consumer_id.as_deref())
        .bind(filter.lifecycle.map(ReviewLifecycle::as_str))
        .bind(filter.lifecycle.map(ReviewLifecycle::as_str))
        .bind(filter.owner_type.map(ReviewOwnerType::as_str))
        .bind(filter.owner_id.as_deref())
        .bind(filter.owner_type.map(ReviewOwnerType::as_str))
        .bind(filter.owner_type.map(ReviewOwnerType::as_str))
        .bind(filter.owner_id.as_deref())
        .bind(filter.owner_id.as_deref())
        .fetch_all(&self.pool)
        .await?;
        let mut records = Vec::with_capacity(rows.len());
        for row in rows {
            let review_item_id: String = row.try_get("review_item_id")?;
            records.push(self.load_review_record(&review_item_id).await?.ok_or_else(|| {
                CatalogError::SurfaceNotFound {
                    entity: "surface review item",
                    id: review_item_id,
                }
            })?);
        }
        Ok(records)
    }

    pub async fn load_review_record(
        &self,
        review_item_id: &str,
    ) -> Result<Option<SurfaceReviewRecord>> {
        let mut connection = self.pool.acquire().await?;
        load_review_record_by_id(&mut connection, review_item_id).await
    }

    pub async fn load_review_record_in_transaction(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        review_item_id: &str,
    ) -> Result<Option<SurfaceReviewRecord>> {
        load_review_record_by_id(transaction, review_item_id).await
    }

    async fn load_review_item_in_transaction(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        review_item_id: &str,
    ) -> Result<Option<SurfaceReviewItem>> {
        load_review_item_by_id(&mut **transaction, review_item_id).await
    }

    pub async fn publish_and_bind_in_transaction(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        publication: &SurfacePublication,
        expected_generation: Option<i64>,
    ) -> Result<ConsumerSurfaceBinding> {
        let manifest_consumer: Option<String> =
            sqlx::query_scalar("SELECT consumer_id FROM surface_manifests WHERE manifest_id = ?")
                .bind(publication.manifest_id.as_str())
                .fetch_optional(&mut **transaction)
                .await?;
        if manifest_consumer.as_deref() != Some(publication.consumer_id.as_str()) {
            return Err(CatalogError::InvalidSurfaceValue {
                field: "publication.consumer_id",
                value: publication.consumer_id.clone(),
            });
        }
        let current = load_binding_with_executor(&mut **transaction, &publication.consumer_id).await?;
        match (&current, expected_generation) {
            (None, None) if publication.supersedes_publication_id.is_none() => {}
            (Some(binding), Some(expected))
                if binding.generation == expected
                    && publication.supersedes_publication_id.as_deref()
                        == Some(binding.active_publication_id.as_str()) => {}
            _ => {
                return Err(CatalogError::ConcurrencyConflict {
                    entity: "consumer surface binding",
                    id: publication.consumer_id.clone(),
                });
            }
        }

        sqlx::query(
            r#"
            INSERT INTO surface_publications (
                publication_id, consumer_id, manifest_id, proposal_id, reason,
                published_by, published_at, supersedes_publication_id
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&publication.publication_id)
        .bind(&publication.consumer_id)
        .bind(publication.manifest_id.as_str())
        .bind(&publication.proposal_id)
        .bind(&publication.reason)
        .bind(&publication.published_by)
        .bind(publication.published_at.to_rfc3339())
        .bind(&publication.supersedes_publication_id)
        .execute(&mut **transaction)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO consumer_surface_generations (consumer_id, last_generation)
            VALUES (?, ?)
            ON CONFLICT(consumer_id) DO UPDATE SET
                last_generation = MAX(last_generation, excluded.last_generation)
            "#,
        )
        .bind(&publication.consumer_id)
        .bind(expected_generation.unwrap_or(0))
        .execute(&mut **transaction)
        .await?;
        let generation: i64 = sqlx::query_scalar(
            r#"
            UPDATE consumer_surface_generations
            SET last_generation = last_generation + 1
            WHERE consumer_id = ?
            RETURNING last_generation
            "#,
        )
        .bind(&publication.consumer_id)
        .fetch_one(&mut **transaction)
        .await?;
        let binding_result = if let Some(expected) = expected_generation {
            sqlx::query(
                r#"
                UPDATE consumer_surface_bindings
                SET active_publication_id = ?, generation = ?
                WHERE consumer_id = ? AND generation = ?
                "#,
            )
            .bind(&publication.publication_id)
            .bind(generation)
            .bind(&publication.consumer_id)
            .bind(expected)
            .execute(&mut **transaction)
            .await?
        } else {
            sqlx::query(
                r#"
                INSERT OR IGNORE INTO consumer_surface_bindings (
                    consumer_id, active_publication_id, generation
                ) VALUES (?, ?, ?)
                "#,
            )
            .bind(&publication.consumer_id)
            .bind(&publication.publication_id)
            .bind(generation)
            .execute(&mut **transaction)
            .await?
        };
        if binding_result.rows_affected() != 1 {
            sqlx::query("DELETE FROM surface_publications WHERE publication_id = ?")
                .bind(&publication.publication_id)
                .execute(&mut **transaction)
                .await?;
            return Err(CatalogError::ConcurrencyConflict {
                entity: "consumer surface binding",
                id: publication.consumer_id.clone(),
            });
        }
        Ok(ConsumerSurfaceBinding {
            consumer_id: publication.consumer_id.clone(),
            active_publication_id: publication.publication_id.clone(),
            generation,
        })
    }

    pub async fn is_publication_rollback_eligible_in_transaction(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        publication_id: &str,
    ) -> Result<std::result::Result<(), Vec<RollbackBlock>>> {
        let entries = sqlx::query(
            r#"
            SELECT e.ref_id, e.capability_id AS pinned_capability_id,
                   c.capability_id AS current_capability_id, r.state
            FROM surface_publications p
            JOIN surface_manifest_entries e ON e.manifest_id = p.manifest_id
            JOIN capability_refs r ON r.ref_id = e.ref_id
            LEFT JOIN capability_ref_current c ON c.ref_id = e.ref_id
            WHERE p.publication_id = ?
            ORDER BY e.position
            "#,
        )
        .bind(publication_id)
        .fetch_all(&mut **transaction)
        .await?;
        let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM surface_publications WHERE publication_id = ?")
            .bind(publication_id)
            .fetch_one(&mut **transaction)
            .await?;
        if exists != 1 {
            return Err(CatalogError::SurfaceNotFound {
                entity: "surface publication",
                id: publication_id.to_string(),
            });
        }
        let mut blocks = Vec::new();
        for row in entries {
            let ref_id = parse_identity::<CapabilityRefId>("capability ref", row.try_get("ref_id")?)?;
            let pinned = parse_identity::<CapabilityId>("capability", row.try_get("pinned_capability_id")?)?;
            let current = row
                .try_get::<Option<String>, _>("current_capability_id")?
                .map(|value| parse_identity::<CapabilityId>("capability", value))
                .transpose()?;
            let state: String = row.try_get("state")?;
            if state != "active" || current.as_ref() != Some(&pinned) {
                blocks.push(RollbackBlock {
                    ref_id,
                    pinned_capability_id: pinned,
                    current_capability_id: current,
                });
            }
        }
        if blocks.is_empty() { Ok(Ok(())) } else { Ok(Err(blocks)) }
    }

    pub async fn load_publication_history(
        &self,
        consumer_id: &str,
    ) -> Result<Vec<SurfacePublication>> {
        let rows = sqlx::query(
            r#"
            SELECT publication_id, consumer_id, manifest_id, proposal_id, reason,
                   published_by, published_at, supersedes_publication_id
            FROM surface_publications
            WHERE consumer_id = ?
            ORDER BY published_at DESC, publication_id DESC
            "#,
        )
        .bind(consumer_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(parse_publication).collect()
    }

    pub async fn load_publication(
        &self,
        publication_id: &str,
    ) -> Result<Option<SurfacePublication>> {
        load_publication_with_executor(&self.pool, publication_id).await
    }

    pub async fn load_publication_in_transaction(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        publication_id: &str,
    ) -> Result<Option<SurfacePublication>> {
        load_publication_with_executor(&mut **transaction, publication_id).await
    }

    pub async fn load_binding(
        &self,
        consumer_id: &str,
    ) -> Result<Option<ConsumerSurfaceBinding>> {
        load_binding_with_executor(&self.pool, consumer_id).await
    }

    pub async fn load_binding_in_transaction(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        consumer_id: &str,
    ) -> Result<Option<ConsumerSurfaceBinding>> {
        load_binding_with_executor(&mut **transaction, consumer_id).await
    }

    pub async fn load_publication_manifest_id_in_transaction(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        publication_id: &str,
    ) -> Result<SurfaceManifestId> {
        let value: Option<String> =
            sqlx::query_scalar("SELECT manifest_id FROM surface_publications WHERE publication_id = ?")
                .bind(publication_id)
                .fetch_optional(&mut **transaction)
                .await?;
        parse_identity(
            "surface manifest",
            value.ok_or_else(|| CatalogError::SurfaceNotFound {
                entity: "surface publication",
                id: publication_id.to_string(),
            })?,
        )
    }

    pub async fn load_manifest_entry_capability_in_transaction(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        manifest_id: &SurfaceManifestId,
        ref_id: &CapabilityRefId,
    ) -> Result<Option<CapabilityId>> {
        sqlx::query_scalar::<_, String>(
            "SELECT capability_id FROM surface_manifest_entries WHERE manifest_id = ? AND ref_id = ?",
        )
        .bind(manifest_id.as_str())
        .bind(ref_id.as_str())
        .fetch_optional(&mut **transaction)
        .await?
        .map(|value| parse_identity("capability", value))
        .transpose()
    }

    pub async fn enqueue_reconciliation_job_in_transaction(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        job: &SurfaceReconciliationJob,
    ) -> Result<()> {
        let target_revision_set = serde_json::to_string(&job.target_revision_set)?;
        let inserted = sqlx::query(
            r#"
            INSERT OR IGNORE INTO surface_reconciliation_jobs (
                idempotency_key, cause_kind, cause_id, consumer_id, target_revision_set,
                expected_binding_generation, status, attempt_count, leased_by,
                lease_expires_at, next_attempt_at, last_error, success_receipt,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&job.idempotency_key)
        .bind(&job.cause_kind)
        .bind(&job.cause_id)
        .bind(&job.consumer_id)
        .bind(&target_revision_set)
        .bind(job.expected_binding_generation)
        .bind(job.status.as_str())
        .bind(job.attempt_count)
        .bind(&job.leased_by)
        .bind(job.lease_expires_at.map(|value| value.to_rfc3339()))
        .bind(job.next_attempt_at.to_rfc3339())
        .bind(&job.last_error)
        .bind(job.success_receipt.as_ref().map(serde_json::to_string).transpose()?)
        .bind(job.created_at.to_rfc3339())
        .bind(job.updated_at.to_rfc3339())
        .execute(&mut **transaction)
        .await?
        .rows_affected();
        if inserted == 0 {
            let existing = load_reconciliation_job_with_executor(&mut **transaction, &job.idempotency_key)
                .await?
                .ok_or_else(|| CatalogError::SurfaceNotFound {
                    entity: "surface reconciliation job",
                    id: job.idempotency_key.clone(),
                })?;
            if existing.cause_kind != job.cause_kind
                || existing.cause_id != job.cause_id
                || existing.consumer_id != job.consumer_id
                || existing.target_revision_set != job.target_revision_set
                || existing.expected_binding_generation != job.expected_binding_generation
            {
                return Err(CatalogError::IntegrityMismatch {
                    identity: job.idempotency_key.clone(),
                });
            }
        }
        Ok(())
    }

    pub async fn lease_next_reconciliation_job(
        &self,
        worker_id: &str,
        lease_duration: Duration,
    ) -> Result<Option<SurfaceReconciliationJob>> {
        let lease_delta = TimeDelta::from_std(lease_duration).map_err(|_| CatalogError::InvalidSurfaceValue {
            field: "reconciliation lease duration",
            value: format!("{lease_duration:?}"),
        })?;
        let now = Utc::now();
        let lease_expires_at = now + lease_delta;
        let mut connection = self.pool.acquire().await?;
        sqlx::query("BEGIN IMMEDIATE").execute(&mut *connection).await?;
        let result = async {
            let key: Option<String> = sqlx::query_scalar(
                r#"
                SELECT idempotency_key
                FROM surface_reconciliation_jobs
                WHERE (
                    status IN ('pending', 'failed') AND next_attempt_at <= ?
                ) OR (
                    status = 'leased' AND lease_expires_at <= ?
                )
                ORDER BY next_attempt_at, created_at, idempotency_key
                LIMIT 1
                "#,
            )
            .bind(now.to_rfc3339())
            .bind(now.to_rfc3339())
            .fetch_optional(&mut *connection)
            .await?;
            let Some(key) = key else {
                return Ok(None);
            };
            let updated = sqlx::query(
                r#"
                UPDATE surface_reconciliation_jobs
                SET status = 'leased', attempt_count = attempt_count + 1,
                    leased_by = ?, lease_expires_at = ?, updated_at = ?
                WHERE idempotency_key = ?
                  AND (
                    (status IN ('pending', 'failed') AND next_attempt_at <= ?)
                    OR (status = 'leased' AND lease_expires_at <= ?)
                  )
                "#,
            )
            .bind(worker_id)
            .bind(lease_expires_at.to_rfc3339())
            .bind(now.to_rfc3339())
            .bind(&key)
            .bind(now.to_rfc3339())
            .bind(now.to_rfc3339())
            .execute(&mut *connection)
            .await?;
            if updated.rows_affected() != 1 {
                return Err(CatalogError::ConcurrencyConflict {
                    entity: "surface reconciliation lease",
                    id: key,
                });
            }
            load_reconciliation_job_with_executor(&mut *connection, &key).await
        }
        .await;
        match result {
            Ok(job) => {
                sqlx::query("COMMIT").execute(&mut *connection).await?;
                Ok(job)
            }
            Err(error) => {
                let _ = sqlx::query("ROLLBACK").execute(&mut *connection).await;
                Err(error)
            }
        }
    }

    pub async fn record_reconciliation_failure(
        &self,
        idempotency_key: &str,
        worker_id: &str,
        error: &str,
        failed_at: DateTime<Utc>,
    ) -> Result<()> {
        let next_attempt_at = failed_at + TimeDelta::seconds(30);
        let updated = sqlx::query(
            r#"
            UPDATE surface_reconciliation_jobs
            SET status = 'failed', leased_by = NULL, lease_expires_at = NULL,
                next_attempt_at = ?, last_error = ?, updated_at = ?
            WHERE idempotency_key = ? AND status = 'leased' AND leased_by = ?
            "#,
        )
        .bind(next_attempt_at.to_rfc3339())
        .bind(error)
        .bind(failed_at.to_rfc3339())
        .bind(idempotency_key)
        .bind(worker_id)
        .execute(&self.pool)
        .await?;
        if updated.rows_affected() != 1 {
            return Err(CatalogError::ConcurrencyConflict {
                entity: "surface reconciliation job",
                id: idempotency_key.to_string(),
            });
        }
        Ok(())
    }

    pub async fn record_reconciliation_success(
        &self,
        idempotency_key: &str,
        worker_id: &str,
        receipt: serde_json::Value,
    ) -> Result<()> {
        let now = Utc::now();
        let updated = sqlx::query(
            r#"
            UPDATE surface_reconciliation_jobs
            SET status = 'succeeded', leased_by = NULL, lease_expires_at = NULL,
                success_receipt = ?, last_error = NULL, updated_at = ?
            WHERE idempotency_key = ? AND status = 'leased' AND leased_by = ?
            "#,
        )
        .bind(serde_json::to_string(&receipt)?)
        .bind(now.to_rfc3339())
        .bind(idempotency_key)
        .bind(worker_id)
        .execute(&self.pool)
        .await?;
        if updated.rows_affected() != 1 {
            return Err(CatalogError::ConcurrencyConflict {
                entity: "surface reconciliation job",
                id: idempotency_key.to_string(),
            });
        }
        Ok(())
    }

    pub async fn record_reconciliation_success_in_transaction(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        idempotency_key: &str,
        worker_id: &str,
        receipt: serde_json::Value,
    ) -> Result<()> {
        let now = Utc::now();
        let updated = sqlx::query(
            r#"
            UPDATE surface_reconciliation_jobs
            SET status = 'succeeded', leased_by = NULL, lease_expires_at = NULL,
                success_receipt = ?, last_error = NULL, updated_at = ?
            WHERE idempotency_key = ? AND status = 'leased' AND leased_by = ?
            "#,
        )
        .bind(serde_json::to_string(&receipt)?)
        .bind(now.to_rfc3339())
        .bind(idempotency_key)
        .bind(worker_id)
        .execute(&mut **transaction)
        .await?;
        if updated.rows_affected() != 1 {
            return Err(CatalogError::ConcurrencyConflict {
                entity: "surface reconciliation job",
                id: idempotency_key.to_string(),
            });
        }
        Ok(())
    }

    pub async fn load_reconciliation_job(
        &self,
        idempotency_key: &str,
    ) -> Result<Option<SurfaceReconciliationJob>> {
        load_reconciliation_job_with_executor(&self.pool, idempotency_key).await
    }

    pub async fn validate_reconciliation_lease_in_transaction(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        job: &SurfaceReconciliationJob,
        worker_id: &str,
    ) -> Result<()> {
        let current = self
            .load_and_validate_reconciliation_lease_owner_in_transaction(transaction, job, worker_id)
            .await?;
        let revisions = current
            .target_revision_set
            .as_object()
            .ok_or_else(|| CatalogError::InvalidSurfaceValue {
                field: "target revision set",
                value: current.target_revision_set.to_string(),
            })?;
        for (server_id, expected) in revisions {
            let expected = expected.as_i64().ok_or_else(|| CatalogError::InvalidSurfaceValue {
                field: "target catalog revision",
                value: expected.to_string(),
            })?;
            let actual: Option<i64> =
                sqlx::query_scalar("SELECT catalog_revision FROM capability_server_snapshots WHERE server_id = ?")
                    .bind(server_id)
                    .fetch_optional(&mut **transaction)
                    .await?;
            if actual != Some(expected) {
                return Err(CatalogError::ConcurrencyConflict {
                    entity: "capability catalog revision",
                    id: server_id.clone(),
                });
            }
        }
        let binding_generation: Option<i64> =
            sqlx::query_scalar("SELECT generation FROM consumer_surface_bindings WHERE consumer_id = ?")
                .bind(&current.consumer_id)
                .fetch_optional(&mut **transaction)
                .await?;
        if binding_generation != Some(current.expected_binding_generation) {
            return Err(CatalogError::ConcurrencyConflict {
                entity: "consumer surface binding",
                id: current.consumer_id,
            });
        }
        Ok(())
    }

    pub async fn validate_reconciliation_lease_owner_in_transaction(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        job: &SurfaceReconciliationJob,
        worker_id: &str,
    ) -> Result<()> {
        self.load_and_validate_reconciliation_lease_owner_in_transaction(transaction, job, worker_id)
            .await
            .map(|_| ())
    }

    async fn load_and_validate_reconciliation_lease_owner_in_transaction(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        job: &SurfaceReconciliationJob,
        worker_id: &str,
    ) -> Result<SurfaceReconciliationJob> {
        let current = load_reconciliation_job_with_executor(&mut **transaction, &job.idempotency_key)
            .await?
            .ok_or_else(|| CatalogError::SurfaceNotFound {
                entity: "surface reconciliation job",
                id: job.idempotency_key.clone(),
            })?;
        if current.status != ReconciliationJobStatus::Leased
            || current.leased_by.as_deref() != Some(worker_id)
            || current.attempt_count != job.attempt_count
        {
            return Err(CatalogError::ConcurrencyConflict {
                entity: "surface reconciliation lease",
                id: job.idempotency_key.clone(),
            });
        }
        Ok(current)
    }

    pub async fn enqueue_outbox_event_in_transaction(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        event: &SurfaceOutboxEvent,
    ) -> Result<()> {
        let payload = serde_json::to_string(&event.payload)?;
        let inserted = sqlx::query(
            r#"
            INSERT OR IGNORE INTO surface_outbox_events (
                event_id, event_kind, aggregate_id, payload, created_at, delivered_at
            ) VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&event.event_id)
        .bind(&event.event_kind)
        .bind(&event.aggregate_id)
        .bind(&payload)
        .bind(event.created_at.to_rfc3339())
        .bind(event.delivered_at.map(|value| value.to_rfc3339()))
        .execute(&mut **transaction)
        .await?
        .rows_affected();
        if inserted == 0 {
            let row =
                sqlx::query("SELECT event_kind, aggregate_id, payload FROM surface_outbox_events WHERE event_id = ?")
                    .bind(&event.event_id)
                    .fetch_one(&mut **transaction)
                    .await?;
            if row.try_get::<String, _>("event_kind")? != event.event_kind
                || row.try_get::<String, _>("aggregate_id")? != event.aggregate_id
                || row.try_get::<String, _>("payload")? != payload
            {
                return Err(CatalogError::IntegrityMismatch {
                    identity: event.event_id.clone(),
                });
            }
        }
        Ok(())
    }

    pub async fn load_pending_outbox_events(
        &self,
        limit: i64,
    ) -> Result<Vec<SurfaceOutboxEvent>> {
        if limit <= 0 {
            return Err(CatalogError::InvalidSurfaceValue {
                field: "outbox limit",
                value: limit.to_string(),
            });
        }
        let rows = sqlx::query(
            r#"
            SELECT event_id, event_kind, aggregate_id, payload, created_at, delivered_at
            FROM surface_outbox_events
            WHERE delivered_at IS NULL
            ORDER BY created_at, event_id
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(parse_outbox_event).collect()
    }

    pub async fn mark_outbox_event_delivered(
        &self,
        event_id: &str,
    ) -> Result<()> {
        let updated = sqlx::query(
            "UPDATE surface_outbox_events SET delivered_at = ? WHERE event_id = ? AND delivered_at IS NULL",
        )
        .bind(Utc::now().to_rfc3339())
        .bind(event_id)
        .execute(&self.pool)
        .await?;
        if updated.rows_affected() != 1 {
            return Err(CatalogError::ConcurrencyConflict {
                entity: "surface outbox event",
                id: event_id.to_string(),
            });
        }
        Ok(())
    }
}

fn review_payload_has_owner(payload: &serde_json::Value) -> bool {
    let Some(owner) = payload.get("owner") else {
        return false;
    };
    owner
        .get("owner_type")
        .and_then(serde_json::Value::as_str)
        .and_then(ReviewOwnerType::parse)
        .is_some()
        && owner
            .get("owner_id")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|owner_id| !owner_id.is_empty())
}

async fn load_review_item_with_executor<'e, E>(
    executor: E,
    consumer_id: &str,
    ref_id: &CapabilityRefId,
    target_key: &str,
) -> Result<Option<SurfaceReviewItem>>
where
    E: sqlx::Executor<'e, Database = Sqlite>,
{
    let row = sqlx::query(
        r#"
        SELECT review_item_id, created_by_proposal_id, consumer_id, ref_id,
               before_capability_id, target_capability_id, target_key,
               change_class, policy_action, lifecycle, current_decision_id
        FROM surface_review_items
        WHERE consumer_id = ? AND ref_id = ? AND target_key = ?
        "#,
    )
    .bind(consumer_id)
    .bind(ref_id.as_str())
    .bind(target_key)
    .fetch_optional(executor)
    .await?;
    row.map(parse_review_item).transpose()
}

async fn load_review_item_by_id<'e, E>(
    executor: E,
    review_item_id: &str,
) -> Result<Option<SurfaceReviewItem>>
where
    E: sqlx::Executor<'e, Database = Sqlite>,
{
    let row = sqlx::query(
        r#"
        SELECT review_item_id, created_by_proposal_id, consumer_id, ref_id,
               before_capability_id, target_capability_id, target_key,
               change_class, policy_action, lifecycle, current_decision_id
        FROM surface_review_items WHERE review_item_id = ?
        "#,
    )
    .bind(review_item_id)
    .fetch_optional(executor)
    .await?;
    row.map(parse_review_item).transpose()
}

fn parse_review_item(row: sqlx::sqlite::SqliteRow) -> Result<SurfaceReviewItem> {
    let lifecycle_value: String = row.try_get("lifecycle")?;
    Ok(SurfaceReviewItem {
        review_item_id: row.try_get("review_item_id")?,
        created_by_proposal_id: row.try_get("created_by_proposal_id")?,
        consumer_id: row.try_get("consumer_id")?,
        ref_id: parse_identity("capability ref", row.try_get("ref_id")?)?,
        before_capability_id: row
            .try_get::<Option<String>, _>("before_capability_id")?
            .map(|value| parse_identity("capability", value))
            .transpose()?,
        target_capability_id: row
            .try_get::<Option<String>, _>("target_capability_id")?
            .map(|value| parse_identity("capability", value))
            .transpose()?,
        target_key: crate::ReviewTargetKey::parse(row.try_get("target_key")?)?,
        change_class: row.try_get("change_class")?,
        policy_action: row.try_get("policy_action")?,
        lifecycle: ReviewLifecycle::parse(&lifecycle_value).ok_or_else(|| CatalogError::InvalidSurfaceValue {
            field: "review lifecycle",
            value: lifecycle_value,
        })?,
        current_decision_id: row.try_get("current_decision_id")?,
    })
}

async fn load_review_record_by_id(
    executor: &mut SqliteConnection,
    review_item_id: &str,
) -> Result<Option<SurfaceReviewRecord>> {
    let row = sqlx::query(
        r#"
        SELECT review_item_id, created_by_proposal_id, consumer_id, ref_id,
               before_capability_id, target_capability_id, target_key,
               change_class, policy_action, lifecycle, current_decision_id,
               created_at, updated_at
        FROM surface_review_items WHERE review_item_id = ?
        "#,
    )
    .bind(review_item_id)
    .fetch_optional(&mut *executor)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let created_at_value: String = row.try_get("created_at")?;
    let updated_at_value: String = row.try_get("updated_at")?;
    let created_at = parse_timestamp("review created_at", &created_at_value)?;
    let updated_at = parse_timestamp("review updated_at", &updated_at_value)?;
    let item = parse_review_item(row)?;

    let owner_rows = sqlx::query(
        r#"
        SELECT owner_type, owner_id
        FROM surface_review_owners
        WHERE review_item_id = ? AND active = 1
        "#,
    )
    .bind(review_item_id)
    .fetch_all(&mut *executor)
    .await?;
    let mut owners = owner_rows
        .into_iter()
        .map(|row| {
            let owner_type_value: String = row.try_get("owner_type")?;
            let owner_type =
                ReviewOwnerType::parse(&owner_type_value).ok_or_else(|| CatalogError::InvalidSurfaceValue {
                    field: "review owner type",
                    value: owner_type_value,
                })?;
            Ok(SurfaceReviewOwner::new(
                owner_type,
                row.try_get::<String, _>("owner_id")?,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    owners.sort_by(|left, right| {
        left.owner_type
            .cmp(&right.owner_type)
            .then(left.owner_id.cmp(&right.owner_id))
    });

    let current_decision = match item.current_decision_id.as_deref() {
        Some(decision_id) => {
            let row = sqlx::query(
                r#"
                SELECT decision_id, review_item_id, resolution_action, resolution_payload,
                       actor, decided_at, supersedes_decision_id
                FROM surface_review_decisions
                WHERE decision_id = ?
                "#,
            )
            .bind(decision_id)
            .fetch_optional(&mut *executor)
            .await?
            .ok_or_else(|| CatalogError::SurfaceNotFound {
                entity: "surface review decision",
                id: decision_id.to_string(),
            })?;
            let action_value: String = row.try_get("resolution_action")?;
            let resolution_action =
                ReviewResolutionAction::parse(&action_value).ok_or_else(|| CatalogError::InvalidSurfaceValue {
                    field: "review resolution action",
                    value: action_value,
                })?;
            let resolution_payload = row
                .try_get::<Option<String>, _>("resolution_payload")?
                .map(|value| serde_json::from_str(&value))
                .transpose()?;
            let decided_at_value: String = row.try_get("decided_at")?;
            Some(SurfaceReviewDecision {
                decision_id: row.try_get("decision_id")?,
                review_item_id: row.try_get("review_item_id")?,
                resolution_action,
                resolution_payload,
                actor: row.try_get("actor")?,
                decided_at: parse_timestamp("review decided_at", &decided_at_value)?,
                supersedes_decision_id: row.try_get("supersedes_decision_id")?,
            })
        }
        None => None,
    };

    Ok(Some(SurfaceReviewRecord {
        item,
        owners,
        current_decision,
        created_at,
        updated_at,
    }))
}

async fn load_binding_with_executor<'e, E>(
    executor: E,
    consumer_id: &str,
) -> Result<Option<ConsumerSurfaceBinding>>
where
    E: sqlx::Executor<'e, Database = Sqlite>,
{
    let row = sqlx::query(
        r#"
        SELECT consumer_id, active_publication_id, generation
        FROM consumer_surface_bindings WHERE consumer_id = ?
        "#,
    )
    .bind(consumer_id)
    .fetch_optional(executor)
    .await?;
    row.map(|row| {
        Ok(ConsumerSurfaceBinding {
            consumer_id: row.try_get("consumer_id")?,
            active_publication_id: row.try_get("active_publication_id")?,
            generation: row.try_get("generation")?,
        })
    })
    .transpose()
}

fn parse_publication(row: sqlx::sqlite::SqliteRow) -> Result<SurfacePublication> {
    let published_at: String = row.try_get("published_at")?;
    Ok(SurfacePublication {
        publication_id: row.try_get("publication_id")?,
        consumer_id: row.try_get("consumer_id")?,
        manifest_id: parse_identity("surface manifest", row.try_get("manifest_id")?)?,
        proposal_id: row.try_get("proposal_id")?,
        reason: row.try_get("reason")?,
        published_by: row.try_get("published_by")?,
        published_at: parse_timestamp("surface publication published_at", &published_at)?,
        supersedes_publication_id: row.try_get("supersedes_publication_id")?,
    })
}

async fn load_publication_with_executor<'e, E>(
    executor: E,
    publication_id: &str,
) -> Result<Option<SurfacePublication>>
where
    E: sqlx::Executor<'e, Database = Sqlite>,
{
    let row = sqlx::query(
        r#"
        SELECT publication_id, consumer_id, manifest_id, proposal_id, reason,
               published_by, published_at, supersedes_publication_id
        FROM surface_publications
        WHERE publication_id = ?
        "#,
    )
    .bind(publication_id)
    .fetch_optional(executor)
    .await?;
    row.map(parse_publication).transpose()
}

async fn load_reconciliation_job_with_executor<'e, E>(
    executor: E,
    idempotency_key: &str,
) -> Result<Option<SurfaceReconciliationJob>>
where
    E: sqlx::Executor<'e, Database = Sqlite>,
{
    let row = sqlx::query(
        r#"
        SELECT idempotency_key, cause_kind, cause_id, consumer_id, target_revision_set,
               expected_binding_generation, status, attempt_count, leased_by,
               lease_expires_at, next_attempt_at, last_error, success_receipt,
               created_at, updated_at
        FROM surface_reconciliation_jobs
        WHERE idempotency_key = ?
        "#,
    )
    .bind(idempotency_key)
    .fetch_optional(executor)
    .await?;
    row.map(parse_reconciliation_job).transpose()
}

fn parse_reconciliation_job(row: sqlx::sqlite::SqliteRow) -> Result<SurfaceReconciliationJob> {
    let status_value: String = row.try_get("status")?;
    let lease_expires_at = row.try_get::<Option<String>, _>("lease_expires_at")?;
    let next_attempt_at: String = row.try_get("next_attempt_at")?;
    let success_receipt = row.try_get::<Option<String>, _>("success_receipt")?;
    let created_at: String = row.try_get("created_at")?;
    let updated_at: String = row.try_get("updated_at")?;
    Ok(SurfaceReconciliationJob {
        idempotency_key: row.try_get("idempotency_key")?,
        cause_kind: row.try_get("cause_kind")?,
        cause_id: row.try_get("cause_id")?,
        consumer_id: row.try_get("consumer_id")?,
        target_revision_set: serde_json::from_str(&row.try_get::<String, _>("target_revision_set")?)?,
        expected_binding_generation: row.try_get("expected_binding_generation")?,
        status: ReconciliationJobStatus::parse(&status_value).ok_or_else(|| CatalogError::InvalidSurfaceValue {
            field: "reconciliation job status",
            value: status_value,
        })?,
        attempt_count: row.try_get("attempt_count")?,
        leased_by: row.try_get("leased_by")?,
        lease_expires_at: lease_expires_at
            .map(|value| parse_timestamp("reconciliation lease_expires_at", &value))
            .transpose()?,
        next_attempt_at: parse_timestamp("reconciliation next_attempt_at", &next_attempt_at)?,
        last_error: row.try_get("last_error")?,
        success_receipt: success_receipt.map(|value| serde_json::from_str(&value)).transpose()?,
        created_at: parse_timestamp("reconciliation created_at", &created_at)?,
        updated_at: parse_timestamp("reconciliation updated_at", &updated_at)?,
    })
}

fn parse_outbox_event(row: sqlx::sqlite::SqliteRow) -> Result<SurfaceOutboxEvent> {
    let created_at: String = row.try_get("created_at")?;
    let delivered_at = row.try_get::<Option<String>, _>("delivered_at")?;
    Ok(SurfaceOutboxEvent {
        event_id: row.try_get("event_id")?,
        event_kind: row.try_get("event_kind")?,
        aggregate_id: row.try_get("aggregate_id")?,
        payload: serde_json::from_str(&row.try_get::<String, _>("payload")?)?,
        created_at: parse_timestamp("outbox created_at", &created_at)?,
        delivered_at: delivered_at
            .map(|value| parse_timestamp("outbox delivered_at", &value))
            .transpose()?,
    })
}

fn parse_identity<T>(
    identity_kind: &'static str,
    value: String,
) -> Result<T>
where
    T: FromStr<Err = CatalogError>,
{
    value
        .parse()
        .map_err(|_| CatalogError::InvalidIdentity { identity_kind, value })
}

fn parse_timestamp(
    field: &'static str,
    value: &str,
) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .map_err(|_| CatalogError::InvalidTimestamp {
            field,
            value: value.to_string(),
        })
}
