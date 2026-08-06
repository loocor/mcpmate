use std::collections::{HashMap, HashSet};

use mcpmate_capability_store::{CapabilityKind, CatalogError, Result};
use sqlx::{Pool, Row, Sqlite};
use uuid::Uuid;

use super::materializer::{
    MaterializationCommit, MaterializationCoordinator, MaterializationTrigger, SurfaceAuthoringLoader,
    load_default_config_mode,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileRelationshipAction {
    Enable,
    Disable,
    Remove,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileActivationAction {
    Activate,
    Deactivate,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileCapabilityMutation {
    pub ref_id: String,
    pub kind: CapabilityKind,
}

pub struct ConsumerMaterialization {
    pub consumer_id: String,
    pub commit: MaterializationCommit,
}

pub struct ProfileCapabilityManagementResult {
    pub mutations: Vec<ProfileCapabilityMutation>,
    pub materializations: Vec<ConsumerMaterialization>,
}

pub struct ProfileActivationMutation {
    pub profile_id: String,
    pub name: String,
    pub is_active: bool,
}

pub struct ProfileActivationManagementResult {
    pub mutations: Vec<ProfileActivationMutation>,
    pub materializations: Vec<ConsumerMaterialization>,
}

pub struct ProfileDeletionManagementResult {
    pub profile_name: String,
    pub materializations: Vec<ConsumerMaterialization>,
}

pub struct ProfileSurfaceManagement;

pub struct ServerStatusManagementResult {
    pub server_id: String,
    pub server_name: String,
    pub enabled: bool,
    pub materializations: Vec<ConsumerMaterialization>,
}

pub struct ServerDirectExposureManagementResult {
    pub server_id: String,
    pub server_name: String,
    pub unify_direct_exposure_eligible: bool,
    pub materializations: Vec<ConsumerMaterialization>,
}

pub struct ServerSurfaceManagement;

impl ServerSurfaceManagement {
    pub async fn set_server_enabled(
        pool: &Pool<Sqlite>,
        server_id: &str,
        enabled: bool,
        actor: &str,
    ) -> Result<ServerStatusManagementResult> {
        let default_config_mode = load_default_config_mode(pool).await?;
        let coordinator = MaterializationCoordinator::new(pool.clone());
        let mut transaction = pool.begin().await?;
        let server_name: String = sqlx::query_scalar(
            "UPDATE server_config SET enabled = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? RETURNING name",
        )
        .bind(enabled)
        .bind(server_id)
        .fetch_optional(&mut *transaction)
        .await?
        .ok_or_else(|| CatalogError::InvalidSurfaceValue {
            field: "server status",
            value: server_id.to_string(),
        })?;
        let consumer_ids = load_server_consumer_ids(&mut transaction, server_id, &default_config_mode).await?;
        let trigger = MaterializationTrigger::for_consumer(
            "server_status_save",
            format!("{server_id}:{}", Uuid::new_v4()),
            actor,
        );
        let mut materializations = Vec::with_capacity(consumer_ids.len());
        for consumer_id in consumer_ids {
            let commit = coordinator
                .compile_consumer_in_transaction_with_default(
                    &mut transaction,
                    &consumer_id,
                    &default_config_mode,
                    &trigger,
                )
                .await?;
            materializations.push(ConsumerMaterialization { consumer_id, commit });
        }
        transaction.commit().await?;
        Ok(ServerStatusManagementResult {
            server_id: server_id.to_string(),
            server_name,
            enabled,
            materializations,
        })
    }

    pub async fn set_direct_exposure_eligible(
        pool: &Pool<Sqlite>,
        server_id: &str,
        eligible: bool,
        source_revision_set: HashMap<String, i64>,
        actor: &str,
    ) -> Result<ServerDirectExposureManagementResult> {
        let default_config_mode = load_default_config_mode(pool).await?;
        let coordinator = MaterializationCoordinator::new(pool.clone());
        let mut transaction = pool.begin().await?;
        coordinator
            .verify_catalog_revision_set_in_transaction(&mut transaction, &source_revision_set)
            .await?;
        let server_name: String = sqlx::query_scalar("SELECT name FROM server_config WHERE id = ?")
            .bind(server_id)
            .fetch_optional(&mut *transaction)
            .await?
            .ok_or_else(|| CatalogError::InvalidSurfaceValue {
                field: "server direct exposure eligibility",
                value: server_id.to_string(),
            })?;
        let consumer_ids = load_direct_server_consumer_ids(&mut transaction, server_id, &default_config_mode).await?;
        let updated = sqlx::query(
            r#"
            UPDATE server_config
            SET unify_direct_exposure_eligible = ?, updated_at = CURRENT_TIMESTAMP
            WHERE id = ?
            "#,
        )
        .bind(eligible)
        .bind(server_id)
        .execute(&mut *transaction)
        .await?;
        if updated.rows_affected() != 1 {
            return Err(CatalogError::ConcurrencyConflict {
                entity: "server direct exposure eligibility",
                id: server_id.to_string(),
            });
        }
        let trigger = MaterializationTrigger::for_consumer(
            "server_direct_exposure_eligibility_save",
            format!("{server_id}:{}", Uuid::new_v4()),
            actor,
        );
        let mut materializations = Vec::with_capacity(consumer_ids.len());
        for consumer_id in consumer_ids {
            let commit = coordinator
                .compile_consumer_in_transaction_with_default(
                    &mut transaction,
                    &consumer_id,
                    &default_config_mode,
                    &trigger,
                )
                .await?;
            materializations.push(ConsumerMaterialization { consumer_id, commit });
        }
        transaction.commit().await?;
        Ok(ServerDirectExposureManagementResult {
            server_id: server_id.to_string(),
            server_name,
            unify_direct_exposure_eligible: eligible,
            materializations,
        })
    }
}

async fn load_direct_server_consumer_ids(
    transaction: &mut sqlx::Transaction<'_, Sqlite>,
    server_id: &str,
    default_config_mode: &str,
) -> Result<Vec<String>> {
    let rows = sqlx::query_as::<_, (String, Option<String>)>(
        r#"
        SELECT DISTINCT client.identifier, client.config_mode
        FROM client
        WHERE client.approval_status = 'approved'
          AND (
                EXISTS (
                    SELECT 1
                    FROM direct_exposure_servers direct_server
                    WHERE direct_server.consumer_id = client.identifier
                      AND direct_server.server_id = ?
                )
             OR EXISTS (
                    SELECT 1
                    FROM direct_exposure_refs direct_ref
                    JOIN capability_refs capability_ref ON capability_ref.ref_id = direct_ref.ref_id
                    WHERE direct_ref.consumer_id = client.identifier
                      AND direct_ref.enabled = 1
                      AND capability_ref.server_id = ?
                )
          )
        ORDER BY client.identifier
        "#,
    )
    .bind(server_id)
    .bind(server_id)
    .fetch_all(&mut **transaction)
    .await?;
    Ok(filter_managed_consumers(rows, default_config_mode))
}

async fn load_server_consumer_ids(
    transaction: &mut sqlx::Transaction<'_, Sqlite>,
    server_id: &str,
    default_config_mode: &str,
) -> Result<Vec<String>> {
    let rows = sqlx::query_as::<_, (String, Option<String>)>(
        r#"
        SELECT DISTINCT client.identifier, client.config_mode
        FROM client
        WHERE client.approval_status = 'approved'
          AND (
                EXISTS (
                    SELECT 1
                    FROM direct_exposure_servers direct_server
                    WHERE direct_server.consumer_id = client.identifier
                      AND direct_server.server_id = ?
                )
             OR EXISTS (
                    SELECT 1
                    FROM direct_exposure_refs direct_ref
                    JOIN capability_refs capability_ref ON capability_ref.ref_id = direct_ref.ref_id
                    WHERE direct_ref.consumer_id = client.identifier
                      AND direct_ref.enabled = 1
                      AND capability_ref.server_id = ?
                )
             OR EXISTS (
                    SELECT 1
                    FROM profile
                    LEFT JOIN profile_server_relationships profile_server
                      ON profile_server.profile_id = profile.id
                    LEFT JOIN profile_capability_refs profile_ref
                      ON profile_ref.profile_id = profile.id AND profile_ref.enabled = 1
                    LEFT JOIN capability_refs capability_ref ON capability_ref.ref_id = profile_ref.ref_id
                    WHERE (profile_server.server_id = ? OR capability_ref.server_id = ?)
                      AND (
                            client.custom_profile_id = profile.id
                         OR EXISTS (
                                SELECT 1 FROM json_each(client.selected_profile_ids)
                                WHERE json_each.value = profile.id
                            )
                         OR (client.capability_source = 'activated' AND profile.is_active = 1)
                      )
                )
          )
        ORDER BY client.identifier
        "#,
    )
    .bind(server_id)
    .bind(server_id)
    .bind(server_id)
    .bind(server_id)
    .fetch_all(&mut **transaction)
    .await?;
    Ok(filter_managed_consumers(rows, default_config_mode))
}

fn filter_managed_consumers(
    rows: Vec<(String, Option<String>)>,
    default_config_mode: &str,
) -> Vec<String> {
    rows.into_iter()
        .filter_map(|(consumer_id, config_mode)| {
            let effective_mode =
                crate::config::client::init::effective_client_config_mode(config_mode.as_deref(), default_config_mode);
            crate::config::client::init::is_managed_client_config_mode(effective_mode).then_some(consumer_id)
        })
        .collect()
}

impl ProfileSurfaceManagement {
    pub async fn delete_profile(
        pool: &Pool<Sqlite>,
        profile_id: &str,
        source_revision_set: HashMap<String, i64>,
        actor: &str,
    ) -> Result<ProfileDeletionManagementResult> {
        let default_config_mode = load_default_config_mode(pool).await?;
        let coordinator = MaterializationCoordinator::new(pool.clone());
        let mut transaction = pool.begin().await?;
        coordinator
            .verify_catalog_revision_set_in_transaction(&mut transaction, &source_revision_set)
            .await?;
        let row = sqlx::query("SELECT name, is_default, role FROM profile WHERE id = ?")
            .bind(profile_id)
            .fetch_optional(&mut *transaction)
            .await?
            .ok_or_else(|| CatalogError::InvalidSurfaceValue {
                field: "profile deletion",
                value: profile_id.to_string(),
            })?;
        let profile_name: String = row.try_get("name")?;
        let is_default: bool = row.try_get("is_default")?;
        let role: String = row.try_get("role")?;
        if is_default || role == "default_anchor" {
            return Err(CatalogError::InvalidSurfaceValue {
                field: "profile deletion",
                value: "default profiles cannot be deleted".to_string(),
            });
        }
        let consumer_ids = SurfaceAuthoringLoader::load_profile_consumer_ids_in_transaction(
            &mut transaction,
            profile_id,
            &default_config_mode,
        )
        .await?;
        let deleted = sqlx::query("DELETE FROM profile WHERE id = ?")
            .bind(profile_id)
            .execute(&mut *transaction)
            .await?;
        if deleted.rows_affected() != 1 {
            return Err(CatalogError::ConcurrencyConflict {
                entity: "profile deletion",
                id: profile_id.to_string(),
            });
        }
        let trigger =
            MaterializationTrigger::for_consumer("profile_delete", format!("{profile_id}:{}", Uuid::new_v4()), actor);
        let mut materializations = Vec::with_capacity(consumer_ids.len());
        for consumer_id in consumer_ids {
            let commit = coordinator
                .compile_consumer_in_transaction_with_default(
                    &mut transaction,
                    &consumer_id,
                    &default_config_mode,
                    &trigger,
                )
                .await?;
            materializations.push(ConsumerMaterialization { consumer_id, commit });
        }
        transaction.commit().await?;
        Ok(ProfileDeletionManagementResult {
            profile_name,
            materializations,
        })
    }

    pub async fn set_profiles_active(
        pool: &Pool<Sqlite>,
        profile_ids: &[String],
        action: ProfileActivationAction,
        source_revision_set: HashMap<String, i64>,
        actor: &str,
    ) -> Result<ProfileActivationManagementResult> {
        let unique_profile_ids = profile_ids.iter().collect::<HashSet<_>>();
        if unique_profile_ids.len() != profile_ids.len() {
            return Err(CatalogError::InvalidSurfaceValue {
                field: "profile activation",
                value: "duplicate profile id".to_string(),
            });
        }

        let default_config_mode = load_default_config_mode(pool).await?;
        let coordinator = MaterializationCoordinator::new(pool.clone());
        let mut transaction = pool.begin().await?;
        coordinator
            .verify_catalog_revision_set_in_transaction(&mut transaction, &source_revision_set)
            .await?;

        let mut mutations = Vec::with_capacity(profile_ids.len());
        for profile_id in profile_ids {
            let row = sqlx::query("SELECT name, multi_select, role FROM profile WHERE id = ?")
                .bind(profile_id)
                .fetch_optional(&mut *transaction)
                .await?
                .ok_or_else(|| CatalogError::InvalidSurfaceValue {
                    field: "profile activation",
                    value: profile_id.clone(),
                })?;
            let name: String = row.try_get("name")?;
            let multi_select: bool = row.try_get("multi_select")?;
            let role: String = row.try_get("role")?;
            let is_active = action == ProfileActivationAction::Activate;
            if !is_active && role == "default_anchor" {
                return Err(CatalogError::InvalidSurfaceValue {
                    field: "profile activation",
                    value: "default anchor cannot be deactivated".to_string(),
                });
            }
            if is_active && !multi_select {
                sqlx::query(
                    r#"
                    UPDATE profile
                    SET is_active = 0, updated_at = CURRENT_TIMESTAMP
                    WHERE id != ? AND is_default = 0
                    "#,
                )
                .bind(profile_id)
                .execute(&mut *transaction)
                .await?;
            }
            let updated = sqlx::query("UPDATE profile SET is_active = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                .bind(is_active)
                .bind(profile_id)
                .execute(&mut *transaction)
                .await?;
            if updated.rows_affected() != 1 {
                return Err(CatalogError::ConcurrencyConflict {
                    entity: "profile activation",
                    id: profile_id.clone(),
                });
            }
            mutations.push(ProfileActivationMutation {
                profile_id: profile_id.clone(),
                name,
                is_active,
            });
        }

        let consumer_ids =
            SurfaceAuthoringLoader::load_activated_consumer_ids_in_transaction(&mut transaction, &default_config_mode)
                .await?;
        let trigger =
            MaterializationTrigger::for_consumer("profile_activation_save", Uuid::new_v4().to_string(), actor);
        let mut materializations = Vec::with_capacity(consumer_ids.len());
        for consumer_id in consumer_ids {
            let commit = coordinator
                .compile_consumer_in_transaction_with_default(
                    &mut transaction,
                    &consumer_id,
                    &default_config_mode,
                    &trigger,
                )
                .await?;
            materializations.push(ConsumerMaterialization { consumer_id, commit });
        }
        transaction.commit().await?;
        Ok(ProfileActivationManagementResult {
            mutations,
            materializations,
        })
    }

    pub async fn mutate_capabilities(
        pool: &Pool<Sqlite>,
        profile_id: &str,
        ref_ids: &[String],
        action: ProfileRelationshipAction,
        source_revision_set: HashMap<String, i64>,
        actor: &str,
    ) -> Result<ProfileCapabilityManagementResult> {
        let unique_ref_ids = ref_ids.iter().collect::<HashSet<_>>();
        if unique_ref_ids.len() != ref_ids.len() {
            return Err(CatalogError::InvalidSurfaceValue {
                field: "profile capability refs",
                value: "duplicate ref id".to_string(),
            });
        }

        let default_config_mode = load_default_config_mode(pool).await?;
        let coordinator = MaterializationCoordinator::new(pool.clone());
        let mut transaction = pool.begin().await?;
        coordinator
            .verify_catalog_revision_set_in_transaction(&mut transaction, &source_revision_set)
            .await?;

        let mut mutations = Vec::with_capacity(ref_ids.len());
        for ref_id in ref_ids {
            let row = sqlx::query("SELECT kind, server_id FROM capability_refs WHERE ref_id = ?")
                .bind(ref_id)
                .fetch_optional(&mut *transaction)
                .await?
                .ok_or_else(|| CatalogError::InvalidSurfaceValue {
                    field: "profile capability ref",
                    value: ref_id.clone(),
                })?;
            let kind_value: String = row.try_get("kind")?;
            let kind = CapabilityKind::parse(&kind_value).ok_or_else(|| CatalogError::InvalidSurfaceValue {
                field: "capability kind",
                value: kind_value,
            })?;
            let server_id: String = row.try_get("server_id")?;
            if action == ProfileRelationshipAction::Enable {
                let server_enabled: Option<bool> = sqlx::query_scalar(
                    r#"
                    SELECT enabled
                    FROM profile_server_relationships
                    WHERE profile_id = ? AND server_id = ?
                    "#,
                )
                .bind(profile_id)
                .bind(&server_id)
                .fetch_optional(&mut *transaction)
                .await?;
                if server_enabled != Some(true) {
                    sqlx::query(
                        r#"
                        INSERT INTO profile_capability_refs (profile_id, ref_id, enabled)
                        SELECT ?, ref_id, 0
                        FROM capability_refs
                        WHERE server_id = ? AND state = 'active'
                        ON CONFLICT(profile_id, ref_id) DO NOTHING
                        "#,
                    )
                    .bind(profile_id)
                    .bind(&server_id)
                    .execute(&mut *transaction)
                    .await?;
                }
            }
            let updated = match action {
                ProfileRelationshipAction::Enable | ProfileRelationshipAction::Disable => {
                    sqlx::query(
                        r#"
                        INSERT INTO profile_capability_refs (profile_id, ref_id, enabled)
                        VALUES (?, ?, ?)
                        ON CONFLICT(profile_id, ref_id) DO UPDATE SET enabled = excluded.enabled
                        "#,
                    )
                    .bind(profile_id)
                    .bind(ref_id)
                    .bind(action == ProfileRelationshipAction::Enable)
                    .execute(&mut *transaction)
                    .await?
                }
                ProfileRelationshipAction::Remove => {
                    sqlx::query("DELETE FROM profile_capability_refs WHERE profile_id = ? AND ref_id = ?")
                        .bind(profile_id)
                        .bind(ref_id)
                        .execute(&mut *transaction)
                        .await?
                }
            };
            if updated.rows_affected() != 1 {
                return Err(CatalogError::ConcurrencyConflict {
                    entity: "profile capability relationship",
                    id: format!("{profile_id}/{ref_id}"),
                });
            }
            match action {
                ProfileRelationshipAction::Enable => {
                    sqlx::query(
                        r#"
                        INSERT INTO profile_server_relationships (profile_id, server_id, enabled, new_ref_policy)
                        VALUES (?, ?, 1, 'follow')
                        ON CONFLICT(profile_id, server_id) DO UPDATE SET enabled = 1
                        "#,
                    )
                    .bind(profile_id)
                    .bind(&server_id)
                    .execute(&mut *transaction)
                    .await?;
                }
                ProfileRelationshipAction::Disable => {
                    let has_enabled_capabilities: bool = sqlx::query_scalar(
                        r#"
                        SELECT EXISTS(
                            SELECT 1
                            FROM capability_refs capability_ref
                            LEFT JOIN profile_capability_refs profile_ref
                              ON profile_ref.profile_id = ?
                             AND profile_ref.ref_id = capability_ref.ref_id
                            LEFT JOIN profile_server_relationships profile_server
                              ON profile_server.profile_id = ?
                             AND profile_server.server_id = capability_ref.server_id
                            WHERE capability_ref.server_id = ?
                              AND capability_ref.state = 'active'
                              AND CASE
                                      WHEN profile_ref.ref_id IS NULL THEN COALESCE(profile_server.enabled, 0)
                                      ELSE profile_ref.enabled
                                  END = 1
                        )
                        "#,
                    )
                    .bind(profile_id)
                    .bind(profile_id)
                    .bind(&server_id)
                    .fetch_one(&mut *transaction)
                    .await?;
                    if !has_enabled_capabilities {
                        sqlx::query(
                            r#"
                            UPDATE profile_server_relationships
                            SET enabled = 0
                            WHERE profile_id = ? AND server_id = ?
                            "#,
                        )
                        .bind(profile_id)
                        .bind(&server_id)
                        .execute(&mut *transaction)
                        .await?;
                    }
                }
                ProfileRelationshipAction::Remove => {}
            }
            mutations.push(ProfileCapabilityMutation {
                ref_id: ref_id.clone(),
                kind,
            });
        }

        let materializations = Self::materialize_profile_consumers(
            &coordinator,
            &mut transaction,
            profile_id,
            &default_config_mode,
            actor,
            "profile_capability_save",
        )
        .await?;
        transaction.commit().await?;
        Ok(ProfileCapabilityManagementResult {
            mutations,
            materializations,
        })
    }

    pub async fn mutate_server(
        pool: &Pool<Sqlite>,
        profile_id: &str,
        server_id: &str,
        action: ProfileRelationshipAction,
        source_revision_set: HashMap<String, i64>,
        actor: &str,
    ) -> Result<Vec<ConsumerMaterialization>> {
        Self::mutate_servers(
            pool,
            profile_id,
            &[server_id.to_string()],
            action,
            source_revision_set,
            actor,
        )
        .await
    }

    pub async fn mutate_servers(
        pool: &Pool<Sqlite>,
        profile_id: &str,
        server_ids: &[String],
        action: ProfileRelationshipAction,
        source_revision_set: HashMap<String, i64>,
        actor: &str,
    ) -> Result<Vec<ConsumerMaterialization>> {
        let unique_server_ids = server_ids.iter().collect::<HashSet<_>>();
        if unique_server_ids.len() != server_ids.len() {
            return Err(CatalogError::InvalidSurfaceValue {
                field: "profile servers",
                value: "duplicate server id".to_string(),
            });
        }
        let default_config_mode = load_default_config_mode(pool).await?;
        let coordinator = MaterializationCoordinator::new(pool.clone());
        let mut transaction = pool.begin().await?;
        coordinator
            .verify_catalog_revision_set_in_transaction(&mut transaction, &source_revision_set)
            .await?;
        for server_id in server_ids {
            let server_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM server_config WHERE id = ?)")
                .bind(server_id)
                .fetch_one(&mut *transaction)
                .await?;
            if !server_exists {
                return Err(CatalogError::InvalidSurfaceValue {
                    field: "profile server",
                    value: server_id.clone(),
                });
            }
            match action {
                ProfileRelationshipAction::Enable => {
                    sqlx::query(
                        r#"
                        INSERT INTO profile_server_relationships (profile_id, server_id, enabled, new_ref_policy)
                        VALUES (?, ?, 1, 'follow')
                        ON CONFLICT(profile_id, server_id) DO UPDATE SET enabled = 1
                        "#,
                    )
                    .bind(profile_id)
                    .bind(server_id)
                    .execute(&mut *transaction)
                    .await?;
                    sqlx::query(
                        r#"
                        INSERT INTO profile_capability_refs (profile_id, ref_id, enabled)
                        SELECT ?, ref_id, 1
                        FROM capability_refs
                        WHERE server_id = ? AND state = 'active'
                        ON CONFLICT(profile_id, ref_id) DO UPDATE SET enabled = 1
                        "#,
                    )
                    .bind(profile_id)
                    .bind(server_id)
                    .execute(&mut *transaction)
                    .await?;
                }
                ProfileRelationshipAction::Disable => {
                    let updated = sqlx::query(
                        "UPDATE profile_server_relationships SET enabled = 0 WHERE profile_id = ? AND server_id = ?",
                    )
                    .bind(profile_id)
                    .bind(server_id)
                    .execute(&mut *transaction)
                    .await?;
                    if updated.rows_affected() != 1 {
                        return Err(CatalogError::ConcurrencyConflict {
                            entity: "profile server relationship",
                            id: format!("{profile_id}/{server_id}"),
                        });
                    }
                    sqlx::query(
                        r#"
                        UPDATE profile_capability_refs
                        SET enabled = 0
                        WHERE profile_id = ?
                          AND ref_id IN (SELECT ref_id FROM capability_refs WHERE server_id = ?)
                        "#,
                    )
                    .bind(profile_id)
                    .bind(server_id)
                    .execute(&mut *transaction)
                    .await?;
                    sqlx::query(
                        r#"
                        INSERT INTO profile_capability_refs (profile_id, ref_id, enabled)
                        SELECT ?, ref_id, 0
                        FROM capability_refs
                        WHERE server_id = ? AND state = 'active'
                        ON CONFLICT(profile_id, ref_id) DO UPDATE SET enabled = 0
                        "#,
                    )
                    .bind(profile_id)
                    .bind(server_id)
                    .execute(&mut *transaction)
                    .await?;
                }
                ProfileRelationshipAction::Remove => {
                    sqlx::query(
                        r#"
                        DELETE FROM profile_capability_refs
                        WHERE profile_id = ?
                          AND ref_id IN (SELECT ref_id FROM capability_refs WHERE server_id = ?)
                        "#,
                    )
                    .bind(profile_id)
                    .bind(server_id)
                    .execute(&mut *transaction)
                    .await?;
                    let deleted =
                        sqlx::query("DELETE FROM profile_server_relationships WHERE profile_id = ? AND server_id = ?")
                            .bind(profile_id)
                            .bind(server_id)
                            .execute(&mut *transaction)
                            .await?;
                    if deleted.rows_affected() != 1 {
                        return Err(CatalogError::ConcurrencyConflict {
                            entity: "profile server relationship",
                            id: format!("{profile_id}/{server_id}"),
                        });
                    }
                }
            }
        }

        let materializations = Self::materialize_profile_consumers(
            &coordinator,
            &mut transaction,
            profile_id,
            &default_config_mode,
            actor,
            "profile_server_save",
        )
        .await?;
        transaction.commit().await?;
        Ok(materializations)
    }

    pub async fn replace_servers(
        pool: &Pool<Sqlite>,
        profile_id: &str,
        server_ids: &[String],
        source_revision_set: HashMap<String, i64>,
        actor: &str,
    ) -> Result<Vec<ConsumerMaterialization>> {
        let unique_server_ids = server_ids.iter().collect::<HashSet<_>>();
        if unique_server_ids.len() != server_ids.len() {
            return Err(CatalogError::InvalidSurfaceValue {
                field: "profile servers",
                value: "duplicate server id".to_string(),
            });
        }
        let default_config_mode = load_default_config_mode(pool).await?;
        let coordinator = MaterializationCoordinator::new(pool.clone());
        let mut transaction = pool.begin().await?;
        coordinator
            .verify_catalog_revision_set_in_transaction(&mut transaction, &source_revision_set)
            .await?;
        let profile_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM profile WHERE id = ?)")
            .bind(profile_id)
            .fetch_one(&mut *transaction)
            .await?;
        if !profile_exists {
            return Err(CatalogError::InvalidSurfaceValue {
                field: "profile",
                value: profile_id.to_string(),
            });
        }
        for server_id in server_ids {
            let server_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM server_config WHERE id = ?)")
                .bind(server_id)
                .fetch_one(&mut *transaction)
                .await?;
            if !server_exists {
                return Err(CatalogError::InvalidSurfaceValue {
                    field: "profile server",
                    value: server_id.clone(),
                });
            }
        }
        let existing_server_ids: Vec<String> =
            sqlx::query_scalar("SELECT server_id FROM profile_server_relationships WHERE profile_id = ?")
                .bind(profile_id)
                .fetch_all(&mut *transaction)
                .await?;
        for existing_server_id in existing_server_ids {
            if unique_server_ids.contains(&existing_server_id) {
                continue;
            }
            sqlx::query(
                r#"
                DELETE FROM profile_capability_refs
                WHERE profile_id = ?
                  AND ref_id IN (SELECT ref_id FROM capability_refs WHERE server_id = ?)
                "#,
            )
            .bind(profile_id)
            .bind(&existing_server_id)
            .execute(&mut *transaction)
            .await?;
            sqlx::query("DELETE FROM profile_server_relationships WHERE profile_id = ? AND server_id = ?")
                .bind(profile_id)
                .bind(&existing_server_id)
                .execute(&mut *transaction)
                .await?;
        }
        for server_id in server_ids {
            sqlx::query(
                r#"
                INSERT INTO profile_server_relationships (profile_id, server_id, enabled, new_ref_policy)
                VALUES (?, ?, 1, 'follow')
                ON CONFLICT(profile_id, server_id) DO NOTHING
                "#,
            )
            .bind(profile_id)
            .bind(server_id)
            .execute(&mut *transaction)
            .await?;
        }
        let materializations = Self::materialize_profile_consumers(
            &coordinator,
            &mut transaction,
            profile_id,
            &default_config_mode,
            actor,
            "profile_server_save",
        )
        .await?;
        transaction.commit().await?;
        Ok(materializations)
    }

    async fn materialize_profile_consumers(
        coordinator: &MaterializationCoordinator,
        transaction: &mut sqlx::Transaction<'_, Sqlite>,
        profile_id: &str,
        default_config_mode: &str,
        actor: &str,
        trigger_kind: &str,
    ) -> Result<Vec<ConsumerMaterialization>> {
        let consumer_ids = SurfaceAuthoringLoader::load_profile_consumer_ids_in_transaction(
            transaction,
            profile_id,
            default_config_mode,
        )
        .await?;
        let trigger =
            MaterializationTrigger::for_consumer(trigger_kind, format!("{profile_id}:{}", Uuid::new_v4()), actor);
        let mut commits = Vec::with_capacity(consumer_ids.len());
        for consumer_id in consumer_ids {
            let commit = coordinator
                .compile_consumer_in_transaction_with_default(transaction, &consumer_id, default_config_mode, &trigger)
                .await?;
            commits.push(ConsumerMaterialization { consumer_id, commit });
        }
        Ok(commits)
    }
}
