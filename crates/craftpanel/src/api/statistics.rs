use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use axum::extract::State;
use axum::routing::get;
use axum::{Extension, Json, Router};
use serde::Serialize;

use crate::auth::error::Result;
use crate::auth::{extract, usage, Admin};
use crate::model::{Id, LoaderId, PowerState, ServerStatus, Timestamp, UserRef};
use crate::servers::manager::Manager;
use crate::AppState;

pub fn router(manager: Arc<Manager>) -> Router<AppState> {
    Router::new()
        .route("/admin/statistics", get(statistics))
        .layer(Extension(manager))
        .layer(axum::middleware::from_fn(extract::same_origin))
}

#[derive(Debug, Serialize)]
struct Statistics {
    host: HostFacts,
    servers: ServerCounts,
    memory: MemoryFacts,
    games: GameUsage,
    backups: BackupFacts,
    users: u32,
    active_operations: u32,
    running: Vec<RunningServer>,
    accounts: Vec<AccountUsage>,
    measured_at: Timestamp,
}

#[derive(Debug, Serialize)]
struct HostFacts {
    cpu_cores: u32,
    load_average: [f64; 3],
    memory_total_bytes: u64,
    memory_available_bytes: u64,
    disk_total_bytes: u64,
    disk_free_bytes: u64,
    uptime_seconds: u64,
    version: &'static str,
}

#[derive(Debug, Default, Serialize)]
struct ServerCounts {
    total: u32,
    running: u32,
    starting: u32,
    stopping: u32,
    crashed: u32,
    stopped: u32,
    installing: u32,
    broken: u32,
}

#[derive(Debug, Serialize)]
struct MemoryFacts {
    assignable_mib: u32,
    promised_mib: u32,
    running_mib: u32,
}

#[derive(Debug, Serialize)]
struct GameUsage {
    memory_bytes: u64,
    cpu_cores: f64,
    pids: u32,
}

#[derive(Debug, Serialize)]
struct BackupFacts {
    count: u32,
    bytes: u64,
}

#[derive(Debug, Serialize)]
struct RunningServer {
    id: Id,
    name: String,
    owner: UserRef,
    state: PowerState,
    loader: Option<LoaderId>,
    game_version: Option<String>,
    memory_mib: u32,
    ram_usage_bytes: u64,
    cpu_percent: f64,
    running_since: Option<Timestamp>,
    port: Option<u16>,
}

#[derive(Debug, Serialize)]
struct AccountUsage {
    user: UserRef,
    servers: u32,
    running: u32,
    memory_bytes: u64,
    cpu_cores: f64,
    memory_limit_mib: Option<u32>,
}

#[derive(Debug, sqlx::FromRow)]
struct ServerRow {
    id: Id,
    name: String,
    owner_id: Id,
    owner_name: String,
    status: ServerStatus,
    power_state: PowerState,
    loader: Option<LoaderId>,
    game_version: Option<String>,
    memory_mib: u32,
    running_since: Option<Timestamp>,
    port: Option<i64>,
}

async fn statistics(
    State(state): State<AppState>,
    Admin(_): Admin,
    Extension(manager): Extension<Arc<Manager>>,
) -> Result<Json<Statistics>> {
    let rows = sqlx::query_as::<_, ServerRow>(
        "SELECT s.id, s.name, s.owner_id, u.username AS owner_name, s.status, s.power_state, \
                s.loader, s.game_version, s.memory_mib, s.running_since, \
                (SELECT a.port FROM allocations a WHERE a.server_id = s.id AND a.is_primary = 1) AS port \
           FROM servers s JOIN users u ON u.id = s.owner_id \
          ORDER BY s.name",
    )
    .fetch_all(&state.pool)
    .await?;

    let mut counts = ServerCounts { total: rows.len() as u32, ..ServerCounts::default() };
    let mut running_mib = 0u32;
    let mut running = Vec::new();
    for row in &rows {
        match row.status {
            ServerStatus::Installing => counts.installing += 1,
            ServerStatus::Broken => counts.broken += 1,
            _ => {}
        }
        match row.power_state {
            PowerState::Running => counts.running += 1,
            PowerState::Starting => counts.starting += 1,
            PowerState::Stopping => counts.stopping += 1,
            PowerState::Crashed => counts.crashed += 1,
            PowerState::Stopped => counts.stopped += 1,
        }
        if matches!(row.power_state, PowerState::Running | PowerState::Starting | PowerState::Stopping) {
            running_mib = running_mib.saturating_add(row.memory_mib);
            let sample = manager.sample(row.id).await.ok();
            running.push(RunningServer {
                id: row.id,
                name: row.name.clone(),
                owner: UserRef { id: row.owner_id, username: row.owner_name.clone(), avatar_url: None },
                state: row.power_state,
                loader: row.loader,
                game_version: row.game_version.clone(),
                memory_mib: row.memory_mib,
                ram_usage_bytes: sample.map_or(0, |sample| sample.ram_usage_bytes),
                cpu_percent: sample.map_or(0.0, |sample| sample.cpu_percent),
                running_since: row.running_since,
                port: row.port.and_then(|port| u16::try_from(port).ok()),
            });
        }
    }
    running.sort_by(|left, right| right.ram_usage_bytes.cmp(&left.ram_usage_bytes));

    let users: Vec<(Id, String, u32, crate::model::PanelRole)> =
        sqlx::query_as("SELECT id, username, memory_mib, role FROM users ORDER BY username")
            .fetch_all(&state.pool)
            .await?;
    let cgroups = usage::shared();
    let mut per_user: BTreeMap<Id, (u32, u32)> = BTreeMap::new();
    for row in &rows {
        let entry = per_user.entry(row.owner_id).or_default();
        entry.0 += 1;
        if matches!(row.power_state, PowerState::Running | PowerState::Starting | PowerState::Stopping) {
            entry.1 += 1;
        }
    }
    let mut accounts: Vec<AccountUsage> = users
        .iter()
        .map(|(id, username, memory_mib, role)| {
            let sample = cgroups.sample(*id);
            let (servers, running) = per_user.get(id).copied().unwrap_or_default();
            AccountUsage {
                user: UserRef { id: *id, username: username.clone(), avatar_url: None },
                servers,
                running,
                memory_bytes: sample.memory_bytes,
                cpu_cores: sample.used_cores,
                memory_limit_mib: (*role != crate::model::PanelRole::Admin).then_some(*memory_mib),
            }
        })
        .collect();
    accounts.sort_by(|left, right| {
        right.memory_bytes.cmp(&left.memory_bytes).then(right.servers.cmp(&left.servers))
    });
    let games = cgroups.total(users.iter().map(|(id, ..)| *id));

    let (backup_count, backup_bytes): (i64, i64) =
        sqlx::query_as("SELECT count(*), coalesce(sum(size_bytes), 0) FROM backups")
            .fetch_one(&state.pool)
            .await?;
    let active_operations: i64 =
        sqlx::query_scalar("SELECT count(*) FROM operations WHERE state IN ('queued', 'ongoing')")
            .fetch_one(&state.pool)
            .await?;

    let machine = usage::Host::measure();
    let data_dir = state.config.data_dir.clone();
    let host = HostFacts {
        cpu_cores: machine.cpu_cores,
        load_average: load_average(),
        memory_total_bytes: machine.memory_total_bytes,
        memory_available_bytes: meminfo_bytes("MemAvailable:").unwrap_or(0),
        disk_total_bytes: crate::files::filesystem_total_bytes(&data_dir),
        disk_free_bytes: filesystem_free_bytes(&data_dir),
        uptime_seconds: uptime_seconds(),
        version: env!("CARGO_PKG_VERSION"),
    };

    Ok(Json(Statistics {
        host,
        servers: counts,
        memory: MemoryFacts {
            assignable_mib: machine.assignable_memory_mib(),
            promised_mib: rows.iter().map(|row| row.memory_mib).sum(),
            running_mib,
        },
        games: GameUsage { memory_bytes: games.memory_bytes, cpu_cores: games.used_cores, pids: games.pids },
        backups: BackupFacts { count: backup_count.max(0) as u32, bytes: backup_bytes.max(0) as u64 },
        users: users.len() as u32,
        active_operations: active_operations.max(0) as u32,
        running,
        accounts,
        measured_at: Timestamp::now(),
    }))
}

fn load_average() -> [f64; 3] {
    let text = std::fs::read_to_string("/proc/loadavg").unwrap_or_default();
    let mut parts = text.split_whitespace().map(|part| part.parse::<f64>().unwrap_or(0.0));
    [parts.next().unwrap_or(0.0), parts.next().unwrap_or(0.0), parts.next().unwrap_or(0.0)]
}

fn meminfo_bytes(field: &str) -> Option<u64> {
    let text = std::fs::read_to_string("/proc/meminfo").ok()?;
    let kib: u64 = text.lines().find_map(|line| line.strip_prefix(field))?.split_whitespace().next()?.parse().ok()?;
    Some(kib * 1024)
}

fn uptime_seconds() -> u64 {
    std::fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|text| text.split_whitespace().next().and_then(|first| first.parse::<f64>().ok()))
        .map_or(0, |seconds| seconds as u64)
}

fn filesystem_free_bytes(path: &Path) -> u64 {
    use std::os::unix::ffi::OsStrExt;

    let Ok(name) = std::ffi::CString::new(path.as_os_str().as_bytes()) else {
        return 0;
    };
    let mut stats: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(name.as_ptr(), &mut stats) } != 0 {
        return 0;
    }
    (stats.f_bavail as u64).saturating_mul(stats.f_frsize as u64)
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    use super::*;
    use crate::auth::harness::{a_server, a_user, an_admin, as_user, fetch, sign_in, state_with, test_pool, FakeHelper};
    use crate::auth::Disks;
    use crate::config::Config;
    use crate::ops::testing::DataDir;
    use crate::ops::Operations;
    use crate::servers::manager::fake::Shelf;
    use crate::servers::Hub;

    #[tokio::test]
    async fn an_administrator_sees_how_busy_the_machine_is_and_nobody_else_does() {
        let dir = DataDir::new();
        let pool = test_pool().await;
        let helper = FakeHelper::obliging().await;
        let config = Arc::new(Config {
            data_dir: dir.path().to_path_buf(),
            helper_socket: helper.socket(),
            ..Config::default()
        });
        let manager = Manager::new(
            pool.clone(),
            Arc::clone(&config),
            Operations::new(pool.clone(), dir.path()),
            Arc::new(Hub::new(dir.path().join("supervisors.sock"))),
            crate::helper::Helper::new(helper.socket()),
            Shelf::new(),
            Disks::none(),
            Arc::new(crate::java::Runtimes::with_base(dir.path(), "http://127.0.0.1:9").expect("a client")),
        );
        let app = router(manager).with_state(state_with(&pool, (*config).clone()));

        let max = a_user(&pool, "max").await;
        let lobby = a_server(&pool, max, "Lobby", 2048).await;
        a_server(&pool, max, "Survival", 4096).await;
        sqlx::query("UPDATE servers SET power_state = 'running', running_since = ? WHERE id = ?")
            .bind(Timestamp::now())
            .bind(lobby)
            .execute(&pool)
            .await
            .unwrap();

        let call = |request: Request<Body>| {
            let app = app.clone();
            async move {
                let response = app.oneshot(request).await.unwrap();
                let status = response.status();
                (status, crate::auth::harness::body_json(response).await)
            }
        };

        let (status, _) = call(as_user(fetch("/admin/statistics"), &sign_in(&pool, max).await)).await;
        assert_eq!(status, StatusCode::FORBIDDEN);

        let boss = an_admin(&pool, "boss").await;
        let (status, seen) = call(as_user(fetch("/admin/statistics"), &sign_in(&pool, boss).await)).await;
        assert_eq!(status, StatusCode::OK, "{seen}");
        assert_eq!(seen["servers"]["total"], 2);
        assert_eq!(seen["servers"]["running"], 1);
        assert_eq!(seen["servers"]["stopped"], 1);
        assert_eq!(seen["memory"]["promised_mib"], 6144);
        assert_eq!(seen["memory"]["running_mib"], 2048);
        assert_eq!(seen["running"][0]["name"], "Lobby");
        assert_eq!(seen["running"][0]["owner"]["username"], "max");
        assert_eq!(seen["users"], 2);
        let max_row = seen["accounts"].as_array().unwrap().iter().find(|row| row["user"]["username"] == "max").unwrap();
        assert_eq!((max_row["servers"].as_u64(), max_row["running"].as_u64()), (Some(2), Some(1)));
        assert!(seen["accounts"].as_array().unwrap().iter().any(|row| row["user"]["username"] == "boss" && row["memory_limit_mib"].is_null()));
        assert!(seen["host"]["cpu_cores"].as_u64().unwrap() >= 1);
    }
}
