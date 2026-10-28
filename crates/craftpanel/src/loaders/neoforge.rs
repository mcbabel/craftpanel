use super::error::{LoaderError, Result};
use super::http::Http;
use super::maven::{self, Remembered};
use super::{Build, Channel, Version};

const SERVICE: &str = "NeoForged";
const LOADER: &str = "NeoForge";
const MAVEN: &str = "https://maven.neoforged.net/releases/net/neoforged/neoforge";

static METADATA: Remembered = Remembered::new();

pub async fn versions(http: &Http) -> Result<Vec<Version>> {
    let metadata = METADATA.fetch(http, SERVICE, &format!("{MAVEN}/maven-metadata.xml")).await?;
    Ok(parse_versions(&metadata))
}

pub async fn builds(http: &Http, version: &str) -> Result<Vec<Build>> {
    let metadata = METADATA.fetch(http, SERVICE, &format!("{MAVEN}/maven-metadata.xml")).await?;
    let builds = parse_builds(&metadata, version);
    if builds.is_empty() {
        return Err(LoaderError::UnknownVersion { loader: LOADER, version: version.to_owned() });
    }
    Ok(builds)
}

pub fn game_of(neoforge: &str) -> Option<String> {
    if neoforge.contains('+') {
        return None;
    }
    let base = neoforge.split('-').next()?;
    let parts: Vec<u32> = base.split('.').map(|part| part.parse().ok()).collect::<Option<_>>()?;
    match parts.as_slice() {
        [major, minor, _] if *major < 26 => {
            Some(if *minor == 0 { format!("1.{major}") } else { format!("1.{major}.{minor}") })
        }
        [major, minor, patch, _] if *major >= 26 => {
            Some(if *patch == 0 { format!("{major}.{minor}") } else { format!("{major}.{minor}.{patch}") })
        }
        _ => None,
    }
}

fn parse_versions(metadata: &[u8]) -> Vec<Version> {
    let mut games: Vec<String> =
        maven::versions(metadata).iter().filter_map(|version| game_of(version)).collect();
    games.sort();
    games.dedup();
    super::newest_first(&mut games);
    games.into_iter().map(|id| Version { id, channel: Channel::Stable }).collect()
}

fn parse_builds(metadata: &[u8], game: &str) -> Vec<Build> {
    let mut versions: Vec<String> = maven::versions(metadata)
        .into_iter()
        .filter(|version| game_of(version).as_deref() == Some(game))
        .collect();
    super::newest_first(&mut versions);

    let mut builds: Vec<Build> = versions
        .into_iter()
        .map(|version| Build {
            channel: if version.contains('-') { Channel::Experimental } else { Channel::Stable },
            url: format!("{MAVEN}/{version}/neoforge-{version}-installer.jar"),
            filename: format!("neoforge-{version}-installer.jar"),
            id: version,
            checksum: None,
            size: None,
            java_major: None,
        })
        .collect();
    if !builds.iter().any(|build| build.channel == Channel::Stable) {
        if let Some(newest) = builds.first_mut() {
            newest.channel = Channel::Stable;
        }
    }
    builds
}

#[cfg(test)]
mod tests {
    use super::*;

    const METADATA_XML: &[u8] = include_bytes!("testdata/neoforge_metadata.xml");

    #[test]
    fn neoforge_names_the_minecraft_version_inside_its_own() {
        assert_eq!(game_of("21.1.77").as_deref(), Some("1.21.1"));
        assert_eq!(game_of("21.0.166").as_deref(), Some("1.21"));
        assert_eq!(game_of("20.2.86-beta").as_deref(), Some("1.20.2"));
        assert_eq!(game_of("26.1.2.123").as_deref(), Some("26.1.2"));
        assert_eq!(game_of("26.3.0.99").as_deref(), Some("26.3"));
        assert_eq!(game_of("26.1.0.0-alpha.1+snapshot-1"), None, "snapshots are left out");
    }

    #[test]
    fn versions_and_builds_come_newest_first_with_betas_marked() {
        let ids: Vec<String> = parse_versions(METADATA_XML).into_iter().map(|v| v.id).collect();
        assert_eq!(ids, ["26.3", "26.1.2", "1.21.1", "1.21", "1.20.5", "1.20.2"]);

        let builds = parse_builds(METADATA_XML, "1.21.1");
        assert_eq!(builds.iter().map(|b| b.id.as_str()).collect::<Vec<_>>(), ["21.1.265", "21.1.77"]);
        assert_eq!(
            builds[0].url,
            "https://maven.neoforged.net/releases/net/neoforged/neoforge/21.1.265/neoforge-21.1.265-installer.jar"
        );

        let only_beta = parse_builds(METADATA_XML, "1.20.5");
        assert_eq!(only_beta[0].channel, Channel::Stable, "the newest stands in when there is no release");
    }
}
