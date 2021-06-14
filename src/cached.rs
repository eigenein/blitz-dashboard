use std::any::type_name;
use std::fmt::Debug;
use std::future::Future;
use std::sync::Arc;

use async_std::sync::Mutex;
use lru_time_cache::LruCache;

pub struct Cached<K, V>(Arc<Mutex<LruCache<K, Arc<V>>>>);

impl<K: Ord + Clone + Debug, V> Cached<K, V> {
    pub fn new(cache: LruCache<K, Arc<V>>) -> Self {
        Self(Arc::new(Mutex::new(cache)))
    }

    pub async fn get<G, Fut>(&self, key: &K, getter: G) -> crate::Result<Arc<V>>
    where
        G: FnOnce() -> Fut,
        Fut: Future<Output = crate::Result<Arc<V>>>,
    {
        let mut cache = self.0.lock().await;
        let model = match cache.get(&key) {
            Some(model) => {
                log::debug!(r#"Hit: {:?} => {:?}"#, key, type_name::<V>());
                model.clone()
            }
            None => {
                let value = getter().await?;
                log::debug!(r#"Insert: {:?} => {:?}"#, key, type_name::<V>());
                cache.insert(key.clone(), value.clone());
                value
            }
        };
        Ok(model)
    }
}

impl<K, V> Clone for Cached<K, V> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
