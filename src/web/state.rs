use std::time::Duration;

use lru_time_cache::LruCache;

use crate::cached::Cached;
use crate::database::Database;
use crate::wargaming::WargamingApi;
use crate::web::index::model::IndexViewModel;
use crate::web::player::model::PlayerViewModel;

/// Web application global state.
#[derive(Clone)]
pub struct State {
    pub api: WargamingApi,
    pub database: Database,
    pub index_model_cache: Cached<String, IndexViewModel>,
    pub player_model_cache: Cached<i32, PlayerViewModel>,
}

impl State {
    pub fn new(api: WargamingApi, database: Database) -> Self {
        State {
            api,
            database,
            index_model_cache: Cached::new(LruCache::with_expiry_duration_and_capacity(
                Duration::from_secs(86400),
                1000,
            )),
            player_model_cache: Cached::new(LruCache::with_expiry_duration_and_capacity(
                Duration::from_secs(60),
                1000,
            )),
        }
    }
}
