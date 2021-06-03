use std::any::type_name;
use std::fmt::Debug;
use std::future::Future;
use std::sync::Arc;

use async_std::sync::Mutex;
use lru_time_cache::LruCache;

/// [`LruCache`] proxy that automatically calls value getter when needed.
/// And is also thread-safe.
pub struct Cached<K: Ord + Clone + Debug, V>(Mutex<LruCache<K, Arc<V>>>);

impl<K: Ord + Clone + Debug, V> Cached<K, V> {
    pub fn new(cache: LruCache<K, Arc<V>>) -> Self {
        Self(Mutex::new(cache))
    }

    pub async fn get<G, Fut>(&self, key: &K, getter: G) -> crate::Result<Arc<V>>
    where
        G: FnOnce() -> Fut,
        Fut: Future<Output = crate::Result<V>>,
    {
        let mut cache = self.0.lock().await;
        let model = match cache.get(&key) {
            Some(model) => {
                log::debug!(r#"{} hit: {:?}"#, type_name::<Self>(), key);
                model.clone()
            }
            None => {
                let model = Arc::new(getter().await?);
                log::debug!(r#"{} insert: {:?}"#, type_name::<Self>(), key);
                cache.insert(key.clone(), model.clone());
                model
            }
        };
        Ok(model)
    }
}
