//! Async task queue with concurrency control.

use std::future::Future;
use std::sync::Arc;
use tokio::sync::Semaphore;

pub struct AsyncTaskQueue {
    semaphore: Arc<Semaphore>,
}

impl AsyncTaskQueue {
    pub fn new(concurrency: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(concurrency)),
        }
    }

    pub async fn add<T, F>(&self, task: F) -> T
    where
        F: Future<Output = T> + Send,
        T: Send,
    {
        let _permit = self.semaphore.acquire().await.unwrap();
        task.await
    }
}
