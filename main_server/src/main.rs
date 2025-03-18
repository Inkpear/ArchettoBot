use actix_web::{web, App, HttpServer};
use cq_models::MsgTarget;
use log::{error, info};
use router::message_routes;
use state::AppState;
use std::io;
use tokio::spawn;

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

    let (ip, port) = {
        let config = app_state.config.read().await;
        (
            config.main_server_addr().0.to_string(),
            config.main_server_addr().1,
        )
    };

    spawn({
        let app_state = app_state.clone();
        async move {
            if let Err(e) = interval_task_update(app_state).await {
                error!("定时任务失败: {}", e);
            }
        }
    });

    spawn({
        let app_state = app_state.clone();
        async move {
            heart_beat(app_state).await;
        }
    });

    let app = {
        move || {
            App::new()
                .app_data(app_state.clone())
                .configure(message_routes)
        }
    };
    HttpServer::new(app).bind((ip.as_str(), port))?.run().await
}

async fn interval_task_update(
    app_state: web::Data<AppState>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use chrono::Local;
    use tokio::time::{sleep, Duration};

    info!("开始初始化比赛信息");
    app_state.update_competitions().await.map_err(|e| {
        error!("初始化失败: {}", e);
        e
    })?;
    info!("比赛信息初始化完成!");

    loop {
        let now = Local::now();
        let next_day = now.date_naive().succ_opt().unwrap_or(now.date_naive());
        let next_run_time = next_day.and_hms_opt(2, 0, 0).expect("无效时间设置");

        let duration = if next_run_time > now.naive_local() {
            (next_run_time - now.naive_local()).to_std()?
        } else {
            Duration::from_secs(0)
        };

        sleep(duration).await;

        info!("开始执行周期任务");
        match app_state.update_competitions().await {
            Ok(_) => info!("比赛信息更新完成!"),
            Err(e) => error!("更新失败: {}", e),
        }
    }
}

pub async fn heart_beat(app_state: web::Data<AppState>) {
    use chrono::Utc;
    use scheduled_task_models::Task;
    use tokio::time::{sleep, Duration};
    let (duration, master) = {
        let config = app_state.config.read().await;
        (Duration::from_secs(config.heart_beat.1), config.master)
    };

    loop {
        let (heart_beat_status, heart_beat_text) = {
            let config = app_state.config.read().await;
            (config.heart_beat.0, config.heart_beat.2.clone())
        };

        if heart_beat_status {
            let msg = MsgTarget::new_private_message(master).text(&heart_beat_text);
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
