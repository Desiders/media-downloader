use serde::Deserialize;
use std::{
    env::{self, VarError},
    fs,
    path::Path,
};

#[derive(Deserialize, Clone, Debug)]
pub struct Bot {
    pub token: Box<str>,
    pub src_url: Box<str>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Chat {
    pub receiver_chat_id: i64,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Blacklisted {
    pub domains: Vec<String>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Logging {
    pub dirs: Box<str>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Database {
    pub host: Box<str>,
    pub port: i16,
    pub user: Box<str>,
    pub password: Box<str>,
    pub database: Box<str>,
}

impl Database {
    pub fn get_postgres_url(&self) -> String {
        format!(
            "postgres://{user}:{password}@{host}:{port}/{database}",
            user = self.user,
            password = self.password,
            host = self.host,
            port = self.port,
            database = self.database,
        )
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct Limits {
    pub max_file_size: u32,
}

#[derive(Deserialize, Clone, Debug)]
pub struct TgBotApi {
    pub url: Box<str>,
    // pub api_id: Box<str>,
    // pub api_hash: Box<str>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    pub bot: Bot,
    pub chat: Chat,
    pub blacklisted: Blacklisted,
    pub logging: Logging,
    pub database: Database,
    pub limits: Limits,
    pub tg_bot_api: TgBotApi,
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
