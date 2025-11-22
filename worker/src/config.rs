use serde::Deserialize;
use std::{
    env::{self, VarError},
    fmt::Display,
    fs,
    path::Path,
};

#[derive(Clone, Debug)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    pub fn from_env(key: &str) -> Result<Self, anyhow::Error> {
        let version = env::var(key)?;
        let mut parts = version.split('.');
        let major = parts.next().expect("major unset").parse()?;
        let minor = parts.next().expect("minor unset").parse()?;
        let patch = parts.next().expect("patch unset").parse()?;

        Ok(Self {
            major,
            minor,
            patch,
        })
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct Server {
    pub host: Box<str>,
    pub port: u16,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Logging {
    pub dirs: Box<str>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Limits {
    pub max_file_size: u32,
}

#[derive(Deserialize, Clone, Debug)]
pub struct YtDlp {
    pub executable_path: Box<str>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct YtToolkit {
    pub url: Box<str>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct YtPotProvider {
    pub url: Box<str>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    pub server: Server,
    pub logging: Logging,
    pub limits: Limits,
    pub yt_dlp: YtDlp,
    pub yt_toolkit: YtToolkit,
    pub yt_pot_provider: YtPotProvider,
}

impl Config {
    pub fn from_fs(path: impl AsRef<Path>) -> Result<Config, anyhow::Error> {
        let raw = fs::read_to_string(path)?;
        let cfg = toml::from_str(&raw)?;
        Ok(cfg)
    }
}

/// # Panics
///
/// Panics if the `CONFIG_PATH` environment variable is not valid UTF-8.
#[must_use]
pub fn get_config_path() -> Box<str> {
    let path = match env::var("CONFIG_PATH") {
        Ok(val) => val,
        Err(VarError::NotPresent) => String::from("config.toml"),
        Err(VarError::NotUnicode(_)) => {
            panic!("`CONFIG_PATH` env variable is not a valid UTF-8 string!");
        }
    };
    path.into_boxed_str()
}
