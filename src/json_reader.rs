use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;
use anyhow::{Result};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub list: Vec<ServerInfo>,
    #[serde(default = "default_scheme")]
    pub subscription: String,
    pub select: Vec<u16>,
    #[serde(default = "default_scheme")]
    pub update_time: String,
}

#[derive(Debug, Deserialize)]
pub struct ServerInfo {
    pub scheme: String,
    pub port: u16,
    #[serde(default = "default_scheme")]
    pub query: String,
    pub host: String,
    pub name: String,
    pub key: String,
    #[serde(default = "default_scheme")]
    pub sni: String,
    pub index: u16,
}

// Custom default value function
fn default_scheme() -> String {
    "".to_string()
}

pub fn load_json<T: DeserializeOwned>(path: &str) -> Result<T> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let obj = serde_json::from_reader(reader)?;
    Ok(obj)
}
