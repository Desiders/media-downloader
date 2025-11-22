use serde::Deserialize;
use std::{
    env::{self, VarError},
    fs, io,
    path::Path,
};
use thiserror::Error;

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

#[derive(Error, Debug)]
pub enum ParseError {
    #[error(transparent)]
    IO(#[from] io::Error),
    #[error(transparent)]
    Toml(#[from] toml::de::Error),
}

/// # Panics
///
/// Panics if the `CONFIG_PATH` environment variable is not valid UTF-8.
#[must_use]
pub fn get_path() -> Box<str> {
    let path = match env::var("CONFIG_PATH") {
        Ok(val) => val,
        Err(VarError::NotPresent) => String::from("config.toml"),
        Err(VarError::NotUnicode(_)) => {
            panic!("`CONFIG_PATH` env variable is not a valid UTF-8 string!");
        }
    };
    path.into_boxed_str()
}

#[allow(clippy::missing_errors_doc)]
pub fn parse_from_fs(path: impl AsRef<Path>) -> Result<Config, ParseError> {
    let raw = fs::read_to_string(path)?;
    let cfg = toml::from_str(&raw)?;
    Ok(cfg)
}
