use crate::settings;
use crate::settings::{ServerInfo};

fn is_match(rule: &str, target_addr: &str) -> bool {
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

fn is_direct(target_addr: &str) -> (bool, String) {
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

use std::sync::OnceLock;
static WORKDIR: OnceLock<Router> = OnceLock::new();

impl Router {
    pub fn new(size: u16) -> Self {
        let list = (0..size).map(|_| AtomicU16::new(0)).collect::<Vec<_>>();
        Router {
            server_list: list,
        }
    }

    fn select_index(&self, target_addr: &str) -> Option<u16> {
        let config = settings::get_config();
        for rule in &config.proxy {
            for (_k, v) in rule.other.iter() {
                if settings::is_mode_default_proxy() && rule.select.is_empty() {
                    continue;
                }

                for it in v.iter() {
                    if is_match(it, target_addr) {
                        let index:u16 = rule.select.get(0).map(|x| *x).unwrap_or(self.get_default_proxy_index());
                        return Some(index);
                    }
                }
            }
        }

        None
    }

    fn get_default_proxy_index(&self) -> u16 {
        let config = settings::get_config();
        let index = *config.select.get(0).expect("not found default_backend index: 0");
        index
    }

    fn get_info(index: u16) -> &'static ServerInfo {
        let info = settings::get_server_list()
            .list
            .get(index as usize - 1)
            .expect(format!("Index {index} out of bounds").as_str());
        assert_eq!(info.index, index, "index check failed");
        info
    }

    pub fn get_factory() -> &'static Router {
        WORKDIR.get_or_init(|| {
            let size = settings::get_config().proxy.len() + 1;
            Router::new(size as u16)
        })
    }

    fn get_server(&self, target_addr: &str) -> Option<&'static ServerInfo> {
        self.select_index(target_addr).map(Self::get_info)
    }

    pub fn do_route(&self, addr: &str) -> (String, Option<&'static ServerInfo>) {
        if settings::is_mode_default_proxy() {
            let (is_direct, label) = is_direct(addr);
            if is_direct {
                return (label, None)
            }
        }

        let router = Self::get_factory();
        if let Some(info) = router.get_server(addr) {
            return (String::new(), Some(info))
        }

        if settings::is_mode_default_proxy() {
            let index = self.get_default_proxy_index();
            (String::new(), Some(Self::get_info(index)))
        } else {
            // direct;
            ("default".to_string(), None)
        }
    }
}
