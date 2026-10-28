use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use crate::model::LoaderId;

pub const DEFAULT_MEMORY_MIB: u32 = 2048;
const MIN_MEMORY_MIB: u32 = 512;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detected {
    pub loader: LoaderId,
    pub game_version: String,
    pub loader_version: Option<String>,
    pub jar: String,
    pub port: Option<u16>,
    pub vanilla_jar: Option<String>,
}

pub const LAUNCHER_PROPERTIES: &str = "fabric-server-launcher.properties";

pub fn detect(dir: &Path, start_command: &str) -> Result<Detected, String> {
    if let Some(found) = installed_modloader(dir)? {
        return Ok(found);
    }

    let jar = jar_of(dir, start_command)?;
    let name = jar.to_ascii_lowercase();
    if name.contains("bungee") || name.contains("waterfall") {
        return Err("BungeeCord and Waterfall are not supported; Velocity is".to_owned());
    }
    if name == "fabric-server-launch.jar" {
        return old_fabric(dir, jar);
    }
    if let Some((game_version, loader_version)) = legacy_forge(&name) {
        return Ok(Detected {
            loader: LoaderId::Forge,
            game_version,
            loader_version: Some(loader_version),
            port: port_of(dir, LoaderId::Forge),
            jar,
            vanilla_jar: None,
        });
    }

    let loader = loader_of(dir, &name);
    let (game_version, loader_version) = match loader {
        LoaderId::Velocity => velocity_version(dir, &jar)?,
        LoaderId::Fabric => fabric_version(&jar)?,
        LoaderId::Vanilla => (vanilla_version(dir, &jar)?, None),
        LoaderId::CraftBukkit => (craftbukkit_version(&jar)?, None),
        LoaderId::Limbo => limbo_version(&jar)?,
        _ => paper_version(dir, &jar, loader)?,
    };

    Ok(Detected { loader, game_version, loader_version, jar, port: port_of(dir, loader), vanilla_jar: None })
}

fn installed_modloader(dir: &Path) -> Result<Option<Detected>, String> {
    for loader in [LoaderId::NeoForge, LoaderId::Forge] {
        let Some(file) = crate::settings::startup::args_file(dir, Some(loader), None) else {
            continue;
        };
        let text = read_plain(&dir.join(&file)).ok().flatten().unwrap_or_default();
        let value = |key: &str| {
            let mut words = text.split_whitespace();
            words.find(|word| *word == key).and_then(|_| words.next()).map(str::to_owned)
        };
        let game_version = value("--fml.mcVersion");
        let loader_version = value("--fml.neoForgeVersion").or_else(|| value("--fml.forgeVersion"));
        let Some(game_version) = game_version.filter(|version| looks_like_game_version(version)) else {
            return Err(format!("the Minecraft version cannot be read from {file}"));
        };
        return Ok(Some(Detected {
            loader,
            game_version,
            loader_version,
            port: port_of(dir, loader),
            jar: "server.jar".to_owned(),
            vanilla_jar: None,
        }));
    }
    Ok(None)
}

fn legacy_forge(jar: &str) -> Option<(String, String)> {
    let stem = jar.strip_suffix(".jar")?.strip_suffix("-universal").unwrap_or(jar.strip_suffix(".jar")?);
    let rest = stem.strip_prefix("forge-")?;
    let (game, loader) = rest.split_once('-')?;
    (looks_like_game_version(game) && loader.chars().all(|c| c.is_ascii_digit() || c == '.'))
        .then(|| (game.to_owned(), loader.to_owned()))
}

fn old_fabric(dir: &Path, jar: String) -> Result<Detected, String> {
    let vanilla = read_plain(&dir.join(LAUNCHER_PROPERTIES))
        .ok()
        .flatten()
        .and_then(|text| {
            text.lines()
                .find_map(|line| line.strip_prefix("serverJar="))
                .map(|value| value.trim().to_owned())
        })
        .unwrap_or_else(|| "server.jar".to_owned());
    if vanilla.contains('/') || vanilla.starts_with('.') {
        return Err(format!("the old Fabric launcher points at {vanilla}, outside its folder"));
    }
    if !std::fs::symlink_metadata(dir.join(&vanilla)).is_ok_and(|meta| meta.is_file()) {
        return Err(format!("the Minecraft jar {vanilla} the old Fabric launcher needs is missing"));
    }
    let game_version = version_in_jar(&dir.join(&vanilla))
        .ok_or_else(|| format!("the Minecraft version cannot be read from {vanilla}"))?;
    let loader_version = fabric_loader_in(dir, &jar);
    Ok(Detected {
        loader: LoaderId::Fabric,
        game_version,
        loader_version,
        port: port_of(dir, LoaderId::Fabric),
        jar,
        vanilla_jar: Some(vanilla),
    })
}

fn fabric_loader_in(dir: &Path, jar: &str) -> Option<String> {
    let from_manifest = entry_in_jar(&dir.join(jar), "META-INF/MANIFEST.MF").and_then(|manifest| {
        let joined = manifest.replace("\r\n ", "").replace("\n ", "");
        let start = joined.find("fabric-loader-")? + "fabric-loader-".len();
        let rest = &joined[start..];
        rest.find(".jar").map(|end| rest[..end].to_owned())
    });
    from_manifest.or_else(|| {
        let folder = dir.join("libraries/net/fabricmc/fabric-loader");
        let mut versions: Vec<String> = std::fs::read_dir(folder)
            .ok()?
            .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
            .collect();
        versions.sort();
        versions.pop()
    })
}

pub fn memory_of(start_command: &str) -> u32 {
    start_command
        .split_whitespace()
        .find_map(|token| token.strip_prefix("-Xmx"))
        .and_then(mebibytes)
        .map_or(DEFAULT_MEMORY_MIB, |mib| mib.max(MIN_MEMORY_MIB))
}

fn mebibytes(size: &str) -> Option<u32> {
    let split = size.find(|c: char| !c.is_ascii_digit()).unwrap_or(size.len());
    let (digits, unit) = size.split_at(split);
    let amount: u64 = digits.parse().ok()?;
    let mib = match unit.to_ascii_lowercase().as_str() {
        "g" | "gb" => amount * 1024,
        "m" | "mb" => amount,
        "k" | "kb" => amount / 1024,
        "" => amount / (1024 * 1024),
        _ => return None,
    };
    u32::try_from(mib).ok()
}

fn jar_of(dir: &Path, start_command: &str) -> Result<String, String> {
    let tokens: Vec<&str> = start_command.split_whitespace().collect();
    if let Some(at) = tokens.iter().position(|token| *token == "-jar") {
        if let Some(named) = tokens.get(at + 1) {
            let named = named.trim_matches(|c| c == '"' || c == '\'');
            let file = Path::new(named).file_name().and_then(|name| name.to_str());
            let plain = |file: &&str| std::fs::symlink_metadata(dir.join(file)).is_ok_and(|meta| meta.is_file());
            if let Some(file) = file.filter(plain) {
                return Ok(file.to_owned());
            }
        }
    }

    let jars: Vec<String> = std::fs::read_dir(dir)
        .map_err(|err| format!("its folder cannot be read: {err}"))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.to_ascii_lowercase().ends_with(".jar"))
        .collect();
    match jars.as_slice() {
        [only] => Ok(only.clone()),
        [] => Err("there is no server jar in its folder".to_owned()),
        _ => Err(format!(
            "its start command names no jar and the folder has several: {}",
            jars.join(", ")
        )),
    }
}

fn loader_of(dir: &Path, jar: &str) -> LoaderId {
    if jar.starts_with("limbo") {
        return LoaderId::Limbo;
    }
    if jar.contains("craftbukkit") {
        return LoaderId::CraftBukkit;
    }
    if dir.join("velocity.toml").is_file() || jar.contains("velocity") {
        return LoaderId::Velocity;
    }
    if jar.contains("fabric") || dir.join(".fabric").is_dir() {
        return LoaderId::Fabric;
    }
    for (word, loader) in [
        ("folia", LoaderId::Folia),
        ("leaf", LoaderId::Leaf),
        ("purpur", LoaderId::Purpur),
        ("paper", LoaderId::Paper),
    ] {
        if jar.contains(word) {
            return loader;
        }
    }
    if let Some(brand) = history(dir).and_then(|text| brand_of(&text)) {
        return brand;
    }
    if dir.join("purpur.yml").is_file() {
        return LoaderId::Purpur;
    }
    if dir.join("config").join("paper-global.yml").is_file() || dir.join("paper.yml").is_file() {
        return LoaderId::Paper;
    }
    LoaderId::Vanilla
}

fn history(dir: &Path) -> Option<String> {
    let text = read_plain(&dir.join("version_history.json")).ok()??;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    value.get("currentVersion")?.as_str().map(str::to_owned)
}

fn brand_of(current: &str) -> Option<LoaderId> {
    let lower = current.to_ascii_lowercase();
    [
        ("folia", LoaderId::Folia),
        ("leaf", LoaderId::Leaf),
        ("purpur", LoaderId::Purpur),
        ("paper", LoaderId::Paper),
    ]
    .into_iter()
    .find_map(|(word, loader)| lower.contains(word).then_some(loader))
}

fn paper_version(dir: &Path, jar: &str, loader: LoaderId) -> Result<(String, Option<String>), String> {
    if let Some(current) = history(dir) {
        if let Some(game) = between(&current, "(MC: ", ")") {
            return Ok((game.to_owned(), build_in_history(&current)));
        }
    }
    if let Some((game, build)) = game_and_build_in_name(jar) {
        return Ok((game, build));
    }
    match version_in_jar(&dir.join(jar)) {
        Some(game) => Ok((game, None)),
        None => Err(format!("the {} version cannot be read from {jar}", loader.as_str())),
    }
}

fn build_in_history(current: &str) -> Option<String> {
    if let Some(rest) = current.strip_prefix("git-") {
        let build = rest.split_whitespace().next()?.rsplit('-').next()?;
        return build.chars().all(|c| c.is_ascii_digit()).then(|| build.to_owned());
    }
    let mut parts = current.split_whitespace().next()?.split('-');
    let _game = parts.next()?;
    let build = parts.next()?;
    build.chars().all(|c| c.is_ascii_digit()).then(|| build.to_owned())
}

fn game_and_build_in_name(jar: &str) -> Option<(String, Option<String>)> {
    let stem = jar.strip_suffix(".jar").unwrap_or(jar);
    let mut parts = stem.split('-').skip(1);
    let game = parts.next().filter(|part| looks_like_game_version(part))?;
    let build = parts.next().filter(|part| part.chars().all(|c| c.is_ascii_digit()));
    Some((game.to_owned(), build.map(str::to_owned)))
}

fn looks_like_game_version(text: &str) -> bool {
    text.contains('.') && text.chars().all(|c| c.is_ascii_digit() || c == '.')
}

fn fabric_version(jar: &str) -> Result<(String, Option<String>), String> {
    let game = between(jar, "fabric-server-mc.", "-loader.");
    let loader = between(jar, "-loader.", "-launcher.");
    match (game, loader) {
        (Some(game), Some(loader)) => Ok((game.to_owned(), Some(loader.to_owned()))),
        _ => Err(format!("the Fabric version cannot be read from {jar}")),
    }
}

fn velocity_version(dir: &Path, jar: &str) -> Result<(String, Option<String>), String> {
    let stem = jar.strip_suffix(".jar").unwrap_or(jar);
    if let Some(rest) = stem.strip_prefix("velocity-") {
        if let Some((version, build)) = rest.rsplit_once('-') {
            if build.chars().all(|c| c.is_ascii_digit()) {
                return Ok((version.to_owned(), Some(build.to_owned())));
            }
        }
        return Ok((rest.to_owned(), None));
    }
    let manifest = entry_in_jar(&dir.join(jar), "META-INF/MANIFEST.MF")
        .ok_or_else(|| format!("the Velocity version cannot be read from {jar}"))?;
    let version = manifest
        .lines()
        .find_map(|line| line.strip_prefix("Implementation-Version:"))
        .map(|value| value.trim().split_whitespace().next().unwrap_or_default().to_owned())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("the Velocity version cannot be read from {jar}"))?;
    Ok((version, None))
}

fn craftbukkit_version(jar: &str) -> Result<String, String> {
    let lower = jar.to_ascii_lowercase();
    let stem = lower.strip_suffix(".jar").unwrap_or(&lower);
    stem.strip_prefix("craftbukkit-")
        .and_then(|rest| rest.split('-').next())
        .filter(|version| looks_like_game_version(version))
        .map(str::to_owned)
        .ok_or_else(|| format!("the Minecraft version cannot be read from {jar}"))
}

fn limbo_version(jar: &str) -> Result<(String, Option<String>), String> {
    let stem = jar.strip_suffix(".jar").unwrap_or(jar);
    let (rest, game) = stem
        .rsplit_once('-')
        .filter(|(_, game)| looks_like_game_version(game))
        .ok_or_else(|| format!("the Minecraft version cannot be read from {jar}"))?;
    let build = rest.split_once('-').map(|(_, build)| build.to_owned()).filter(|build| !build.is_empty());
    Ok((game.to_owned(), build))
}

fn vanilla_version(dir: &Path, jar: &str) -> Result<String, String> {
    if let Some(game) = version_in_jar(&dir.join(jar)) {
        return Ok(game);
    }
    let stem = jar.strip_suffix(".jar").unwrap_or(jar);
    stem.strip_prefix("minecraft_server.")
        .map(str::to_owned)
        .ok_or_else(|| format!("the Minecraft version cannot be read from {jar}"))
}

fn version_in_jar(jar: &Path) -> Option<String> {
    let text = entry_in_jar(jar, "version.json")?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    value.get("id").or_else(|| value.get("name"))?.as_str().map(str::to_owned)
}

fn entry_in_jar(jar: &Path, name: &str) -> Option<String> {
    let file = std::fs::File::open(jar).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;
    let mut entry = archive.by_name(name).ok()?;
    let mut text = String::new();
    entry.by_ref().take(1024 * 1024).read_to_string(&mut text).ok()?;
    Some(text)
}

fn between<'a>(text: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let from = text.find(start)? + start.len();
    let length = text[from..].find(end)?;
    Some(&text[from..from + length]).filter(|found| !found.is_empty())
}

pub fn port_of(dir: &Path, loader: LoaderId) -> Option<u16> {
    if loader == LoaderId::Velocity {
        let text = read_plain(&dir.join("velocity.toml")).ok()??;
        let bind = text.lines().find_map(|line| {
            let (key, value) = line.split_once('=')?;
            (key.trim() == "bind").then(|| value.trim().trim_matches('"').to_owned())
        })?;
        return bind.rsplit(':').next()?.parse().ok();
    }
    let text = read_plain(&dir.join("server.properties")).ok()??;
    text.lines()
        .find_map(|line| line.strip_prefix("server-port="))
        .and_then(|port| port.trim().parse().ok())
}

pub fn read_plain(path: &Path) -> std::io::Result<Option<String>> {
    let mut file = match OpenOptions::new().read(true).custom_flags(libc::O_NOFOLLOW).open(path) {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err),
    };
    plain(&file)?;
    let mut text = String::new();
    file.read_to_string(&mut text)?;
    Ok(Some(text))
}

pub fn write_plain(path: &Path, text: &str) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o660)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)?;
    plain(&file)?;
    file.set_len(0)?;
    file.write_all(text.as_bytes())
}

fn plain(file: &File) -> std::io::Result<()> {
    let meta = file.metadata()?;
    if !meta.is_file() || std::os::unix::fs::MetadataExt::nlink(&meta) != 1 {
        return Err(std::io::Error::other("not a plain file with a single link"));
    }
    Ok(())
}

pub fn set_port(dir: &Path, loader: LoaderId, port: u16) -> std::io::Result<()> {
    if loader == LoaderId::Velocity {
        let path = dir.join("velocity.toml");
        let Some(text) = read_plain(&path)? else { return Ok(()) };
        let rewritten: Vec<String> = text
            .lines()
            .map(|line| match line.split_once('=') {
                Some((key, value)) if key.trim() == "bind" => {
                    let host = value.trim().trim_matches('"').rsplit_once(':').map_or("0.0.0.0", |(host, _)| host).to_owned();
                    format!("bind = \"{host}:{port}\"")
                }
                _ => line.to_owned(),
            })
            .collect();
        return write_plain(&path, &(rewritten.join("\n") + "\n"));
    }

    let path = dir.join("server.properties");
    let text = read_plain(&path)?.unwrap_or_default();
    let mut lines: Vec<String> = text
        .lines()
        .filter(|line| !line.starts_with("server-port=") && !line.starts_with("query.port="))
        .map(str::to_owned)
        .collect();
    lines.push(format!("server-port={port}"));
    lines.push(format!("query.port={port}"));
    write_plain(&path, &(lines.join("\n") + "\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("craftpanel-detect-{}", crate::model::Id::new()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn jar_with(path: &Path, entries: &[(&str, &str)]) {
        let file = std::fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        for (name, body) in entries {
            zip.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
            std::io::Write::write_all(&mut zip, body.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }

    #[test]
    fn paper_is_read_from_its_version_history() {
        let dir = folder();
        std::fs::write(dir.join("paper.jar"), b"").unwrap();
        std::fs::write(dir.join("version_history.json"), r#"{"currentVersion":"git-Paper-130 (MC: 1.21.4)"}"#).unwrap();
        std::fs::write(dir.join("server.properties"), "motd=hi\nserver-port=25570\n").unwrap();

        let found = detect(&dir, "java -Xmx4G -jar paper.jar nogui").unwrap();
        assert_eq!(found.loader, LoaderId::Paper);
        assert_eq!(found.game_version, "1.21.4");
        assert_eq!(found.loader_version.as_deref(), Some("130"));
        assert_eq!(found.jar, "paper.jar");
        assert_eq!(found.port, Some(25570));
    }

    #[test]
    fn the_newer_paper_history_format_and_a_named_purpur_jar_are_understood() {
        let dir = folder();
        std::fs::write(dir.join("purpur-1.21.4-2380.jar"), b"").unwrap();
        let found = detect(&dir, "java -jar purpur-1.21.4-2380.jar").unwrap();
        assert_eq!((found.loader, found.game_version.as_str()), (LoaderId::Purpur, "1.21.4"));
        assert_eq!(found.loader_version.as_deref(), Some("2380"));

        assert_eq!(build_in_history("1.21.4-232-12ab3c4 (MC: 1.21.4)").as_deref(), Some("232"));
    }

    #[test]
    fn fabric_velocity_and_vanilla_are_told_apart() {
        let dir = folder();
        std::fs::write(dir.join("fabric-server-mc.1.21.1-loader.0.16.10-launcher.1.0.1.jar"), b"").unwrap();
        let fabric = detect(&dir, "java -Xmx3072M -jar fabric-server-mc.1.21.1-loader.0.16.10-launcher.1.0.1.jar nogui").unwrap();
        assert_eq!((fabric.loader, fabric.game_version.as_str()), (LoaderId::Fabric, "1.21.1"));
        assert_eq!(fabric.loader_version.as_deref(), Some("0.16.10"));

        let proxy = folder();
        std::fs::write(proxy.join("velocity-3.4.0-SNAPSHOT-436.jar"), b"").unwrap();
        std::fs::write(proxy.join("velocity.toml"), "bind = \"0.0.0.0:25577\"\n").unwrap();
        let velocity = detect(&proxy, "java -jar velocity-3.4.0-SNAPSHOT-436.jar").unwrap();
        assert_eq!((velocity.loader, velocity.game_version.as_str()), (LoaderId::Velocity, "3.4.0-SNAPSHOT"));
        assert_eq!(velocity.port, Some(25577));

        let plain = folder();
        jar_with(&plain.join("server.jar"), &[("version.json", r#"{"id":"1.20.6","name":"1.20.6"}"#)]);
        let vanilla = detect(&plain, "java -jar server.jar nogui").unwrap();
        assert_eq!((vanilla.loader, vanilla.game_version.as_str()), (LoaderId::Vanilla, "1.20.6"));
    }

    #[test]
    fn what_cannot_run_here_is_refused_with_a_reason() {
        let forge = folder();
        std::fs::create_dir_all(forge.join("libraries/net/minecraftforge/forge/1.20.1-47.3.0")).unwrap();
        assert!(detect(&forge, "bash run.sh").unwrap_err().contains("no server jar"));

        let bungee = folder();
        std::fs::write(bungee.join("BungeeCord.jar"), b"").unwrap();
        assert!(detect(&bungee, "java -jar BungeeCord.jar").unwrap_err().contains("BungeeCord"));

        let empty = folder();
        assert!(detect(&empty, "java -jar server.jar").unwrap_err().contains("no server jar"));
    }

    #[test]
    fn a_planted_link_is_never_written_through() {
        let dir = folder();
        let elsewhere = folder().join("target.txt");
        std::fs::write(&elsewhere, "untouched\n").unwrap();
        std::os::unix::fs::symlink(&elsewhere, dir.join("server.properties")).unwrap();

        assert!(set_port(&dir, LoaderId::Paper, 25601).is_err());
        assert_eq!(std::fs::read_to_string(&elsewhere).unwrap(), "untouched\n");

        let hard = folder();
        std::fs::hard_link(&elsewhere, hard.join("server.properties")).unwrap();
        assert!(set_port(&hard, LoaderId::Paper, 25601).is_err());
        assert_eq!(std::fs::read_to_string(&elsewhere).unwrap(), "untouched\n");
    }

    #[test]
    fn paper_with_neoforged_libraries_is_still_paper() {
        let dir = folder();
        std::fs::create_dir_all(dir.join("libraries/net/neoforged/AutoRenamingTool/2.0.3")).unwrap();
        std::fs::write(dir.join("paper-1.21.11-39.jar"), b"").unwrap();
        let found = detect(&dir, "java -jar paper-1.21.11-39.jar nogui").unwrap();
        assert_eq!((found.loader, found.game_version.as_str()), (LoaderId::Paper, "1.21.11"));
    }

    #[test]
    fn the_old_fabric_launcher_is_read_with_its_minecraft_jar() {
        let dir = folder();
        jar_with(&dir.join("fabric-server-launch.jar"), &[(
            "META-INF/MANIFEST.MF",
            "Manifest-Version: 1.0\nClass-Path: libraries/net/fabricmc/fabric-loader/0.16.14/fabric-l\n oader-0.16.14.jar libraries/x.jar\n",
        )]);
        jar_with(&dir.join("server.jar"), &[("version.json", r#"{"id":"1.21.10"}"#)]);
        std::fs::write(dir.join(LAUNCHER_PROPERTIES), "serverJar=server.jar\n").unwrap();

        let found = detect(&dir, "java -Xmx4G -jar fabric-server-launch.jar nogui").unwrap();
        assert_eq!((found.loader, found.game_version.as_str()), (LoaderId::Fabric, "1.21.10"));
        assert_eq!(found.loader_version.as_deref(), Some("0.16.14"));
        assert_eq!(found.jar, "fabric-server-launch.jar");
        assert_eq!(found.vanilla_jar.as_deref(), Some("server.jar"));

        std::fs::remove_file(dir.join("server.jar")).unwrap();
        assert!(detect(&dir, "java -jar fabric-server-launch.jar").unwrap_err().contains("missing"));
    }

    #[test]
    fn craftbukkit_and_limbo_are_told_by_their_jars() {
        let old = folder();
        std::fs::write(old.join("craftbukkit-1.6.4-R2.0.jar"), b"").unwrap();
        let found = detect(&old, "java -jar craftbukkit-1.6.4-R2.0.jar").unwrap();
        assert_eq!((found.loader, found.game_version.as_str()), (LoaderId::CraftBukkit, "1.6.4"));

        let lobby = folder();
        std::fs::write(lobby.join("Limbo-2026.0.1-ALPHA-26.1.2.jar"), b"").unwrap();
        std::fs::write(lobby.join("server.properties"), "server-port=30000\n").unwrap();
        let found = detect(&lobby, "java -jar Limbo-2026.0.1-ALPHA-26.1.2.jar --nogui").unwrap();
        assert_eq!((found.loader, found.game_version.as_str()), (LoaderId::Limbo, "26.1.2"));
        assert_eq!(found.loader_version.as_deref(), Some("2026.0.1-ALPHA"));
        assert_eq!(found.port, Some(30000));
    }

    #[test]
    fn forge_and_neoforge_are_read_from_the_args_their_installer_wrote() {
        let forge = folder();
        let home = forge.join("libraries/net/minecraftforge/forge/1.20.1-47.3.0");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(home.join("unix_args.txt"), "-p libraries/x.jar --launchTarget forge_server --fml.forgeVersion 47.3.0 --fml.mcVersion 1.20.1 --fml.forgeGroup net.minecraftforge\n").unwrap();
        std::fs::write(forge.join("server.properties"), "server-port=25570\n").unwrap();
        let found = detect(&forge, "bash run.sh").unwrap();
        assert_eq!((found.loader, found.game_version.as_str()), (LoaderId::Forge, "1.20.1"));
        assert_eq!(found.loader_version.as_deref(), Some("47.3.0"));
        assert_eq!(found.jar, "server.jar");
        assert_eq!(found.port, Some(25570));

        let neo = folder();
        let home = neo.join("libraries/net/neoforged/neoforge/21.1.77");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(home.join("unix_args.txt"), "--launchTarget forgeserver --fml.neoForgeVersion 21.1.77 --fml.fmlVersion 4.0.31 --fml.mcVersion 1.21.1\n").unwrap();
        let found = detect(&neo, "bash run.sh").unwrap();
        assert_eq!((found.loader, found.game_version.as_str()), (LoaderId::NeoForge, "1.21.1"));
        assert_eq!(found.loader_version.as_deref(), Some("21.1.77"));

        let old = folder();
        std::fs::write(old.join("forge-1.12.2-14.23.5.2860.jar"), b"").unwrap();
        let found = detect(&old, "java -Xmx4G -jar forge-1.12.2-14.23.5.2860.jar nogui").unwrap();
        assert_eq!((found.loader, found.game_version.as_str()), (LoaderId::Forge, "1.12.2"));
        assert_eq!(found.loader_version.as_deref(), Some("14.23.5.2860"));
        assert_eq!(found.jar, "forge-1.12.2-14.23.5.2860.jar");
    }

    #[test]
    fn memory_comes_from_xmx() {
        assert_eq!(memory_of("java -Xms1G -Xmx4G -jar a.jar"), 4096);
        assert_eq!(memory_of("java -Xmx3072M -jar a.jar"), 3072);
        assert_eq!(memory_of("java -Xmx256m -jar a.jar"), 512);
        assert_eq!(memory_of("bash start.sh"), DEFAULT_MEMORY_MIB);
    }

    #[test]
    fn a_new_port_lands_in_the_properties() {
        let dir = folder();
        std::fs::write(dir.join("server.properties"), "motd=hi\nserver-port=25565\nquery.port=25565\n").unwrap();
        set_port(&dir, LoaderId::Paper, 25601).unwrap();
        let text = std::fs::read_to_string(dir.join("server.properties")).unwrap();
        assert!(text.contains("motd=hi") && text.contains("server-port=25601") && text.contains("query.port=25601"));
        assert!(!text.contains("25565"));

        std::fs::write(dir.join("velocity.toml"), "motd = \"x\"\nbind = \"0.0.0.0:25577\"\n").unwrap();
        set_port(&dir, LoaderId::Velocity, 25602).unwrap();
        assert!(std::fs::read_to_string(dir.join("velocity.toml")).unwrap().contains("bind = \"0.0.0.0:25602\""));
    }
}
