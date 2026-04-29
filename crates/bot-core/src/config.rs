use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub ws: WsConfig,
    pub master: i64,
    #[serde(default)]
    pub heart_beat: HeartBeatConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WsConfig {
    pub host: String,
    pub port: u16,
    pub access_token: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HeartBeatConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_heartbeat_interval")]
    pub interval_secs: u64,
    #[serde(default = "default_heartbeat_message")]
    pub message: String,
}

fn default_heartbeat_interval() -> u64 {
    43200
}

fn default_heartbeat_message() -> String {
    "bot is running".into()
}

impl Default for HeartBeatConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_secs: default_heartbeat_interval(),
            message: default_heartbeat_message(),
        }
    }
}

impl Config {
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = serde_yaml::from_str(&content)?;
        Ok(config)
    }
}
