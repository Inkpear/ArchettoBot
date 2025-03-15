use crate::scheduled_task_models::Task;
use chrono::Utc;
use dashmap::DashMap;
use std::{future::Future, sync::Arc};
use thiserror::Error;
use tokio::spawn;
use tokio::task::JoinHandle;
use tokio::time::{sleep_until, Duration, Instant};

#[derive(Debug, Error)]
pub enum ScheduledTaskError {
    #[error("Task has been Expired!")]
    TaskExpiredError,
}

#[derive(Debug)]
pub struct ScheduledTaskService {
    task_pool: Arc<DashMap<String, JoinHandle<()>>>,
}

impl ScheduledTaskService {
    pub fn new() -> Self {
        ScheduledTaskService {
            task_pool: Arc::new(DashMap::new()),
        }
    }

    pub async fn add_task<T>(&self, task: Task<T>) -> Result<String, ScheduledTaskError>
    where
        T: Send + Future<Output = ()> + 'static,
    {
        if Utc::now().ge(&task.target_time) {
            return Err(ScheduledTaskError::TaskExpiredError);
        }
        let task_id = task.id;
        let target_time = task.target_time;
        let task = task.task;
        let task_pool = self.task_pool.clone();

        if let Some((_, handle)) = task_pool.remove(&task_id) {
            handle.abort();
        }

        let task_id = task_id.to_string();
        let handle = spawn({
            let task_id = task_id.clone();
            let task_pool = self.task_pool.clone();
            let duration = Duration::from_secs((target_time - Utc::now()).num_seconds() as u64);
            async move {
                sleep_until(Instant::now() + duration).await;
                task.await;
                task_pool.remove(&task_id);
            }
        });

        task_pool.insert(task_id.clone(), handle);

        Ok(task_id)
    }

    pub async fn abort_task(&self, task_id: &str) -> bool {
        let task_pool = self.task_pool.clone();
        if let Some((_, handle)) = task_pool.remove(task_id) {
            handle.abort();
            return true;
        }
        false
    }
}

#[tokio::test]
async fn test_task_services() {
    let services = ScheduledTaskService::new();
    let target_time = Utc::now() + Duration::from_secs(3);
    let task1 = Task::builder()
        .id("test1")
        .target_time(target_time.clone())
        .task(async move { println!("hello! here is task: test1") })
        .build();

    let task2 = Task::builder()
        .id("test2")
        .target_time(target_time.clone())
        .task(async move { println!("hello! here is task: test2") })
        .build();

    let _ = services.add_task(task1.unwrap()).await;
    let task2 = services.add_task(task2.unwrap()).await.unwrap();
    if services.abort_task(&task2).await {
        println!("task2 has been aborted!");
    }

    tokio::time::sleep(Duration::from_secs(4)).await;
}
