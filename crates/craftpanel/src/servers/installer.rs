use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::Duration;

use craftpanel_proto::SpawnRequest;

use super::hub::Hub;
use super::state::RunState;
use crate::helper::Helper;
use crate::model::{Id, LoaderId};

pub const INSTALLER: &str = "craftpanel-installer.jar";
const PATIENCE: Duration = Duration::from_secs(20 * 60);
const ARRIVAL: Duration = Duration::from_secs(60);
const KEPT_LINES: usize = 20;

pub fn needed(loader: LoaderId) -> bool {
    matches!(loader, LoaderId::Forge | LoaderId::NeoForge)
}

pub async fn run(hub: &Hub, helper: &Helper, owner: Id, server: Id, java: PathBuf) -> Result<(), String> {
    let key = server.to_string();
    let before = hub.link(&key).await.map(|link| link.pid);
    let token = hex::encode(rand::random::<[u8; 32]>());
    hub.set_token(key.clone(), token.clone()).await;

    let request = SpawnRequest {
        user_id: owner.to_string(),
        server_id: key.clone(),
        working_dir: crate::helper::in_servers(server),
        program: java,
        args: vec!["-jar".to_owned(), INSTALLER.to_owned(), "--installServer".to_owned()],
        env: Vec::new(),
        supervisor_socket: hub.socket().to_path_buf(),
        token,
    };
    let outcome = attend(hub, helper, &key, before, request).await;
    hub.forget_token(&key).await;
    outcome
}

async fn attend(
    hub: &Hub,
    helper: &Helper,
    key: &str,
    before: Option<u32>,
    request: SpawnRequest,
) -> Result<(), String> {
    helper.spawn(request).await.map_err(|err| format!("the installer could not be started: {err:#}"))?;

    let arrived = tokio::time::Instant::now() + ARRIVAL;
    let link = loop {
        if let Some(link) = hub.link(key).await.filter(|link| Some(link.pid) != before) {
            break link;
        }
        if tokio::time::Instant::now() > arrived {
            return Err("the installer never reported back".to_owned());
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    };

    let mut console = link.subscribe();
    let mut last: VecDeque<String> = VecDeque::with_capacity(KEPT_LINES);
    let deadline = tokio::time::Instant::now() + PATIENCE;
    loop {
        while let Ok(line) = console.try_recv() {
            if last.len() == KEPT_LINES {
                last.pop_front();
            }
            last.push_back(line.line);
        }
        let state = link.state().await;
        if !state.is_live() {
            return match state {
                RunState::Stopped => Ok(()),
                _ => Err(format!(
                    "the installer ended without success. Its last words:\n{}",
                    Vec::from(last).join("\n")
                )),
            };
        }
        if tokio::time::Instant::now() > deadline {
            let _ = link.kill().await;
            return Err("the installer took longer than twenty minutes and was stopped".to_owned());
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

pub fn tidy_up(dir: &Path, loader: LoaderId) -> std::io::Result<()> {
    for leftover in [INSTALLER.to_owned(), format!("{INSTALLER}.log"), "installer.log".to_owned()] {
        match std::fs::remove_file(dir.join(leftover)) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(err),
        }
    }
    if crate::settings::startup::args_file(dir, Some(loader), None).is_some() {
        return Ok(());
    }
    if let Some(launcher) = legacy_launcher(dir)? {
        std::fs::rename(dir.join(launcher), dir.join(super::manager::JAR))?;
    }
    Ok(())
}

fn legacy_launcher(dir: &Path) -> std::io::Result<Option<String>> {
    let mut found: Vec<String> = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let Ok(name) = entry.file_name().into_string() else { continue };
        let lower = name.to_ascii_lowercase();
        if lower.starts_with("forge-") && lower.ends_with(".jar") && !lower.contains("installer") {
            found.push(name);
        }
    }
    found.sort();
    Ok(found.pop())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("craftpanel-installer-{}", Id::new()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_modern_install_keeps_its_args_and_loses_the_installer() {
        let dir = folder();
        std::fs::write(dir.join(INSTALLER), b"installer").unwrap();
        std::fs::write(dir.join(format!("{INSTALLER}.log")), b"log").unwrap();
        let home = dir.join("libraries/net/minecraftforge/forge/1.20.1-47.4.28");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(home.join("unix_args.txt"), b"-p x").unwrap();

        tidy_up(&dir, LoaderId::Forge).unwrap();
        assert!(!dir.join(INSTALLER).exists());
        assert!(!dir.join(format!("{INSTALLER}.log")).exists());
        assert!(!dir.join("server.jar").exists(), "a modern Forge starts from its args");
    }

    #[test]
    fn an_old_forge_becomes_server_jar() {
        let dir = folder();
        std::fs::write(dir.join(INSTALLER), b"installer").unwrap();
        std::fs::write(dir.join("forge-1.12.2-14.23.5.2859.jar"), b"forge").unwrap();
        std::fs::write(dir.join("minecraft_server.1.12.2.jar"), b"vanilla").unwrap();

        tidy_up(&dir, LoaderId::Forge).unwrap();
        assert_eq!(std::fs::read(dir.join("server.jar")).unwrap(), b"forge");
        assert!(dir.join("minecraft_server.1.12.2.jar").exists());
    }

    async fn a_supervisor_that_says(hub: &std::sync::Arc<Hub>, server: &str, lines: &[&str], code: i32) {
        use craftpanel_proto::{OutputStream, SupervisorMessage, HELPER_PROTOCOL_VERSION};
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

        let mut stream = None;
        for _ in 0..100 {
            if let Ok(open) = tokio::net::UnixStream::connect(hub.socket()).await {
                stream = Some(open);
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let (reader, mut writer) = stream.expect("the hub listens").into_split();
        let send = |message: SupervisorMessage| {
            let mut line = serde_json::to_vec(&message).unwrap();
            line.push(b'\n');
            line
        };
        let hello = send(SupervisorMessage::Hello {
            server_id: server.to_owned(),
            token: "a-token".to_owned(),
            pid: 4242,
            protocol: HELPER_PROTOCOL_VERSION,
        });
        writer.write_all(&hello).await.unwrap();
        let mut answer = String::new();
        BufReader::new(reader).read_line(&mut answer).await.unwrap();
        tokio::time::sleep(Duration::from_millis(300)).await;
        for (seq, line) in lines.iter().enumerate() {
            let output = send(SupervisorMessage::Output {
                seq: seq as u64,
                line: (*line).to_owned(),
                stream: OutputStream::Stdout,
            });
            writer.write_all(&output).await.unwrap();
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
        let exited = send(SupervisorMessage::Exited { code: Some(code), signal: None, oom_killed: false });
        writer.write_all(&exited).await.unwrap();
        writer.flush().await.unwrap();
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    async fn a_hub() -> std::sync::Arc<Hub> {
        let socket = std::env::temp_dir().join(format!("craftpanel-installer-hub-{}.sock", Id::new()));
        let hub = std::sync::Arc::new(Hub::new(socket));
        tokio::spawn({
            let hub = std::sync::Arc::clone(&hub);
            async move {
                let _ = hub.listen().await;
            }
        });
        hub
    }

    fn a_request(server: &str) -> SpawnRequest {
        SpawnRequest {
            user_id: Id::new().to_string(),
            server_id: server.to_owned(),
            working_dir: vec!["servers".to_owned(), server.to_owned()],
            program: PathBuf::from("java"),
            args: vec!["-jar".to_owned(), INSTALLER.to_owned(), "--installServer".to_owned()],
            env: Vec::new(),
            supervisor_socket: PathBuf::from("/nowhere"),
            token: "a-token".to_owned(),
        }
    }

    #[tokio::test]
    async fn an_installer_that_ends_well_is_a_success() {
        let hub = a_hub().await;
        let fake = crate::auth::harness::FakeHelper::obliging().await;
        let helper = Helper::new(fake.socket());
        let server = Id::new().to_string();
        hub.set_token(server.clone(), "a-token").await;

        let (outcome, ()) = tokio::join!(
            attend(&hub, &helper, &server, None, a_request(&server)),
            a_supervisor_that_says(&hub, &server, &["Downloading libraries", "The server installed successfully"], 0),
        );
        assert_eq!(outcome, Ok(()));
    }

    #[tokio::test]
    async fn an_installer_that_fails_brings_its_last_words_along() {
        let hub = a_hub().await;
        let fake = crate::auth::harness::FakeHelper::obliging().await;
        let helper = Helper::new(fake.socket());
        let server = Id::new().to_string();
        hub.set_token(server.clone(), "a-token").await;

        let (outcome, ()) = tokio::join!(
            attend(&hub, &helper, &server, None, a_request(&server)),
            a_supervisor_that_says(&hub, &server, &["Downloading libraries", "Failed to download vanilla jar"], 1),
        );
        let message = outcome.unwrap_err();
        assert!(message.contains("Failed to download vanilla jar"), "{message}");
    }

    #[test]
    fn only_the_installers_need_one() {
        assert!(needed(LoaderId::Forge) && needed(LoaderId::NeoForge));
        assert!(!needed(LoaderId::Fabric) && !needed(LoaderId::Paper));
    }
}
