use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, Pool, Sqlite};

/// database entity trait
/// defines common CRUD operations
#[async_trait]
pub trait DatabaseEntity: Sized + Send + Unpin + for<'r> FromRow<'r, sqlx::sqlite::SqliteRow> {
    /// table name of the entity
    fn table_name() -> &'static str;

    /// id field name of the entity
    fn id_field() -> &'static str {
        "id"
    }

    /// get entity id
    fn get_id(&self) -> Option<String>;

    /// set entity id
    fn set_id(
        &mut self,
        id: String,
    );

    /// get created time
    fn get_created_at(&self) -> Option<DateTime<Utc>>;

    /// set created time
    fn set_created_at(
        &mut self,
        time: DateTime<Utc>,
    );

    /// get updated time
    fn get_updated_at(&self) -> Option<DateTime<Utc>>;

    /// set updated time
    fn set_updated_at(
        &mut self,
        time: DateTime<Utc>,
    );

    /// create entity
    async fn create(
        &mut self,
        pool: &Pool<Sqlite>,
    ) -> Result<()> {
        // generate id
        if self.get_id().is_none() {
            let table_name = Self::table_name();
            self.set_id(format!("{}_{}", table_name, crate::generate_raw_id!(12)));
        }

        // set timestamp
        let now = Utc::now();
        self.set_created_at(now);
        self.set_updated_at(now);

        // build insert statement
        let mut query_builder = sqlx::QueryBuilder::new(format!(
            "INSERT INTO {} ({}) VALUES ",
            Self::table_name(),
            Self::id_field()
        ));

        query_builder.push_values([self.get_id().unwrap()], |mut b, id| {
            b.push_bind(id);
        });

        // execute insert
        query_builder.build().execute(pool).await?;

        Ok(())
    }

    /// update entity
    async fn update(
        &mut self,
        pool: &Pool<Sqlite>,
    ) -> Result<()> {
        // check if id exists
        let id = self.get_id().ok_or_else(|| anyhow::anyhow!("Entity ID not set"))?;

        // set updated time
        self.set_updated_at(Utc::now());

        // build update statement
        let mut query_builder = sqlx::QueryBuilder::new(format!(
            "UPDATE {} SET updated_at = ? WHERE {} = ?",
            Self::table_name(),
            Self::id_field()
        ));

        // execute update
        query_builder
            .build()
            .bind(self.get_updated_at().unwrap())
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// delete entity
    async fn delete(
        &self,
        pool: &Pool<Sqlite>,
    ) -> Result<()> {
        use sqlx::query;

        // check if id exists
        let id = self.get_id().ok_or_else(|| anyhow::anyhow!("Entity ID not set"))?;

        // execute delete
        query(&format!(
            "DELETE FROM {} WHERE {} = ?",
            Self::table_name(),
            Self::id_field()
        ))
        .bind(id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// find entity by id
    async fn find_by_id(
        pool: &Pool<Sqlite>,
        id: &str,
    ) -> Result<Option<Self>> {
        use sqlx::query_as;

        // execute query
        let entity = query_as(&format!(
            "SELECT * FROM {} WHERE {} = ?",
            Self::table_name(),
            Self::id_field()
        ))
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(entity)
    }

    /// find all entities
    async fn find_all(pool: &Pool<Sqlite>) -> Result<Vec<Self>> {
        use sqlx::query_as;

        // execute query
        let entities = query_as(&format!("SELECT * FROM {}", Self::table_name()))
            .fetch_all(pool)
            .await?;

        Ok(entities)
    }

    /// find entity by conditions
    async fn find_by(
        pool: &Pool<Sqlite>,
        conditions: &str,
    ) -> Result<Vec<Self>>;

    /// check if entity exists by id
    async fn exists(
        pool: &Pool<Sqlite>,
        id: &str,
    ) -> Result<bool> {
        let count: (i32,) = sqlx::query_as(&format!(
            "SELECT COUNT(*) FROM {} WHERE {} = ?",
            Self::table_name(),
            Self::id_field()
        ))
        .bind(id)
        .fetch_one(pool)
        .await?;

        Ok(count.0 > 0)
    }
}

/// generate CRUD implementation for entity
#[macro_export]
macro_rules! impl_database_entity {
    ($type:ty, $table:expr) => {
        #[async_trait::async_trait]
        impl DatabaseEntity for $type {
            fn table_name() -> &'static str {
                $table
            }

            fn get_id(&self) -> Option<String> {
                self.id.clone()
            }

            fn set_id(
                &mut self,
                id: String,
            ) {
                self.id = Some(id);
            }

            fn get_created_at(&self) -> Option<DateTime<Utc>> {
                self.created_at
            }

            fn set_created_at(
                &mut self,
                time: DateTime<Utc>,
            ) {
                self.created_at = Some(time);
            }

            fn get_updated_at(&self) -> Option<DateTime<Utc>> {
                self.updated_at
            }

            fn set_updated_at(
                &mut self,
                time: DateTime<Utc>,
            ) {
                self.updated_at = Some(time);
            }

            async fn do_create(
                &self,
                pool: &Pool<Sqlite>,
            ) -> Result<()> {
                // implementation provided by the user
                unimplemented!()
            }

            async fn do_update(
                &self,
                pool: &Pool<Sqlite>,
            ) -> Result<()> {
                // implementation provided by the user
                unimplemented!()
            }

            async fn find_by_id(
                pool: &Pool<Sqlite>,
                id: &str,
            ) -> Result<Option<Self>> {
                // implementation provided by the user
                unimplemented!()
            }

            async fn find_all(pool: &Pool<Sqlite>) -> Result<Vec<Self>> {
                // implementation provided by the user
                unimplemented!()
            }

            async fn find_by(
                pool: &Pool<Sqlite>,
                conditions: &str,
            ) -> Result<Vec<Self>> {
                // implementation provided by the user
                unimplemented!()
            }
        }
    };
}
