mod card_gen;
mod card_template;
mod command;
mod config;
mod db;
mod error;
mod handler;
mod schedule;
mod scheduler;
mod util;

use std::sync::Arc;

use log::{error, info};
use tokio::sync::RwLock;

use crate::card_gen::RenderManager;
use crate::config::Config;
use crate::db::DbPool;
use crate::scheduler::Scheduler;

pub struct AppState {
    pub config: Config,
    pub nap: napcat_sdk::NapClient,
    pub db: DbPool,
    pub scheduler: Scheduler,
    pub renderer: RenderManager,
    pub bot_qq: RwLock<Option<i64>>,
}

impl AppState {
    pub async fn bot_qq(&self) -> i64 {
        *self.bot_qq.read().await.as_ref().unwrap_or(&0)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--test-cards") {
        return card_template::generate_test_cards().await;
    }

    let config = Config::load("config.yaml")?;
    let db = DbPool::open("bot.db").await?;

    let addr = format!("{}:{}", config.ws.host, config.ws.port);
    let nap = napcat_sdk::NapClient::bind(&addr, &config.ws.access_token).await?;

    let state = Arc::new(AppState {
        config,
        nap,
        db,
        scheduler: Scheduler::new(),
        renderer: RenderManager::new(),
        bot_qq: RwLock::new(None),
    });

    handler::register_handlers(&state);
    schedule::spawn_scheduled_tasks(&state).await;

    // Initial competition fetch
    {
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            match crawler::get_all_competitions().await {
                Ok(list) => {
                    if let Err(e) = state.db.upsert_competitions(&list).await {
                        error!("Initial competition upsert failed: {}", e);
                    }
                    if let Err(e) = state.db.clean_expired().await {
                        error!("Initial clean_expired failed: {}", e);
                    }
                    info!("Initial competition fetch done, {} contests", list.len());
                }
                Err(e) => error!("Initial competition fetch failed: {}", e),
            }
        });
    }

    info!("ArchettoBot running, master={}", state.config.master);
    tokio::signal::ctrl_c().await?;
    info!("Shutting down...");
    drop(state);
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    Ok(())
}
