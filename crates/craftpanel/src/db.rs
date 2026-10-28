use std::path::Path;

use anyhow::{Context, Result};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{ConnectOptions, Connection, SqlitePool};

pub async fn connect(path: &Path) -> Result<SqlitePool> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating data directory {}", parent.display()))?;
    }

    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true)
        .busy_timeout(std::time::Duration::from_secs(10));

    migrate(options.clone()).await?;

    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(options)
        .await
        .with_context(|| format!("opening database at {}", path.display()))?;

    Ok(pool)
}

pub async fn migrate(options: SqliteConnectOptions) -> Result<()> {
    let mut connection = options
        .foreign_keys(false)
        .connect()
        .await
        .context("opening the database to migrate it")?;
    sqlx::migrate!("./migrations")
        .run(&mut connection)
        .await
        .context("running database migrations")?;
    let broken = sqlx::query("PRAGMA foreign_key_check").fetch_all(&mut connection).await?;
    if !broken.is_empty() {
        anyhow::bail!("the migrations left {} rows pointing at nothing", broken.len());
    }
    connection.close().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use sqlx::Row;

    use crate::model::{Id, Timestamp};

    #[tokio::test]
    async fn rebuilding_servers_keeps_every_row_that_hangs_off_them() {
        let path = std::env::temp_dir().join(format!("craftpanel-rebuild-{}.db", Id::new()));
        let pool = super::connect(&path).await.expect("a migrated database");
        let owner = crate::auth::harness::an_admin(&pool, "owner").await;
        let server = Id::new();
        let now = Timestamp::now();
        sqlx::query(
            "INSERT INTO servers (id, name, owner_id, status, loader, memory_mib, created_at, updated_at) \
             VALUES (?, 'Kept', ?, 'available', 'paper', 2048, ?, ?)",
        )
        .bind(server)
        .bind(owner)
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO allocations (port, server_id, name, is_primary, created_at) VALUES (25565, ?, 'primary', 1, ?)")
            .bind(server)
            .bind(now)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO backups (id, server_id, name, created_at) VALUES (?, ?, 'one', ?)")
            .bind(Id::new())
            .bind(server)
            .bind(now)
            .execute(&pool)
            .await
            .unwrap();
        pool.close().await;

        let options = sqlx::sqlite::SqliteConnectOptions::new().filename(&path).foreign_keys(false);
        let mut connection = sqlx::ConnectOptions::connect(&options).await.unwrap();
        let mut tx = sqlx::Connection::begin(&mut connection).await.unwrap();
        sqlx::raw_sql(include_str!("../migrations/0024_more_loaders.sql")).execute(&mut *tx).await.unwrap();
        tx.commit().await.unwrap();
        drop(connection);

        let pool = super::connect(&path).await.expect("the database opens again");
        let count = |table: &'static str| {
            let pool = pool.clone();
            async move {
                sqlx::query(&format!("SELECT count(*) AS n FROM {table}"))
                    .fetch_one(&pool)
                    .await
                    .unwrap()
                    .get::<i64, _>("n")
            }
        };
        assert_eq!(count("servers").await, 1);
        assert_eq!(count("allocations").await, 1, "the port stayed with its server");
        assert_eq!(count("backups").await, 1, "and so did the backup");

        sqlx::query("UPDATE servers SET loader = 'craftbukkit'").execute(&pool).await.unwrap();
        assert!(sqlx::query("UPDATE servers SET loader = 'spigot'").execute(&pool).await.is_err());

        sqlx::query("DELETE FROM servers").execute(&pool).await.unwrap();
        assert_eq!(count("allocations").await, 0, "the cascade still points at the new table");
        pool.close().await;
        let _ = std::fs::remove_file(&path);
    }
}
