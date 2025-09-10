use std::collections::HashMap;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;
use std::sync::{LazyLock};
use anyhow::{Result};
use tracing as LOG;

static SERVER_LIST_FILE:&str = "trojan_servers.json";
static CORE_CONFIG_FILE:&str = "config.json";

#[derive(Debug, Deserialize)]
pub struct CoreConfig {
    pub socks_bind: String,
    pub http_bind: String,
    pub select: Vec<u16>,
    pub special_domains: HashMap<String, u16>,
    pub default_backend: u16,
}

static SERVER_LIST_CONFIG: LazyLock<Config> = LazyLock::new(|| {
    let config:Config = load_json(SERVER_LIST_FILE).expect("failed to load config");
    config
});

static CORE_CONFIG: LazyLock<CoreConfig> = LazyLock::new(|| {
    let config:CoreConfig = load_json(CORE_CONFIG_FILE).expect("failed to load config");
    // check
    for index in &config.select {
        let conf = &SERVER_LIST_CONFIG
            .list
            .get(*index as usize - 1)
            .ok_or_else(|| anyhow::anyhow!("Index {} out of bounds", *index))
            .expect("index out of bounds");
        if !conf.scheme.eq_ignore_ascii_case("trojan") {
            anyhow::anyhow!("server index check failed");
        }
        if conf.index != *index {
            anyhow::anyhow!("server index check failed");
        }
        LOG::info!("** Usage trojan server[{}] {} **", *index, conf.name);
    }
    config
});

#[derive(Debug, Deserialize)]
struct Config {
    pub list: Vec<ServerInfo>,
    #[serde(default = "default_scheme")]
    pub subscription: String,
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

pub fn get_config() -> &'static CoreConfig {
    &CORE_CONFIG
}

fn select_index(target_addr: &str) -> u16 {
    let config = get_config();
    let index = config.special_domains
        .iter()
        .find(|(domain, _)| target_addr.ends_with(*domain))
        .map(|(_, idx)| *idx)
        .unwrap_or(config.default_backend);
    *config.select.get(index as usize).expect("")
}

pub fn get_trojan_server(target_addr: &str) -> Result<&'static ServerInfo> {
    let index: u16 = select_index(target_addr);
    let info = &SERVER_LIST_CONFIG
        .list
        .get(index as usize - 1)
        .ok_or_else(|| anyhow::anyhow!("Index {} out of bounds", index))?;
    Ok(info)
}
