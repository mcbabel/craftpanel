use std::sync::Arc;

use axum::extract::{FromRequestParts, Path, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use axum::{Extension, Json, Router};
use serde::Deserialize;

use crate::auth::error::{Failure, Result};
use crate::auth::{access, extract, Admin, Caller, JsonBody, Params};
use crate::model::{Id, Permission};
use crate::playit::store::{Slot, TunnelKind};
use crate::playit::{Playit, PlayitClaim, PlayitOverview, PlayitStatus, PublishedPort, ServerTunnel, Tunnels};
use crate::settings::allocations;
use crate::AppState;

pub fn router(playit: Arc<Playit>) -> Router<AppState> {
    Router::new()
        .route("/playit", get(status).delete(disconnect))
        .route("/playit/claim", get(claim).post(begin_claim).delete(cancel_claim))
        .route("/playit/agent/restart", post(restart))
        .route("/admin/playit", get(overview))
        .route("/admin/playit/{user}", delete(disconnect_user))
        .route(
            "/servers/{server}/playit",
            get(tunnel).post(request_tunnel).delete(drop_tunnel),
        )
        .route("/servers/{server}/playit/ports", get(published).post(publish))
        .route("/servers/{server}/playit/ports/{tunnel}", delete(unpublish))
        .layer(Extension(playit))
        .layer(axum::middleware::from_fn(extract::same_origin))
}

struct OfServer(Id);

impl FromRequestParts<AppState> for OfServer {
    type Rejection = Failure;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self> {
        let Path(raw) = Path::<String>::from_request_parts(parts, state)
            .await
            .map_err(|_| unknown_server())?;
        raw.parse().map(Self).map_err(|_| unknown_server())
    }
}

async fn status(
    caller: Caller,
    Extension(playit): Extension<Arc<Playit>>,
) -> Result<Json<PlayitStatus>> {
    Ok(Json(playit.of(caller.id()).status().await?))
}

async fn begin_claim(
    caller: Caller,
    Extension(playit): Extension<Arc<Playit>>,
) -> Result<(StatusCode, Json<PlayitClaim>)> {
    let claim = playit.of(caller.id()).begin_claim().await?;
    Ok((StatusCode::CREATED, Json(claim)))
}

async fn claim(
    caller: Caller,
    Extension(playit): Extension<Arc<Playit>>,
) -> Result<Json<PlayitClaim>> {
    playit
        .of(caller.id())
        .claim()
        .await?
        .map(Json)
        .ok_or_else(|| Failure::not_found("playit_claim_not_found", "no sign-up is under way"))
}

async fn cancel_claim(
    caller: Caller,
    Extension(playit): Extension<Arc<Playit>>,
) -> Result<StatusCode> {
    playit.of(caller.id()).cancel_claim().await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct DisconnectQuery {
    tunnels: Option<String>,
}

fn disposal(query: &DisconnectQuery) -> Result<Tunnels> {
    match query.tunnels.as_deref() {
        None => Ok(Tunnels::Refuse),
        Some("delete") => Ok(Tunnels::Delete),
        Some("keep") => Ok(Tunnels::Keep),
        Some(other) => Err(Failure::invalid_request(format!(
            "tunnels is delete or keep, and {other:?} is neither"
        ))),
    }
}

async fn disconnect(
    caller: Caller,
    Extension(playit): Extension<Arc<Playit>>,
    Params(query): Params<DisconnectQuery>,
) -> Result<StatusCode> {
    playit.of(caller.id()).disconnect(disposal(&query)?).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn restart(
    caller: Caller,
    Extension(playit): Extension<Arc<Playit>>,
) -> Result<(StatusCode, Json<PlayitStatus>)> {
    let connection = playit.of(caller.id());
    connection.restart_agent().await?;
    Ok((StatusCode::ACCEPTED, Json(connection.status().await?)))
}

async fn overview(
    _: Admin,
    Extension(playit): Extension<Arc<Playit>>,
) -> Result<Json<Vec<PlayitOverview>>> {
    Ok(Json(playit.overview().await?))
}

async fn disconnect_user(
    admin: Admin,
    State(state): State<AppState>,
    Extension(playit): Extension<Arc<Playit>>,
    Path(user): Path<String>,
    Params(query): Params<DisconnectQuery>,
) -> Result<StatusCode> {
    let mode = disposal(&query)?;
    let user: Id = user.parse().map_err(|_| unknown_user())?;

    let username: Option<String> = sqlx::query_scalar("SELECT username FROM users WHERE id = ?")
        .bind(user)
        .fetch_optional(&state.pool)
        .await?;
    let username = username.ok_or_else(unknown_user)?;

    playit.of(user).disconnect(mode).await?;
    tracing::warn!(
        by = %admin.0.user.username,
        user = %username,
        "a panel administrator disconnected somebody else's playit.gg account"
    );
    Ok(StatusCode::NO_CONTENT)
}

async fn tunnel(
    State(state): State<AppState>,
    Extension(playit): Extension<Arc<Playit>>,
    caller: Caller,
    OfServer(server): OfServer,
) -> Result<Json<ServerTunnel>> {
    access::require(&state.pool, &caller, server, Permission::BaseRead).await?;
    Ok(Json(playit.tunnel(server).await?))
}

async fn request_tunnel(
    State(state): State<AppState>,
    Extension(playit): Extension<Arc<Playit>>,
    caller: Caller,
    OfServer(server): OfServer,
) -> Result<(StatusCode, Json<ServerTunnel>)> {
    access::of(&state.pool, &caller, server).await?.require_ownership(&caller)?;

    let row: Option<(String, Id)> =
        sqlx::query_as("SELECT name, owner_id FROM servers WHERE id = ?")
            .bind(server)
            .fetch_optional(&state.pool)
            .await?;
    let (name, owner) = row.ok_or_else(unknown_server)?;

    let local_port = allocations::primary(&state.pool, server).await?.ok_or_else(|| {
        Failure::conflict(
            "playit_no_primary_port",
            "this server has no primary port yet, so there is nothing to point a tunnel at",
        )
    })?;

    let view = playit.of(owner).request_tunnel(server, &name, local_port).await?;
    Ok((StatusCode::ACCEPTED, Json(view)))
}

async fn drop_tunnel(
    State(state): State<AppState>,
    Extension(playit): Extension<Arc<Playit>>,
    caller: Caller,
    OfServer(server): OfServer,
) -> Result<StatusCode> {
    access::of(&state.pool, &caller, server).await?.require_ownership(&caller)?;
    playit.drop_tunnel(server).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
struct PublishRequest {
    kind: TunnelKind,
    port: u16,
    #[serde(default)]
    label: Option<String>,
}

async fn published(
    State(state): State<AppState>,
    Extension(playit): Extension<Arc<Playit>>,
    caller: Caller,
    OfServer(server): OfServer,
) -> Result<Json<Vec<PublishedPort>>> {
    access::require(&state.pool, &caller, server, Permission::BaseRead).await?;
    Ok(Json(playit.published(server).await?))
}

async fn publish(
    State(state): State<AppState>,
    Extension(playit): Extension<Arc<Playit>>,
    caller: Caller,
    OfServer(server): OfServer,
    JsonBody(body): JsonBody<PublishRequest>,
) -> Result<(StatusCode, Json<Vec<PublishedPort>>)> {
    access::of(&state.pool, &caller, server).await?.require_ownership(&caller)?;

    let row: Option<(String, Id)> =
        sqlx::query_as("SELECT name, owner_id FROM servers WHERE id = ?")
            .bind(server)
            .fetch_optional(&state.pool)
            .await?;
    let (name, owner) = row.ok_or_else(unknown_server)?;

    if body.kind == TunnelKind::MinecraftJava {
        return Err(Failure::bad_request(
            "playit_kind_is_main",
            "the Minecraft Java address of a server is requested on its overview page",
        ));
    }
    let primary = allocations::primary(&state.pool, server).await?;
    let held = primary == Some(body.port)
        || allocations::list(&state.pool, server).await?.iter().any(|allocation| allocation.port == body.port);
    if !held {
        return Err(Failure::bad_request(
            "playit_port_not_held",
            format!("port {} does not belong to this server; add it on the network tab first", body.port),
        ));
    }
    if primary == Some(body.port) && !body.kind.is_udp() {
        return Err(Failure::bad_request(
            "playit_primary_is_main",
            "the primary port is published through the server's own address on the overview page",
        ));
    }
    let label = match body.label.as_deref().map(str::trim).filter(|label| !label.is_empty()) {
        Some(label) if label.chars().count() <= 32 && !label.chars().any(char::is_control) => Some(label),
        Some(_) => {
            return Err(Failure::bad_request("invalid_label", "a label has at most 32 characters"))
        }
        None => None,
    };

    let slot = Slot { id: Id::new(), server, kind: body.kind, local_port: body.port, label };
    let tunnel_name = match label {
        Some(label) => format!("{name} {label}"),
        None => format!("{name} {}", body.kind.as_str()),
    };
    playit.of(owner).request(&slot, &tunnel_name).await?;
    Ok((StatusCode::ACCEPTED, Json(playit.published(server).await?)))
}

async fn unpublish(
    State(state): State<AppState>,
    Extension(playit): Extension<Arc<Playit>>,
    caller: Caller,
    Path((server, tunnel)): Path<(String, String)>,
) -> Result<StatusCode> {
    let server: Id = server.parse().map_err(|_| unknown_server())?;
    let tunnel: Id = tunnel
        .parse()
        .map_err(|_| Failure::not_found("playit_tunnel_not_found", "no such published port"))?;
    access::of(&state.pool, &caller, server).await?.require_ownership(&caller)?;
    playit.unpublish(server, tunnel).await?;
    Ok(StatusCode::NO_CONTENT)
}

fn unknown_server() -> Failure {
    Failure::not_found("server_not_found", "no such server")
}

fn unknown_user() -> Failure {
    Failure::not_found("user_not_found", "no such user")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::harness::{
        a_server, a_user, an_admin, as_user, body_json, empty, sign_in, state_with, test_pool,
    };
    use crate::config::Config;
    use crate::model::Timestamp;
    use crate::playit::{store, Secret};
    use axum::body::Body;
    use axum::http::Request;
    use sqlx::SqlitePool;
    use tower::ServiceExt;

    struct Panel {
        app: Router,
        pool: SqlitePool,
        playit: Arc<Playit>,
        dir: std::path::PathBuf,
    }

    impl Drop for Panel {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    async fn panel() -> Panel {
        let pool = test_pool().await;
        let dir = std::env::temp_dir().join(format!("craftpanel-playit-api-{}", Id::new()));
        let _ = std::fs::remove_dir_all(&dir);

        let mut config = Config::default();
        config.data_dir = dir.clone();
        let config = Arc::new(config);

        let playit = Playit::against(pool.clone(), Arc::clone(&config), "http://127.0.0.1:1")
            .unwrap();
        let app = Router::new()
            .nest("/api/v1", router(Arc::clone(&playit)))
            .with_state(state_with(&pool, (*config).clone()));

        Panel { app, pool, playit, dir }
    }

    async fn send(panel: &Panel, request: Request<Body>) -> axum::response::Response {
        panel.app.clone().oneshot(request).await.expect("a response")
    }

    #[tokio::test]
    async fn an_ordinary_user_reads_his_own_playit_page_and_not_the_overview() {
        let panel = panel().await;
        let anna = a_user(&panel.pool, "anna").await;
        let cookie = sign_in(&panel.pool, anna).await;

        let anonymous = send(&panel, empty("GET", "/api/v1/playit")).await;
        assert_eq!(anonymous.status(), StatusCode::UNAUTHORIZED);

        let mine = send(&panel, as_user(empty("GET", "/api/v1/playit"), &cookie)).await;
        assert_eq!(mine.status(), StatusCode::OK);

        let body = body_json(mine).await;
        assert_eq!(body["configured"], false);
        assert_eq!(body["ports"], serde_json::json!({ "used": 0, "udp_used": 0, "limit": 4, "for_others": 0 }));
        assert_eq!(body["claim"], serde_json::Value::Null);
        assert_eq!(body["agent"]["state"], "absent");

        let refused = send(&panel, as_user(empty("GET", "/api/v1/admin/playit"), &cookie)).await;
        assert_eq!(refused.status(), StatusCode::FORBIDDEN);

        let root = an_admin(&panel.pool, "root").await;
        let cookie = sign_in(&panel.pool, root).await;
        let allowed = send(&panel, as_user(empty("GET", "/api/v1/admin/playit"), &cookie)).await;
        assert_eq!(allowed.status(), StatusCode::OK);
        assert_eq!(body_json(allowed).await, serde_json::json!([]));
    }

    #[tokio::test]
    async fn the_answer_never_carries_a_field_that_could_hold_the_key() {
        let panel = panel().await;
        let anna = a_user(&panel.pool, "anna").await;
        let cookie = sign_in(&panel.pool, anna).await;

        let body =
            body_json(send(&panel, as_user(empty("GET", "/api/v1/playit"), &cookie)).await).await;

        let fields: Vec<&String> = body.as_object().unwrap().keys().collect();
        assert_eq!(
            fields,
            [
                "account_status",
                "agent",
                "agent_id",
                "binary",
                "checked_at",
                "claim",
                "configured",
                "has_premium",
                "is_self_managed",
                "last_error",
                "ports",
            ]
        );
        assert!(!body.to_string().contains("secret"), "{body}");
    }

    #[tokio::test]
    async fn the_overview_names_the_accounts_and_no_way_into_them() {
        let panel = panel().await;
        let anna = a_user(&panel.pool, "anna").await;
        let root = an_admin(&panel.pool, "root").await;
        let hers = a_server(&panel.pool, anna, "survival", 1024).await;

        panel.playit.of(anna).write_secret(&Secret::parse("aaaaaaaa").unwrap()).await.unwrap();
        store::claim_slot(&panel.pool, anna, hers, 25565, 4).await.unwrap();
        store::begin_claim(
            &panel.pool,
            anna,
            &store::Claim {
                code: "34ddf358a8".to_owned(),
                state: crate::playit::claim::ClaimState::WaitingForUser,
                started_at: Timestamp::now(),
            },
        )
        .await
        .unwrap();

        let cookie = sign_in(&panel.pool, root).await;
        let body =
            body_json(send(&panel, as_user(empty("GET", "/api/v1/admin/playit"), &cookie)).await)
                .await;

        assert_eq!(body.as_array().unwrap().len(), 1);
        let line = &body[0];
        assert_eq!(line["user_id"], anna.to_string());
        assert_eq!(line["username"], "anna");
        assert_eq!(line["configured"], true);
        assert_eq!(line["ports"], serde_json::json!({ "used": 1, "udp_used": 0, "limit": 4, "for_others": 0 }));
        assert_eq!(line["agent"]["state"], "absent");

        let fields: Vec<&String> = line.as_object().unwrap().keys().collect();
        assert_eq!(
            fields,
            [
                "account_status",
                "agent",
                "checked_at",
                "configured",
                "has_premium",
                "is_self_managed",
                "last_error",
                "ports",
                "user_id",
                "username",
            ]
        );
        assert!(!body.to_string().contains("34ddf358a8"), "the claim code is in the list: {body}");
        assert!(!body.to_string().contains("secret"), "{body}");
    }

    #[tokio::test]
    async fn a_sign_up_that_is_not_running_is_a_404_with_the_contract_code() {
        let panel = panel().await;
        let anna = a_user(&panel.pool, "anna").await;
        let cookie = sign_in(&panel.pool, anna).await;

        let missing = send(&panel, as_user(empty("GET", "/api/v1/playit/claim"), &cookie)).await;
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);
        assert_eq!(body_json(missing).await["error"], "playit_claim_not_found");

        let cancelled =
            send(&panel, as_user(empty("DELETE", "/api/v1/playit/claim"), &cookie)).await;
        assert_eq!(cancelled.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn a_sign_up_left_behind_is_still_there_when_the_page_comes_back() {
        let panel = panel().await;
        let anna = a_user(&panel.pool, "anna").await;
        let cookie = sign_in(&panel.pool, anna).await;

        store::begin_claim(
            &panel.pool,
            anna,
            &store::Claim {
                code: "34ddf358a8".to_owned(),
                state: crate::playit::claim::ClaimState::WaitingForUser,
                started_at: Timestamp::now(),
            },
        )
        .await
        .unwrap();

        let found = send(&panel, as_user(empty("GET", "/api/v1/playit/claim"), &cookie)).await;
        assert_eq!(found.status(), StatusCode::OK);

        let body = body_json(found).await;
        assert_eq!(body["code"], "34ddf358a8");
        assert_eq!(body["url"], "https://playit.gg/claim/34ddf358a8");
        assert_eq!(body["state"], "waiting_for_user");
        assert!(body["expires_at"].as_str().unwrap() > body["started_at"].as_str().unwrap());

        let gone = send(&panel, as_user(empty("DELETE", "/api/v1/playit/claim"), &cookie)).await;
        assert_eq!(gone.status(), StatusCode::NO_CONTENT);
        assert!(store::account(&panel.pool, anna).await.unwrap().unwrap().claim.is_none());
    }

    #[tokio::test]
    async fn one_users_sign_up_is_not_on_another_users_page() {
        let panel = panel().await;
        let anna = a_user(&panel.pool, "anna").await;
        let ben = a_user(&panel.pool, "ben").await;

        store::begin_claim(
            &panel.pool,
            anna,
            &store::Claim {
                code: "aaaaaaaaaa".to_owned(),
                state: crate::playit::claim::ClaimState::WaitingForUser,
                started_at: Timestamp::now(),
            },
        )
        .await
        .unwrap();

        let hers = sign_in(&panel.pool, anna).await;
        let his = sign_in(&panel.pool, ben).await;

        let mine = send(&panel, as_user(empty("GET", "/api/v1/playit/claim"), &hers)).await;
        assert_eq!(body_json(mine).await["code"], "aaaaaaaaaa");

        let theirs = send(&panel, as_user(empty("GET", "/api/v1/playit/claim"), &his)).await;
        assert_eq!(theirs.status(), StatusCode::NOT_FOUND, "ben sees anna's sign-up");

        let cancelled = send(&panel, as_user(empty("DELETE", "/api/v1/playit/claim"), &his)).await;
        assert_eq!(cancelled.status(), StatusCode::NOT_FOUND);
        assert!(store::account(&panel.pool, anna).await.unwrap().unwrap().claim.is_some());
    }

    #[tokio::test]
    async fn disconnecting_only_takes_the_two_words_it_knows() {
        let panel = panel().await;
        let anna = a_user(&panel.pool, "anna").await;
        let cookie = sign_in(&panel.pool, anna).await;

        let nonsense =
            send(&panel, as_user(empty("DELETE", "/api/v1/playit?tunnels=maybe"), &cookie)).await;
        assert_eq!(nonsense.status(), StatusCode::BAD_REQUEST);
        assert_eq!(body_json(nonsense).await["error"], "invalid_request");

        let bare = send(&panel, as_user(empty("DELETE", "/api/v1/playit"), &cookie)).await;
        assert_eq!(bare.status(), StatusCode::NO_CONTENT, "nothing to hold on to");
    }

    #[tokio::test]
    async fn only_an_administrator_can_cut_somebody_else_loose() {
        let panel = panel().await;
        let anna = a_user(&panel.pool, "anna").await;
        let ben = a_user(&panel.pool, "ben").await;
        let root = an_admin(&panel.pool, "root").await;

        panel.playit.of(anna).write_secret(&Secret::parse("aaaaaaaa").unwrap()).await.unwrap();
        panel.playit.of(ben).write_secret(&Secret::parse("bbbbbbbb").unwrap()).await.unwrap();

        let his = sign_in(&panel.pool, ben).await;
        let path = format!("/api/v1/admin/playit/{anna}");
        assert_eq!(
            send(&panel, as_user(empty("DELETE", &path), &his)).await.status(),
            StatusCode::FORBIDDEN
        );
        assert!(panel.playit.of(anna).configured().await);

        let cookie = sign_in(&panel.pool, root).await;
        let cut = send(&panel, as_user(empty("DELETE", &path), &cookie)).await;
        assert_eq!(cut.status(), StatusCode::NO_CONTENT);
        assert!(!panel.playit.of(anna).configured().await);
        assert!(panel.playit.of(ben).configured().await, "ben went with anna");

        let nobody = send(
            &panel,
            as_user(empty("DELETE", "/api/v1/admin/playit/01JZZZZZZZZZZZZZZZZZZZZZZZ"), &cookie),
        )
        .await;
        assert_eq!(nobody.status(), StatusCode::NOT_FOUND);
        assert_eq!(body_json(nobody).await["error"], "user_not_found");
    }

    #[tokio::test]
    async fn the_endpoints_that_signed_the_whole_panel_up_are_gone() {
        let panel = panel().await;
        let root = an_admin(&panel.pool, "root").await;
        let cookie = sign_in(&panel.pool, root).await;

        for (method, path) in [
            ("GET", "/api/v1/admin/playit/claim"),
            ("POST", "/api/v1/admin/playit/claim"),
            ("DELETE", "/api/v1/admin/playit/claim"),
            ("POST", "/api/v1/admin/playit/agent/restart"),
            ("DELETE", "/api/v1/admin/playit"),
        ] {
            let answer = send(&panel, as_user(empty(method, path), &cookie)).await;
            assert!(
                answer.status() == StatusCode::NOT_FOUND
                    || answer.status() == StatusCode::METHOD_NOT_ALLOWED,
                "{method} {path} answered {}",
                answer.status()
            );
        }
    }

    #[tokio::test]
    async fn restarting_the_agent_before_there_is_one_names_the_step_missed() {
        let panel = panel().await;
        let anna = a_user(&panel.pool, "anna").await;
        let cookie = sign_in(&panel.pool, anna).await;

        let refused =
            send(&panel, as_user(empty("POST", "/api/v1/playit/agent/restart"), &cookie)).await;
        assert_eq!(refused.status(), StatusCode::CONFLICT);
        assert_eq!(body_json(refused).await["error"], "playit_not_configured");
    }

    #[tokio::test]
    async fn a_viewer_may_read_the_address_and_may_not_ask_for_one() {
        let panel = panel().await;
        let anna = a_user(&panel.pool, "anna").await;
        let ben = a_user(&panel.pool, "ben").await;
        let server = a_server(&panel.pool, anna, "survival", 1024).await;

        sqlx::query(
            "INSERT INTO server_members (id, server_id, user_id, role, invited_at, joined_at) \
             VALUES (?, ?, ?, 'viewer', ?, ?)",
        )
        .bind(Id::new())
        .bind(server)
        .bind(ben)
        .bind(Timestamp::now())
        .bind(Timestamp::now())
        .execute(&panel.pool)
        .await
        .unwrap();

        let cookie = sign_in(&panel.pool, ben).await;
        let path = format!("/api/v1/servers/{server}/playit");

        let read = send(&panel, as_user(empty("GET", &path), &cookie)).await;
        assert_eq!(read.status(), StatusCode::OK);
        let body = body_json(read).await;
        assert_eq!(body["state"], "none");
        assert_eq!(body["addresses"], serde_json::json!([]));
        assert_eq!(body["local_port"], serde_json::Value::Null);
        assert!(body.get("ports").is_none(), "a viewer was told the owner's budget");

        let refused = send(&panel, as_user(empty("POST", &path), &cookie)).await;
        assert_eq!(refused.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn an_editor_may_not_put_the_server_on_the_internet() {
        let panel = panel().await;
        let anna = a_user(&panel.pool, "anna").await;
        let ben = a_user(&panel.pool, "ben").await;
        let server = a_server(&panel.pool, anna, "survival", 1024).await;

        sqlx::query(
            "INSERT INTO server_members (id, server_id, user_id, role, invited_at, joined_at) \
             VALUES (?, ?, ?, 'editor', ?, ?)",
        )
        .bind(Id::new())
        .bind(server)
        .bind(ben)
        .bind(Timestamp::now())
        .bind(Timestamp::now())
        .execute(&panel.pool)
        .await
        .unwrap();

        let cookie = sign_in(&panel.pool, ben).await;
        let path = format!("/api/v1/servers/{server}/playit");

        assert_eq!(
            send(&panel, as_user(empty("POST", &path), &cookie)).await.status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            send(&panel, as_user(empty("DELETE", &path), &cookie)).await.status(),
            StatusCode::FORBIDDEN
        );
    }

    #[tokio::test]
    async fn a_stranger_is_told_the_server_does_not_exist() {
        let panel = panel().await;
        let anna = a_user(&panel.pool, "anna").await;
        let stranger = a_user(&panel.pool, "mallory").await;
        let server = a_server(&panel.pool, anna, "survival", 1024).await;
        let cookie = sign_in(&panel.pool, stranger).await;

        let hidden = send(
            &panel,
            as_user(empty("GET", &format!("/api/v1/servers/{server}/playit")), &cookie),
        )
        .await;
        assert_eq!(hidden.status(), StatusCode::NOT_FOUND);
        assert_eq!(body_json(hidden).await["error"], "server_not_found");

        let nonsense =
            send(&panel, as_user(empty("GET", "/api/v1/servers/not-a-ulid/playit"), &cookie)).await;
        assert_eq!(nonsense.status(), StatusCode::NOT_FOUND);
        assert_eq!(body_json(nonsense).await["error"], "server_not_found");
    }

    #[tokio::test]
    async fn a_server_with_no_primary_port_has_nothing_to_point_a_tunnel_at() {
        let panel = panel().await;
        let anna = a_user(&panel.pool, "anna").await;
        let server = a_server(&panel.pool, anna, "survival", 1024).await;
        let cookie = sign_in(&panel.pool, anna).await;

        let refused = send(
            &panel,
            as_user(empty("POST", &format!("/api/v1/servers/{server}/playit")), &cookie),
        )
        .await;
        assert_eq!(refused.status(), StatusCode::CONFLICT);
        assert_eq!(body_json(refused).await["error"], "playit_no_primary_port");
    }

    async fn with_a_primary_port(pool: &SqlitePool, server: Id) {
        sqlx::query(
            "INSERT INTO allocations (port, server_id, name, is_primary, created_at) \
             VALUES (25565, ?, 'primary', 1, ?)",
        )
        .bind(server)
        .bind(Timestamp::now())
        .execute(pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn the_owner_gets_as_far_as_the_missing_sign_up_and_no_further() {
        let panel = panel().await;
        let anna = a_user(&panel.pool, "anna").await;
        let server = a_server(&panel.pool, anna, "survival", 1024).await;
        with_a_primary_port(&panel.pool, server).await;

        let cookie = sign_in(&panel.pool, anna).await;
        let refused = send(
            &panel,
            as_user(empty("POST", &format!("/api/v1/servers/{server}/playit")), &cookie),
        )
        .await;

        assert_eq!(refused.status(), StatusCode::CONFLICT);
        assert_eq!(body_json(refused).await["error"], "playit_not_configured");
        assert_eq!(store::used(&panel.pool, anna).await.unwrap(), 0, "nothing was written");
    }

    #[tokio::test]
    async fn an_address_is_always_made_on_the_owners_account() {
        let panel = panel().await;
        let anna = a_user(&panel.pool, "anna").await;
        let root = an_admin(&panel.pool, "root").await;
        let server = a_server(&panel.pool, anna, "survival", 1024).await;
        with_a_primary_port(&panel.pool, server).await;

        panel.playit.of(root).write_secret(&Secret::parse("dddddddd").unwrap()).await.unwrap();

        let cookie = sign_in(&panel.pool, root).await;
        let refused = send(
            &panel,
            as_user(empty("POST", &format!("/api/v1/servers/{server}/playit")), &cookie),
        )
        .await;
        assert_eq!(refused.status(), StatusCode::CONFLICT);
        assert_eq!(body_json(refused).await["error"], "playit_not_configured");
        assert_eq!(store::used(&panel.pool, root).await.unwrap(), 0, "he spent his own port");

        panel.playit.of(anna).write_secret(&Secret::parse("aaaaaaaa").unwrap()).await.unwrap();
        let accepted = send(
            &panel,
            as_user(empty("POST", &format!("/api/v1/servers/{server}/playit")), &cookie),
        )
        .await;
        assert_eq!(accepted.status(), StatusCode::ACCEPTED);

        let row = store::tunnel(&panel.pool, server).await.unwrap().expect("the row");
        assert_eq!(row.user_id, anna, "the tunnel went on the administrator's account");
        assert_eq!(store::used(&panel.pool, anna).await.unwrap(), 1);
        assert_eq!(store::used(&panel.pool, root).await.unwrap(), 0);
    }

    async fn an_extra_port(pool: &SqlitePool, server: Id, port: u16) {
        sqlx::query(
            "INSERT INTO allocations (port, server_id, name, is_primary, created_at) \
             VALUES (?, ?, 'extra', 0, ?)",
        )
        .bind(port)
        .bind(server)
        .bind(Timestamp::now())
        .execute(pool)
        .await
        .unwrap();
    }

    fn publish(server: Id, body: serde_json::Value) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri(format!("/api/v1/servers/{server}/playit/ports"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    }

    #[tokio::test]
    async fn a_voice_chat_port_is_published_as_udp_next_to_the_game_address() {
        let panel = panel().await;
        let anna = a_user(&panel.pool, "anna").await;
        let server = a_server(&panel.pool, anna, "survival", 1024).await;
        with_a_primary_port(&panel.pool, server).await;
        an_extra_port(&panel.pool, server, 25600).await;
        panel.playit.of(anna).write_secret(&Secret::parse("aaaaaaaa").unwrap()).await.unwrap();
        let cookie = sign_in(&panel.pool, anna).await;

        let accepted = send(
            &panel,
            as_user(publish(server, serde_json::json!({ "kind": "udp", "port": 25600, "label": "Voice chat" })), &cookie),
        )
        .await;
        assert_eq!(accepted.status(), StatusCode::ACCEPTED);
        let listed = body_json(accepted).await;
        assert_eq!(listed[0]["kind"], "udp");
        assert_eq!(listed[0]["label"], "Voice chat");
        assert_eq!(listed[0]["local_port"], 25600);
        assert_eq!(listed[0]["state"], "pending");
        assert_eq!(store::used_udp(&panel.pool, anna).await.unwrap(), 1);
        assert_eq!(store::used(&panel.pool, anna).await.unwrap(), 0, "UDP does not eat a TCP port");

        let again = send(&panel, as_user(publish(server, serde_json::json!({ "kind": "udp", "port": 25600 })), &cookie)).await;
        assert_eq!(again.status(), StatusCode::CONFLICT);
        assert_eq!(body_json(again).await["error"], "playit_tunnel_exists");

        for (body, code) in [
            (serde_json::json!({ "kind": "udp", "port": 30000 }), "playit_port_not_held"),
            (serde_json::json!({ "kind": "tcp", "port": 25565 }), "playit_primary_is_main"),
            (serde_json::json!({ "kind": "minecraft-java", "port": 25600 }), "playit_kind_is_main"),
        ] {
            let refused = send(&panel, as_user(publish(server, body.clone()), &cookie)).await;
            assert_eq!(refused.status(), StatusCode::BAD_REQUEST, "{body}");
            assert_eq!(body_json(refused).await["error"], code, "{body}");
        }
        let bedrock = send(&panel, as_user(publish(server, serde_json::json!({ "kind": "minecraft-bedrock", "port": 25565 })), &cookie)).await;
        assert_eq!(bedrock.status(), StatusCode::ACCEPTED, "Geyser may share the primary port number over UDP");

        let viewer = a_user(&panel.pool, "ben").await;
        let viewer_cookie = sign_in(&panel.pool, viewer).await;
        sqlx::query(
            "INSERT INTO server_members (id, server_id, user_id, role, invited_at, joined_at) \
             VALUES (?, ?, ?, 'viewer', ?, ?)",
        )
        .bind(Id::new())
        .bind(server)
        .bind(viewer)
        .bind(Timestamp::now())
        .bind(Timestamp::now())
        .execute(&panel.pool)
        .await
        .unwrap();
        let read = send(&panel, as_user(empty("GET", &format!("/api/v1/servers/{server}/playit/ports")), &viewer_cookie)).await;
        assert_eq!(body_json(read).await.as_array().unwrap().len(), 2, "a viewer sees the addresses");
        let not_his = send(&panel, as_user(publish(server, serde_json::json!({ "kind": "tcp", "port": 25600 })), &viewer_cookie)).await;
        assert_eq!(not_his.status(), StatusCode::FORBIDDEN);

        let id = listed[0]["id"].as_str().unwrap().to_owned();
        let gone = send(&panel, as_user(empty("DELETE", &format!("/api/v1/servers/{server}/playit/ports/{id}")), &cookie)).await;
        assert_eq!(gone.status(), StatusCode::NO_CONTENT);
        let main = send(&panel, as_user(empty("DELETE", &format!("/api/v1/servers/{server}/playit/ports/{server}")), &cookie)).await;
        assert_eq!(main.status(), StatusCode::NOT_FOUND, "the game address is not one of the extra ports");
    }

    #[tokio::test]
    async fn udp_and_tcp_each_count_against_their_own_four() {
        let panel = panel().await;
        let anna = a_user(&panel.pool, "anna").await;
        let mut servers = Vec::new();
        for index in 0..5u16 {
            let server = a_server(&panel.pool, anna, &format!("s{index}"), 1024).await;
            an_extra_port(&panel.pool, server, 25600 + index).await;
            let slot = store::Slot { id: Id::new(), server, kind: store::TunnelKind::Udp, local_port: 25600 + index, label: None };
            servers.push(server);
            let claimed = store::claim(&panel.pool, anna, &slot, 4).await.unwrap();
            assert_eq!(matches!(claimed, store::Claimed::Full), index == 4, "UDP tunnel {index}");
        }
        assert!(matches!(store::claim_slot(&panel.pool, anna, servers[0], 25565, 4).await.unwrap(), store::Claimed::Ok));
    }

    #[tokio::test]
    async fn the_database_keeps_tunnels_and_ports_from_drifting_apart() {
        let panel = panel().await;
        let anna = a_user(&panel.pool, "anna").await;
        let ben = a_user(&panel.pool, "ben").await;
        let hers = a_server(&panel.pool, anna, "hers", 1024).await;
        let his = a_server(&panel.pool, ben, "his", 1024).await;
        an_extra_port(&panel.pool, hers, 25600).await;

        let stray = store::Slot { id: Id::new(), server: hers, kind: store::TunnelKind::Udp, local_port: 25601, label: None };
        assert!(store::claim(&panel.pool, anna, &stray, 4).await.is_err(), "no tunnel onto a port the server does not hold");

        let voice = store::Slot { id: Id::new(), server: hers, kind: store::TunnelKind::Udp, local_port: 25600, label: None };
        store::claim(&panel.pool, anna, &voice, 4).await.unwrap();
        store::attach(&panel.pool, voice.id, "voice-tunnel").await.unwrap();
        let refused = crate::settings::allocations::remove(&panel.pool, hers, 25600).await.unwrap_err();
        assert_eq!(refused.code(), "port_published", "even without the check in the handler");

        let squatter = sqlx::query(
            "INSERT INTO allocations (port, server_id, name, is_primary, created_at) VALUES (25600, ?, 'x', 0, ?)",
        )
        .bind(his)
        .bind(Timestamp::now())
        .execute(&panel.pool)
        .await;
        assert!(squatter.is_err());

        store::forget(&panel.pool, voice.id).await.unwrap();
        crate::settings::allocations::remove(&panel.pool, hers, 25600).await.unwrap();
        let still_out_there = sqlx::query(
            "INSERT INTO allocations (port, server_id, name, is_primary, created_at) VALUES (25600, ?, 'x', 0, ?)",
        )
        .bind(his)
        .bind(Timestamp::now())
        .execute(&panel.pool)
        .await;
        assert!(still_out_there.is_err(), "until playit.gg has the tunnel back, nobody else gets that port");
        let in_use: Vec<i64> = sqlx::query_scalar("SELECT port FROM ports_in_use").fetch_all(&panel.pool).await.unwrap();
        assert!(in_use.contains(&25600));

        store::settled(&panel.pool, "voice-tunnel").await.unwrap();
        sqlx::query(
            "INSERT INTO allocations (port, server_id, name, is_primary, created_at) VALUES (25600, ?, 'x', 0, ?)",
        )
        .bind(his)
        .bind(Timestamp::now())
        .execute(&panel.pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn a_server_being_deleted_lets_go_of_its_published_ports() {
        let panel = panel().await;
        let anna = a_user(&panel.pool, "anna").await;
        let server = a_server(&panel.pool, anna, "survival", 1024).await;
        an_extra_port(&panel.pool, server, 25600).await;
        let voice = store::Slot { id: Id::new(), server, kind: store::TunnelKind::Udp, local_port: 25600, label: None };
        store::claim(&panel.pool, anna, &voice, 4).await.unwrap();
        store::attach(&panel.pool, voice.id, "voice-tunnel").await.unwrap();

        sqlx::query("UPDATE servers SET status = 'deleting' WHERE id = ?").bind(server).execute(&panel.pool).await.unwrap();
        sqlx::query("DELETE FROM allocations WHERE server_id = ?").bind(server).execute(&panel.pool).await.unwrap();
        sqlx::query("DELETE FROM servers WHERE id = ?").bind(server).execute(&panel.pool).await.unwrap();

        assert_eq!(store::released(&panel.pool, anna).await.unwrap(), ["voice-tunnel"], "the debt is written down");
    }

    #[tokio::test]
    async fn an_owner_reads_a_tunnel_that_is_already_on_the_books() {
        let panel = panel().await;
        let anna = a_user(&panel.pool, "anna").await;
        let server = a_server(&panel.pool, anna, "survival", 1024).await;
        let cookie = sign_in(&panel.pool, anna).await;

        store::claim_slot(&panel.pool, anna, server, 25565, 4).await.unwrap();
        store::attach(&panel.pool, server, "c0ffee11").await.unwrap();
        store::set_state(
            &panel.pool,
            server,
            store::TunnelState::Online,
            &[crate::playit::tunnels::Address {
                address: "quiet-forest.gl.at.ply.gg".to_owned(),
                kind: crate::playit::tunnels::AddressKind::Auto,
            }],
            None,
        )
        .await
        .unwrap();

        let body = body_json(
            send(
                &panel,
                as_user(empty("GET", &format!("/api/v1/servers/{server}/playit")), &cookie),
            )
            .await,
        )
        .await;

        assert_eq!(body["state"], "online");
        assert_eq!(body["local_port"], 25565);
        assert_eq!(body["addresses"][0]["address"], "quiet-forest.gl.at.ply.gg");
        assert_eq!(body["addresses"][0]["kind"], "auto");
    }
}
