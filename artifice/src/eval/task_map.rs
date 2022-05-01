use futures::{future, FutureExt};
use std::{collections::HashMap, future::Future, hash::Hash};
use tokio::{sync::RwLock, task};

////////////////////////////////////////////////////////////////////////////////////////////////////
// TaskMap
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct TaskMap<K, V> {
    tasks: RwLock<HashMap<K, future::Shared<task::JoinHandle<V>>>>,
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
    pub async fn fetch_or_spawn<F>(&self, key: K, fut: F) -> V
    where
        F: Future<Output = V> + Send + 'static,
    {
        {
            let mut tasks = self.tasks.read().await;
            if let Some(fut) = tasks.get(&key) {
                return fut.clone().await;
            }
        }

        let fut = task::spawn(fut).shared();
        self.tasks.write().await.insert(key, fut.clone());
        fut.await
    }

    /// Returns the value with the key, or spawns
    pub async fn fetch_or_spawn_blocking<F>(&self, key: K, f: F) -> V
    where
        F: FnOnce() -> V + Send + 'static,
    {
        {
            let mut tasks = self.tasks.read().await;
            if let Some(fut) = tasks.get(&key) {
                return fut.clone().await;
            }
        }

        let fut = task::spawn_blocking(f).shared();
        self.tasks.write().await.insert(key, fut.clone());
        fut.await
    }
}
