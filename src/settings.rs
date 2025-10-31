use std::cmp::PartialEq;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::fs::File;
use std::io::{BufReader, Read};
use std::sync::{LazyLock};
use anyhow::{Result};
use log as LOG;

static SERVER_LIST_FILE:&str = "trojan_servers.json";
static CORE_CONFIG_FILE:&str = "config.toml";

// Custom default value function
fn default_direct() -> HashMap<String, Vec<String>> { HashMap::new() }

#[derive(Debug, Deserialize)]
pub struct CoreConfig {
    pub socks_bind: String,
    pub http_bind: String,
    #[serde(default = "DefaultMode::default")]
    #[serde(rename = "default_mode")]  // config key name
    default: DefaultMode,
    pub select: Vec<u16>,
    #[serde(default = "Vec::new")]
    pub proxy: Vec<Rule>,
    #[serde(default = "default_direct")]
    pub direct: HashMap<String, Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct Rule {
    #[serde(default = "Vec::new")]
    pub select: Vec<u16>,
    #[serde(flatten)]
    pub other: HashMap<String, Vec<String>>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")] // auto match "proxy" or "direct"
pub enum DefaultMode {
    Proxy,
    Direct,
}
impl Default for DefaultMode {
    fn default() -> Self {
        DefaultMode::Proxy
    }
}

impl Display for DefaultMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DefaultMode::Proxy => {f.write_str("proxy")},
            DefaultMode::Direct => {f.write_str("direct")},
        }
    }
}

static SERVER_LIST_CONFIG: LazyLock<Config> = LazyLock::new(|| {
    let config:Config = load_json(SERVER_LIST_FILE).expect("failed to load config");
    config
});

static CORE_CONFIG: LazyLock<CoreConfig> = LazyLock::new(|| {
    let config:CoreConfig = load_toml(CORE_CONFIG_FILE).expect("failed to load config");
    LOG::info!("** Default working mode: {}.", config.default);

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

    for rule in &config.proxy {
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
    #[serde(default)]       // String::default() -> ""
    pub update_time: String,
}

#[derive(Debug, Deserialize)]
pub struct ServerInfo {
    pub scheme: String,
    pub port: u16,
    #[serde(default = "String::default")]  // String::default() -> ""
    pub query: String,
    pub host: String,
    pub name: String,
    pub key: String,
    #[serde(default = "String::default")]
    pub sni: String,
    pub index: u16,
}

pub fn load_toml<T: DeserializeOwned>(path: &str) -> Result<T> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let mut string = String::new();
    let _len = reader.read_to_string(&mut string)?;
    let obj = toml::from_str(string.as_str())?;
    Ok(obj)
}

pub fn load_json<T: DeserializeOwned>(path: &str) -> Result<T> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let obj = serde_json::from_reader(reader)?;
    Ok(obj)
}

pub fn get_config() -> &'static CoreConfig {
    &CORE_CONFIG
}

pub fn get_server_list() -> &'static Config {
    &SERVER_LIST_CONFIG
}

pub fn is_mode_default_proxy() -> bool {
    get_config().default == DefaultMode::Proxy
}