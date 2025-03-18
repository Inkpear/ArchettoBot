use actix_web::{web, App, HttpServer};
use chrono::{DateTime, Local};
use cq_models::MsgTarget;
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

    let config = app_state.config.read().await;
    let (ip, port) = (
        config.main_server_addr().0.to_string(),
        config.main_server_addr().1,
    );
    drop(config);
    spawn(check_date_update(app_state.clone()));

    let app = {
        move || {
            App::new()
                .app_data(app_state.clone())
                .configure(message_routes)
        }
    };
    HttpServer::new(app)
        .bind((ip.as_str(), port))?
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

pub async fn heart_beat(app_state: web::Data<AppState>) {
    use chrono::Utc;
    use tokio::time::{sleep, Duration};
    use scheduled_task_models::Task;
    let (duration, master) = {
        let config = app_state.config.read().await;
        (Duration::from_secs(config.heart_beat.1), config.master)
    };

    loop {
        let (heart_beat_status, _, heart_beat_text) = {
            app_state.config.read().await.heart_beat.clone()
        };
        
        if heart_beat_status {
            let msg = MsgTarget::new_private_message(
                master
            )
            .text(&heart_beat_text);
            let http_services = app_state.http_services.clone();
            let task = Task::builder()
                .id("bot_heart_beat")
                .target_time(Utc::now() + Duration::from_secs(1))
                .task(async move {
                    let res = http_services.send_message(msg).await;
                    if let Ok(status) = res {
                        if status {
                            info!("向[{}]:发送心跳成功!", master);
                        } else {
                            error!("发送心跳事件失败!");
                        }
                    } else {
                        error!("bot server 连接失败!");
                    }
                })
                .build()
                .unwrap();
                let _ = app_state.scheduled_task_services.add_task(task).await;
        }

        sleep(duration).await;
    }
}