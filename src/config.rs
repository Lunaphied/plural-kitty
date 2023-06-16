use std::fs::File;
use std::net::SocketAddr;
use std::path::PathBuf;

use matrix_sdk::ruma::OwnedUserId;
use once_cell::sync::Lazy;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub listen: SocketAddr,
    pub synapse: SynapseInfo,
    pub bot: BotInfo,
}

#[derive(Deserialize)]
pub struct BotInfo {
    pub user: OwnedUserId,
    homeserver_url: Option<String>,
    pub state_store: PathBuf,
    session_file: Option<PathBuf>,
    pub db: DbInfo,
}

impl BotInfo {
    pub fn homeserver_url(&self) -> String {
        match &self.homeserver_url {
            Some(url) => url.to_owned(),
            None => format!("https://{}", self.user.server_name().as_str()),
        }
    }

    pub fn session_file_path(&self) -> PathBuf {
        match &self.session_file {
            Some(path) => path.clone(),
            None => self.state_store.join("session.json"),
        }
    }
}

#[derive(Deserialize)]
pub struct SynapseInfo {
    pub host: String,
    pub db: DbInfo,
}

#[derive(Deserialize)]
pub struct DbInfo {
    user: String,
    password: String,
    host: String,
    database: String,
}

impl DbInfo {
    pub fn db_uri(&self) -> String {
        format!(
            "postgres://{}:{}@{}/{}",
            self.user, self.password, self.host, self.database
        )
    }
}

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    let config_path = match std::env::args().nth(1) {
        Some(p) => p,
        None => {
            eprintln!("usage: hydra <path to config file>");
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
