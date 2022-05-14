use crate::eval::EvalError;
use futures::{future, FutureExt};
use std::{
    collections::HashMap,
    future::Future,
    hash::Hash,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::{sync::RwLock, task};

////////////////////////////////////////////////////////////////////////////////////////////////////
// TaskMap
////////////////////////////////////////////////////////////////////////////////////////////////////

//type TaskFuture = future::Shared<future::Map<>>

#[derive(Debug, Clone, thiserror::Error)]
pub enum TaskError {
    #[error("task was cancelled")]
    Cancelled,
    #[error("task panicked: `{0}`")]
    Panic(String),
}

/// A wrapper for task join futures that
struct TaskFuture<V>(task::JoinHandle<V>);

impl<V> Future for TaskFuture<V> {
    type Output = Result<V, TaskError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.0).poll(cx) {
            Poll::Ready(value) => Poll::Ready(value.map_err(|join_err| {
                if join_err.is_cancelled() {
                    TaskError::Cancelled
                } else {
                    TaskError::Panic(join_err.to_string())
                }
            })),
            Poll::Pending => Poll::Pending,
        }
    }
}

pub struct TaskMap<K, V> {
    tasks: RwLock<HashMap<K, future::Shared<TaskFuture<V>>>>,
}

impl<K, V> TaskMap<K, V> {
    pub fn new() -> TaskMap<K, V> {
        TaskMap {
            tasks: RwLock::new(HashMap::new()),
        }
    }
}

impl<K, V> TaskMap<K, V>
where
    K: Eq + Hash,
    V: Clone + Send + 'static,
{
    /// Returns the value with the key, or spawns
    pub async fn fetch_or_spawn<F>(&self, key: K, fut: F) -> Result<V, TaskError>
    where
        F: Future<Output = V> + Send + 'static,
    {
        {
            let mut tasks = self.tasks.read().await;
            if let Some(fut) = tasks.get(&key) {
                return fut.clone().await;
            }
        }

        let fut = TaskFuture(task::spawn(fut)).shared();
        self.tasks.write().await.insert(key, fut.clone());
        fut.await
    }

    /// Returns the value with the key, or spawns
    pub async fn fetch_or_spawn_blocking<F>(&self, key: K, f: F) -> Result<V, TaskError>
    where
        F: FnOnce() -> V + Send + 'static,
    {
        {
            let mut tasks = self.tasks.read().await;
            if let Some(fut) = tasks.get(&key) {
                return fut.clone().await;
            }
        }

        let fut = TaskFuture(task::spawn_blocking(f)).shared();
        self.tasks.write().await.insert(key, fut.clone());
        fut.await
    }
}
