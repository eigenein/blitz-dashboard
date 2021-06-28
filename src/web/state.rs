use std::sync::Arc;
use std::time::Duration;

use async_std::sync::Mutex;
use chrono::Utc;
use lru_time_cache::LruCache;

use crate::database::Database;
use crate::models::{TankType, Vehicle};
use crate::wargaming::WargamingApi;

/// Web application global state.
#[derive(Clone)]
pub struct State {
    pub api: WargamingApi,
    pub database: Arc<Mutex<Database>>,

    tankopedia_cache: Arc<Mutex<LruCache<i32, Arc<Vehicle>>>>,
}

impl State {
    pub fn new(api: WargamingApi, database: Database) -> Self {
        State {
            api,
            database: Arc::new(Mutex::new(database)),
            tankopedia_cache: Arc::new(Mutex::new(LruCache::with_expiry_duration(
                Duration::from_secs(86400),
            ))),
        }
    }

    /// Retrieves cached vehicle information.
    pub async fn get_vehicle(&self, tank_id: i32) -> crate::Result<Arc<Vehicle>> {
        let mut cache = self.tankopedia_cache.lock().await;
        match cache.get(&tank_id) {
            Some(vehicle) => Ok(vehicle.clone()),
            None => {
                let vehicle = match self.database.lock().await.retrieve_vehicle(tank_id)? {
                    Some(vehicle) => Arc::new(vehicle),
                    None => {
                        log::warn!("Tank #{} is not found in the tankopedia.", tank_id);
                        Arc::new(Vehicle {
                            tank_id,
                            name: format!("#{}", tank_id),
                            tier: 0,
                            is_premium: false,
                            type_: TankType::Unknown,
                            imported_at: Utc::now(),
                        })
                    }
                };
                cache.insert(tank_id, vehicle.clone());
                Ok(vehicle)
            }
        }
    }
}
