use serde::Serialize;
use sqlx::SqlitePool;

use crate::auth::error::{Failure, Result};
use crate::console::mclogs::Uploaded;
use crate::model::{Id, Timestamp};

pub const CONSOLE: &str = "console";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Sharer {
    pub id: Id,
    pub username: String,
}

/// One log this server has on mclo.gs, as the browser sees it: without the token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SharedLogEntry {
    pub id: Id,
    pub url: String,
    pub source: String,
    pub shared_by: Option<Sharer>,
    pub created_at: Timestamp,
}

#[derive(Debug, Serialize)]
pub struct SharedLogList {
    pub logs: Vec<SharedLogEntry>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct Row {
    pub id: Id,
    pub shared_by: Option<Id>,
    pub mclogs_id: String,
    pub token: Option<String>,
}

pub async fn record(
    pool: &SqlitePool,
    server: Id,
    by: Id,
    uploaded: &Uploaded,
    source: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO shared_logs (id, server_id, shared_by, mclogs_id, url, token, source, \
         created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(Id::new())
    .bind(server)
    .bind(by)
    .bind(&uploaded.id)
    .bind(&uploaded.url)
    .bind(&uploaded.token)
    .bind(source)
    .bind(Timestamp::now())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list(pool: &SqlitePool, server: Id) -> Result<SharedLogList> {
    let rows: Vec<(Id, String, String, Option<Id>, Option<String>, Timestamp)> = sqlx::query_as(
        "SELECT l.id, l.url, l.source, l.shared_by, u.username, l.created_at \
         FROM shared_logs l LEFT JOIN users u ON u.id = l.shared_by \
         WHERE l.server_id = ? ORDER BY l.created_at DESC, l.id DESC",
    )
    .bind(server)
    .fetch_all(pool)
    .await?;

    let logs = rows
        .into_iter()
        .map(|(id, url, source, by, username, created_at)| SharedLogEntry {
            id,
            url,
            source,
            shared_by: by.zip(username).map(|(id, username)| Sharer { id, username }),
            created_at,
        })
        .collect();
    Ok(SharedLogList { logs })
}

pub async fn find(pool: &SqlitePool, server: Id, log: Id) -> Result<Row> {
    let row: Option<Row> = sqlx::query_as(
        "SELECT id, shared_by, mclogs_id, token FROM shared_logs WHERE id = ? AND server_id = ?",
    )
    .bind(log)
    .bind(server)
    .fetch_optional(pool)
    .await?;
    row.ok_or_else(|| Failure::not_found("shared_log_not_found", "no such shared log"))
}

pub async fn remove(pool: &SqlitePool, log: Id) -> Result<()> {
    sqlx::query("DELETE FROM shared_logs WHERE id = ?").bind(log).execute(pool).await?;
    Ok(())
}
