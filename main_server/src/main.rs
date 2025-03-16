use actix_web::{web, App, HttpServer};
use router::message_routes;
use state::AppState;
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
