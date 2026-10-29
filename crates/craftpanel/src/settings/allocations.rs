use std::path::Path;

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::auth::error::{Failure, Result};
use crate::model::{Allocation, Id, LoaderId, Timestamp};

pub const PER_SERVER: usize = 8;
const MIN_PORT: u16 = 1024;
pub const FIRST_PORT: u16 = 25565;
const MAX_NAME: usize = 32;

#[derive(Debug, Clone, Deserialize)]
pub struct CreateAllocationRequest {
    pub name: String,
    #[serde(default)]
    pub port: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RenameAllocationRequest {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SetPrimaryResponse {
    pub primary_port: u16,
    pub allocations: Vec<Allocation>,
    pub restart_required: bool,
}

pub async fn list(pool: &SqlitePool, server: Id) -> sqlx::Result<Vec<Allocation>> {
    let rows: Vec<(u16, String)> = sqlx::query_as(
        "SELECT port, name FROM allocations WHERE server_id = ? AND is_primary = 0 ORDER BY port",
    )
    .bind(server)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|(port, name)| Allocation { port, name }).collect())
}

pub async fn primary(pool: &SqlitePool, server: Id) -> sqlx::Result<Option<u16>> {
    sqlx::query_scalar("SELECT port FROM allocations WHERE server_id = ? AND is_primary = 1")
        .bind(server)
        .fetch_optional(pool)
        .await
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Followed {
    Same,
    Moved { from: u16 },
    Taken { by: Option<Id> },
    Published,
    NoPrimary,
}

pub async fn follow(pool: &SqlitePool, server: Id, wanted: u16) -> sqlx::Result<Followed> {
    let mut transaction = pool.begin().await?;
    let Some(current) = sqlx::query_scalar::<_, u16>(
        "SELECT port FROM allocations WHERE server_id = ? AND is_primary = 1",
    )
    .bind(server)
    .fetch_optional(&mut *transaction)
    .await?
    else {
        return Ok(Followed::NoPrimary);
    };
    if current == wanted {
        return Ok(Followed::Same);
    }

    let published: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM playit_tunnels WHERE server_id = ? AND local_port = ?)",
    )
    .bind(server)
    .bind(current)
    .fetch_one(&mut *transaction)
    .await?;
    if published {
        return Ok(Followed::Published);
    }

    let holder: Option<Option<Id>> = sqlx::query_scalar(
        "SELECT server_id FROM allocations WHERE port = ?
         UNION ALL SELECT NULL FROM playit_tunnels WHERE local_port = ? AND server_id <> ?
         UNION ALL SELECT NULL FROM playit_released WHERE local_port = ?
         LIMIT 1",
    )
    .bind(wanted)
    .bind(wanted)
    .bind(server)
    .bind(wanted)
    .fetch_optional(&mut *transaction)
    .await?;
    match holder {
        Some(Some(owner)) if owner == server => {
            sqlx::query("DELETE FROM allocations WHERE server_id = ? AND port = ?")
                .bind(server)
                .bind(wanted)
                .execute(&mut *transaction)
                .await?;
        }
        Some(by) => return Ok(Followed::Taken { by }),
        None => {}
    }

    sqlx::query("UPDATE allocations SET port = ? WHERE server_id = ? AND is_primary = 1")
        .bind(wanted)
        .bind(server)
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;
    Ok(Followed::Moved { from: current })
}

pub async fn follow_files(pool: &SqlitePool, users_dir: &Path, server: Id) -> sqlx::Result<()> {
    let Some(wanted) = port_in_files(pool, users_dir, server).await? else { return Ok(()) };
    let mut outcome = follow(pool, server, wanted).await?;
    if let Followed::Taken { by: Some(other) } = outcome {
        if let Some(theirs) = port_in_files(pool, users_dir, other).await? {
            if theirs != wanted {
                follow(pool, other, theirs).await?;
                outcome = follow(pool, server, wanted).await?;
            }
        }
    }
    match outcome {
        Followed::Taken { by } => tracing::warn!(
            %server, port = wanted, other = ?by,
            "the port in this server's files belongs to another server here; both cannot run at once"
        ),
        Followed::Published => tracing::warn!(
            %server, port = wanted,
            "this server's files name a new port, but playit.gg still publishes the old one"
        ),
        _ => {}
    }
    Ok(())
}

pub async fn follow_everyone(pool: &SqlitePool, users_dir: &Path) -> sqlx::Result<()> {
    let servers: Vec<Id> = sqlx::query_scalar("SELECT id FROM servers").fetch_all(pool).await?;
    for server in servers {
        follow_files(pool, users_dir, server).await?;
    }
    Ok(())
}

async fn port_in_files(pool: &SqlitePool, users_dir: &Path, server: Id) -> sqlx::Result<Option<u16>> {
    let row: Option<(Id, Option<LoaderId>)> =
        sqlx::query_as("SELECT owner_id, loader FROM servers WHERE id = ?")
            .bind(server)
            .fetch_optional(pool)
            .await?;
    let Some((owner, loader)) = row else { return Ok(None) };
    let dir = users_dir.join(owner.to_string()).join("servers").join(server.to_string());
    let loader = loader.unwrap_or(LoaderId::Vanilla);
    Ok(tokio::task::spawn_blocking(move || crate::import::detect::port_of(&dir, loader))
        .await
        .ok()
        .flatten()
        .filter(|port| *port >= MIN_PORT))
}

pub async fn create(
    pool: &SqlitePool,
    server: Id,
    request: &CreateAllocationRequest,
) -> Result<Allocation> {
    let name = check_name(&request.name)?;

    let mut transaction = pool.begin().await?;

    let held: Vec<u16> = sqlx::query_scalar("SELECT port FROM allocations WHERE server_id = ?")
        .bind(server)
        .fetch_all(&mut *transaction)
        .await?;
    if held.len() >= PER_SERVER {
        return Err(Failure::conflict(
            "allocation_limit",
            format!("a server keeps at most {PER_SERVER} ports"),
        ));
    }

    let port = match request.port {
        Some(asked) => {
            let wanted = u16::try_from(asked)
                .ok()
                .filter(|port| *port >= MIN_PORT)
                .ok_or_else(|| {
                    Failure::bad_request(
                        "invalid_port",
                        format!("{asked} is outside 1024-65535"),
                    )
                })?;
            if !free_on_the_machine(wanted) {
                return Err(Failure::conflict(
                    "port_unavailable",
                    format!("another process on this machine holds {wanted}"),
                ));
            }
            wanted
        }
        None => {
            let game: Option<u16> = sqlx::query_scalar(
                "SELECT port FROM allocations WHERE server_id = ? AND is_primary = 1",
            )
            .bind(server)
            .fetch_optional(&mut *transaction)
            .await?;
            next_free(&mut transaction, game.map_or(FIRST_PORT, |port| port.saturating_add(1)))
                .await?
        }
    };

    let inserted = sqlx::query(
        "INSERT INTO allocations (port, server_id, name, is_primary, created_at) \
         VALUES (?, ?, ?, 0, ?)",
    )
    .bind(port)
    .bind(server)
    .bind(&name)
    .bind(Timestamp::now())
    .execute(&mut *transaction)
    .await;

    match inserted {
        Ok(_) => {
            transaction.commit().await?;
            Ok(Allocation { port, name })
        }
        Err(err) if is_taken(&err) => {
            Err(Failure::conflict("port_in_use", format!("{port} belongs to another server here")))
        }
        Err(err) => Err(err.into()),
    }
}

pub async fn rename(pool: &SqlitePool, server: Id, port: u16, name: &str) -> Result<Allocation> {
    let name = check_name(name)?;
    let changed = sqlx::query("UPDATE allocations SET name = ? WHERE server_id = ? AND port = ?")
        .bind(&name)
        .bind(server)
        .bind(port)
        .execute(pool)
        .await?;

    if changed.rows_affected() == 0 {
        return Err(unknown(port));
    }
    Ok(Allocation { port, name })
}

pub async fn remove(pool: &SqlitePool, server: Id, port: u16) -> Result<()> {
    let is_primary: Option<bool> =
        sqlx::query_scalar("SELECT is_primary FROM allocations WHERE server_id = ? AND port = ?")
            .bind(server)
            .bind(port)
            .fetch_optional(pool)
            .await?;

    match is_primary {
        None => Err(unknown(port)),
        Some(true) => Err(Failure::conflict(
            "primary_allocation",
            "the primary port is swapped, not deleted",
        )),
        Some(false) => {
            let removed = sqlx::query(
                "DELETE FROM allocations WHERE server_id = ? AND port = ? \
                   AND NOT EXISTS (SELECT 1 FROM playit_tunnels \
                                    WHERE server_id = ? AND local_port = ?)",
            )
            .bind(server)
            .bind(port)
            .bind(server)
            .bind(port)
            .execute(pool)
            .await?;
            if removed.rows_affected() == 0 {
                return Err(Failure::conflict(
                    "port_published",
                    "this port is published through playit.gg; take it off there first",
                ));
            }
            Ok(())
        }
    }
}

pub async fn set_primary(pool: &SqlitePool, server: Id, port: u16) -> Result<()> {
    let mut transaction = pool.begin().await?;

    let is_primary: Option<bool> =
        sqlx::query_scalar("SELECT is_primary FROM allocations WHERE server_id = ? AND port = ?")
            .bind(server)
            .bind(port)
            .fetch_optional(&mut *transaction)
            .await?;

    match is_primary {
        None => return Err(unknown(port)),
        Some(true) => {
            return Err(Failure::conflict("already_primary", format!("{port} is already primary")))
        }
        Some(false) => {}
    }

    let published: Option<u16> =
        sqlx::query_scalar("SELECT local_port FROM playit_tunnels WHERE server_id = ?")
            .bind(server)
            .fetch_optional(&mut *transaction)
            .await?;
    if published.is_some() {
        return Err(Failure::conflict(
            "playit_tunnel_exists",
            "this server has a public address through playit.gg; give that address back \
             before swapping the primary port",
        ));
    }

    sqlx::query("UPDATE allocations SET is_primary = 0 WHERE server_id = ? AND is_primary = 1")
        .bind(server)
        .execute(&mut *transaction)
        .await?;
    sqlx::query("UPDATE allocations SET is_primary = 1 WHERE server_id = ? AND port = ?")
        .bind(server)
        .bind(port)
        .execute(&mut *transaction)
        .await?;

    transaction.commit().await?;
    Ok(())
}

pub async fn next_free(transaction: &mut sqlx::SqliteConnection, from: u16) -> Result<u16> {
    let taken: Vec<u16> =
        sqlx::query_scalar("SELECT port FROM ports_in_use WHERE port >= ? ORDER BY port")
            .bind(from)
            .fetch_all(&mut *transaction)
            .await?;

    (from.max(MIN_PORT)..=u16::MAX)
        .find(|port| !taken.contains(port) && free_on_the_machine(*port))
        .ok_or_else(|| Failure::conflict("no_free_port", format!("no port from {from} up is free")))
}

pub fn free_on_the_machine(port: u16) -> bool {
    match std::net::TcpListener::bind(("0.0.0.0", port)) {
        Ok(listener) => {
            drop(listener);
            true
        }
        Err(err) => err.kind() != std::io::ErrorKind::AddrInUse,
    }
}

fn check_name(name: &str) -> Result<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed.chars().count() > MAX_NAME {
        return Err(Failure::bad_request(
            "invalid_name",
            format!("a port name is 1 to {MAX_NAME} characters"),
        ));
    }
    if trimmed.contains(|letter: char| letter.is_control()) {
        return Err(Failure::bad_request("invalid_name", "a port name has no control characters"));
    }
    Ok(trimmed.to_owned())
}

fn unknown(port: u16) -> Failure {
    Failure::not_found("allocation_not_found", format!("this server has no port {port}"))
}

fn is_taken(err: &sqlx::Error) -> bool {
    err.as_database_error()
        .is_some_and(|db| db.is_unique_violation() || db.message().contains("port_still_published"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::harness::{an_allocation, pool_with_server};
    use std::sync::atomic::{AtomicU16, Ordering};

    fn a_base() -> u16 {
        static NEXT: AtomicU16 = AtomicU16::new(61000);
        NEXT.fetch_add(40, Ordering::Relaxed)
    }

    fn wanted(name: &str, port: Option<u16>) -> CreateAllocationRequest {
        CreateAllocationRequest { name: name.to_owned(), port: port.map(i64::from) }
    }

    fn asked_for(name: &str, port: i64) -> CreateAllocationRequest {
        CreateAllocationRequest { name: name.to_owned(), port: Some(port) }
    }

    #[tokio::test]
    async fn the_list_is_every_port_but_the_primary_one_in_order() {
        let (pool, server, _) = pool_with_server().await;
        an_allocation(&pool, server, 25570, "voice", false).await;
        an_allocation(&pool, server, 25565, "game", true).await;
        an_allocation(&pool, server, 25567, "map", false).await;

        let ports: Vec<u16> = list(&pool, server).await.unwrap().iter().map(|a| a.port).collect();
        assert_eq!(ports, [25567, 25570], "9.6: ascending, and the primary is not in it");
        assert_eq!(primary(&pool, server).await.unwrap(), Some(25565));
    }

    #[tokio::test]
    async fn an_extra_port_is_the_next_free_number_after_the_game_port() {
        let (pool, server, _) = pool_with_server().await;
        let base = a_base();
        an_allocation(&pool, server, base, "game", true).await;

        let first = create(&pool, server, &wanted("map", None)).await.unwrap();
        assert_eq!(first.port, base + 1);

        let second = create(&pool, server, &wanted("voice", None)).await.unwrap();
        assert_eq!(second.port, base + 2);
    }

    #[tokio::test]
    async fn a_port_of_another_server_is_in_use_and_not_merely_taken() {
        let (pool, mine, owner) = pool_with_server().await;
        let theirs = crate::auth::harness::a_server(&pool, owner, "Other", 2048).await;
        let base = a_base();
        an_allocation(&pool, theirs, base, "game", true).await;

        let refusal = create(&pool, mine, &wanted("map", Some(base))).await.unwrap_err();
        assert_eq!(refusal.code(), "port_in_use");
        assert_eq!(refusal.status(), axum::http::StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn the_bounds_of_9_7_each_refuse_on_their_own() {
        let (pool, server, _) = pool_with_server().await;
        an_allocation(&pool, server, a_base(), "game", true).await;

        for outside in [80, 0, -1, 70_000] {
            let refusal =
                create(&pool, server, &asked_for("far out", outside)).await.unwrap_err();
            assert_eq!(refusal.code(), "invalid_port", "{outside}");
            assert_eq!(refusal.status(), axum::http::StatusCode::BAD_REQUEST);
        }

        for bad in ["", "   ", &"x".repeat(33)] {
            let refusal = create(&pool, server, &wanted(bad, None)).await.unwrap_err();
            assert_eq!(refusal.code(), "invalid_name", "{bad:?}");
        }

        for index in 1..PER_SERVER {
            create(&pool, server, &wanted(&format!("p{index}"), None)).await.unwrap();
        }
        let refusal = create(&pool, server, &wanted("one too many", None))
            .await
            .unwrap_err();
        assert_eq!(refusal.code(), "allocation_limit");
    }

    #[tokio::test]
    async fn the_game_port_follows_what_the_server_file_says() {
        let (pool, server, _) = pool_with_server().await;
        let base = a_base();
        an_allocation(&pool, server, base, "game", true).await;

        assert_eq!(follow(&pool, server, base).await.unwrap(), Followed::Same);
        assert_eq!(follow(&pool, server, base + 7).await.unwrap(), Followed::Moved { from: base });
        assert_eq!(primary(&pool, server).await.unwrap(), Some(base + 7));
    }

    #[tokio::test]
    async fn a_file_port_that_was_an_extra_port_of_the_same_server_becomes_the_game_port() {
        let (pool, server, _) = pool_with_server().await;
        let base = a_base();
        an_allocation(&pool, server, base, "game", true).await;
        an_allocation(&pool, server, base + 1, "voice", false).await;

        assert_eq!(follow(&pool, server, base + 1).await.unwrap(), Followed::Moved { from: base });
        assert_eq!(primary(&pool, server).await.unwrap(), Some(base + 1));
        assert!(list(&pool, server).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_file_port_another_server_holds_is_named_and_left_alone() {
        let (pool, mine, owner) = pool_with_server().await;
        let theirs = crate::auth::harness::a_server(&pool, owner, "Other", 2048).await;
        let base = a_base();
        an_allocation(&pool, mine, base, "game", true).await;
        an_allocation(&pool, theirs, base + 1, "game", true).await;

        assert_eq!(
            follow(&pool, mine, base + 1).await.unwrap(),
            Followed::Taken { by: Some(theirs) }
        );
        assert_eq!(primary(&pool, mine).await.unwrap(), Some(base));
    }

    #[tokio::test]
    async fn renaming_touches_the_name_and_nothing_else() {
        let (pool, server, _) = pool_with_server().await;
        an_allocation(&pool, server, 25570, "voice", false).await;

        let renamed = rename(&pool, server, 25570, "  Language  ").await.unwrap();
        assert_eq!(renamed, Allocation { port: 25570, name: "Language".to_owned() });
        assert_eq!(rename(&pool, server, 25599, "x").await.unwrap_err().code(), "allocation_not_found");
        assert_eq!(rename(&pool, server, 25570, "").await.unwrap_err().code(), "invalid_name");
    }

    #[tokio::test]
    async fn the_primary_port_is_swapped_and_never_deleted() {
        let (pool, server, _) = pool_with_server().await;
        an_allocation(&pool, server, 25565, "game", true).await;
        an_allocation(&pool, server, 25570, "voice", false).await;

        assert_eq!(remove(&pool, server, 25565).await.unwrap_err().code(), "primary_allocation");
        assert_eq!(remove(&pool, server, 25599).await.unwrap_err().code(), "allocation_not_found");

        set_primary(&pool, server, 25570).await.unwrap();
        assert_eq!(primary(&pool, server).await.unwrap(), Some(25570));

        let left: Vec<u16> = list(&pool, server).await.unwrap().iter().map(|a| a.port).collect();
        assert_eq!(left, [25565], "the old primary stays with the server");

        assert_eq!(set_primary(&pool, server, 25570).await.unwrap_err().code(), "already_primary");
        assert_eq!(
            set_primary(&pool, server, 25599).await.unwrap_err().code(),
            "allocation_not_found"
        );
    }

    #[tokio::test]
    async fn a_deleted_port_is_handed_out_again() {
        let (pool, server, _) = pool_with_server().await;
        an_allocation(&pool, server, a_base(), "game", true).await;
        let handed = create(&pool, server, &wanted("map", None)).await.unwrap();

        remove(&pool, server, handed.port).await.unwrap();
        let again = create(&pool, server, &wanted("map", None)).await.unwrap();
        assert_eq!(again.port, handed.port, "9.9: the number comes back");
    }

    #[tokio::test]
    async fn a_port_a_foreign_process_holds_is_neither_handed_out_nor_accepted() {
        let (pool, server, _) = pool_with_server().await;
        let stranger = std::net::TcpListener::bind(("0.0.0.0", 0)).unwrap();
        let held = stranger.local_addr().unwrap().port();

        let refusal = create(&pool, server, &wanted("held", Some(held))).await.unwrap_err();
        assert_eq!(refusal.code(), "port_unavailable");

        an_allocation(&pool, server, held - 1, "game", true).await;
        let next = create(&pool, server, &wanted("held", None)).await.unwrap();
        assert_ne!(next.port, held, "the search steps over it");

        drop(stranger);
    }
}
