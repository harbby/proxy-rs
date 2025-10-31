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
    #[serde(default = "default_vec_rule")]
    pub rule: Vec<Rule>,
    #[serde(default = "default_direct")]
    pub direct: HashMap<String, Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct Rule {
    pub select: Vec<u16>,
    #[serde(flatten)]
    pub other: HashMap<String, Vec<String>>,
}

static SERVER_LIST_CONFIG: LazyLock<Config> = LazyLock::new(|| {
    let config:Config = load_json(SERVER_LIST_FILE).expect("failed to load config");
    config
});

static CORE_CONFIG: LazyLock<CoreConfig> = LazyLock::new(|| {
    let config:CoreConfig = load_json(CORE_CONFIG_FILE).expect("failed to load config");
    let check_select = |index: &u16| {
        let conf = *&SERVER_LIST_CONFIG
            .list
            .get(*index as usize - 1)
            .ok_or_else(|| anyhow::anyhow!("Index {} out of bounds", *index))
            .expect("index out of bounds");
        if !conf.scheme.eq_ignore_ascii_case("trojan") {
            let _ = anyhow::anyhow!("server index check failed");
        }
        if conf.index != *index {
            let _ = anyhow::anyhow!("server index check failed");
        }
        return conf;
    };
    // check
    config.select.iter().for_each(|index| {
        let conf = check_select(index);
        LOG::info!("** Usage [{}] {}", *index, conf.name);
    });

    for rule in &config.rule {
        for index in &rule.select {
            let conf = check_select(index);
            for (k, _v) in rule.other.iter() {
                LOG::info!("[{}] usage[{}] {}", k, index, conf.name);
            }
        }
    }
    config
});

#[derive(Debug, Deserialize)]
pub struct Config {
    pub list: Vec<ServerInfo>,
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
    String::new()
}

fn default_direct() -> HashMap<String, Vec<String>> {
    HashMap::new()
}

fn default_vec_rule() -> Vec<Rule> {
    Vec::new()
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

pub fn get_server_list() -> &'static Config {
    &SERVER_LIST_CONFIG
}
