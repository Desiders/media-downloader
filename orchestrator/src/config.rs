use serde::Deserialize;
use std::{
    env::{self, VarError},
    fs,
    path::Path,
};

#[derive(Deserialize, Clone, Debug)]
pub struct Server {
    pub host: Box<str>,
    pub port: u16,
}

#[derive(Deserialize, Clone, Debug)]
pub struct BlacklistedConfig {
    pub domains: Vec<String>,
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
pub struct Config {
    pub server: Server,
    pub logging: Logging,
    pub limits: Limits,
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
