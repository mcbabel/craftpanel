use std::collections::HashMap;

use serde::Deserialize;

use super::error::{LoaderError, Result};
use super::http::{self, Http};
use super::maven::{self, Remembered};
use super::{Build, Channel, Version};

const SERVICE: &str = "MinecraftForge";
const LOADER: &str = "Forge";
const MAVEN: &str = "https://maven.minecraftforge.net/net/minecraftforge/forge";
const PROMOTIONS: &str = "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json";

static METADATA: Remembered = Remembered::new();
static PROMOS: Remembered = Remembered::new();

#[derive(Deserialize)]
struct Promotions {
    promos: HashMap<String, String>,
}

pub async fn versions(http: &Http) -> Result<Vec<Version>> {
    let metadata = METADATA.fetch(http, SERVICE, &format!("{MAVEN}/maven-metadata.xml")).await?;
    Ok(parse_versions(&metadata))
}

pub async fn builds(http: &Http, version: &str) -> Result<Vec<Build>> {
    let metadata = METADATA.fetch(http, SERVICE, &format!("{MAVEN}/maven-metadata.xml")).await?;
    let promotions = PROMOS.fetch(http, SERVICE, PROMOTIONS).await?;
    let builds = parse_builds(&metadata, &promotions, version)?;
    if builds.is_empty() {
        return Err(LoaderError::UnknownVersion { loader: LOADER, version: version.to_owned() });
    }
    Ok(builds)
}

fn split(full: &str) -> Option<(&str, &str)> {
    let (game, forge) = full.split_once('-')?;
    supported_game(game).then_some((game, forge))
}

fn supported_game(game: &str) -> bool {
    let mut parts = game.split('.');
    match (parts.next(), parts.next().and_then(|minor| minor.parse::<u32>().ok())) {
        (Some("1"), Some(minor)) => minor >= 7,
        (Some(major), _) => major.parse::<u32>().is_ok_and(|major| major > 1),
        _ => false,
    }
}

fn parse_versions(metadata: &[u8]) -> Vec<Version> {
    let mut games: Vec<String> = maven::versions(metadata)
        .iter()
        .filter_map(|full| split(full).map(|(game, _)| game.to_owned()))
        .collect();
    games.sort();
    games.dedup();
    super::newest_first(&mut games);
    games.into_iter().map(|id| Version { id, channel: Channel::Stable }).collect()
}

fn parse_builds(metadata: &[u8], promotions: &[u8], game: &str) -> Result<Vec<Build>> {
    let promotions: Promotions = http::parse(SERVICE, promotions)?;
    let recommended = promotions
        .promos
        .get(&format!("{game}-recommended"))
        .or_else(|| promotions.promos.get(&format!("{game}-latest")))
        .cloned();

    let mut fulls: Vec<String> = maven::versions(metadata)
        .into_iter()
        .filter(|full| split(full).is_some_and(|(found, _)| found == game))
        .collect();
    super::newest_first(&mut fulls);

    let mut builds: Vec<Build> = fulls
        .into_iter()
        .map(|full| {
            let forge = split(&full).map(|(_, forge)| forge.to_owned()).unwrap_or_default();
            Build {
                channel: if recommended.as_deref() == Some(forge.as_str()) {
                    Channel::Stable
                } else {
                    Channel::Experimental
                },
                url: format!("{MAVEN}/{full}/forge-{full}-installer.jar"),
                filename: format!("forge-{full}-installer.jar"),
                id: forge,
                checksum: None,
                size: None,
                java_major: None,
            }
        })
        .collect();
    if !builds.iter().any(|build| build.channel == Channel::Stable) {
        if let Some(newest) = builds.first_mut() {
            newest.channel = Channel::Stable;
        }
    }
    Ok(builds)
}

#[cfg(test)]
mod tests {
    use super::*;

    const METADATA_XML: &[u8] = include_bytes!("testdata/forge_metadata.xml");
    const PROMOTIONS_JSON: &[u8] = include_bytes!("testdata/forge_promotions.json");

    #[test]
    fn game_versions_come_from_the_maven_list_newest_first() {
        let ids: Vec<String> = parse_versions(METADATA_XML).into_iter().map(|v| v.id).collect();
        assert_eq!(ids, ["1.21.1", "1.20.1", "1.12.2", "1.7.10"], "1.5.2 has no installer");
    }

    #[test]
    fn the_recommended_build_is_the_stable_one_and_the_installer_is_named_after_it() {
        let builds = parse_builds(METADATA_XML, PROMOTIONS_JSON, "1.12.2").unwrap();
        assert_eq!(builds.iter().map(|b| b.id.as_str()).collect::<Vec<_>>(), ["14.23.5.2864", "14.23.5.2859"]);
        assert_eq!(builds[1].channel, Channel::Stable, "2859 is recommended");
        assert_eq!(builds[0].channel, Channel::Experimental);
        assert_eq!(
            builds[1].url,
            "https://maven.minecraftforge.net/net/minecraftforge/forge/1.12.2-14.23.5.2859/forge-1.12.2-14.23.5.2859-installer.jar"
        );

        let odd = parse_builds(METADATA_XML, PROMOTIONS_JSON, "1.7.10").unwrap();
        assert_eq!(odd[0].id, "10.13.4.1614-1.7.10");
        assert_eq!(odd[0].filename, "forge-1.7.10-10.13.4.1614-1.7.10-installer.jar");
    }
}
