use chrono::{DateTime, FixedOffset, Utc};
use serde::{Deserialize, Serialize};
use serde_yaml;
use std::fs;
use std::collections::HashMap;
use dashmap::DashMap;

use crate::cq_models::MsgTarget;

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

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FuncScope {
    pub competition: bool,
}

impl FuncScope {
    pub fn new() -> Self {
        Self {
            competition: false
        }
    }
}
pub struct FuncScopeServices {
    pub func_scope_pool: DashMap<MsgTarget, FuncScope>,
}

impl FuncScopeServices {
    pub fn new() -> Self {
        Self {
            func_scope_pool: DashMap::new()
        }
    }

    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let res = Self::new();
        let file = fs::read_to_string("../func_scope.yaml")?;
        let _ = serde_yaml::from_str::<HashMap<MsgTarget, FuncScope>>(&file)?
            .into_iter()
            .for_each(|(k, v)| { res.func_scope_pool.insert(k, v); });
        Ok(res)
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut map = HashMap::new();
        let _ = self.func_scope_pool
            .iter()
            .for_each(|entry| { map.insert(entry.key().clone(), entry.value().clone()); });
        let _ = fs::write("../func_scope.yaml", serde_yaml::to_string(&map)?);
        
        Ok(())
    }

    pub fn new_scope(&self, target: MsgTarget) -> Option<FuncScope> {
        self.func_scope_pool.insert(target, FuncScope::new())
    }
}

pub struct TimeConverter;

impl TimeConverter {
    pub fn from_utc_to_utc8(utc_time: &DateTime<Utc>) -> DateTime<FixedOffset> {
        utc_time.with_timezone(&FixedOffset::east_opt(8 * 3600).unwrap())
    }
}

#[test]
fn test_config() {
    let config = Config::new();
    let res = config.save();

    assert!(res.is_ok(), "{:#?}", res.err());
}