use actix_web::{web, App, HttpServer};
use chrono::Local;
use log::{info, error};
use router::message_routes;
use state::AppState;
use tokio::spawn;
use std::io;


#[path = "utils/http_services.rs"]
mod http_services;

#[path = "utils/scheduled_task_services.rs"]
mod scheduled_task_services;

#[path = "utils/cq_models.rs"]
mod cq_models;

#[path = "utils/crawler_models.rs"]
mod crawler_models;

#[path = "utils/scheduled_task_models.rs"]
mod scheduled_task_models;

mod handler;
mod models;
mod router;
mod state;

#[actix_web::main]
async fn main() -> io::Result<()> {
    let app_state = web::Data::new(AppState::load().unwrap());

    let config = app_state.config.clone();

    spawn(check_date_update(app_state.clone()));

    let app = {
        move || {
            App::new()
                .app_data(app_state.clone())
                .configure(message_routes)
        }
    };
    HttpServer::new(app)
        .bind(config.main_server_addr())?
        .run()
        .await
}

async fn check_date_update(app_state: web::Data<AppState>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use tokio::time::{sleep, Duration};

    let mut last_date = Local::now().date_naive();
    info!("开始初始化信息");
    let resp = app_state.update_competitions().await;
    if let Ok(_) = resp {
        info!("比赛信息初始化完成!");
    } else {
        error!("初始化失败! 发生错误{}", resp.err().unwrap())
    }

    loop {
        let now = Local::now();
        let next_day = (now + chrono::Duration::days(1))
            .date_naive()
            .and_hms_opt(2, 0, 0)
            .unwrap();
        let duration = (next_day - now.naive_local())
            .to_std()
            .unwrap_or(Duration::from_secs(60));

        sleep(duration).await;

        let current_date = Local::now().date_naive();
        if current_date != last_date {
            last_date = current_date;
            info!("开始执行周期任务");
            let resp = app_state.update_competitions().await;
            if let Ok(_) = resp {
                info!("比赛信息更新完成!");
            } else {
                error!("更新失败! 发生错误{}", resp.err().unwrap());
            }
        }
    }
}