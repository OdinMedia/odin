use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use failure::Error;
use std::net::IpAddr;

#[derive(Deserialize, Clone)]
pub struct Config {

    #[cfg(feature = "web-api")]
    pub http: Option<http_frontend::HttpConfig>,
    #[cfg(feature = "local-files")]
    pub local: Option<local_provider::LocalProvider>,
    pub library: Option<LibraryConfig>,
    #[serde(rename = "player", default = "default_backend")]
    pub players: Vec<PlayerBackendConfig>,
    #[serde(default)]
    pub extensions: ExtensionConfig,
}

impl Default for Config {
    fn default() -> Self {
        Config {

            #[cfg(feature = "web-api")]
            http: Some(http_frontend::HttpConfig::default()),
            #[cfg(feature = "local-files")]
            local: local_provider::LocalProvider::default(),
            library: Some(LibraryConfig::default()),
            players: default_backend(),
            extensions: ExtensionConfig::default(),
        }
    }
}

#[derive(Deserialize, Clone)]
#[serde(tag = "store", rename_all = "lowercase")]
pub enum LibraryConfig {
    Memory,
    #[cfg(feature = "sqlite-store")]
    SQLite {
        path: String,
    },
}

impl Default for LibraryConfig {
    fn default() -> Self {
        LibraryConfig::Memory
    }
}

#[derive(Deserialize, Clone, PartialEq, Eq)]
pub struct PlayerBackendConfig {
    pub name: String,
    #[serde(default)]
    pub default: bool,
    #[serde(flatten)]
    pub backend_type: PlayerBackend,
}

#[derive(Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum PlayerBackend {
    #[cfg(feature = "gstreamer")]
    GStreamer,
}

#[derive(Deserialize, Clone, Serialize, Default)]
pub struct ExtensionConfig {
    pub path: Option<String>,
}

fn default_backend() -> Vec<PlayerBackendConfig> {
    #[cfg(feature = "gstreamer")]
    let backend_type = PlayerBackend::GStreamer;
    #[cfg(feature = "rodio")]
    let backend_type = PlayerBackend::Rodio;
    let config = PlayerBackendConfig {
        name: "default".to_string(),
        default: true,
        backend_type,
    };

    vec![config]
}

pub fn read_config(path: &Path) -> Result<Config, Error> {
    if path.exists() {
        let mut config_file = File::open(path)?;
        let mut config = String::new();
        config_file.read_to_string(&mut config)?;
        let config = toml::from_str(config.as_str())?;
        Ok(config)
    } else {
        Ok(Config::default())
    }
}
