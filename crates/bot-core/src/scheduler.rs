use std::future::Future;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use tokio::task::JoinHandle;

pub struct Scheduler {
    tasks: Arc<DashMap<String, JoinHandle<()>>>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(DashMap::new()),
        }
    }

    /// One-shot task that runs at `fire_at` and then removes itself.
    pub fn once(
        &self,
        id: &str,
        fire_at: DateTime<Utc>,
        fut: impl Future<Output = ()> + Send + 'static,
    ) {
        let id_owned = id.to_owned();
        let tasks = Arc::clone(&self.tasks);

        let handle = tokio::spawn(async move {
            let now = Utc::now();
            let delay = (fire_at - now).num_milliseconds().max(0) as u64;
            tokio::time::sleep(std::time::Duration::from_millis(delay)).await;

            fut.await;

            tasks.remove(&id_owned);
        });

        self.tasks.insert(id.to_owned(), handle);
    }

    /// Periodic task that runs every `interval_secs`.
    /// The `factory` is called each interval to produce a fresh future.
    pub fn every<F, Fut>(&self, id: &str, interval_secs: u64, factory: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let interval = std::time::Duration::from_secs(interval_secs);

        let handle = tokio::spawn(async move {
            loop {
                let fut = factory();
                fut.await;
                tokio::time::sleep(interval).await;
            }
        });

        self.tasks.insert(id.to_owned(), handle);
    }

    /// Cancel and remove a task by id. Returns true if it existed.
    pub fn cancel(&self, id: &str) -> bool {
        if let Some((_, handle)) = self.tasks.remove(id) {
            handle.abort();
            true
        } else {
            false
        }
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}
