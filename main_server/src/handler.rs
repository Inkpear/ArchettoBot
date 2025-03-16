use actix_web::{web, HttpResponse, Responder};
use serde_json::Value;
use crate::{cq_models::MessageHandler, state::AppState};

pub async fn message_handler(
    mut message: web::Json<Value>,
    app_state: web::Data<AppState>,
) -> impl Responder {

    MessageHandler::handle(app_state, message.take()).await;

    HttpResponse::Ok()
}