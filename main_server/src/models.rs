use chrono::{DateTime, FixedOffset, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_yaml;
use std::collections::{HashMap, HashSet};
use std::fs;

use crate::cq_models::MsgTarget;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    main_server_addr: (String, u16),
    crawler_server_addr: (String, u16),
    bot_server_addr: (String, u16),
    pub master: u64,
    bot_id: u64,
    pub heart_beat: (bool, u64, String),
}

impl Config {
    pub fn new() -> Self {
        Config {
            main_server_addr: ("localhost".to_string(), 8085),
            crawler_server_addr: ("localhost".to_string(), 8086),
            bot_server_addr: ("localhost".to_string(), 3000),
            master: 123456789,
            bot_id: 123456789,
            heart_beat: (false, 3600 * 12, "bot is running".to_string()),
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
pub struct UserConfig {
    admin: HashSet<u64>,
}

impl UserConfig {
    pub fn new() -> Self {
        Self {
            admin: HashSet::new(),
        }
    }

    pub fn check_admin(&self, user_id: u64) -> bool {
        self.admin.contains(&user_id)
    }

    pub fn add_admin(&mut self, user_id: u64) -> bool {
        let res = self.admin.insert(user_id);
        let _ = self.save();
        res
    }

    pub fn delet_admin(&mut self, user_id: u64) -> bool {
        let res = self.admin.remove(&user_id);
        let _ = self.save();
        res
    }

    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let file = fs::read_to_string("../user_config.yaml")?;
        let set = serde_yaml::from_str::<HashSet<u64>>(&file)?;
        Ok(Self { admin: set })
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let _ = fs::write("../user_config.yaml", serde_yaml::to_string(&self.admin)?);

        Ok(())
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FuncScope {
    pub bili_parse: bool,
    pub competition: bool,
    pub group_increase_welcome: bool,
}

impl FuncScope {
    pub fn new() -> Self {
        Self {
            competition: false,
            bili_parse: false,
            group_increase_welcome: false,
        }
    }
}
pub struct FuncScopeServices {
    pub func_scope_pool: DashMap<MsgTarget, FuncScope>,
}

impl FuncScopeServices {
    pub fn new() -> Self {
        Self {
            func_scope_pool: DashMap::new(),
        }
    }

    pub fn set_scope(&self, func: &str, status: bool, target: &MsgTarget) {
        match func {
            "bili_parse" => self.func_scope_pool.get_mut(target).unwrap().bili_parse = status,
            "competition" => self.func_scope_pool.get_mut(target).unwrap().competition = status,
            "group_increase_welcome" => {
                self.func_scope_pool
                    .get_mut(target)
                    .unwrap()
                    .group_increase_welcome = status
            }
            _ => (),
        }
        let _ = self.save();
    }

    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let res = Self::new();
        let file = fs::read_to_string("../func_scope.yaml")?;
        let _ = serde_yaml::from_str::<HashMap<MsgTarget, FuncScope>>(&file)?
            .into_iter()
            .for_each(|(k, v)| {
                res.func_scope_pool.insert(k, v);
            });
        Ok(res)
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut map = HashMap::new();
        let _ = self.func_scope_pool.iter().for_each(|entry| {
            map.insert(entry.key().clone(), entry.value().clone());
        });
        let _ = fs::write("../func_scope.yaml", serde_yaml::to_string(&map)?);

        Ok(())
    }

    pub fn contains(&self, taget: &MsgTarget) -> bool {
        self.func_scope_pool.contains_key(taget)
    }

    pub fn insert(&self, key: MsgTarget, value: FuncScope) {
        self.func_scope_pool.insert(key, value);
    }

    pub fn get_value(&self, key: &MsgTarget) -> FuncScope {
        self.func_scope_pool.get(key).unwrap().clone()
    }
}

pub struct TimeConverter;

impl TimeConverter {
    pub fn from_utc_to_utc8(utc_time: &DateTime<Utc>) -> DateTime<FixedOffset> {
        utc_time.with_timezone(&FixedOffset::east_opt(8 * 3600).unwrap())
    }
}

#[derive(Serialize, Debug, Deserialize)]
pub struct GroupData {
    data: HashMap<u64, String>,
}

impl GroupData {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn get_welcome_message(&self, group_id: u64) -> Option<&String> {
        self.data.get(&group_id)
    }

    pub fn set_welcome_message(&mut self, group_id: u64, msg: &str) {
        self.data.insert(group_id, msg.to_string());
        let _ = self.save();
    }

    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let file = fs::read_to_string("../group_data.yaml")?;
        let map = serde_yaml::from_str::<HashMap<u64, String>>(&file)?;

        Ok(Self { data: map })
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let _ = fs::write("../group_data.yaml", serde_yaml::to_string(&self.data)?);

        Ok(())
    }
}

#[test]
fn test_config() {
    let config = Config::new();
    let res = config.save();

    assert!(res.is_ok(), "{:#?}", res.err());
}
