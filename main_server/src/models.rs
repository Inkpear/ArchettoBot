use serde::{Deserialize, Serialize};
use serde_yaml;
use std::fs;

#[derive(Deserialize, Serialize, Debug)]
pub struct Config {
    main_server_addr: (String, u32),
    crawler_server_addr: (String, u32),
    bot_server_addr: (String, u32),
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
        Ok(fs::write("./config.yaml", serde_yaml::to_string(self)?)?)
    }
}
