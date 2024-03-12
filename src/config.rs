use std::fs::File;
use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::Context;
use matrix_sdk::ruma::OwnedMxcUri;
use matrix_sdk::ruma::OwnedUserId;
use once_cell::sync::Lazy;
use serde::Deserialize;
use sqlx::postgres::PgConnectOptions;

#[derive(Deserialize)]
pub struct Config {
    pub listen: SocketAddr,
    pub dendrite: DendriteInfo,
    pub bot: BotInfo,
}

#[derive(Deserialize)]
pub struct BotInfo {
    pub user: OwnedUserId,
    homeserver_url: Option<String>,
    pub state_store: PathBuf,
    pub secret_file: Option<PathBuf>,
    pub password_file: Option<PathBuf>,
    pub db: DbInfo,
    pub display_name: Option<String>,
    pub avatar: Option<OwnedMxcUri>,
}

impl BotInfo {
    pub fn homeserver_url(&self) -> String {
        match &self.homeserver_url {
            Some(url) => url.to_owned(),
            None => format!("https://{}", self.user.server_name().as_str()),
        }
    }

    pub fn session_file_path(&self) -> PathBuf {
        match &self.secret_file {
            Some(path) => path.clone(),
            None => self.state_store.join("session.json"),
        }
    }
}

#[derive(Deserialize)]
pub struct DendriteInfo {
    pub host: String,
    pub db: DbInfo,
}

#[derive(Deserialize)]
pub struct DbInfo {
    user: String,
    password: Option<String>,
    password_file: Option<PathBuf>,
    host: String,
    database: String,
}

impl DbInfo {
    pub async fn db_con_opts(&self) -> anyhow::Result<PgConnectOptions> {
        let opts = PgConnectOptions::new()
            .host(&self.host)
            .database(&self.database)
            .username(&self.user);
        match (&self.password, &self.password_file) {
            (Some(pass), _) => Ok(opts.password(pass)),
            (None, Some(path)) => {
                let pass = tokio::fs::read_to_string(path)
                    .await
                    .context("Error reading password_file")?;
                Ok(opts.password(&pass))
            }
            _ => Ok(opts),
        }
    }
}

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    let config_path = match std::env::args().nth(1) {
        Some(p) => p,
        None => {
            eprintln!("usage: plural-kitty <path to config file>");
            std::process::exit(1);
        }
    };
    let file = match File::open(config_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Error opening config file: {e:#}");
            std::process::exit(2);
        }
    };
    match serde_yaml::from_reader(file) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Error opening config file: {e:#}");
            std::process::exit(2);
        }
    }
});
