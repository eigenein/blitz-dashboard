use std::any::type_name;
use std::fmt::Debug;
use std::future::Future;
use std::ops::Deref;
use std::sync::Arc;
use std::time::{Duration, Instant};

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

pub struct CachedScalar<T> {
    #[allow(clippy::type_complexity)]
    value: Arc<Mutex<Option<(Instant, Arc<T>)>>>,

    ttl: Duration,
}

impl<T> CachedScalar<T> {
    pub fn new(ttl: Duration) -> Self {
        Self {
            value: Arc::new(Mutex::new(None)),
            ttl,
        }
    }

    pub async fn get<G, Fut>(&self, getter: G) -> crate::Result<Arc<T>>
    where
        G: FnOnce() -> Fut,
        Fut: Future<Output = crate::Result<Arc<T>>>,
    {
        let mut guard = self.value.lock().await;
        let value = match guard.deref() {
            Some((expiry_time, value)) if &Instant::now() < expiry_time => value.clone(),
            _ => {
                let value = getter().await?;
                guard.replace((Instant::now() + self.ttl, value.clone()));
                value
            }
        };
        Ok(value)
    }
}

impl<T> Clone for CachedScalar<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            ttl: self.ttl,
        }
    }
}
