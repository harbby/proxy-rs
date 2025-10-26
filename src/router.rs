use crate::settings;
use crate::settings::{get_config, ServerInfo};

pub fn is_match(rule: &str, target_addr: &str) -> bool {
    if rule.contains('*') {
        let pattern = regex::escape(rule).replace(r"\*", ".*");
        let re = regex::Regex::new(&format!("^{}$", pattern)).unwrap();
        if re.is_match(target_addr) {
            return true;
        }
    } else if target_addr == rule {
        return true;
    }
    return false;
}

pub fn is_direct(target_addr: &str) -> (bool, String) {
    if target_addr == "127.0.0.1" || target_addr == "::1" || target_addr == "localhost" {
        return (true, "local".to_string());
    }

    let config = settings::get_config();
    for (label, rules) in &config.direct {
        for rule in rules {
            if is_match(rule, target_addr) {
                return (true, label.to_string());
            }
        }
    }

    (false, String::new())
}

use std::sync::atomic::{AtomicU16};
pub struct Router {
    server_list: Vec<AtomicU16>,
}

impl Router {
    pub fn new(size: u16) -> Self {
        let list = (0..size).map(|_| AtomicU16::new(0)).collect::<Vec<_>>();
        Router {
            server_list: list,
        }
    }

    fn select_index(&self, target_addr: &str) -> u16 {
        let config = get_config();
        for rule in &config.rule {
            for (_k, v) in rule.other.iter() {
                for it in v.iter() {
                    if is_match(it, target_addr) {
                        return rule.select[0];
                    }
                }
            }
        }
        *config.select.get(0).expect("not found default_backend index: 0")
    }

    pub fn get_server(&self, target_addr: &str) -> &'static ServerInfo {
        let index = self.select_index(target_addr);
        let info = settings::get_server_list()
            .list
            .get(index as usize - 1)
            .expect(format!("Index {index} out of bounds").as_str());
        assert_eq!(info.index, index, "index check failed");
        info
    }
}

use std::sync::OnceLock;
static WORKDIR: OnceLock<Router> = OnceLock::new();

pub fn get_or_router() -> &'static Router {
    WORKDIR.get_or_init(|| {
        let size = get_config().rule.len() + 1;
        Router::new(size as u16)
    })
}
