use std::collections::HashMap;

pub struct Config {
    settings: HashMap<String, String>,
}

impl Config {
    pub fn new() -> Self {
        let mut settings = HashMap::new();
        settings.insert("timeout".to_string(), "30".to_string());
        settings.insert("retries".to_string(), "3".to_string());
        settings.insert("debug".to_string(), "false".to_string());

        Self { settings }
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.settings.get(key)
    }

    pub fn set(&mut self, key: String, value: String) {
        self.settings.insert(key, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_timeout() {
        let config = Config::new();
        assert_eq!(config.get("timeout"), Some(&"30".to_string()));
    }
}
