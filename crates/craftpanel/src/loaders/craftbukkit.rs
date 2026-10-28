use serde::Deserialize;

use super::checksum::Checksum;
use super::error::{LoaderError, Result};
use super::http::{self, Http};
use super::{Build, Channel, Version};

const SERVICE: &str = "GetBukkit";
const LOADER: &str = "CraftBukkit";
const SITE: &str = "https://getbukkit.org";
const API: &str = "https://getbukkit.org/api/jars/craftbukkit";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Jar {
    version: String,
    file_name: String,
    size: Option<u64>,
    built_at: String,
    sha256: String,
    download_url: String,
}

pub async fn versions(http: &Http) -> Result<Vec<Version>> {
    parse_versions(&http.fetch(SERVICE, API).await?)
}

pub async fn builds(http: &Http, version: &str) -> Result<Vec<Build>> {
    if !version.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-') {
        return Err(LoaderError::UnknownVersion { loader: LOADER, version: version.to_owned() });
    }
    let url = format!("{API}/{version}");
    let body = http.maybe_fetch(SERVICE, &url).await?.ok_or_else(|| {
        LoaderError::UnknownVersion { loader: LOADER, version: version.to_owned() }
    })?;
    Ok(vec![build_of(http::parse(SERVICE, &body)?)])
}

fn parse_versions(body: &[u8]) -> Result<Vec<Version>> {
    let jars: Vec<Jar> = http::parse(SERVICE, body)?;
    let mut ids: Vec<String> = jars.into_iter().map(|jar| jar.version).collect();
    ids.dedup();
    super::newest_first(&mut ids);
    Ok(ids.into_iter().map(|id| Version { id, channel: Channel::Stable }).collect())
}

fn build_of(jar: Jar) -> Build {
    Build {
        id: jar.built_at.chars().take(10).collect(),
        channel: Channel::Stable,
        url: format!("{SITE}{}", jar.download_url),
        filename: jar.file_name,
        checksum: Some(Checksum::sha256(jar.sha256)),
        size: jar.size,
        java_major: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST: &[u8] = include_bytes!("testdata/getbukkit_craftbukkit.json");
    const ONE: &[u8] = include_bytes!("testdata/getbukkit_craftbukkit_26.3.json");

    #[test]
    fn versions_come_newest_first() {
        let versions = parse_versions(LIST).unwrap();
        let ids: Vec<&str> = versions.iter().map(|v| v.id.as_str()).collect();
        assert_eq!(ids.first(), Some(&"26.3"));
        assert_eq!(ids.last(), Some(&"1.8"));
        assert!(versions.iter().all(|v| v.channel == Channel::Stable));
    }

    #[test]
    fn one_jar_is_one_build_named_after_its_day() {
        let build = build_of(http::parse(SERVICE, ONE).unwrap());
        assert_eq!(build.id, "2026-10-26");
        assert_eq!(build.filename, "craftbukkit-26.3.jar");
        assert_eq!(build.url, "https://getbukkit.org/craftbukkit/craftbukkit-26.3.jar");
        assert_eq!(build.size, Some(85339589));
        assert_eq!(
            build.checksum,
            Some(Checksum::sha256("5a0d737a972defb8a0c0c263b727eebe799ee28e52dff9f47c398d50d597fb05"))
        );
    }
}
