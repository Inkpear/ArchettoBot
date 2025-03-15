use actix_web::web;
use crate::handler::*;

pub fn message_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/", web::post().to(message_handler));
}