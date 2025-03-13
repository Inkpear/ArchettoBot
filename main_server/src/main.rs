use std::io;
use actix_web::{App, web, HttpServer};


#[path="utils/http_services.rs"]
mod http_services;

mod models;
mod handler;
mod router;
mod state;

#[actix_web::main]
async fn main() -> io::Result<()> {
    
    HttpServer::new(|| {
        App::new()
    })
    .bind(("127.0.0.1", 8085))?
    .run()
    .await   
}