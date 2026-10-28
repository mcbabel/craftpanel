use std::io::Read;
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
}

pub fn detect(dir: &Path, start_command: &str) -> Result<Detected, String> {
    let libraries = dir.join("libraries").join("net");
    if libraries.join("minecraftforge").is_dir()
        || libraries.join("neoforged").is_dir()
        || dir.join("user_jvm_args.txt").is_file()
    {
        return Err("Forge and NeoForge servers are not supported yet".to_owned());
    }

    let jar = jar_of(dir, start_command)?;
    let name = jar.to_ascii_lowercase();
    if name.contains("bungee") || name.contains("waterfall") {
        return Err("BungeeCord and Waterfall are not supported; Velocity is".to_owned());
    }
    if name == "fabric-server-launch.jar" {
        return Err("this is the old Fabric launcher; install Fabric fresh in the panel".to_owned());
    }

    let loader = loader_of(dir, &name);
    let (game_version, loader_version) = match loader {
        LoaderId::Velocity => velocity_version(dir, &jar)?,
        LoaderId::Fabric => fabric_version(&jar)?,
        LoaderId::Vanilla => (vanilla_version(dir, &jar)?, None),
        _ => paper_version(dir, &jar, loader)?,
    };

    Ok(Detected { loader, game_version, loader_version, jar, port: port_of(dir, loader) })
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
            if let Some(file) = file.filter(|file| dir.join(file).is_file()) {
                return Ok(file.to_owned());
            }
        }
    }

    let jars: Vec<String> = std::fs::read_dir(dir)
        .map_err(|err| format!("its folder cannot be read: {err}"))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_file())
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
    let text = std::fs::read_to_string(dir.join("version_history.json")).ok()?;
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
        let text = std::fs::read_to_string(dir.join("velocity.toml")).ok()?;
        let bind = text.lines().find_map(|line| {
            let (key, value) = line.split_once('=')?;
            (key.trim() == "bind").then(|| value.trim().trim_matches('"').to_owned())
        })?;
        return bind.rsplit(':').next()?.parse().ok();
    }
    let text = std::fs::read_to_string(dir.join("server.properties")).ok()?;
    text.lines()
        .find_map(|line| line.strip_prefix("server-port="))
        .and_then(|port| port.trim().parse().ok())
}

pub fn set_port(dir: &Path, loader: LoaderId, port: u16) -> std::io::Result<()> {
    if loader == LoaderId::Velocity {
        let path = dir.join("velocity.toml");
        let Ok(text) = std::fs::read_to_string(&path) else { return Ok(()) };
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
        return std::fs::write(path, rewritten.join("\n") + "\n");
    }

    let path = dir.join("server.properties");
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let mut lines: Vec<String> = text
        .lines()
        .filter(|line| !line.starts_with("server-port=") && !line.starts_with("query.port="))
        .map(str::to_owned)
        .collect();
    lines.push(format!("server-port={port}"));
    lines.push(format!("query.port={port}"));
    std::fs::write(path, lines.join("\n") + "\n")
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
        std::fs::create_dir_all(forge.join("libraries/net/minecraftforge")).unwrap();
        assert!(detect(&forge, "bash run.sh").unwrap_err().contains("Forge"));

        let bungee = folder();
        std::fs::write(bungee.join("BungeeCord.jar"), b"").unwrap();
        assert!(detect(&bungee, "java -jar BungeeCord.jar").unwrap_err().contains("BungeeCord"));

        let empty = folder();
        assert!(detect(&empty, "java -jar server.jar").unwrap_err().contains("no server jar"));
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
