use crate::models::Config;
use actix_web::{web, App, HttpServer};
use http_services::HttpServices;
use models::FuncScopeServices;
use scheduled_task_services::ScheduledTaskService;
use state::AppState;
use std::collections::BinaryHeap;
use std::io;
use std::sync::Arc;
use tokio::sync::RwLock;

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
    let config = Config::from_path("./config.yaml").unwrap_or_else(|_| {
        let config = Config::new();
        let _ = config.save();
        
        config
    });

    let app_state = web::Data::new(AppState::load().unwrap());

    let app = { move || App::new().app_data(app_state.clone()) };

    HttpServer::new(app)
        .bind(config.main_server_addr())?
        .run()
        .await
}
