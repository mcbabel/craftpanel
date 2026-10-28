use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use axum::{Extension, Json, Router};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::auth::error::{Failure, Result};
use crate::auth::users::{self, UserRow};
use crate::auth::{extract, Caller, Disks, JsonBody, LiveServers};
use crate::config::Config;
use crate::helper::Helper;
use crate::model::{Id, ServerStatus, SystemUserState, Timestamp, UserRef};
use crate::playit::Playit;
use crate::AppState;

const MIB: u64 = 1024 * 1024;

pub fn router(live: LiveServers, disks: Disks, playit: Arc<Playit>) -> Router<AppState> {
    Router::new()
        .route("/servers/{server}/transfer", post(start))
        .route("/server-transfers", get(list))
        .route("/server-transfers/{transfer}/accept", post(accept))
        .route("/server-transfers/{transfer}", delete(withdraw))
        .layer(Extension(live))
        .layer(Extension(disks))
        .layer(Extension(playit))
        .layer(axum::middleware::from_fn(extract::same_origin))
}

#[derive(Debug, Deserialize)]
struct TransferRequest {
    to: String,
}

#[derive(Debug, Serialize)]
struct ServerBrief {
    id: Id,
    name: String,
    memory_mib: u32,
}

#[derive(Debug, Serialize)]
struct Transfer {
    id: Id,
    server: ServerBrief,
    from: UserRef,
    to: UserRef,
    created_at: Timestamp,
}

#[derive(Debug, Serialize)]
struct TransferList {
    incoming: Vec<Transfer>,
    outgoing: Vec<Transfer>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
enum Started {
    Transferred { server: Id, owner: UserRef },
    Requested { transfer: Transfer },
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct ServerFacts {
    id: Id,
    name: String,
    owner_id: Id,
    memory_mib: u32,
    status: ServerStatus,
}

#[derive(Debug, sqlx::FromRow)]
struct TransferRow {
    id: Id,
    server_id: Id,
    server_name: String,
    memory_mib: u32,
    from_user_id: Id,
    from_name: String,
    to_user_id: Id,
    to_name: String,
    created_at: Timestamp,
}

impl TransferRow {
    fn into_transfer(self) -> Transfer {
        Transfer {
            id: self.id,
            server: ServerBrief { id: self.server_id, name: self.server_name, memory_mib: self.memory_mib },
            from: UserRef { id: self.from_user_id, username: self.from_name, avatar_url: None },
            to: UserRef { id: self.to_user_id, username: self.to_name, avatar_url: None },
            created_at: self.created_at,
        }
    }
}

const TRANSFER_SELECT: &str = "SELECT t.id, t.server_id, s.name AS server_name, s.memory_mib, \
     t.from_user_id, f.username AS from_name, t.to_user_id, r.username AS to_name, t.created_at \
     FROM server_transfers t \
     JOIN servers s ON s.id = t.server_id \
     JOIN users f ON f.id = t.from_user_id \
     JOIN users r ON r.id = t.to_user_id";

async fn start(
    State(state): State<AppState>,
    caller: Caller,
    Extension(live): Extension<LiveServers>,
    Extension(disks): Extension<Disks>,
    Extension(playit): Extension<Arc<Playit>>,
    Path(server): Path<String>,
    JsonBody(body): JsonBody<TransferRequest>,
) -> Result<(StatusCode, Json<Started>)> {
    let server = server_facts(&state.pool, &server).await?;
    if server.owner_id != caller.id() && !caller.is_admin() {
        return Err(unknown_or_forbidden(&state.pool, &caller, server.id).await);
    }

    let receiver = receiver(&state.pool, body.to.trim()).await?;
    if receiver.id == server.owner_id {
        return Err(Failure::bad_request("transfer_to_owner", "this account already owns the server"));
    }
    if pending(&state.pool, server.id).await?.is_some() {
        return Err(Failure::conflict(
            "transfer_pending",
            "a transfer of this server is already waiting for an answer; withdraw it first",
        ));
    }
    room_for(&state, &disks, &receiver, &server).await?;

    if caller.is_admin() {
        settle_now(&state, &live, &playit, &server, &receiver).await?;
        return Ok((
            StatusCode::OK,
            Json(Started::Transferred { server: server.id, owner: receiver.reference() }),
        ));
    }

    let id = Id::new();
    sqlx::query(
        "INSERT INTO server_transfers (id, server_id, from_user_id, to_user_id, created_at) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(server.id)
    .bind(server.owner_id)
    .bind(receiver.id)
    .bind(Timestamp::now())
    .execute(&state.pool)
    .await?;
    let transfer = one(&state.pool, id).await?.ok_or_else(gone)?.into_transfer();
    Ok((StatusCode::CREATED, Json(Started::Requested { transfer })))
}

async fn list(State(state): State<AppState>, caller: Caller) -> Result<Json<TransferList>> {
    let incoming = sqlx::query_as::<_, TransferRow>(&format!(
        "{TRANSFER_SELECT} WHERE t.to_user_id = ? ORDER BY t.created_at DESC"
    ))
    .bind(caller.id())
    .fetch_all(&state.pool)
    .await?;
    let outgoing = sqlx::query_as::<_, TransferRow>(&format!(
        "{TRANSFER_SELECT} WHERE t.from_user_id = ? ORDER BY t.created_at DESC"
    ))
    .bind(caller.id())
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(TransferList {
        incoming: incoming.into_iter().map(TransferRow::into_transfer).collect(),
        outgoing: outgoing.into_iter().map(TransferRow::into_transfer).collect(),
    }))
}

async fn accept(
    State(state): State<AppState>,
    caller: Caller,
    Extension(live): Extension<LiveServers>,
    Extension(disks): Extension<Disks>,
    Extension(playit): Extension<Arc<Playit>>,
    Path(transfer): Path<String>,
) -> Result<Json<Started>> {
    let row = transfer_for(&state.pool, &transfer).await?;
    if row.to_user_id != caller.id() {
        return Err(gone());
    }
    let server = server_facts(&state.pool, &row.server_id.to_string()).await?;
    if server.owner_id != row.from_user_id {
        sqlx::query("DELETE FROM server_transfers WHERE id = ?").bind(row.id).execute(&state.pool).await?;
        return Err(Failure::conflict("transfer_outdated", "the server changed hands in the meantime"));
    }
    let receiver = users::load(&state.pool, caller.id()).await?;
    room_for(&state, &disks, &receiver, &server).await?;
    settle_now(&state, &live, &playit, &server, &receiver).await?;
    Ok(Json(Started::Transferred { server: server.id, owner: receiver.reference() }))
}

async fn withdraw(
    State(state): State<AppState>,
    caller: Caller,
    Path(transfer): Path<String>,
) -> Result<StatusCode> {
    let row = transfer_for(&state.pool, &transfer).await?;
    if row.to_user_id != caller.id() && row.from_user_id != caller.id() && !caller.is_admin() {
        return Err(gone());
    }
    sqlx::query("DELETE FROM server_transfers WHERE id = ?").bind(row.id).execute(&state.pool).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn settle_now(
    state: &AppState,
    live: &LiveServers,
    playit: &Playit,
    server: &ServerFacts,
    receiver: &UserRow,
) -> Result<()> {
    if server.status != ServerStatus::Available {
        return Err(Failure::conflict("server_not_ready", "only a server that is set up and idle can change hands"));
    }
    if !live.among(&[server.id]).await.is_empty() {
        return Err(Failure::conflict("server_running", "stop the server before it changes hands"));
    }
    let busy: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM operations WHERE server_id = ? AND state IN ('queued', 'ongoing')",
    )
    .bind(server.id)
    .fetch_one(&state.pool)
    .await?;
    if busy > 0 {
        return Err(Failure::conflict("server_busy", "wait until the server has finished what it is doing"));
    }
    if !users::claim_busy(&state.pool, receiver.id).await? {
        return Err(users::busy());
    }
    let outcome = carry(state, playit, server, receiver).await;
    users::set_busy(&state.pool, receiver.id, false).await?;
    outcome
}

async fn carry(state: &AppState, playit: &Playit, server: &ServerFacts, receiver: &UserRow) -> Result<()> {
    let helper = Helper::new(&state.config.helper_socket);
    let from = server_dir(&state.config, server.owner_id, server.id);
    let to = server_dir(&state.config, receiver.id, server.id);
    let present = std::fs::symlink_metadata(&from).is_ok_and(|meta| meta.is_dir());
    if std::fs::symlink_metadata(&to).is_ok() {
        return Err(Failure::conflict("transfer_target_taken", "the receiving account already has a folder for this server"));
    }

    playit.release_tunnel(server.id).await;

    if present {
        let parent = to.parent().ok_or_else(|| Failure::internal(anyhow::anyhow!("a server folder has a parent")))?;
        if !parent.is_dir() {
            return Err(Failure::conflict(
                "receiver_not_ready",
                "the receiving account has no server folder yet; an administrator has to fix the account first",
            ));
        }
        std::fs::rename(&from, &to).map_err(|err| Failure::internal(anyhow::Error::from(err).context("moving the server folder")))?;
        if let Err(err) = helper.chown_tree(&receiver.id.to_string(), crate::helper::in_servers(server.id)).await {
            undo_move(&helper, &to, &from, server).await;
            return Err(Failure::internal(err.context("handing the files to the receiving account")));
        }
    }

    let recorded = record(&state.pool, server, receiver.id).await;
    if let Err(err) = recorded {
        if present {
            undo_move(&helper, &to, &from, server).await;
        }
        return Err(err.into());
    }
    Ok(())
}

async fn undo_move(helper: &Helper, to: &PathBuf, from: &PathBuf, server: &ServerFacts) {
    if let Err(err) = std::fs::rename(to, from) {
        tracing::error!("{} could not be put back to {}: {err}", to.display(), from.display());
        return;
    }
    if let Err(err) = helper.chown_tree(&server.owner_id.to_string(), crate::helper::in_servers(server.id)).await {
        tracing::error!("{} went back but still belongs to the other account: {err:#}", from.display());
    }
}

async fn record(pool: &SqlitePool, server: &ServerFacts, receiver: Id) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("UPDATE servers SET owner_id = ?, updated_at = ? WHERE id = ?")
        .bind(receiver)
        .bind(Timestamp::now())
        .bind(server.id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM server_members WHERE server_id = ? AND user_id = ?")
        .bind(server.id)
        .bind(receiver)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM server_group_members WHERE server_id = ? AND user_id = ?")
        .bind(server.id)
        .bind(server.owner_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM server_transfers WHERE server_id = ?")
        .bind(server.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await
}

async fn room_for(state: &AppState, disks: &Disks, receiver: &UserRow, server: &ServerFacts) -> Result<()> {
    let budget = receiver.budget();
    let allocated: u32 = users::owned_servers(&state.pool, receiver.id)
        .await?
        .iter()
        .map(|owned| owned.memory_mib)
        .sum();
    if !budget.has_room_for(allocated, server.memory_mib) {
        let limit = budget.memory_mib().unwrap_or_default();
        return Err(Failure::conflict(
            "receiver_memory_budget",
            format!(
                "{} has {} MiB of memory left of {limit} MiB, and {} needs {} MiB",
                receiver.username,
                limit.saturating_sub(allocated),
                server.name,
                server.memory_mib
            ),
        ));
    }

    if let Some(limit) = budget.disk_limit_bytes() {
        let dir = server_dir(&state.config, server.owner_id, server.id);
        let size = tokio::task::spawn_blocking(move || crate::files::measure(&dir).bytes)
            .await
            .unwrap_or_default();
        let used = disks.of(receiver.id).await.used_bytes();
        if !budget.has_disk_room_for(used, size) {
            return Err(Failure::conflict(
                "receiver_disk_budget",
                format!(
                    "{} has {} MiB of disk space left of {} MiB, and {} takes {} MiB",
                    receiver.username,
                    limit.saturating_sub(used) / MIB,
                    limit / MIB,
                    server.name,
                    size.div_ceil(MIB)
                ),
            ));
        }
    }
    Ok(())
}

async fn receiver(pool: &SqlitePool, wanted: &str) -> Result<UserRow> {
    let missing = || Failure::not_found("user_not_found", "there is no panel user by that name");
    let found = match wanted.parse::<Id>() {
        Ok(id) => users::find(pool, id).await?,
        Err(_) => users::by_name(pool, wanted).await?,
    };
    let row = found.ok_or_else(missing)?;
    if row.system_state != SystemUserState::Ready {
        return Err(Failure::conflict(
            "receiver_not_ready",
            format!("the system account of {} is not ready, so it cannot run a server yet", row.username),
        ));
    }
    users::refuse_if_busy(&row)?;
    Ok(row)
}

async fn server_facts(pool: &SqlitePool, raw: &str) -> Result<ServerFacts> {
    let missing = || Failure::not_found("server_not_found", "no such server");
    let id: Id = raw.parse().map_err(|_| missing())?;
    sqlx::query_as::<_, ServerFacts>("SELECT id, name, owner_id, memory_mib, status FROM servers WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(missing)
}

async fn unknown_or_forbidden(pool: &SqlitePool, caller: &Caller, server: Id) -> Failure {
    match crate::auth::access::visible_servers(pool, caller).await {
        Ok(visible) if visible.contains(&server) => Failure::forbidden(),
        _ => Failure::not_found("server_not_found", "no such server"),
    }
}

async fn pending(pool: &SqlitePool, server: Id) -> sqlx::Result<Option<Id>> {
    sqlx::query_scalar("SELECT id FROM server_transfers WHERE server_id = ?")
        .bind(server)
        .fetch_optional(pool)
        .await
}

async fn one(pool: &SqlitePool, id: Id) -> sqlx::Result<Option<TransferRow>> {
    sqlx::query_as::<_, TransferRow>(&format!("{TRANSFER_SELECT} WHERE t.id = ?"))
        .bind(id)
        .fetch_optional(pool)
        .await
}

async fn transfer_for(pool: &SqlitePool, raw: &str) -> Result<TransferRow> {
    let id: Id = raw.parse().map_err(|_| gone())?;
    one(pool, id).await?.ok_or_else(gone)
}

fn gone() -> Failure {
    Failure::not_found("transfer_not_found", "no such transfer")
}

fn server_dir(config: &Config, owner: Id, server: Id) -> PathBuf {
    config.users_dir().join(owner.to_string()).join("servers").join(server.to_string())
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::Request;
    use serde_json::{json, Value};
    use tower::ServiceExt;

    use super::*;
    use crate::auth::harness::{
        a_server, a_user, an_admin, as_user, empty, fetch, send, sign_in, state_with, test_pool, FakeHelper,
    };

    struct World {
        app: Router,
        pool: SqlitePool,
        config: Config,
        fake: FakeHelper,
    }

    async fn a_world(live: LiveServers) -> World {
        let pool = test_pool().await;
        let fake = FakeHelper::obliging().await;
        let data = std::env::temp_dir().join(format!("craftpanel-transfers-{}", Id::new()));
        let config = Config { data_dir: data, helper_socket: fake.socket(), ..Config::default() };
        let app = Router::new().with_state(state_with(&pool, config.clone()));
        World { app, pool, config, fake }.with_live(live)
    }

    impl World {
        fn with_live(self, live: LiveServers) -> Self {
            let playit = Playit::against(self.pool.clone(), Arc::new(self.config.clone()), "http://127.0.0.1:1")
                .expect("the playit service");
            let app = router(live, Disks::none(), playit).with_state(state_with(&self.pool, self.config.clone()));
            Self { app, ..self }
        }

        async fn call(&self, request: Request<Body>) -> (StatusCode, Value) {
            let response = self.app.clone().oneshot(request).await.expect("a response");
            let status = response.status();
            (status, crate::auth::harness::body_json(response).await)
        }

        fn home(&self, user: Id) -> PathBuf {
            let dir = self.config.users_dir().join(user.to_string()).join("servers");
            std::fs::create_dir_all(&dir).unwrap();
            dir
        }

        fn folder(&self, owner: Id, server: Id) -> PathBuf {
            let dir = self.home(owner).join(server.to_string());
            std::fs::create_dir_all(dir.join("world")).unwrap();
            std::fs::write(dir.join("world/level.dat"), b"a world").unwrap();
            dir
        }

        async fn owner_of(&self, server: Id) -> Id {
            sqlx::query_scalar("SELECT owner_id FROM servers WHERE id = ?")
                .bind(server)
                .fetch_one(&self.pool)
                .await
                .unwrap()
        }
    }

    #[tokio::test]
    async fn a_user_asks_and_the_server_moves_once_the_receiver_accepts() {
        let world = a_world(LiveServers::none()).await;
        let max = a_user(&world.pool, "max").await;
        let anna = a_user(&world.pool, "anna").await;
        let max_secret = sign_in(&world.pool, max).await;
        let anna_secret = sign_in(&world.pool, anna).await;
        let server = a_server(&world.pool, max, "Survival", 2048).await;
        let old = world.folder(max, server);
        world.home(anna);

        let (status, asked) = world
            .call(as_user(send("POST", &format!("/servers/{server}/transfer"), json!({ "to": "anna" })), &max_secret))
            .await;
        assert_eq!(status, StatusCode::CREATED, "{asked}");
        assert_eq!(asked["outcome"], "requested");
        assert_eq!(world.owner_of(server).await, max, "nothing moves before anna says yes");
        assert!(old.exists());

        let (_, inbox) = world.call(as_user(fetch("/server-transfers"), &anna_secret)).await;
        let transfer = inbox["incoming"][0]["id"].as_str().unwrap().to_owned();
        assert_eq!(inbox["incoming"][0]["server"]["name"], "Survival");
        let (_, theirs) = world.call(as_user(fetch("/server-transfers"), &max_secret)).await;
        assert_eq!(theirs["outgoing"][0]["id"], transfer.as_str());

        let (status, _) = world
            .call(as_user(empty("POST", &format!("/server-transfers/{transfer}/accept")), &max_secret))
            .await;
        assert_eq!(status, StatusCode::NOT_FOUND, "only the receiver accepts");

        let (status, done) = world
            .call(as_user(empty("POST", &format!("/server-transfers/{transfer}/accept")), &anna_secret))
            .await;
        assert_eq!(status, StatusCode::OK, "{done}");
        assert_eq!(done["outcome"], "transferred");
        assert_eq!(world.owner_of(server).await, anna);
        assert!(!old.exists());
        let new = world.config.users_dir().join(anna.to_string()).join("servers").join(server.to_string());
        assert_eq!(std::fs::read(new.join("world/level.dat")).unwrap(), b"a world");
        assert!(world.fake.calls().iter().any(|call| matches!(
            call,
            craftpanel_proto::HelperRequest::ChownTree { user_id, .. } if *user_id == anna.to_string()
        )));
        let (_, after) = world.call(as_user(fetch("/server-transfers"), &anna_secret)).await;
        assert!(after["incoming"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_receiver_without_room_stops_the_request_before_it_is_sent() {
        let world = a_world(LiveServers::none()).await;
        let max = a_user(&world.pool, "max").await;
        let anna = a_user(&world.pool, "anna").await;
        let max_secret = sign_in(&world.pool, max).await;
        a_server(&world.pool, anna, "Hers", 3072).await;
        let server = a_server(&world.pool, max, "Big", 2048).await;

        let (status, answer) = world
            .call(as_user(send("POST", &format!("/servers/{server}/transfer"), json!({ "to": "anna" })), &max_secret))
            .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(answer["error"], "receiver_memory_budget");
        assert!(answer["message"].as_str().unwrap().contains("1024 MiB"), "{answer}");
        let waiting: i64 = sqlx::query_scalar("SELECT count(*) FROM server_transfers").fetch_one(&world.pool).await.unwrap();
        assert_eq!(waiting, 0);

        let boss = an_admin(&world.pool, "boss").await;
        let boss_secret = sign_in(&world.pool, boss).await;
        let (status, answer) = world
            .call(as_user(send("POST", &format!("/servers/{server}/transfer"), json!({ "to": "anna" })), &boss_secret))
            .await;
        assert_eq!(status, StatusCode::CONFLICT, "an administrator is told no as well");
        assert_eq!(answer["error"], "receiver_memory_budget");
        assert_eq!(world.owner_of(server).await, max);
    }

    #[tokio::test]
    async fn an_administrator_moves_a_server_without_asking() {
        let world = a_world(LiveServers::none()).await;
        let max = a_user(&world.pool, "max").await;
        let anna = a_user(&world.pool, "anna").await;
        let boss = an_admin(&world.pool, "boss").await;
        let boss_secret = sign_in(&world.pool, boss).await;
        let server = a_server(&world.pool, max, "Survival", 1024).await;
        world.folder(max, server);
        world.home(anna);

        let (status, done) = world
            .call(as_user(send("POST", &format!("/servers/{server}/transfer"), json!({ "to": anna.to_string() })), &boss_secret))
            .await;
        assert_eq!(status, StatusCode::OK, "{done}");
        assert_eq!(done["outcome"], "transferred");
        assert_eq!(done["owner"]["username"], "anna");
        assert_eq!(world.owner_of(server).await, anna);
    }

    #[tokio::test]
    async fn only_the_owner_hands_a_server_on_and_a_running_one_stays() {
        let world = a_world(LiveServers::none()).await;
        let max = a_user(&world.pool, "max").await;
        let eve = a_user(&world.pool, "eve").await;
        let eve_secret = sign_in(&world.pool, eve).await;
        let server = a_server(&world.pool, max, "Survival", 1024).await;
        let (status, _) = world
            .call(as_user(send("POST", &format!("/servers/{server}/transfer"), json!({ "to": "eve" })), &eve_secret))
            .await;
        assert_eq!(status, StatusCode::NOT_FOUND, "eve cannot even see it");

        let max_secret = sign_in(&world.pool, max).await;
        let (status, answer) = world
            .call(as_user(send("POST", &format!("/servers/{server}/transfer"), json!({ "to": "nobody" })), &max_secret))
            .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(answer["error"], "user_not_found");
        let (status, answer) = world
            .call(as_user(send("POST", &format!("/servers/{server}/transfer"), json!({ "to": "max" })), &max_secret))
            .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(answer["error"], "transfer_to_owner");

        let (_, asked) = world
            .call(as_user(send("POST", &format!("/servers/{server}/transfer"), json!({ "to": "eve" })), &max_secret))
            .await;
        let transfer = asked["transfer"]["id"].as_str().unwrap().to_owned();
        let (status, answer) = world
            .call(as_user(send("POST", &format!("/servers/{server}/transfer"), json!({ "to": "eve" })), &max_secret))
            .await;
        assert_eq!((status, answer["error"].as_str()), (StatusCode::CONFLICT, Some("transfer_pending")));
        let (status, _) = world
            .call(as_user(empty("DELETE", &format!("/server-transfers/{transfer}")), &max_secret))
            .await;
        assert_eq!(status, StatusCode::NO_CONTENT, "the sender can take it back");

        let boss = an_admin(&world.pool, "boss").await;
        let boss_secret = sign_in(&world.pool, boss).await;
        let running = world.with_live(LiveServers::fixed([server]));
        let (status, answer) = running
            .call(as_user(send("POST", &format!("/servers/{server}/transfer"), json!({ "to": "eve" })), &boss_secret))
            .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(answer["error"], "server_running");
    }
}
