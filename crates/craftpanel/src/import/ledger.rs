use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio::io::AsyncWriteExt;

use crate::config::Config;
use crate::model::{Id, LoaderId, SystemUserState, Timestamp};

pub enum Ledger {
    Open(SqlitePool),
    Through { program: PathBuf, config: PathBuf, uid: u32, gid: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub server: Id,
    pub owner: Id,
    pub name: String,
    pub loader: LoaderId,
    pub loader_version: Option<String>,
    pub game_version: String,
    pub memory_mib: u32,
    pub flags: Vec<String>,
    pub port: u16,
    pub instance: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    Owner { username: String },
    Ports,
    Record(Entry),
    Forget { server: Id },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Answer {
    Owner { id: Id, state: SystemUserState },
    Ports { taken: Vec<u16>, from: u16, to: u16 },
    Done,
    Failed(String),
}

impl Ledger {
    pub async fn open(config_path: &Path, config: &Config) -> Result<Self> {
        let database = config.database_path();
        let meta = std::fs::metadata(&database)
            .with_context(|| format!("there is no panel database at {}; install the panel first", database.display()))?;
        if unsafe { libc::geteuid() } == 0 && meta.uid() != 0 {
            return Ok(Self::Through {
                program: std::env::current_exe().context("finding the craftpanel program")?,
                config: config_path.to_owned(),
                uid: meta.uid(),
                gid: meta.gid(),
            });
        }
        Ok(Self::Open(crate::db::connect(&database).await?))
    }

    pub async fn owner(&self, username: &str) -> Result<(Id, SystemUserState)> {
        match self.ask(Request::Owner { username: username.to_owned() }).await? {
            Answer::Owner { id, state } => Ok((id, state)),
            other => unexpected(other),
        }
    }

    pub async fn free_port(&self, wanted: Option<u16>) -> Result<u16> {
        let Answer::Ports { taken, from, to } = self.ask(Request::Ports).await? else {
            bail!("the database step answered something other than the ports");
        };
        let free = |port: u16| !taken.contains(&port);
        if let Some(port) = wanted.filter(|port| *port >= 1024 && free(*port)) {
            return Ok(port);
        }
        (from..=to)
            .find(|port| free(*port))
            .context("the port pool has no free port left; widen it with `craftpanel admin ports`")
    }

    pub async fn record(&self, entry: Entry) -> Result<()> {
        match self.ask(Request::Record(entry)).await? {
            Answer::Done => Ok(()),
            other => unexpected(other),
        }
    }

    pub async fn forget(&self, server: Id) -> Result<()> {
        match self.ask(Request::Forget { server }).await? {
            Answer::Done => Ok(()),
            other => unexpected(other),
        }
    }

    async fn ask(&self, request: Request) -> Result<Answer> {
        let answer = match self {
            Self::Open(pool) => answer(pool, request).await,
            Self::Through { program, config, uid, gid } => {
                let mut child = tokio::process::Command::new(program)
                    .args(["import", "ledger"])
                    .env_clear()
                    .env("CRAFTPANEL_CONFIG", config)
                    .env("CRAFTPANEL_LOG", "off")
                    .uid(*uid)
                    .gid(*gid)
                    .current_dir("/")
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::inherit())
                    .kill_on_drop(true)
                    .spawn()
                    .context("starting the database step under the panel's account")?;
                let mut stdin = child.stdin.take().context("the database step has no input")?;
                stdin.write_all(&serde_json::to_vec(&request)?).await?;
                drop(stdin);
                let output = child.wait_with_output().await?;
                if !output.status.success() {
                    bail!("the database step under the panel's account failed ({})", output.status);
                }
                let text = String::from_utf8_lossy(&output.stdout);
                let line = text.lines().rev().find(|line| !line.trim().is_empty()).unwrap_or_default();
                serde_json::from_str(line).context("reading the answer of the database step")?
            }
        };
        match answer {
            Answer::Failed(why) => bail!("{why}"),
            answer => Ok(answer),
        }
    }
}

fn unexpected<T>(answer: Answer) -> Result<T> {
    bail!("the database step gave an unexpected answer: {answer:?}")
}

pub async fn serve() -> Result<()> {
    let mut input = String::new();
    std::io::Read::read_to_string(&mut std::io::stdin(), &mut input)?;
    let request: Request = serde_json::from_str(&input).context("reading the request")?;
    let config = Config::load(&crate::auth::cli::config_path())?;
    let pool = crate::db::connect(&config.database_path()).await?;
    let answer = answer(&pool, request).await;
    pool.close().await;
    println!("{}", serde_json::to_string(&answer)?);
    Ok(())
}

pub async fn answer(pool: &SqlitePool, request: Request) -> Answer {
    let done = match request {
        Request::Owner { username } => owner(pool, &username).await,
        Request::Ports => ports(pool).await,
        Request::Record(entry) => record(pool, &entry).await.map(|()| Answer::Done),
        Request::Forget { server } => forget(pool, server).await.map(|()| Answer::Done),
    };
    done.unwrap_or_else(|err| Answer::Failed(format!("{err:#}")))
}

async fn owner(pool: &SqlitePool, username: &str) -> Result<Answer> {
    let row: Option<(Id, SystemUserState)> =
        sqlx::query_as("SELECT id, system_state FROM users WHERE username = ? COLLATE NOCASE")
            .bind(username)
            .fetch_optional(pool)
            .await?;
    let (id, state) = row.with_context(|| format!("there is no panel user called {username}"))?;
    Ok(Answer::Owner { id, state })
}

async fn ports(pool: &SqlitePool) -> Result<Answer> {
    let taken: Vec<i64> = sqlx::query_scalar("SELECT port FROM ports_in_use").fetch_all(pool).await?;
    let (from, to): (i64, i64) =
        sqlx::query_as("SELECT port_pool_from, port_pool_to FROM panel_settings WHERE id = 1")
            .fetch_one(pool)
            .await?;
    Ok(Answer::Ports {
        taken: taken.into_iter().filter_map(|port| u16::try_from(port).ok()).collect(),
        from: u16::try_from(from).unwrap_or(u16::MAX),
        to: u16::try_from(to).unwrap_or(u16::MAX),
    })
}

async fn record(pool: &SqlitePool, entry: &Entry) -> Result<()> {
    let mut tx = pool.begin().await?;
    let taken: Option<i64> = sqlx::query_scalar("SELECT port FROM allocations WHERE port = ?")
        .bind(i64::from(entry.port))
        .fetch_optional(&mut *tx)
        .await?;
    if taken.is_some() {
        bail!("port {} was taken by another server while the import ran", entry.port);
    }
    let now = Timestamp::now();
    sqlx::query(
        "INSERT INTO servers (id, name, owner_id, status, loader, loader_version, game_version, \
         memory_mib, flows_intro, extra_flags, created_at, updated_at) \
         VALUES (?, ?, ?, 'available', ?, ?, ?, ?, 0, ?, ?, ?)",
    )
    .bind(entry.server)
    .bind(&entry.name)
    .bind(entry.owner)
    .bind(entry.loader)
    .bind(&entry.loader_version)
    .bind(&entry.game_version)
    .bind(i64::from(entry.memory_mib))
    .bind(serde_json::to_string(&entry.flags)?)
    .bind(now)
    .bind(now)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO allocations (port, server_id, name, is_primary, created_at) \
         VALUES (?, ?, 'primary', 1, ?)",
    )
    .bind(i64::from(entry.port))
    .bind(entry.server)
    .bind(now)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO audit_log (id, server_id, actor_user_id, action, metadata, created_at) \
         VALUES (?, ?, ?, 'server_created', ?, ?)",
    )
    .bind(Id::new())
    .bind(entry.server)
    .bind(entry.owner)
    .bind(serde_json::json!({ "imported_from": "mcsmanager", "instance": entry.instance }).to_string())
    .bind(now)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

async fn forget(pool: &SqlitePool, server: Id) -> Result<()> {
    sqlx::query("DELETE FROM servers WHERE id = ?").bind(server).execute(pool).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::harness::{an_admin, test_pool};

    fn entry(owner: Id, port: u16) -> Entry {
        Entry {
            server: Id::new(),
            owner,
            name: "Survival".to_owned(),
            loader: LoaderId::Paper,
            loader_version: Some("130".to_owned()),
            game_version: "1.21.4".to_owned(),
            memory_mib: 4096,
            flags: Vec::new(),
            port,
            instance: "f00".to_owned(),
        }
    }

    #[tokio::test]
    async fn a_port_taken_in_the_meantime_is_refused_and_nothing_is_written() {
        let pool = test_pool().await;
        let owner = an_admin(&pool, "admin").await;
        let ledger = Ledger::Open(pool.clone());

        let first = entry(owner, 25590);
        ledger.record(first.clone()).await.unwrap();
        let second = entry(owner, 25590);
        let refused = ledger.record(second.clone()).await.unwrap_err();
        assert!(format!("{refused:#}").contains("25590"), "{refused:#}");
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM servers WHERE id = ?")
            .bind(second.server)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0);

        assert_ne!(ledger.free_port(Some(25590)).await.unwrap(), 25590);
    }

    #[tokio::test]
    async fn a_forgotten_server_leaves_no_rows_behind() {
        let pool = test_pool().await;
        let owner = an_admin(&pool, "admin").await;
        let ledger = Ledger::Open(pool.clone());

        let recorded = entry(owner, 25591);
        ledger.record(recorded.clone()).await.unwrap();
        ledger.forget(recorded.server).await.unwrap();
        for table in ["servers", "allocations", "audit_log"] {
            let column = if table == "servers" { "id" } else { "server_id" };
            let count: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM {table} WHERE {column} = ?"))
                .bind(recorded.server)
                .fetch_one(&pool)
                .await
                .unwrap();
            assert_eq!(count, 0, "{table}");
        }
        assert_eq!(ledger.free_port(Some(25591)).await.unwrap(), 25591);
    }

    #[test]
    fn requests_and_answers_survive_the_trip_between_processes() {
        let request = Request::Record(entry(Id::new(), 25565));
        let text = serde_json::to_string(&request).unwrap();
        assert!(matches!(serde_json::from_str(&text).unwrap(), Request::Record(found) if found.port == 25565));

        let answer = Answer::Owner { id: Id::new(), state: SystemUserState::Ready };
        let text = serde_json::to_string(&answer).unwrap();
        assert!(matches!(serde_json::from_str(&text).unwrap(), Answer::Owner { state: SystemUserState::Ready, .. }));
    }
}
