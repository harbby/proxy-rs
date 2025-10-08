pub fn is_no_proxy(target_addr: &str) -> bool {
    if target_addr == "127.0.0.1" || target_addr == "::1" || target_addr == "localhost" {
        return true;
    }

    use crate::settings;
    let config = settings::get_config();
    for rule in &config.no_proxy {
        if rule.contains('*') {
            let pattern = regex::escape(rule).replace(r"\*", ".*");
            let re = regex::Regex::new(&format!("^{}$", pattern)).unwrap();
            if re.is_match(target_addr) {
                return true;
            }
        } else if target_addr == rule {
            return true;
        }
    }

    false
}
