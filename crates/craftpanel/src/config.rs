use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use ipnet::IpNet;
use serde::{Deserialize, Serialize};

use crate::settings::runtimes::Search;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_bind")]
    pub bind: SocketAddr,
    #[serde(default = "default_data_dir")]
    pub data_dir: PathBuf,
    #[serde(default = "default_helper_socket")]
    pub helper_socket: PathBuf,
    #[serde(default)]
    pub trusted_proxies: Vec<Proxy>,
    #[serde(skip)]
    pub java_search: Search,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Proxy(IpNet);

impl Proxy {
    pub fn covers(&self, address: IpAddr) -> bool {
        self.0.contains(&address.to_canonical())
    }
}

impl TryFrom<String> for Proxy {
    type Error = String;

    fn try_from(raw: String) -> std::result::Result<Self, Self::Error> {
        let raw = raw.trim();
        raw.parse::<IpNet>()
            .or_else(|_| raw.parse::<IpAddr>().map(IpNet::from))
            .map(Self)
            .map_err(|_| format!("trusted_proxies: {raw} is neither an address nor a network"))
    }
}

impl From<Proxy> for String {
    fn from(proxy: Proxy) -> Self {
        proxy.0.to_string()
    }
}

fn default_bind() -> SocketAddr {
    "127.0.0.1:8080".parse().expect("valid default bind address")
}

fn default_data_dir() -> PathBuf {
    PathBuf::from("/var/lib/craftpanel")
}

fn default_helper_socket() -> PathBuf {
    PathBuf::from("/run/craftpanel/helper.sock")
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind: default_bind(),
            data_dir: default_data_dir(),
            helper_socket: default_helper_socket(),
            trusted_proxies: Vec::new(),
            java_search: Search::default(),
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("reading config from {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("parsing config at {}", path.display()))
    }

    pub fn database_path(&self) -> PathBuf {
        self.data_dir.join("panel.db")
    }

    pub fn users_dir(&self) -> PathBuf {
        self.data_dir.join("users")
    }

    pub fn cache_dir(&self) -> PathBuf {
        self.data_dir.join("cache")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_an_older_installer_wrote_still_opens() {
        let path = std::env::temp_dir()
            .join(format!("craftpanel-config-{}.toml", std::process::id()));
        std::fs::write(
            &path,
            "bind = \"0.0.0.0:9090\"\n\
             data_dir = \"/var/lib/craftpanel\"\n\
             helper_socket = \"/run/craftpanel/helper.sock\"\n\
             \n\
             [ports]\n\
             start = 25565\n\
             end = 25700\n",
        )
        .unwrap();

        let config = Config::load(&path);
        let _ = std::fs::remove_file(&path);
        let config = config.expect("a table the panel no longer knows is not an error");

        assert_eq!(config.bind.port(), 9090);
        assert_eq!(config.data_dir, PathBuf::from("/var/lib/craftpanel"));
        assert!(config.trusted_proxies.is_empty());
    }

    #[test]
    fn trusted_proxies_take_single_addresses_and_whole_networks() {
        let config: Config = toml::from_str(
            "trusted_proxies = [\"192.168.178.180\", \"10.0.0.0/24\", \"::1\"]",
        )
        .unwrap();

        let covered = |raw: &str| {
            let address: IpAddr = raw.parse().unwrap();
            config.trusted_proxies.iter().any(|proxy| proxy.covers(address))
        };
        assert!(covered("192.168.178.180"));
        assert!(covered("::ffff:192.168.178.180"));
        assert!(covered("10.0.0.1"));
        assert!(covered("::1"));
        assert!(!covered("192.168.178.181"));
        assert!(!covered("10.0.1.1"));
    }

    #[test]
    fn a_proxy_that_is_no_address_is_refused() {
        let refused = toml::from_str::<Config>("trusted_proxies = [\"proxy.local\"]");
        assert!(refused.unwrap_err().to_string().contains("proxy.local"));
    }
}
