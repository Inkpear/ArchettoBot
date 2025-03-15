use crate::models::Config;
use actix_web::{web, App, HttpServer};
use http_services::HttpServices;
use std::io;

#[path = "utils/http_services.rs"]
mod http_services;

#[path = "utils/scheduled_task_services.rs"]
mod scheduled_task_services;

#[path = "utils/cq_models.rs"]
mod cq_models;

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

    let http_services = web::Data::new(
        HttpServices::builder()
            .bot_server(config.bot_server_addr())
            .crawler_server(config.crawler_server_addr())
            .build()
            .unwrap(),
    );

    let app = { move || App::new().app_data(http_services.clone()) };

    HttpServer::new(app)
        .bind(config.main_server_addr())?
        .run()
        .await
}
