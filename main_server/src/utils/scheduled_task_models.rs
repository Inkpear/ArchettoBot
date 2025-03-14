use std::future::Future;

use crate::http_services::BuilderError;
use chrono::{DateTime, Utc};

pub struct Task<T> {
    pub id: String,
    pub task: T,
    pub target_time: DateTime<Utc>,
}

impl<T> Task<T>
where
    T: Send + Future<Output = ()> + 'static,
{
    pub fn builder() -> TaskBuilder<T> {
        TaskBuilder::new()
    }
}

pub struct TaskBuilder<T> {
    id: Option<String>,
    task: Option<T>,
    target_time: Option<DateTime<Utc>>,
}

impl<T> TaskBuilder<T>
where
    T: Send + Future<Output = ()> + 'static,
{
    pub fn new() -> Self {
        TaskBuilder {
            id: None,
            task: None,
            target_time: None,
        }
    }

    pub fn task(mut self, task: T) -> Self {
        self.task = Some(task);
        self
    }

    pub fn id(mut self, id: &str) -> Self {
        self.id = Some(id.to_string());
        self
    }

    pub fn target_time(mut self, target_time: DateTime<Utc>) -> Self {
        self.target_time = Some(target_time);
        self
    }

    pub fn build(self) -> Result<Task<T>, BuilderError> {
        let id = self.id.ok_or(BuilderError::MissingParameter("id"))?;

        let task = self.task.ok_or(BuilderError::MissingParameter("task"))?;

        let target_time = self
            .target_time
            .ok_or(BuilderError::MissingParameter("target_time"))?;

        Ok(Task {
            id,
            task,
            target_time,
        })
    }
}
