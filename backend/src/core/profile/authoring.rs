use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::str::FromStr;

use mcpmate_capability_store::CatalogError;
use sqlx::{Pool, Row, Sqlite, Transaction};

use crate::common::profile::{ProfileRole, ProfileType};
use crate::config::models::Profile;
use crate::core::capability::management::ConsumerMaterialization;
use crate::core::capability::materializer::{
    MaterializationCoordinator, MaterializationTrigger, SurfaceAuthoringLoader, load_default_config_mode,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileAuthoringCommand {
    pub id: Option<String>,
    pub expected_authoring_generation: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub profile_type: String,
    pub multi_select: bool,
    pub priority: i32,
    pub is_active: bool,
    pub is_default: bool,
    pub server_ids: Vec<String>,
    pub clone_from_id: Option<String>,
}

#[derive(Debug)]
pub struct ProfileAuthoringView {
    pub profile: Profile,
    pub server_ids: Vec<String>,
}

#[derive(Debug)]
pub struct ProfileAuthoringSaveResult {
    pub profile: Profile,
    pub server_ids: Vec<String>,
    pub materializations: Vec<ConsumerMaterialization>,
    pub activation_delta: Option<bool>,
    pub automatically_deactivated_profile_ids: Vec<String>,
    pub server_relationship_deltas: Vec<ProfileServerRelationshipDelta>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileServerRelationshipDelta {
    pub server_id: String,
    pub server_name: String,
    pub enabled: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ProfileAuthoringError {
    #[error("invalid Profile authoring request: {0}")]
    InvalidRequest(String),
    #[error("invalid Profile authoring target")]
    InvalidTarget { dependency_server_ids: Vec<String> },
    #[error("Profile '{profile_id}' was not found")]
    NotFound { profile_id: String },
    #[error("Profile was changed by another author")]
    ProfileAuthoringChanged { current_authoring_generation: i64 },
    #[error("Consumer binding changed during Profile authoring")]
    ConsumerBindingChanged { dependency_server_ids: Vec<String> },
    #[error("Profile authoring persistence failed")]
    Persistence(#[source] CatalogError),
    #[error("Profile authoring database operation failed")]
    Database(#[source] sqlx::Error),
}

impl From<sqlx::Error> for ProfileAuthoringError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

#[derive(Clone)]
pub struct ProfileAuthoringService {
    pool: Pool<Sqlite>,
}

impl ProfileAuthoringService {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn view(
        &self,
        profile_id: &str,
    ) -> Result<ProfileAuthoringView, ProfileAuthoringError> {
        let mut transaction = self.pool.begin().await?;
        let view = Self::view_in_transaction(&mut transaction, profile_id).await?;
        transaction.commit().await?;
        Ok(view)
    }

    pub(crate) async fn view_in_transaction(
        transaction: &mut Transaction<'_, Sqlite>,
        profile_id: &str,
    ) -> Result<ProfileAuthoringView, ProfileAuthoringError> {
        let profile = sqlx::query_as::<_, Profile>("SELECT * FROM profile WHERE id = ?")
            .bind(profile_id)
            .fetch_optional(&mut **transaction)
            .await?
            .ok_or_else(|| ProfileAuthoringError::NotFound {
                profile_id: profile_id.to_string(),
            })?;
        let server_ids = load_server_ids_in_transaction(transaction, profile_id).await?;
        Ok(ProfileAuthoringView { profile, server_ids })
    }

    pub async fn save(
        &self,
        mut command: ProfileAuthoringCommand,
        actor: &str,
    ) -> Result<ProfileAuthoringSaveResult, ProfileAuthoringError> {
        validate_command(&command)?;
        command.server_ids = command
            .server_ids
            .into_iter()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        self.validate_targets(&command).await?;

        let coordinator = MaterializationCoordinator::new(self.pool.clone());
        let mut transaction = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let default_config_mode = load_default_config_mode(&self.pool)
            .await
            .map_err(ProfileAuthoringError::Persistence)?;
        validate_targets_in_transaction(&mut transaction, &command).await?;
        let previous_relationships = match command.id.as_deref() {
            Some(profile_id) => load_server_relationships_in_transaction(&mut transaction, profile_id).await?,
            None => BTreeMap::new(),
        };

        let (profile_id, previous_active) = match (&command.id, command.expected_authoring_generation) {
            (None, None) => (create_profile(&mut transaction, &command).await?, false),
            (Some(profile_id), Some(expected_generation)) => {
                update_profile_cas(&mut transaction, profile_id, expected_generation, &command).await?
            }
            _ => unreachable!("request shape was validated before the transaction"),
        };

        if let Some(source_profile_id) = command.clone_from_id.as_deref() {
            clone_profile_intent(&mut transaction, &profile_id, source_profile_id).await?;
        }
        replace_server_relationships(&mut transaction, &profile_id, &command.server_ids).await?;

        let automatically_deactivated = reconcile_activation_rules(&mut transaction, &profile_id, &command).await?;
        let activation_changed = previous_active != command.is_active;
        let consumer_ids = load_affected_consumer_ids(
            &mut transaction,
            &profile_id,
            &automatically_deactivated,
            activation_changed,
            &default_config_mode,
        )
        .await?;
        let trigger = MaterializationTrigger::for_consumer(
            "profile_authoring_save",
            format!("{profile_id}:{}", uuid::Uuid::new_v4()),
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
                .await
                .map_err(map_materialization_error)?;
            materializations.push(ConsumerMaterialization { consumer_id, commit });
        }

        let profile = sqlx::query_as::<_, Profile>("SELECT * FROM profile WHERE id = ?")
            .bind(&profile_id)
            .fetch_one(&mut *transaction)
            .await?;
        let server_ids = load_server_ids_in_transaction(&mut transaction, &profile_id).await?;
        let current_relationships = load_server_relationships_in_transaction(&mut transaction, &profile_id).await?;
        let server_relationship_deltas = relationship_deltas(&previous_relationships, &current_relationships);
        transaction.commit().await?;

        Ok(ProfileAuthoringSaveResult {
            profile,
            server_ids,
            materializations,
            activation_delta: activation_changed.then_some(command.is_active),
            automatically_deactivated_profile_ids: automatically_deactivated,
            server_relationship_deltas,
        })
    }

    async fn validate_targets(
        &self,
        command: &ProfileAuthoringCommand,
    ) -> Result<(), ProfileAuthoringError> {
        let missing = missing_server_ids(&self.pool, &command.server_ids).await?;
        if !missing.is_empty() {
            return Err(ProfileAuthoringError::InvalidTarget {
                dependency_server_ids: missing,
            });
        }
        if let Some(source_profile_id) = command.clone_from_id.as_deref() {
            let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM profile WHERE id = ?)")
                .bind(source_profile_id)
                .fetch_one(&self.pool)
                .await?;
            if !exists {
                return Err(ProfileAuthoringError::NotFound {
                    profile_id: source_profile_id.to_string(),
                });
            }
        }
        Ok(())
    }
}

fn validate_command(command: &ProfileAuthoringCommand) -> Result<(), ProfileAuthoringError> {
    match (&command.id, command.expected_authoring_generation) {
        (None, None) | (Some(_), Some(_)) => {}
        _ => {
            return Err(ProfileAuthoringError::InvalidRequest(
                "create requires no id or generation; update requires both".to_string(),
            ));
        }
    }
    if command.name.trim().is_empty() {
        return Err(ProfileAuthoringError::InvalidRequest(
            "Profile name must not be empty".to_string(),
        ));
    }
    ProfileType::from_str(&command.profile_type).map_err(|_| {
        ProfileAuthoringError::InvalidRequest(format!("invalid Profile type '{}'", command.profile_type))
    })?;
    if command.is_default && !command.is_active {
        return Err(ProfileAuthoringError::InvalidRequest(
            "default Profiles must remain active".to_string(),
        ));
    }
    Ok(())
}

async fn create_profile(
    transaction: &mut Transaction<'_, Sqlite>,
    command: &ProfileAuthoringCommand,
) -> Result<String, ProfileAuthoringError> {
    let duplicate: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM profile WHERE name = ?)")
        .bind(&command.name)
        .fetch_one(&mut **transaction)
        .await?;
    if duplicate {
        return Err(ProfileAuthoringError::InvalidRequest(format!(
            "Profile with name '{}' already exists",
            command.name
        )));
    }
    let profile_id = crate::generate_id!("prof");
    sqlx::query(
        r#"
        INSERT INTO profile (
            id, name, description, type, role, multi_select, priority,
            is_active, is_default, authoring_generation
        ) VALUES (?, ?, ?, ?, 'user', ?, ?, ?, ?, 0)
        "#,
    )
    .bind(&profile_id)
    .bind(&command.name)
    .bind(&command.description)
    .bind(&command.profile_type)
    .bind(command.multi_select)
    .bind(command.priority)
    .bind(command.is_active)
    .bind(command.is_default)
    .execute(&mut **transaction)
    .await?;
    Ok(profile_id)
}

async fn update_profile_cas(
    transaction: &mut Transaction<'_, Sqlite>,
    profile_id: &str,
    expected_generation: i64,
    command: &ProfileAuthoringCommand,
) -> Result<(String, bool), ProfileAuthoringError> {
    let current = sqlx::query("SELECT role, is_active FROM profile WHERE id = ?")
        .bind(profile_id)
        .fetch_optional(&mut **transaction)
        .await?;
    let Some(current) = current else {
        return Err(ProfileAuthoringError::NotFound {
            profile_id: profile_id.to_string(),
        });
    };
    let role: ProfileRole = current.try_get("role")?;
    let previous_active: bool = current.try_get("is_active")?;
    if role.is_default_anchor() && (!command.is_active || !command.is_default) {
        return Err(ProfileAuthoringError::InvalidRequest(
            "default anchor Profile must stay active and default".to_string(),
        ));
    }
    let duplicate: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM profile WHERE name = ? AND id != ?)")
        .bind(&command.name)
        .bind(profile_id)
        .fetch_one(&mut **transaction)
        .await?;
    if duplicate {
        return Err(ProfileAuthoringError::InvalidRequest(format!(
            "Profile with name '{}' already exists",
            command.name
        )));
    }
    let generation: Option<i64> = sqlx::query_scalar(
        r#"
        UPDATE profile
        SET name = ?, description = ?, type = ?, multi_select = ?, priority = ?,
            is_active = ?, is_default = ?,
            authoring_generation = authoring_generation + 1,
            updated_at = CURRENT_TIMESTAMP
        WHERE id = ? AND authoring_generation = ?
        RETURNING authoring_generation
        "#,
    )
    .bind(&command.name)
    .bind(&command.description)
    .bind(&command.profile_type)
    .bind(command.multi_select)
    .bind(command.priority)
    .bind(command.is_active)
    .bind(command.is_default)
    .bind(profile_id)
    .bind(expected_generation)
    .fetch_optional(&mut **transaction)
    .await?;
    if generation.is_none() {
        let current_authoring_generation: i64 =
            sqlx::query_scalar("SELECT authoring_generation FROM profile WHERE id = ?")
                .bind(profile_id)
                .fetch_one(&mut **transaction)
                .await?;
        return Err(ProfileAuthoringError::ProfileAuthoringChanged {
            current_authoring_generation,
        });
    }
    Ok((profile_id.to_string(), previous_active))
}

async fn clone_profile_intent(
    transaction: &mut Transaction<'_, Sqlite>,
    target_profile_id: &str,
    source_profile_id: &str,
) -> Result<(), ProfileAuthoringError> {
    sqlx::query(
        r#"
        INSERT INTO profile_server_relationships (profile_id, server_id, enabled, new_ref_policy)
        SELECT ?, server_id, enabled, new_ref_policy
        FROM profile_server_relationships
        WHERE profile_id = ?
        ON CONFLICT(profile_id, server_id) DO UPDATE SET
            enabled = excluded.enabled,
            new_ref_policy = excluded.new_ref_policy
        "#,
    )
    .bind(target_profile_id)
    .bind(source_profile_id)
    .execute(&mut **transaction)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO profile_capability_refs (profile_id, ref_id, enabled)
        SELECT ?, ref_id, enabled
        FROM profile_capability_refs
        WHERE profile_id = ?
        ON CONFLICT(profile_id, ref_id) DO UPDATE SET enabled = excluded.enabled
        "#,
    )
    .bind(target_profile_id)
    .bind(source_profile_id)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn replace_server_relationships(
    transaction: &mut Transaction<'_, Sqlite>,
    profile_id: &str,
    server_ids: &[String],
) -> Result<(), ProfileAuthoringError> {
    let selected = server_ids.iter().collect::<HashSet<_>>();
    let existing: Vec<String> =
        sqlx::query_scalar("SELECT server_id FROM profile_server_relationships WHERE profile_id = ?")
            .bind(profile_id)
            .fetch_all(&mut **transaction)
            .await?;
    for existing_server_id in existing {
        if selected.contains(&existing_server_id) {
            continue;
        }
        sqlx::query("DELETE FROM profile_server_relationships WHERE profile_id = ? AND server_id = ?")
            .bind(profile_id)
            .bind(&existing_server_id)
            .execute(&mut **transaction)
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
        .execute(&mut **transaction)
        .await?;
    }
    sqlx::query(
        r#"
        DELETE FROM profile_capability_refs
        WHERE profile_id = ?
          AND ref_id IN (
              SELECT capability_ref.ref_id
              FROM capability_refs capability_ref
              WHERE NOT EXISTS (
                  SELECT 1
                  FROM profile_server_relationships profile_server
                  WHERE profile_server.profile_id = ?
                    AND profile_server.server_id = capability_ref.server_id
              )
          )
        "#,
    )
    .bind(profile_id)
    .bind(profile_id)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn reconcile_activation_rules(
    transaction: &mut Transaction<'_, Sqlite>,
    profile_id: &str,
    command: &ProfileAuthoringCommand,
) -> Result<Vec<String>, ProfileAuthoringError> {
    if !command.is_active || command.multi_select {
        return Ok(Vec::new());
    }
    Ok(sqlx::query_scalar(
        r#"
        UPDATE profile
        SET is_active = 0,
            authoring_generation = authoring_generation + 1,
            updated_at = CURRENT_TIMESTAMP
        WHERE id != ? AND is_default = 0 AND is_active = 1
        RETURNING id
        "#,
    )
    .bind(profile_id)
    .fetch_all(&mut **transaction)
    .await?)
}

async fn load_affected_consumer_ids(
    transaction: &mut Transaction<'_, Sqlite>,
    profile_id: &str,
    automatically_deactivated: &[String],
    activation_changed: bool,
    default_config_mode: &str,
) -> Result<Vec<String>, ProfileAuthoringError> {
    let mut consumer_ids =
        SurfaceAuthoringLoader::load_profile_consumer_ids_in_transaction(transaction, profile_id, default_config_mode)
            .await
            .map_err(ProfileAuthoringError::Persistence)?
            .into_iter()
            .collect::<BTreeSet<_>>();
    for deactivated_profile_id in automatically_deactivated {
        consumer_ids.extend(
            SurfaceAuthoringLoader::load_profile_consumer_ids_in_transaction(
                transaction,
                deactivated_profile_id,
                default_config_mode,
            )
            .await
            .map_err(ProfileAuthoringError::Persistence)?,
        );
    }
    if activation_changed || !automatically_deactivated.is_empty() {
        consumer_ids.extend(
            SurfaceAuthoringLoader::load_activated_consumer_ids_in_transaction(transaction, default_config_mode)
                .await
                .map_err(ProfileAuthoringError::Persistence)?,
        );
    }
    Ok(consumer_ids.into_iter().collect())
}

fn map_materialization_error(error: CatalogError) -> ProfileAuthoringError {
    match error {
        CatalogError::ConcurrencyConflict {
            entity: "consumer surface binding",
            ..
        } => ProfileAuthoringError::ConsumerBindingChanged {
            dependency_server_ids: Vec::new(),
        },
        error => ProfileAuthoringError::Persistence(error),
    }
}

async fn validate_targets_in_transaction(
    transaction: &mut Transaction<'_, Sqlite>,
    command: &ProfileAuthoringCommand,
) -> Result<(), ProfileAuthoringError> {
    let missing = missing_server_ids_in_transaction(transaction, &command.server_ids).await?;
    if !missing.is_empty() {
        return Err(ProfileAuthoringError::InvalidTarget {
            dependency_server_ids: missing,
        });
    }
    if let Some(source_profile_id) = command.clone_from_id.as_deref() {
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM profile WHERE id = ?)")
            .bind(source_profile_id)
            .fetch_one(&mut **transaction)
            .await?;
        if !exists {
            return Err(ProfileAuthoringError::NotFound {
                profile_id: source_profile_id.to_string(),
            });
        }
    }
    Ok(())
}

async fn missing_server_ids<'e, E>(
    executor: E,
    server_ids: &[String],
) -> Result<Vec<String>, ProfileAuthoringError>
where
    E: Copy + sqlx::Executor<'e, Database = Sqlite>,
{
    let mut missing = Vec::new();
    for server_id in server_ids {
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM server_config WHERE id = ?)")
            .bind(server_id)
            .fetch_one(executor)
            .await?;
        if !exists {
            missing.push(server_id.clone());
        }
    }
    Ok(missing)
}

async fn missing_server_ids_in_transaction(
    transaction: &mut Transaction<'_, Sqlite>,
    server_ids: &[String],
) -> Result<Vec<String>, ProfileAuthoringError> {
    let mut missing = Vec::new();
    for server_id in server_ids {
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM server_config WHERE id = ?)")
            .bind(server_id)
            .fetch_one(&mut **transaction)
            .await?;
        if !exists {
            missing.push(server_id.clone());
        }
    }
    Ok(missing)
}

async fn load_server_ids_in_transaction(
    transaction: &mut Transaction<'_, Sqlite>,
    profile_id: &str,
) -> Result<Vec<String>, ProfileAuthoringError> {
    Ok(
        sqlx::query_scalar(
            "SELECT server_id FROM profile_server_relationships WHERE profile_id = ? ORDER BY server_id",
        )
        .bind(profile_id)
        .fetch_all(&mut **transaction)
        .await?,
    )
}

async fn load_server_relationships_in_transaction(
    transaction: &mut Transaction<'_, Sqlite>,
    profile_id: &str,
) -> Result<BTreeMap<String, (String, bool)>, ProfileAuthoringError> {
    let rows = sqlx::query_as::<_, (String, String, bool)>(
        r#"
        SELECT relationship.server_id,
               COALESCE(server.name, snapshot.server_name, relationship.server_id),
               relationship.enabled
        FROM profile_server_relationships relationship
        LEFT JOIN server_config server ON server.id = relationship.server_id
        LEFT JOIN capability_server_snapshots snapshot ON snapshot.server_id = relationship.server_id
        WHERE relationship.profile_id = ?
        ORDER BY relationship.server_id
        "#,
    )
    .bind(profile_id)
    .fetch_all(&mut **transaction)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(server_id, server_name, enabled)| (server_id, (server_name, enabled)))
        .collect())
}

fn relationship_deltas(
    previous: &BTreeMap<String, (String, bool)>,
    current: &BTreeMap<String, (String, bool)>,
) -> Vec<ProfileServerRelationshipDelta> {
    let server_ids = previous.keys().chain(current.keys()).collect::<BTreeSet<_>>();
    server_ids
        .into_iter()
        .filter_map(|server_id| match (previous.get(server_id), current.get(server_id)) {
            (Some((server_name, _)), None) => Some(ProfileServerRelationshipDelta {
                server_id: server_id.clone(),
                server_name: server_name.clone(),
                enabled: false,
            }),
            (before, Some((server_name, enabled))) if before.map(|(_, value)| value) != Some(enabled) => {
                Some(ProfileServerRelationshipDelta {
                    server_id: server_id.clone(),
                    server_name: server_name.clone(),
                    enabled: *enabled,
                })
            }
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::{ProfileAuthoringCommand, ProfileAuthoringService};

    #[tokio::test]
    async fn save_reads_default_mode_after_acquiring_the_authoring_write_lock() {
        let directory = tempfile::tempdir().unwrap();
        let options = SqliteConnectOptions::new()
            .filename(directory.path().join("authoring.db"))
            .create_if_missing(true)
            .busy_timeout(Duration::from_secs(2));
        let pool = SqlitePoolOptions::new()
            .min_connections(2)
            .max_connections(2)
            .connect_with(options)
            .await
            .unwrap();
        crate::test_helpers::prepare_config_database(&pool).await;
        crate::system::settings::initialize_settings_file(&pool).await.unwrap();
        crate::system::settings::set_default_config_mode(&pool, "transparent")
            .await
            .unwrap();
        sqlx::query(
            r#"
            INSERT INTO client (
                id, identifier, name, approval_status, capability_source, selected_profile_ids
            ) VALUES (
                'consumer-activated', 'client-activated', 'Activated Client', 'approved', 'activated', '[]'
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        let blocker = pool.begin_with("BEGIN IMMEDIATE").await.unwrap();
        let service = ProfileAuthoringService::new(pool.clone());
        let save = tokio::spawn(async move {
            service
                .save(
                    ProfileAuthoringCommand {
                        id: None,
                        expected_authoring_generation: None,
                        name: "Mode Locked".to_string(),
                        description: None,
                        profile_type: "shared".to_string(),
                        multi_select: true,
                        priority: 0,
                        is_active: true,
                        is_default: false,
                        server_ids: Vec::new(),
                        clone_from_id: None,
                    },
                    "test",
                )
                .await
        });
        tokio::time::timeout(Duration::from_secs(2), async {
            while pool.num_idle() != 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("authoring save should hold the second connection while waiting for the write lock");
        crate::system::settings::set_default_config_mode(&pool, "hosted")
            .await
            .unwrap();
        blocker.rollback().await.unwrap();

        save.await.unwrap().unwrap();
        let publications: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM surface_publications WHERE consumer_id = 'client-activated'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(publications, 1);
    }

    #[tokio::test]
    async fn authoring_view_reads_profile_and_relationships_from_one_snapshot() {
        let directory = tempfile::tempdir().unwrap();
        let options = SqliteConnectOptions::new()
            .filename(directory.path().join("view.db"))
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);
        let pool = SqlitePoolOptions::new()
            .max_connections(3)
            .connect_with(options)
            .await
            .unwrap();
        crate::test_helpers::prepare_config_database(&pool).await;
        for server_id in ["server-a", "server-b"] {
            sqlx::query(
                "INSERT INTO server_config (id, name, server_type, command, enabled) VALUES (?, ?, 'stdio', '', 1)",
            )
            .bind(server_id)
            .bind(server_id)
            .execute(&pool)
            .await
            .unwrap();
        }
        sqlx::query(
            "INSERT INTO profile (id, name, description, type, role) VALUES ('profile-a', 'Before', '', 'shared', 'user')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO profile_server_relationships (profile_id, server_id, enabled, new_ref_policy)
             VALUES ('profile-a', 'server-a', 1, 'follow')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let mut snapshot = pool.begin().await.unwrap();
        let pinned_generation: i64 =
            sqlx::query_scalar("SELECT authoring_generation FROM profile WHERE id = 'profile-a'")
                .fetch_one(&mut *snapshot)
                .await
                .unwrap();
        assert_eq!(pinned_generation, 0);
        let mut writer = pool.begin_with("BEGIN IMMEDIATE").await.unwrap();
        sqlx::query("UPDATE profile SET name = 'After', authoring_generation = 1 WHERE id = 'profile-a'")
            .execute(&mut *writer)
            .await
            .unwrap();
        sqlx::query("DELETE FROM profile_server_relationships WHERE profile_id = 'profile-a'")
            .execute(&mut *writer)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO profile_server_relationships (profile_id, server_id, enabled, new_ref_policy)
             VALUES ('profile-a', 'server-b', 1, 'follow')",
        )
        .execute(&mut *writer)
        .await
        .unwrap();
        writer.commit().await.unwrap();

        let view = ProfileAuthoringService::view_in_transaction(&mut snapshot, "profile-a")
            .await
            .unwrap();
        assert_eq!(view.profile.name, "Before");
        assert_eq!(view.profile.authoring_generation, 0);
        assert_eq!(view.server_ids, vec!["server-a"]);
    }
}
