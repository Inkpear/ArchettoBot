use actix_web::{web, HttpResponse, Responder};

pub async fn message_handler() -> impl Responder {
    HttpResponse::Ok()
}