use std::collections::{BTreeMap, BTreeSet};

use mcpmate_capability_store::{BUILTIN_CAPABILITY_SOURCE_ID, CatalogError, Result};
use sqlx::{Sqlite, Transaction};

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct CatalogDependencyRevisions(pub BTreeMap<String, i64>);

impl CatalogDependencyRevisions {
    pub async fn derive_in_transaction(
        transaction: &mut Transaction<'_, Sqlite>,
        consumer_id: &str,
        authoring_server_ids: &BTreeSet<String>,
        trigger_server_id: Option<&str>,
    ) -> Result<Self> {
        let mut server_ids = authoring_server_ids.clone();
        server_ids.extend(
            sqlx::query_scalar::<_, String>(
                r#"
                SELECT DISTINCT capability_ref.server_id
                FROM consumer_surface_bindings binding
                JOIN surface_publications publication
                  ON publication.publication_id = binding.active_publication_id
                JOIN surface_manifest_entries entry
                  ON entry.manifest_id = publication.manifest_id
                JOIN capability_refs capability_ref
                  ON capability_ref.ref_id = entry.ref_id
                WHERE binding.consumer_id = ?
                ORDER BY capability_ref.server_id
                "#,
            )
            .bind(consumer_id)
            .fetch_all(&mut **transaction)
            .await?,
        );
        if let Some(server_id) = trigger_server_id {
            server_ids.insert(server_id.to_string());
        }
        Self::load_for_server_ids_in_transaction(transaction, &server_ids).await
    }

    pub async fn load_current_for_expected_servers_in_transaction(
        transaction: &mut Transaction<'_, Sqlite>,
        expected: &Self,
    ) -> Result<Self> {
        let server_ids = expected.0.keys().cloned().collect();
        Self::load_for_server_ids_in_transaction(transaction, &server_ids).await
    }

    pub fn server_ids(&self) -> impl Iterator<Item = &str> + '_ {
        self.0.keys().map(String::as_str)
    }

    async fn load_for_server_ids_in_transaction(
        transaction: &mut Transaction<'_, Sqlite>,
        server_ids: &BTreeSet<String>,
    ) -> Result<Self> {
        let mut revisions = BTreeMap::new();
        for server_id in server_ids {
            let revision = sqlx::query_scalar::<_, i64>(
                "SELECT catalog_revision FROM capability_server_snapshots WHERE server_id = ?",
            )
            .bind(server_id)
            .fetch_optional(&mut **transaction)
            .await?;
            match revision {
                Some(revision) => {
                    revisions.insert(server_id.clone(), revision);
                }
                None if server_id == BUILTIN_CAPABILITY_SOURCE_ID => {}
                None => {
                    return Err(CatalogError::SurfaceNotFound {
                        entity: "capability server snapshot",
                        id: server_id.clone(),
                    });
                }
            }
        }
        Ok(Self(revisions))
    }
}
