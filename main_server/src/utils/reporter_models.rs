use crate::cq_models::CqMessage;
use chrono::{DateTime, Utc};
use tokio::time::Duration;

pub struct ReportEvent {
    pub id: String,
    pub message: CqMessage,
    pub time: DateTime<Utc>,
}

impl ReportEvent {
    pub fn get_duration(&self) -> Option<Duration> {
        if self.time < Utc::now() {
            return None
        }
        Some(Duration::from_secs((self.time - Utc::now()).num_seconds() as u64))
    }
}

impl PartialEq for ReportEvent {
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id)
    }
}

