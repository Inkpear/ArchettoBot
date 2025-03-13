use std::io;
use actix_web::{App, web, HttpServer};


#[path ="utils/cqhttp.rs"]
mod cqhttp;

mod models;
mod handler;
mod router;

#[actix_web::main]
async fn main() -> io::Result<()> {
    
    HttpServer::new(|| {
        App::new()
    })
    .bind(("127.0.0.1", 8085))?
    .run()
    .await   
}