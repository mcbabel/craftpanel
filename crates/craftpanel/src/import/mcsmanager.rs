use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use super::detect::{self, Detected};
use super::ledger::{Entry, Ledger};
use super::McsmanagerArgs;
use crate::config::Config;
use crate::helper::Helper;
use crate::model::{Id, LoaderId, SystemUserState};

const GLOBAL_INSTANCE: &str = "global0001";
const JAR: &str = "server.jar";
const MINECRAFT_JAR: &str = "minecraft-server.jar";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instance {
    pub uuid: String,
    pub name: String,
    pub dir: PathBuf,
    pub start_command: String,
    pub kind: String,
}

#[derive(Debug, Clone)]
pub struct Planned {
    pub instance: Instance,
    pub detected: Detected,
    pub memory_mib: u32,
    pub flags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Imported {
    pub server: Id,
    pub port: u16,
    pub port_changed: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Stored {
    #[serde(default)]
    nickname: String,
    #[serde(default)]
    start_command: String,
    #[serde(default)]
    cwd: String,
    #[serde(default, rename = "type")]
    kind: String,
}

pub async fn run(args: McsmanagerArgs) -> Result<()> {
    let config_path = crate::auth::cli::config_path();
    let config = Config::load(&config_path)?;
    let ledger = Ledger::open(&config_path, &config).await?;
    let helper = Helper::new(&config.helper_socket);

    let chosen = select(instances(&args.from)?, &args.only)?;
    let (owner, state) = ledger.owner(&args.owner).await?;
    if !args.dry_run {
        helper.ping().await.context("the privileged helper has to run to hand the files over")?;
        if state != SystemUserState::Ready {
            bail!("the system account of {} is not ready yet; fix it in the panel first", args.owner);
        }
    }

    let guard = Guard { daemon: &args.from, data_dir: &config.data_dir, allow_outside: args.allow_outside };
    let (mut imported, mut skipped) = (0, 0);
    for instance in &chosen {
        let checked = guard.check(instance);
        let planned = match checked.and_then(|()| plan(instance)) {
            Ok(planned) => planned,
            Err(why) => {
                skipped += 1;
                println!("skip    {} ({}): {why}", instance.name, instance.uuid);
                continue;
            }
        };
        if args.dry_run {
            println!("import  {}", describe(&planned));
            continue;
        }
        match import(&ledger, &config, &helper, &guard, owner, &planned, args.copy).await {
            Ok(done) => {
                imported += 1;
                println!("done    {} as {} on port {}", describe(&planned), done.server, done.port);
                if done.port_changed {
                    println!(
                        "        its port {} is also another server's; its files keep {} and the panel notes {} until the other one lets go",
                        planned.detected.port.map_or("?".to_owned(), |port| port.to_string()),
                        planned.detected.port.map_or("?".to_owned(), |port| port.to_string()),
                        done.port
                    );
                }
            }
            Err(err) => {
                skipped += 1;
                println!("failed  {} ({}): {err:#}", instance.name, instance.uuid);
            }
        }
    }

    if args.dry_run {
        println!("{} of {} instances can be imported; nothing was changed", chosen.len() - skipped, chosen.len());
    } else {
        println!("{imported} imported, {skipped} skipped");
    }
    Ok(())
}

pub fn instances(daemon: &Path) -> Result<Vec<Instance>> {
    let folder = daemon.join("data").join("InstanceConfig");
    let entries = std::fs::read_dir(&folder).with_context(|| format!("reading {}", folder.display()))?;

    let mut found = Vec::new();
    for entry in entries {
        let path = entry?.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let Some(uuid) = path.file_stem().and_then(|stem| stem.to_str()).map(str::to_owned) else {
            continue;
        };
        if uuid == GLOBAL_INSTANCE {
            continue;
        }
        let text = std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let stored: Stored = match serde_json::from_str(&text) {
            Ok(stored) => stored,
            Err(err) => {
                eprintln!("skip    {}: {err}", path.display());
                continue;
            }
        };
        if stored.cwd.trim().is_empty() || stored.cwd == "." {
            continue;
        }
        let cwd = PathBuf::from(stored.cwd.trim());
        let dir = if cwd.is_absolute() { cwd } else { daemon.join(cwd) };
        found.push(Instance {
            name: name_of(&stored.nickname, &uuid),
            uuid,
            dir,
            start_command: stored.start_command,
            kind: stored.kind,
        });
    }
    found.sort_by_key(|instance| instance.name.to_lowercase());
    Ok(found)
}

fn name_of(nickname: &str, uuid: &str) -> String {
    let name: String = nickname.trim().chars().take(64).collect();
    if name.is_empty() { uuid.chars().take(8).collect() } else { name }
}

fn select(found: Vec<Instance>, only: &[String]) -> Result<Vec<Instance>> {
    if only.is_empty() {
        return Ok(found);
    }
    let mut chosen = Vec::new();
    for wanted in only {
        let hit = found
            .iter()
            .find(|instance| instance.uuid == *wanted || instance.name.eq_ignore_ascii_case(wanted))
            .with_context(|| format!("there is no instance called {wanted}"))?;
        chosen.push(hit.clone());
    }
    Ok(chosen)
}

pub struct Guard<'a> {
    pub daemon: &'a Path,
    pub data_dir: &'a Path,
    pub allow_outside: bool,
}

impl Guard<'_> {
    pub fn check(&self, instance: &Instance) -> Result<(), String> {
        let meta = std::fs::symlink_metadata(&instance.dir)
            .map_err(|_| format!("its folder {} does not exist", instance.dir.display()))?;
        if !meta.is_dir() {
            return Err(format!("{} is a link or not a folder", instance.dir.display()));
        }
        let folder = instance.dir.canonicalize().map_err(|err| err.to_string())?;
        self.check_resolved(&folder)
    }

    pub fn check_resolved(&self, folder: &Path) -> Result<(), String> {
        if folder.components().count() < 3 {
            return Err(format!("{} is too close to the root of the disk", folder.display()));
        }
        if let Ok(data) = self.data_dir.canonicalize() {
            if folder.starts_with(&data) || data.starts_with(folder) {
                return Err(format!("{} overlaps the panel's own data", folder.display()));
            }
        }
        let instances = self.daemon.join("data").join("InstanceData").canonicalize().ok();
        if !self.allow_outside && !instances.is_some_and(|inside| folder.starts_with(inside)) {
            return Err(format!(
                "{} lies outside MCSManager's InstanceData; pass --allow-outside if it should move",
                folder.display()
            ));
        }
        guarded_ancestors(folder, &self.trusted())
    }

    fn trusted(&self) -> Vec<u32> {
        use std::os::unix::fs::MetadataExt;
        let mut trusted = vec![0, effective_uid()];
        if let Ok(meta) = self.daemon.canonicalize().and_then(std::fs::metadata) {
            trusted.push(meta.uid());
        }
        trusted
    }
}

fn guarded_ancestors(folder: &Path, trusted: &[u32]) -> Result<(), String> {
    use std::os::unix::fs::MetadataExt;
    for ancestor in folder.ancestors().skip(1) {
        let meta = std::fs::symlink_metadata(ancestor).map_err(|err| err.to_string())?;
        let mode = meta.mode();
        if mode & 0o022 != 0 && mode & 0o1000 == 0 {
            return Err(format!(
                "{} can be written by other users, so a folder below it could be swapped while it moves",
                ancestor.display()
            ));
        }
        if !trusted.contains(&meta.uid()) {
            return Err(format!(
                "{} belongs to {}, who is neither root nor the owner of the MCSManager folder",
                ancestor.display(),
                user_name(meta.uid())
            ));
        }
    }
    Ok(())
}

fn user_name(uid: u32) -> String {
    std::fs::read_to_string("/etc/passwd")
        .ok()
        .and_then(|passwd| {
            passwd.lines().find_map(|line| {
                let mut fields = line.split(':');
                let name = fields.next()?;
                let id: u32 = fields.nth(1)?.parse().ok()?;
                (id == uid).then(|| name.to_owned())
            })
        })
        .unwrap_or_else(|| format!("user {uid}"))
}

fn effective_uid() -> u32 {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata("/proc/self").map_or(0, |meta| meta.uid())
}

pub fn plan(instance: &Instance) -> Result<Planned, String> {
    if instance.kind == "minecraft/bedrock" {
        return Err("Bedrock servers are not supported".to_owned());
    }
    if !instance.dir.is_dir() {
        return Err(format!("its folder {} does not exist", instance.dir.display()));
    }
    let detected = detect::detect(&instance.dir, &instance.start_command)?;
    let launch = crate::settings::startup::launch_of(Some(detected.loader));
    let flags = crate::settings::startup::read_command(Some(&instance.start_command), launch)
        .map(|(kept, _)| kept)
        .unwrap_or_default();
    Ok(Planned {
        instance: instance.clone(),
        memory_mib: detect::memory_of(&format!(
            "{} {}",
            instance.start_command,
            detect::read_plain(&instance.dir.join("user_jvm_args.txt"))
                .ok()
                .flatten()
                .unwrap_or_default()
                .lines()
                .filter(|line| !line.trim_start().starts_with('#'))
                .collect::<Vec<_>>()
                .join(" ")
        )),
        flags,
        detected,
    })
}

fn describe(planned: &Planned) -> String {
    let found = &planned.detected;
    let word = if found.loader == LoaderId::Fabric { "loader" } else { "build" };
    let build = found.loader_version.as_deref().map(|build| format!(" {word} {build}")).unwrap_or_default();
    format!(
        "{} ({}): {} {}{build}, {} MiB",
        planned.instance.name,
        planned.instance.uuid,
        found.loader.as_str(),
        found.game_version,
        planned.memory_mib
    )
}

pub async fn import(
    ledger: &Ledger,
    config: &Config,
    helper: &Helper,
    guard: &Guard<'_>,
    owner: Id,
    planned: &Planned,
    copy: bool,
) -> Result<Imported> {
    let opened = open_dir(&planned.instance.dir)
        .with_context(|| format!("opening {}", planned.instance.dir.display()))?;
    let source = std::fs::read_link(fd_path(&opened))?;
    guard.check_resolved(&source).map_err(anyhow::Error::msg)?;
    let identity = identity_of(&opened.metadata()?);
    if let Some(pid) = running_in(&source) {
        bail!("process {pid} still runs in {}; stop the instance in MCSManager first", source.display());
    }
    let source = source.as_path();

    let server = Id::new();
    let target = crate::files::server_dir(config, owner, server);
    let parent = target.parent().context("a server folder has a parent")?;
    if !parent.is_dir() {
        bail!("{} does not exist; the owner's system account looks incomplete", parent.display());
    }

    if copy {
        copy_dir(&opened, &target, 0)?;
    } else {
        move_dir(source, &target, identity)?;
    }
    let settled = settle(ledger, &target, owner, server, planned).await;
    let imported = match settled {
        Ok(imported) => imported,
        Err(err) => {
            if copy {
                let _ = std::fs::remove_dir_all(&target);
            } else {
                put_back(&target, source);
            }
            return Err(err);
        }
    };

    helper
        .chown_tree(&owner.to_string(), crate::helper::in_servers(server))
        .await
        .context("the server was imported, but its files could not be handed to the owner's account")?;
    Ok(imported)
}

async fn settle(
    ledger: &Ledger,
    dir: &Path,
    owner: Id,
    server: Id,
    planned: &Planned,
) -> Result<Imported> {
    let found = &planned.detected;
    let port = ledger.free_port(found.port).await?;
    ledger
        .record(Entry {
            server,
            owner,
            name: planned.instance.name.clone(),
            loader: found.loader,
            loader_version: found.loader_version.clone(),
            game_version: found.game_version.clone(),
            memory_mib: planned.memory_mib,
            flags: planned.flags.clone(),
            port,
            instance: planned.instance.uuid.clone(),
        })
        .await?;
    if let Err(err) = arrange(dir, found) {
        if let Err(undo) = ledger.forget(server).await {
            return Err(err.context(format!("and the server's rows could not be taken back out: {undo:#}")));
        }
        return Err(err);
    }
    Ok(Imported { server, port, port_changed: found.port.is_some_and(|wanted| wanted != port) })
}

fn arrange(dir: &Path, found: &Detected) -> Result<()> {
    if let Some(vanilla) = &found.vanilla_jar {
        if vanilla != MINECRAFT_JAR {
            std::fs::rename(dir.join(vanilla), dir.join(MINECRAFT_JAR))?;
        }
        let properties = dir.join(detect::LAUNCHER_PROPERTIES);
        let kept: Vec<String> = detect::read_plain(&properties)?
            .unwrap_or_default()
            .lines()
            .filter(|line| !line.starts_with("serverJar="))
            .map(str::to_owned)
            .collect();
        let mut text = kept.join("\n");
        text.push_str(&format!("\nserverJar={MINECRAFT_JAR}\n"));
        detect::write_plain(&properties, text.trim_start())?;
    }
    if found.jar != JAR {
        let existing = dir.join(JAR);
        if std::fs::symlink_metadata(&existing).is_ok() {
            std::fs::rename(&existing, dir.join("server.jar.before-import"))?;
        }
        std::fs::rename(dir.join(&found.jar), &existing)?;
    }
    if found.loader != LoaderId::Velocity {
        let created = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o660)
            .custom_flags(libc::O_NOFOLLOW)
            .open(dir.join("eula.txt"));
        if let Ok(mut file) = created {
            std::io::Write::write_all(&mut file, b"eula=true\n")?;
        }
    }

    Ok(())
}

const MAX_DEPTH: usize = 256;

fn open_dir(path: &Path) -> std::io::Result<std::fs::File> {
    std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
}

fn fd_path(file: &std::fs::File) -> PathBuf {
    PathBuf::from(format!("/proc/self/fd/{}", std::os::fd::AsRawFd::as_raw_fd(file)))
}

fn identity_of(meta: &std::fs::Metadata) -> (u64, u64) {
    use std::os::unix::fs::MetadataExt;
    (meta.dev(), meta.ino())
}

fn move_dir(source: &Path, target: &Path, identity: (u64, u64)) -> Result<()> {
    let present = std::fs::symlink_metadata(source)?;
    if !present.is_dir() || identity_of(&present) != identity {
        bail!("{} changed while it was being checked", source.display());
    }
    match std::fs::rename(source, target) {
        Ok(()) => {}
        Err(err) if err.raw_os_error() == Some(libc::EXDEV) => bail!(
            "{} lies on another disk than the panel's data; run again with --copy",
            source.display()
        ),
        Err(err) => return Err(err).with_context(|| format!("moving {}", source.display())),
    }
    let moved = std::fs::symlink_metadata(target)?;
    if !moved.is_dir() || identity_of(&moved) != identity {
        put_back(target, source);
        bail!("{} was swapped for something else while it was being moved", source.display());
    }
    Ok(())
}

fn put_back(target: &Path, source: &Path) {
    let free = matches!(std::fs::symlink_metadata(source), Err(err) if err.kind() == std::io::ErrorKind::NotFound);
    let unchanged = source
        .parent()
        .is_some_and(|parent| parent.canonicalize().is_ok_and(|resolved| resolved == parent));
    if !free || !unchanged || std::fs::rename(target, source).is_err() {
        eprintln!("        the files stay in {}; move them back by hand", target.display());
    }
}

fn copy_dir(dir: &std::fs::File, target: &Path, depth: usize) -> Result<()> {
    if depth > MAX_DEPTH {
        bail!("{} nests too deep", target.display());
    }
    std::fs::create_dir(target).with_context(|| format!("creating {}", target.display()))?;
    let base = fd_path(dir);
    for entry in std::fs::read_dir(&base)? {
        let name = entry?.file_name();
        let inside = base.join(&name);
        let destination = target.join(&name);
        let opened = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC)
            .open(&inside);
        let file = match opened {
            Ok(file) => file,
            Err(err) if err.raw_os_error() == Some(libc::ELOOP) => {
                std::os::unix::fs::symlink(std::fs::read_link(&inside)?, &destination)?;
                continue;
            }
            Err(err) => return Err(err).with_context(|| format!("opening {}", destination.display())),
        };
        let meta = file.metadata()?;
        if meta.is_dir() {
            copy_dir(&file, &destination, depth + 1)?;
        } else if meta.is_file() && std::os::unix::fs::MetadataExt::nlink(&meta) == 1 {
            copy_file(file, &destination)?;
        } else {
            bail!("{} is not a plain file, folder or link", destination.display());
        }
    }
    Ok(())
}

fn copy_file(mut from: std::fs::File, destination: &Path) -> Result<()> {
    let mut to = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o660)
        .custom_flags(libc::O_NOFOLLOW)
        .open(destination)
        .with_context(|| format!("creating {}", destination.display()))?;
    std::io::copy(&mut from, &mut to)?;
    Ok(())
}

fn running_in(dir: &Path) -> Option<u32> {
    let dir = dir.canonicalize().ok()?;
    std::fs::read_dir("/proc").ok()?.filter_map(|entry| entry.ok()).find_map(|entry| {
        let pid: u32 = entry.file_name().to_str()?.parse().ok()?;
        let cwd = std::fs::read_link(entry.path().join("cwd")).ok()?;
        cwd.starts_with(&dir).then_some(pid)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::harness::{an_admin, test_pool, FakeHelper};
    use crate::model::Timestamp;

    struct Setup {
        daemon: PathBuf,
        data: PathBuf,
    }

    fn setup() -> Setup {
        let root = std::env::temp_dir().join(format!("craftpanel-import-{}", Id::new()));
        let daemon = root.join("daemon");
        std::fs::create_dir_all(daemon.join("data/InstanceConfig")).unwrap();
        std::fs::create_dir_all(daemon.join("data/InstanceData")).unwrap();
        Setup { daemon, data: root.join("panel") }
    }

    impl Setup {
        fn instance(&self, uuid: &str, config: serde_json::Value) {
            let path = self.daemon.join("data/InstanceConfig").join(format!("{uuid}.json"));
            std::fs::write(path, config.to_string()).unwrap();
        }

        fn folder(&self, name: &str) -> PathBuf {
            let dir = self.daemon.join("data/InstanceData").join(name);
            std::fs::create_dir_all(&dir).unwrap();
            dir
        }

        fn config(&self) -> Config {
            Config { data_dir: self.data.clone(), ..Config::default() }
        }
    }

    fn paper(dir: &Path, port: u16) {
        std::fs::write(dir.join("paper-1.21.4-130.jar"), b"jar").unwrap();
        std::fs::write(dir.join("server.properties"), format!("motd=hi\nserver-port={port}\n")).unwrap();
        std::fs::create_dir_all(dir.join("world")).unwrap();
        std::fs::write(dir.join("world/level.dat"), b"world").unwrap();
    }

    #[test]
    fn instances_are_read_with_their_names_and_folders() {
        let setup = setup();
        setup.instance("aaa", serde_json::json!({
            "nickname": "  Lobby  ", "cwd": "data/InstanceData/lobby", "type": "minecraft/java",
            "startCommand": "java -Xmx4G -jar paper.jar nogui"
        }));
        setup.instance("bbb", serde_json::json!({ "nickname": "", "cwd": "/srv/elsewhere" }));
        setup.instance("global0001", serde_json::json!({ "nickname": "global", "cwd": "/" }));

        let found = instances(&setup.daemon).unwrap();
        assert_eq!(found.len(), 2);
        let lobby = found.iter().find(|one| one.uuid == "aaa").unwrap();
        assert_eq!(lobby.name, "Lobby");
        assert_eq!(lobby.dir, setup.daemon.join("data/InstanceData/lobby"));
        let unnamed = found.iter().find(|one| one.uuid == "bbb").unwrap();
        assert_eq!((unnamed.name.as_str(), unnamed.dir.as_path()), ("bbb", Path::new("/srv/elsewhere")));
    }

    #[test]
    fn folders_outside_instance_data_need_permission_and_system_paths_never_pass() {
        let setup = setup();
        let data = setup.config().data_dir;
        std::fs::create_dir_all(&data).unwrap();
        let at = |dir: PathBuf| Instance {
            uuid: "x".to_owned(),
            name: "x".to_owned(),
            dir,
            start_command: String::new(),
            kind: String::new(),
        };

        let guard_check = |instance: &Instance, daemon: &Path, data: &Path, allow_outside: bool| {
            Guard { daemon, data_dir: data, allow_outside }.check(instance)
        };
        let inside = setup.folder("fine");
        assert!(guard_check(&at(inside.clone()), &setup.daemon, &data, false).is_ok());

        let outside = setup.daemon.parent().unwrap().join("elsewhere");
        std::fs::create_dir_all(&outside).unwrap();
        assert!(guard_check(&at(outside.clone()), &setup.daemon, &data, false).unwrap_err().contains("--allow-outside"));
        assert!(guard_check(&at(outside), &setup.daemon, &data, true).is_ok());

        assert!(guard_check(&at(PathBuf::from("/")), &setup.daemon, &data, true).is_err());
        assert!(guard_check(&at(PathBuf::from("/etc")), &setup.daemon, &data, true).is_err());
        assert!(guard_check(&at(data.clone()), &setup.daemon, &data, true).is_err());

        let link = setup.daemon.join("data/InstanceData/link");
        std::os::unix::fs::symlink(&inside, &link).unwrap();
        assert!(guard_check(&at(link), &setup.daemon, &data, false).unwrap_err().contains("link"));
    }

    #[tokio::test]
    async fn a_server_moves_over_with_its_name_port_and_memory() {
        let setup = setup();
        let pool = test_pool().await;
        let owner = an_admin(&pool, "admin").await;
        let config = setup.config();
        std::fs::create_dir_all(config.users_dir().join(owner.to_string()).join("servers")).unwrap();
        let helper_fake = FakeHelper::obliging().await;
        let helper = Helper::new(helper_fake.socket());
        let guard = Guard { daemon: &setup.daemon, data_dir: &config.data_dir, allow_outside: false };

        let dir = setup.folder("f00");
        paper(&dir, 25590);
        setup.instance("f00", serde_json::json!({
            "nickname": "Survival", "cwd": dir.display().to_string(), "type": "minecraft/java",
            "startCommand": "java -Xmx6G -XX:+UseG1GC -jar paper-1.21.4-130.jar nogui"
        }));

        let planned = plan(&instances(&setup.daemon).unwrap()[0]).unwrap();
        let done = import(&Ledger::Open(pool.clone()), &config, &helper, &guard, owner, &planned, false).await.unwrap();

        assert_eq!(done.port, 25590);
        assert!(!done.port_changed);
        assert!(!dir.exists(), "moved, not copied");
        let target = crate::files::server_dir(&config, owner, done.server);
        assert_eq!(std::fs::read(target.join("world/level.dat")).unwrap(), b"world");
        assert!(target.join("server.jar").is_file());
        assert!(target.join("eula.txt").is_file());

        let row: (String, String, Option<String>, String, i64, i64, String, String) = sqlx::query_as(
            "SELECT name, loader, loader_version, game_version, memory_mib, flows_intro, status, extra_flags \
             FROM servers WHERE id = ?",
        )
        .bind(done.server)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.0, "Survival");
        assert_eq!((row.1.as_str(), row.2.as_deref(), row.3.as_str()), ("paper", Some("130"), "1.21.4"));
        assert_eq!((row.4, row.5, row.6.as_str()), (6144, 0, "available"));
        assert_eq!(row.7, r#"["-XX:+UseG1GC"]"#);
        assert_eq!(helper_fake.calls().len(), 1, "the files were handed to the owner");
    }

    #[tokio::test]
    async fn the_old_fabric_launcher_becomes_server_jar_beside_its_minecraft() {
        let setup = setup();
        let pool = test_pool().await;
        let owner = an_admin(&pool, "admin").await;
        let config = setup.config();
        std::fs::create_dir_all(config.users_dir().join(owner.to_string()).join("servers")).unwrap();
        let helper_fake = FakeHelper::obliging().await;
        let helper = Helper::new(helper_fake.socket());
        let guard = Guard { daemon: &setup.daemon, data_dir: &config.data_dir, allow_outside: false };

        let dir = setup.folder("oldfabric");
        std::fs::write(dir.join("fabric-server-launch.jar"), b"launcher").unwrap();
        let vanilla = std::fs::File::create(dir.join("server.jar")).unwrap();
        let mut zip = zip::ZipWriter::new(vanilla);
        zip.start_file("version.json", zip::write::SimpleFileOptions::default()).unwrap();
        std::io::Write::write_all(&mut zip, br#"{"id":"1.21.10"}"#).unwrap();
        zip.finish().unwrap();
        std::fs::write(dir.join("fabric-server-launcher.properties"), "#Fabric\nserverJar=server.jar\n").unwrap();
        setup.instance("oldfabric", serde_json::json!({
            "nickname": "VP", "cwd": dir.display().to_string(), "type": "minecraft/java",
            "startCommand": "java -Xmx4G -jar fabric-server-launch.jar nogui"
        }));

        let planned = plan(&instances(&setup.daemon).unwrap()[0]).unwrap();
        let done = import(&Ledger::Open(pool.clone()), &config, &helper, &guard, owner, &planned, false).await.unwrap();
        let target = crate::files::server_dir(&config, owner, done.server);

        assert_eq!(std::fs::read(target.join("server.jar")).unwrap(), b"launcher");
        assert!(target.join("minecraft-server.jar").is_file());
        let properties = std::fs::read_to_string(target.join("fabric-server-launcher.properties")).unwrap();
        assert!(properties.contains("#Fabric") && properties.contains("serverJar=minecraft-server.jar"));
        assert!(!properties.contains("serverJar=server.jar"));
    }

    #[tokio::test]
    async fn a_taken_port_is_noted_elsewhere_and_the_properties_stay_as_they_were() {
        let setup = setup();
        let pool = test_pool().await;
        let owner = an_admin(&pool, "admin").await;
        let config = setup.config();
        std::fs::create_dir_all(config.users_dir().join(owner.to_string()).join("servers")).unwrap();
        let helper_fake = FakeHelper::obliging().await;
        let helper = Helper::new(helper_fake.socket());
        let guard = Guard { daemon: &setup.daemon, data_dir: &config.data_dir, allow_outside: false };

        for (uuid, name) in [("one", "First"), ("two", "Second")] {
            let dir = setup.folder(uuid);
            paper(&dir, 25565);
            setup.instance(uuid, serde_json::json!({
                "nickname": name, "cwd": dir.display().to_string(), "type": "minecraft/java",
                "startCommand": "java -jar paper-1.21.4-130.jar"
            }));
        }

        let found = instances(&setup.daemon).unwrap();
        let first = import(&Ledger::Open(pool.clone()), &config, &helper, &guard, owner, &plan(&found[0]).unwrap(), true).await.unwrap();
        let second = import(&Ledger::Open(pool.clone()), &config, &helper, &guard, owner, &plan(&found[1]).unwrap(), true).await.unwrap();

        assert_eq!(first.port, 25565);
        assert_ne!(second.port, 25565);
        assert!(second.port_changed);
        let properties = std::fs::read_to_string(
            crate::files::server_dir(&config, owner, second.server).join("server.properties"),
        )
        .unwrap();
        assert!(properties.contains("server-port=25565"), "{properties}");
        assert!(setup.daemon.join("data/InstanceData/one").exists(), "copied, the original stays");
    }

    #[test]
    fn a_folder_under_a_parent_anyone_may_change_is_refused() {
        use std::os::unix::fs::PermissionsExt;
        let setup = setup();
        let open = setup.daemon.join("data/InstanceData/open");
        std::fs::create_dir_all(open.join("server")).unwrap();
        std::fs::set_permissions(&open, std::fs::Permissions::from_mode(0o777)).unwrap();
        let guard = Guard { daemon: &setup.daemon, data_dir: &setup.data, allow_outside: false };

        let refused = guard.check_resolved(&open.join("server").canonicalize().unwrap()).unwrap_err();
        assert!(refused.contains("can be written by other users"), "{refused}");

        std::fs::set_permissions(&open, std::fs::Permissions::from_mode(0o1777)).unwrap();
        assert!(guard.check_resolved(&open.join("server").canonicalize().unwrap()).is_ok(), "sticky like /tmp");
    }

    #[test]
    fn a_copy_keeps_links_as_links_and_refuses_hard_links() {
        let setup = setup();
        let secret = setup.daemon.join("secret.txt");
        std::fs::write(&secret, "outside").unwrap();

        let source = setup.folder("linked");
        std::fs::create_dir_all(source.join("plugins")).unwrap();
        std::fs::write(source.join("plugins/config.yml"), "a: 1").unwrap();
        std::os::unix::fs::symlink(&secret, source.join("plugins/peek.txt")).unwrap();
        let target = setup.data.join("copied");
        std::fs::create_dir_all(&setup.data).unwrap();

        copy_dir(&open_dir(&source).unwrap(), &target, 0).unwrap();
        assert_eq!(std::fs::read_to_string(target.join("plugins/config.yml")).unwrap(), "a: 1");
        let peek = std::fs::symlink_metadata(target.join("plugins/peek.txt")).unwrap();
        assert!(peek.file_type().is_symlink(), "the link stays a link, its target is not read");

        std::fs::hard_link(&secret, source.join("hard.txt")).unwrap();
        let again = setup.data.join("again");
        let refused = copy_dir(&open_dir(&source).unwrap(), &again, 0).unwrap_err();
        assert!(format!("{refused:#}").contains("hard.txt"), "{refused:#}");
    }

    #[tokio::test]
    async fn a_failed_import_puts_the_folder_back() {
        let setup = setup();
        let pool = test_pool().await;
        let owner = an_admin(&pool, "admin").await;
        let config = setup.config();
        std::fs::create_dir_all(config.users_dir().join(owner.to_string()).join("servers")).unwrap();
        let helper = Helper::new(setup.data.join("nobody.sock"));
        let guard = Guard { daemon: &setup.daemon, data_dir: &config.data_dir, allow_outside: false };
        sqlx::query("INSERT INTO servers (id, name, owner_id, status, memory_mib, created_at, updated_at) VALUES (?, 'x', ?, 'available', 1024, ?, ?)")
            .bind(Id::new()).bind(owner).bind(Timestamp::now()).bind(Timestamp::now())
            .execute(&pool).await.unwrap();
        let other: Id = sqlx::query_scalar("SELECT id FROM servers LIMIT 1").fetch_one(&pool).await.unwrap();
        sqlx::query("INSERT INTO allocations (port, server_id, name, is_primary, created_at) VALUES (30000, ?, 'primary', 1, ?)")
            .bind(other).bind(Timestamp::now()).execute(&pool).await.unwrap();

        let dir = setup.folder("full");
        paper(&dir, 30000);
        std::fs::write(dir.join("server.jar"), b"in the way").unwrap();
        std::fs::create_dir_all(dir.join("server.jar.before-import/occupied")).unwrap();
        setup.instance("full", serde_json::json!({
            "nickname": "Full", "cwd": dir.display().to_string(), "startCommand": "java -jar paper-1.21.4-130.jar"
        }));

        let planned = plan(&instances(&setup.daemon).unwrap()[0]).unwrap();
        let refused = import(&Ledger::Open(pool.clone()), &config, &helper, &guard, owner, &planned, false).await.unwrap_err();
        assert!(!format!("{refused:#}").is_empty());
        assert!(dir.join("world/level.dat").is_file(), "the folder is back where it was");
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM servers").fetch_one(&pool).await.unwrap();
        assert_eq!(count, 1);
    }
}
