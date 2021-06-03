use std::any::type_name;
use std::sync::Arc;
use std::time::Duration;

use async_std::sync::Mutex;
use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use lru_time_cache::LruCache;
use tide::Request;

use crate::logging::log_anyhow;
use crate::wargaming::models::FullInfo;
use crate::web::State;

lazy_static! {
    static ref PLAYER_MODEL_CACHE: Mutex<LruCache<i32, Arc<PlayerViewModel>>> = Mutex::new(
        LruCache::with_expiry_duration_and_capacity(Duration::from_secs(60), 1000)
    );
}

pub type Percentage = f32;

pub struct PlayerViewModel {
    pub account_id: i32,
    pub nickname: String,
    pub created_at: DateTime<Utc>,
    pub last_battle_time: DateTime<Utc>,
    pub wins: Percentage,
    pub survival: Percentage,
    pub n_battles: i32,
}

impl PlayerViewModel {
    pub async fn new(request: Request<State>) -> crate::Result<Arc<Self>> {
        let account_id: i32 = Self::parse_account_id(&request)?;
        log::info!("{} #{}…", type_name::<Self>(), account_id);
        let state = request.state();
        let mut cache = PLAYER_MODEL_CACHE.lock().await;
        let model = match cache.get(&account_id) {
            Some(model) => {
                log::debug!("Player #{} model is cached.", account_id);
                model.clone()
            }
            None => {
                let full_info = Arc::new(state.api.get_full_info(account_id).await?);
                let model = Self::from_full_info(&full_info.clone());
                let database = state.database.clone();
                async_std::task::spawn(async move {
                    log_anyhow(database.upsert_full_info(&full_info).await);
                });
                cache.insert(account_id, model.clone());
                model
            }
        };
        Ok(model)
    }

    fn parse_account_id(request: &Request<State>) -> crate::Result<i32> {
        Ok(request
            .param("account_id")
            .map_err(surf::Error::into_inner)?
            .parse()?)
    }

    fn from_full_info(full_info: &FullInfo) -> Arc<Self> {
        let account_info = &full_info.account_info;
        let all = &account_info.statistics.all;
        Arc::new(Self {
            account_id: account_info.id,
            nickname: account_info.nickname.clone(),
            created_at: account_info.created_at,
            last_battle_time: account_info.last_battle_time,
            wins: 100.0 * (all.wins as f32) / (all.battles as f32),
            survival: 100.0 * (all.survived_battles as f32) / (all.battles as f32),
            n_battles: all.battles,
        })
    }
}
