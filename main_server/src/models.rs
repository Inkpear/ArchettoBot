use serde::{Deserialize, Serialize};
use serde_yaml;
use std::fs;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    main_server_addr: (String, u16),
    crawler_server_addr: (String, u16),
    bot_server_addr: (String, u16),
}

impl Config {
    pub fn new() -> Self {
        Config {
            main_server_addr: ("localhost".to_string(), 8085),
            crawler_server_addr: ("localhost".to_string(), 8086),
            bot_server_addr: ("localhost".to_string(), 3000),
        }
    }

    pub fn from_path(path: &str) -> Result<Config, Box<dyn std::error::Error>> {
        let file = fs::read_to_string(path)?;
        Ok(serde_yaml::from_str(&file)?)
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(fs::write("../config.yaml", serde_yaml::to_string(self)?)?)
    }

    pub fn main_server_addr<'a>(&'a self) -> (&'a str, u16) {
        (&self.main_server_addr.0, self.main_server_addr.1)
    }

    pub fn bot_server_addr<'a>(&'a self) -> (&'a str, u16) {
        (&self.bot_server_addr.0, self.bot_server_addr.1)
    }

    pub fn crawler_server_addr<'a>(&'a self) -> (&'a str, u16) {
        (&self.crawler_server_addr.0, self.crawler_server_addr.1)
    }
}

#[test]
fn test_config() {
    let config = Config::new();
    let res = config.save();

    assert!(res.is_ok(), "{:#?}", res.err());
}