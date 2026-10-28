use serde::Deserialize;

use super::checksum::Checksum;
use super::error::{LoaderError, Result};
use super::http::{self, Http};
use super::{Build, Channel, Version};

const SERVICE: &str = "Modrinth";
const LOADER: &str = "Limbo";
const VERSIONS: &str = "https://api.modrinth.com/v2/project/limbo-server/version";

#[derive(Deserialize)]
struct Release {
    version_number: String,
    game_versions: Vec<String>,
    files: Vec<File>,
}

#[derive(Deserialize)]
struct File {
    url: String,
    filename: String,
    size: Option<u64>,
    primary: bool,
    hashes: Hashes,
}

#[derive(Deserialize)]
struct Hashes {
    sha1: String,
}

pub async fn versions(http: &Http) -> Result<Vec<Version>> {
    parse_versions(&http.fetch(SERVICE, VERSIONS).await?)
}

pub async fn builds(http: &Http, version: &str) -> Result<Vec<Build>> {
    let builds = parse_builds(&http.fetch(SERVICE, VERSIONS).await?, version)?;
    if builds.is_empty() {
        return Err(LoaderError::UnknownVersion { loader: LOADER, version: version.to_owned() });
    }
    Ok(builds)
}

fn parse_versions(body: &[u8]) -> Result<Vec<Version>> {
    let releases: Vec<Release> = http::parse(SERVICE, body)?;
    let mut ids: Vec<String> =
        releases.into_iter().flat_map(|release| release.game_versions).collect();
    ids.sort();
    ids.dedup();
    super::newest_first(&mut ids);
    Ok(ids.into_iter().map(|id| Version { id, channel: Channel::Stable }).collect())
}

fn parse_builds(body: &[u8], version: &str) -> Result<Vec<Build>> {
    let releases: Vec<Release> = http::parse(SERVICE, body)?;
    let mut builds: Vec<Build> = Vec::new();
    for release in releases.into_iter().filter(|release| release.game_versions.iter().any(|v| v == version)) {
        if builds.iter().any(|build| build.id == release.version_number) {
            continue;
        }
        let Some(file) = release.files.into_iter().find(|file| file.primary) else {
            continue;
        };
        builds.push(Build {
            id: release.version_number,
            channel: Channel::Stable,
            url: file.url,
            filename: file.filename,
            checksum: Some(Checksum::sha1(file.hashes.sha1)),
            size: file.size,
            java_major: None,
        });
    }
    Ok(builds)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RELEASES: &[u8] = include_bytes!("testdata/modrinth_limbo_versions.json");

    #[test]
    fn every_game_version_limbo_was_built_for_is_offered_once() {
        let versions = parse_versions(RELEASES).unwrap();
        let ids: Vec<&str> = versions.iter().map(|v| v.id.as_str()).collect();
        assert_eq!(ids, ["26.3", "26.2", "26.1.2", "26.1.1", "26.1"]);
    }

    #[test]
    fn a_game_version_gets_the_newest_limbo_built_for_it_once() {
        let builds = parse_builds(RELEASES, "26.1.1").unwrap();
        assert_eq!(builds.len(), 1, "the same Limbo version listed twice counts once");
        assert_eq!(builds[0].id, "2026.0.1-ALPHA");
        assert_eq!(builds[0].filename, "Limbo-2026.0.1-ALPHA-26.1.2.jar");
        assert!(matches!(builds[0].checksum, Some(ref sum) if sum.algorithm == crate::loaders::Algorithm::Sha1));
        assert!(parse_builds(RELEASES, "1.8").unwrap().is_empty());
    }
}
