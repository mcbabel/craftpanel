use std::collections::BTreeMap;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, patch, put};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::auth::access;
use crate::auth::error::{Failure, Result};
use crate::auth::{extract, Caller, JsonBody};
use crate::model::{Id, Timestamp};
use crate::AppState;

const MOST_GROUPS: i64 = 100;
const LONGEST_NAME: usize = 64;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/server-groups", get(list).post(create))
        .route("/server-groups/members", put(assign))
        .route("/server-groups/{group}", patch(amend).delete(remove))
        .layer(axum::middleware::from_fn(extract::same_origin))
}

#[derive(Debug, Serialize)]
struct ServerGroup {
    id: Id,
    name: String,
    position: i64,
    server_ids: Vec<Id>,
}

#[derive(Debug, Serialize)]
struct GroupList {
    groups: Vec<ServerGroup>,
}

#[derive(Debug, Deserialize)]
struct NewGroup {
    name: String,
    #[serde(default)]
    server_ids: Vec<Id>,
}

#[derive(Debug, Deserialize)]
struct GroupChange {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    position: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct Assignment {
    server_ids: Vec<Id>,
    group_id: Option<Id>,
}

async fn list(State(state): State<AppState>, caller: Caller) -> Result<Json<GroupList>> {
    Ok(Json(GroupList { groups: groups_of(&state.pool, caller.id()).await? }))
}

async fn create(
    State(state): State<AppState>,
    caller: Caller,
    JsonBody(body): JsonBody<NewGroup>,
) -> Result<(StatusCode, Json<GroupList>)> {
    let name = valid_name(&body.name)?;
    let (count, last): (i64, Option<i64>) =
        sqlx::query_as("SELECT count(*), max(position) FROM server_groups WHERE user_id = ?")
            .bind(caller.id())
            .fetch_one(&state.pool)
            .await?;
    if count >= MOST_GROUPS {
        return Err(Failure::conflict("too_many_groups", format!("{MOST_GROUPS} groups are the most one person can have")));
    }
    let visible = visible_ones(&state.pool, &caller, &body.server_ids).await?;

    let group = Id::new();
    let mut tx = state.pool.begin().await?;
    sqlx::query(
        "INSERT INTO server_groups (id, user_id, name, position, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(group)
    .bind(caller.id())
    .bind(&name)
    .bind(last.map_or(0, |last| last + 1))
    .bind(Timestamp::now())
    .execute(&mut *tx)
    .await
    .map_err(taken)?;
    for server in visible {
        put_in(&mut tx, caller.id(), server, group).await?;
    }
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(GroupList { groups: groups_of(&state.pool, caller.id()).await? })))
}

async fn amend(
    State(state): State<AppState>,
    caller: Caller,
    Path(group): Path<String>,
    JsonBody(body): JsonBody<GroupChange>,
) -> Result<Json<GroupList>> {
    let group = own_group(&state.pool, &caller, &group).await?;
    if let Some(name) = &body.name {
        sqlx::query("UPDATE server_groups SET name = ? WHERE id = ?")
            .bind(valid_name(name)?)
            .bind(group)
            .execute(&state.pool)
            .await
            .map_err(taken)?;
    }
    if let Some(position) = body.position {
        sqlx::query("UPDATE server_groups SET position = ? WHERE id = ?")
            .bind(position.clamp(0, MOST_GROUPS * 2))
            .bind(group)
            .execute(&state.pool)
            .await?;
    }
    Ok(Json(GroupList { groups: groups_of(&state.pool, caller.id()).await? }))
}

async fn remove(
    State(state): State<AppState>,
    caller: Caller,
    Path(group): Path<String>,
) -> Result<Json<GroupList>> {
    let group = own_group(&state.pool, &caller, &group).await?;
    sqlx::query("DELETE FROM server_groups WHERE id = ?").bind(group).execute(&state.pool).await?;
    Ok(Json(GroupList { groups: groups_of(&state.pool, caller.id()).await? }))
}

async fn assign(
    State(state): State<AppState>,
    caller: Caller,
    JsonBody(body): JsonBody<Assignment>,
) -> Result<Json<GroupList>> {
    let group = match body.group_id {
        Some(group) => Some(own_group(&state.pool, &caller, &group.to_string()).await?),
        None => None,
    };
    let servers = visible_ones(&state.pool, &caller, &body.server_ids).await?;

    let mut tx = state.pool.begin().await?;
    for server in servers {
        match group {
            Some(group) => put_in(&mut tx, caller.id(), server, group).await?,
            None => {
                sqlx::query("DELETE FROM server_group_members WHERE user_id = ? AND server_id = ?")
                    .bind(caller.id())
                    .bind(server)
                    .execute(&mut *tx)
                    .await?;
            }
        }
    }
    tx.commit().await?;
    Ok(Json(GroupList { groups: groups_of(&state.pool, caller.id()).await? }))
}

async fn groups_of(pool: &SqlitePool, user: Id) -> Result<Vec<ServerGroup>> {
    let groups: Vec<(Id, String, i64)> = sqlx::query_as(
        "SELECT id, name, position FROM server_groups WHERE user_id = ? ORDER BY position, created_at, id",
    )
    .bind(user)
    .fetch_all(pool)
    .await?;
    let members: Vec<(Id, Id)> = sqlx::query_as(
        "SELECT group_id, server_id FROM server_group_members WHERE user_id = ? ORDER BY server_id",
    )
    .bind(user)
    .fetch_all(pool)
    .await?;
    let mut by_group: BTreeMap<Id, Vec<Id>> = BTreeMap::new();
    for (group, server) in members {
        by_group.entry(group).or_default().push(server);
    }
    Ok(groups
        .into_iter()
        .map(|(id, name, position)| ServerGroup {
            server_ids: by_group.remove(&id).unwrap_or_default(),
            id,
            name,
            position,
        })
        .collect())
}

async fn put_in(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    user: Id,
    server: Id,
    group: Id,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO server_group_members (user_id, server_id, group_id) VALUES (?, ?, ?) \
         ON CONFLICT (user_id, server_id) DO UPDATE SET group_id = excluded.group_id",
    )
    .bind(user)
    .bind(server)
    .bind(group)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn own_group(pool: &SqlitePool, caller: &Caller, raw: &str) -> Result<Id> {
    let missing = || Failure::not_found("group_not_found", "no such group");
    let group: Id = raw.parse().map_err(|_| missing())?;
    let found: Option<Id> = sqlx::query_scalar("SELECT id FROM server_groups WHERE id = ? AND user_id = ?")
        .bind(group)
        .bind(caller.id())
        .fetch_optional(pool)
        .await?;
    found.ok_or_else(missing)
}

async fn visible_ones(pool: &SqlitePool, caller: &Caller, wanted: &[Id]) -> Result<Vec<Id>> {
    if wanted.is_empty() {
        return Ok(Vec::new());
    }
    let visible = access::visible_servers(pool, caller).await?;
    let mut chosen = Vec::with_capacity(wanted.len());
    for server in wanted {
        if !visible.contains(server) {
            return Err(Failure::not_found("server_not_found", "no such server"));
        }
        if !chosen.contains(server) {
            chosen.push(*server);
        }
    }
    Ok(chosen)
}

fn valid_name(raw: &str) -> Result<String> {
    let name = raw.trim();
    if name.is_empty() || name.chars().count() > LONGEST_NAME || name.chars().any(char::is_control) {
        return Err(Failure::bad_request(
            "invalid_group_name",
            format!("a group name has between 1 and {LONGEST_NAME} characters"),
        ));
    }
    Ok(name.to_owned())
}

fn taken(err: sqlx::Error) -> Failure {
    if err.as_database_error().is_some_and(|db| db.is_unique_violation()) {
        return Failure::conflict("group_name_taken", "you already have a group with that name");
    }
    err.into()
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::Request;
    use serde_json::{json, Value};
    use tower::ServiceExt;

    use super::*;
    use crate::auth::harness::{a_user, as_user, empty, fetch, send, sign_in, state_with, test_pool};
    use crate::config::Config;

    struct World {
        app: Router,
        pool: SqlitePool,
    }

    async fn a_world() -> World {
        let pool = test_pool().await;
        let app = router().with_state(state_with(&pool, Config::default()));
        World { app, pool }
    }

    impl World {
        async fn call(&self, request: Request<Body>) -> (StatusCode, Value) {
            let response = self.app.clone().oneshot(request).await.expect("a response");
            let status = response.status();
            (status, crate::auth::harness::body_json(response).await)
        }

        async fn server_of(&self, owner: Id, name: &str) -> Id {
            let id = Id::new();
            sqlx::query(
                "INSERT INTO servers (id, name, owner_id, status, loader, game_version, memory_mib, created_at, updated_at) \
                 VALUES (?, ?, ?, 'available', 'paper', '1.21.4', 1024, ?, ?)",
            )
            .bind(id)
            .bind(name)
            .bind(owner)
            .bind(Timestamp::now())
            .bind(Timestamp::now())
            .execute(&self.pool)
            .await
            .expect("a server");
            id
        }
    }

    fn ids(value: &Value) -> Vec<String> {
        value["server_ids"].as_array().unwrap().iter().map(|id| id.as_str().unwrap().to_owned()).collect()
    }

    #[tokio::test]
    async fn a_group_is_made_from_a_selection_and_servers_move_between_groups() {
        let world = a_world().await;
        let max = a_user(&world.pool, "max").await;
        let secret = sign_in(&world.pool, max).await;
        let lobby = world.server_of(max, "Lobby").await;
        let survival = world.server_of(max, "Survival").await;

        let (status, made) = world
            .call(as_user(send("POST", "/server-groups", json!({ "name": "  Network ", "server_ids": [lobby, survival] })), &secret))
            .await;
        assert_eq!(status, StatusCode::CREATED);
        let network = made["groups"][0].clone();
        assert_eq!(network["name"], "Network");
        assert_eq!(ids(&network).len(), 2);

        let (_, made) = world.call(as_user(send("POST", "/server-groups", json!({ "name": "Tests" })), &secret)).await;
        let tests = made["groups"][1]["id"].as_str().unwrap().to_owned();
        assert_eq!(made["groups"][1]["position"], 1);

        let (status, moved) = world
            .call(as_user(send("PUT", "/server-groups/members", json!({ "server_ids": [survival], "group_id": tests })), &secret))
            .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(ids(&moved["groups"][0]), [lobby.to_string()]);
        assert_eq!(ids(&moved["groups"][1]), [survival.to_string()], "a server sits in one group at a time");

        let (_, loose) = world
            .call(as_user(send("PUT", "/server-groups/members", json!({ "server_ids": [lobby], "group_id": null })), &secret))
            .await;
        assert!(ids(&loose["groups"][0]).is_empty());

        let (status, gone) = world.call(as_user(empty("DELETE", &format!("/server-groups/{tests}")), &secret)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(gone["groups"].as_array().unwrap().len(), 1);
        let left: i64 = sqlx::query_scalar("SELECT count(*) FROM server_group_members").fetch_one(&world.pool).await.unwrap();
        assert_eq!(left, 0, "the servers of a deleted group are simply ungrouped");
        let servers: i64 = sqlx::query_scalar("SELECT count(*) FROM servers").fetch_one(&world.pool).await.unwrap();
        assert_eq!(servers, 2, "deleting a group never deletes a server");
    }

    #[tokio::test]
    async fn groups_are_personal_and_only_hold_servers_their_owner_can_see() {
        let world = a_world().await;
        let max = a_user(&world.pool, "max").await;
        let eve = a_user(&world.pool, "eve").await;
        let max_secret = sign_in(&world.pool, max).await;
        let eve_secret = sign_in(&world.pool, eve).await;
        let mine = world.server_of(max, "Mine").await;
        let hers = world.server_of(eve, "Hers").await;

        let (status, _) = world
            .call(as_user(send("POST", "/server-groups", json!({ "name": "Grab", "server_ids": [hers] })), &max_secret))
            .await;
        assert_eq!(status, StatusCode::NOT_FOUND, "a server max cannot see cannot be grouped");

        let (_, made) = world
            .call(as_user(send("POST", "/server-groups", json!({ "name": "Mine", "server_ids": [mine] })), &max_secret))
            .await;
        let group = made["groups"][0]["id"].as_str().unwrap().to_owned();

        let (_, seen) = world.call(as_user(fetch("/server-groups"), &eve_secret)).await;
        assert!(seen["groups"].as_array().unwrap().is_empty());
        let (status, _) = world
            .call(as_user(send("PATCH", &format!("/server-groups/{group}"), json!({ "name": "Taken" })), &eve_secret))
            .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let (status, _) = world
            .call(as_user(send("PUT", "/server-groups/members", json!({ "server_ids": [hers], "group_id": group })), &eve_secret))
            .await;
        assert_eq!(status, StatusCode::NOT_FOUND, "nobody files servers into someone else's group");
        let (status, _) = world.call(fetch("/server-groups")).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn names_are_checked_and_cannot_repeat() {
        let world = a_world().await;
        let max = a_user(&world.pool, "max").await;
        let secret = sign_in(&world.pool, max).await;

        for bad in ["", "   ", &"x".repeat(65), "line\nbreak"] {
            let (status, answer) =
                world.call(as_user(send("POST", "/server-groups", json!({ "name": bad })), &secret)).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{bad:?}");
            assert_eq!(answer["error"], "invalid_group_name");
        }
        world.call(as_user(send("POST", "/server-groups", json!({ "name": "Lobby" })), &secret)).await;
        let (status, answer) =
            world.call(as_user(send("POST", "/server-groups", json!({ "name": "lobby" })), &secret)).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(answer["error"], "group_name_taken");
    }
}
