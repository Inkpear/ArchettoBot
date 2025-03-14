use tokio::sync::{RwLock, Mutex};
use std::sync::Arc;
use crate::http_services::HttpServices;

pub struct AppState {
    http_service: Arc<HttpServices>,
}