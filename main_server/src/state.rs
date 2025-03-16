use crate::crawler_models::{Competition, CompetitionType};
use crate::http_services::HttpServices;
use crate::models::{Config, FuncScopeServices, UserConfig};
use crate::scheduled_task_models::Task;
use crate::scheduled_task_services::ScheduledTaskService;
use chrono::DateTime;
use log::{error, info};
use std::process::exit;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::io::Write;
use env_logger::{fmt::Color, Builder, Env, Target};

pub struct AppState {
    pub config: Arc<Config>,
    pub http_services: Arc<HttpServices>,
    pub scheduled_task_services: Arc<ScheduledTaskService>,
    pub competitions: Arc<RwLock<Vec<Competition>>>,
    pub func_scope_services: Arc<FuncScopeServices>,
    pub user_config: Arc<RwLock<UserConfig>>,
}

impl AppState {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        Builder::from_env(Env::default().default_filter_or("debug"))
        .target(Target::Stdout)
        .format(|buf, record| {
            let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f %Z");
            let mut style = buf.style();

            let level_style = match record.level() {
                log::Level::Error => style.set_color(Color::Red),
                log::Level::Warn => style.set_color(Color::Yellow),
                log::Level::Info => style.set_color(Color::Green),
                log::Level::Debug => style.set_color(Color::Blue),
                log::Level::Trace => style.set_color(Color::Cyan),
            };

            writeln!(
                buf,
                "[{}] [{}] {} - {}",
                ts,
                level_style.value(record.level()),
                record.target(),
                record.args()
            )
        })
        .init();

        let config = Config::from_path("../config.yaml").unwrap_or_else(|_| {
            info!("未发现配置文件, 请配置config.yaml文件!");
            let _ = Config::new().save();
            exit(0)
        });

        let func_scope_services = FuncScopeServices::load().unwrap_or_else(|_| {
            let services = FuncScopeServices::new();
            let _ = services.save();
            services
        });

        let http_services = Arc::new(
            HttpServices::builder()
                .bot_server(config.bot_server_addr())
                .crawler_server(config.crawler_server_addr())
                .build()
                .unwrap(),
        );

        let scheduled_task_services = Arc::new(ScheduledTaskService::new());

        let user_config = UserConfig::load().unwrap_or_else(|_| {
            let res = UserConfig::new();
            let _ = res.save();
            res
        });

        info!("配置载入完毕!");

        Ok(AppState {
            http_services,
            scheduled_task_services,
            config: Arc::new(config),
            competitions: Arc::new(RwLock::new(Vec::new())),
            func_scope_services: Arc::new(func_scope_services),
            user_config: Arc::new(RwLock::new(user_config)),
        })
    }

    pub async fn update_competitions(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let competitions = self
            .http_services
            .get_competition_info(&CompetitionType::All)
            .await?;

        let mut cpt_pool = self.competitions.write().await;
        cpt_pool.clear();

        competitions
            .into_iter()
            .for_each(|competition| cpt_pool.push(competition));

        cpt_pool.sort();

        for competition in cpt_pool.iter().cloned() {
            let utc_time = DateTime::from_timestamp(competition.start_time, 0).unwrap();
            let mut targets = vec![];
            let http_services = self.http_services.clone();
            let name = competition.name.clone();
            let task_name = competition.name.clone();
            self.func_scope_services
                .func_scope_pool
                .iter()
                .filter(|entry| entry.value().competition)
                .for_each(|entry| targets.push(entry.key().clone()));
            let task = Task::builder()
                .id(&competition.link)
                .target_time(utc_time - chrono::Duration::hours(1))
                .task(async move {
                    for target in targets {
                        let message = target
                            .new_message()
                            .image(&competition.face())
                            .text("\n")
                            .at("all")
                            .text("\n")
                            .text(&competition.fmt_string());
                        match http_services.send_message(message).await {
                            Ok(_) => info!("比赛:{}\n通知完毕!", name),
                            Err(error) => error!("比赛:{} 通知失败!\n{}", name, error),
                        }
                    }
                })
                .build()
                .unwrap();
            match self.scheduled_task_services.add_task(task).await {
                Ok(_) => info!("比赛:{} 等待通知!", task_name),
                Err(error) => error!("比赛:{} 添加通知失败\n{}", task_name, error),
            }
        }
        Ok(())
    }

    pub async fn check_admin(&self, user_id: u64) -> bool {
        self.user_config.read().await.check_admin(user_id)
    }

    pub fn check_master(&self, user_id: u64) -> bool {
        self.config.master.eq(&user_id)
    }
}

#[tokio::test]
pub async fn test_update_competition() {
    let app_state = AppState::load().unwrap();
    let res = app_state.update_competitions().await;

    assert!(res.is_ok(), "{:#?}", res.err());

    tokio::time::sleep(tokio::time::Duration::from_secs(6)).await;
}
