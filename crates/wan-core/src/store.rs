use std::path::Path;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{
    SqlitePool,
    migrate::{MigrateError, Migrator},
};
use thiserror::Error;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");
const LATEST_SCHEMA_VERSION: i64 = 3;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("database schema version {found} is newer than supported version {supported}")]
    SchemaTooNew { found: i64, supported: i64 },
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("migration error: {0}")]
    Migration(#[from] MigrateError),
    #[error("invalid event payload: {0}")]
    InvalidEventPayload(String),
}

#[derive(Clone, Debug)]
pub struct Store {
    pool: SqlitePool,
}

impl Store {
    pub async fn connect(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        let path = path.as_ref();
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;

        if let Some(version) = schema_version(&pool).await?
            && version > LATEST_SCHEMA_VERSION
        {
            return Err(StoreError::SchemaTooNew {
                found: version,
                supported: LATEST_SCHEMA_VERSION,
            });
        }

        MIGRATOR.run(&pool).await?;
        let version = schema_version(&pool).await?.unwrap_or_default();
        if version < LATEST_SCHEMA_VERSION {
            sqlx::query("UPDATE schema_version SET version = ? WHERE id = 1")
                .bind(LATEST_SCHEMA_VERSION)
                .execute(&pool)
                .await?;
        }

        Ok(Self { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn schema_version(&self) -> Result<i64, StoreError> {
        Ok(schema_version(&self.pool).await?.unwrap_or_default())
    }
}

async fn schema_version(pool: &SqlitePool) -> Result<Option<i64>, sqlx::Error> {
    let table_exists = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'schema_version'",
    )
    .fetch_one(pool)
    .await?;
    if table_exists == 0 {
        return Ok(None);
    }

    sqlx::query_scalar("SELECT version FROM schema_version WHERE id = 1")
        .fetch_optional(pool)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::Row;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_database() -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is after Unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("wan-core-store-{}-{suffix}.db", std::process::id()))
    }

    fn cleanup(path: &Path) {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(path.with_extension("db-wal"));
        let _ = std::fs::remove_file(path.with_extension("db-shm"));
    }

    #[tokio::test]
    async fn initializes_new_database_and_is_idempotent() {
        let path = temp_database();
        let store = Store::connect(&path).await.expect("new database opens");
        assert_eq!(store.schema_version().await.expect("version reads"), 3);
        assert_eq!(store.pool().size(), 1);
        assert_eq!(
            sqlx::query_scalar::<_, String>("PRAGMA journal_mode")
                .fetch_one(store.pool())
                .await
                .expect("journal mode reads"),
            "wal"
        );
        drop(store);

        let store = Store::connect(&path).await.expect("database reopens");
        assert_eq!(store.schema_version().await.expect("version reads"), 3);
        drop(store);
        cleanup(&path);
    }

    #[tokio::test]
    async fn migrates_database_from_previous_version() {
        let path = temp_database();
        let options = SqliteConnectOptions::new()
            .filename(&path)
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .expect("old database opens");
        sqlx::query(include_str!("../migrations/0001_initial.sql"))
            .execute(&pool)
            .await
            .expect("version one migration applies");
        drop(pool);

        let store = Store::connect(&path).await.expect("old database migrates");
        assert_eq!(store.schema_version().await.expect("version reads"), 3);
        let column = sqlx::query("PRAGMA table_info(store_metadata)")
            .fetch_one(store.pool())
            .await
            .expect("new table exists");
        assert_eq!(column.get::<String, _>(1), "created_at");
        drop(store);
        cleanup(&path);
    }

    #[tokio::test]
    async fn rejects_database_newer_than_supported() {
        let path = temp_database();
        let store = Store::connect(&path).await.expect("new database opens");
        sqlx::query("UPDATE schema_version SET version = 99")
            .execute(store.pool())
            .await
            .expect("version updates");
        drop(store);

        let error = Store::connect(&path)
            .await
            .expect_err("newer schema rejects");
        assert!(matches!(
            error,
            StoreError::SchemaTooNew {
                found: 99,
                supported: 3
            }
        ));
        cleanup(&path);
    }
}
