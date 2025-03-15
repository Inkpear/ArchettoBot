use crate::crawler_models::{Competition, CompetitionType};
use crate::http_services::HttpServices;
use crate::models::{Config, FuncScopeServices};
use crate::scheduled_task_models::Task;
use crate::scheduled_task_services::ScheduledTaskService;
use chrono::DateTime;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AppState {
    pub http_services: Arc<HttpServices>,
    pub scheduled_task_services: Arc<ScheduledTaskService>,
    pub competitions: Arc<RwLock<Vec<Competition>>>,
    pub func_scope_services: Arc<FuncScopeServices>,
}

impl AppState {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config = Config::from_path("./config.yaml").unwrap_or_else(|_| {
            let config = Config::new();
            let _ = config.save();
            config
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

        Ok(AppState {
            http_services,
            scheduled_task_services,
            competitions: Arc::new(RwLock::new(Vec::new())),
            func_scope_services: Arc::new(func_scope_services),
        })
    }

    pub async fn update_competitions(&self) -> Result<(), Box<dyn std::error::Error>> {
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
            self.func_scope_services
                .func_scope_pool
                .iter()
                .filter(|entry| entry.value().competition)
                .for_each(|entry| targets.push(entry.key().clone()));
            let task = Task::builder()
                .id(&competition.link)
                .target_time(utc_time)
                .task(async move {
                    for target in targets {
                        let message = target.new_message()
                            .at("all")
                            .text("\n")
                            .text(&competition.fmt_string());
                        let _ = http_services.send_message(message).await;
                    }
                })
                .build()
                .unwrap();
            let _ = self.scheduled_task_services.add_task(task).await;
        }
        Ok(())
    }
}

#[tokio::test]
pub async fn test_update_competition() {
    let app_state = AppState::load().unwrap();
    let res = app_state.update_competitions().await;

    assert!(res.is_ok(), "{:#?}", res.err());
}
