use std::sync::Mutex;
use std::time::{Duration, Instant};

use super::checksum::Checksum;
use super::error::Result;
use super::http::Http;

const KEEP_FOR: Duration = Duration::from_secs(10 * 60);

pub struct Remembered {
    body: Mutex<Option<(Instant, String, Vec<u8>)>>,
}

impl Remembered {
    pub const fn new() -> Self {
        Self { body: Mutex::new(None) }
    }

    pub async fn fetch(&self, http: &Http, service: &'static str, url: &str) -> Result<Vec<u8>> {
        if let Some((at, from, body)) = self.body.lock().expect("the remembered body").as_ref() {
            if from == url && at.elapsed() < KEEP_FOR {
                return Ok(body.clone());
            }
        }
        let body = http.fetch(service, url).await?;
        *self.body.lock().expect("the remembered body") =
            Some((Instant::now(), url.to_owned(), body.clone()));
        Ok(body)
    }
}

pub fn versions(metadata: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(metadata);
    text.split("<version>")
        .skip(1)
        .filter_map(|part| part.split_once("</version>").map(|(version, _)| version.trim().to_owned()))
        .filter(|version| !version.is_empty())
        .collect()
}

pub async fn checksum_of(http: &Http, service: &'static str, url: &str) -> Result<Option<Checksum>> {
    for suffix in [".sha256", ".sha1"] {
        if let Some(body) = http.maybe_fetch(service, &format!("{url}{suffix}")).await? {
            let text = String::from_utf8_lossy(&body);
            if let Some(value) = text.split_whitespace().next().filter(|value| value.chars().all(|c| c.is_ascii_hexdigit())) {
                let value = value.to_ascii_lowercase();
                return Ok(Some(if suffix == ".sha256" { Checksum::sha256(value) } else { Checksum::sha1(value) }));
            }
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_versions_are_read_out_of_maven_metadata() {
        let xml = b"<metadata><versioning><versions><version>1.0</version>\n<version> 2.0 </version></versions></versioning></metadata>";
        assert_eq!(versions(xml), ["1.0", "2.0"]);
    }
}
