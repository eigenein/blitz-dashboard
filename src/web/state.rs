use std::sync::Arc;
use std::time::Duration as StdDuration;

use async_std::sync::Mutex;
use chrono::Utc;
use moka::future::{Cache, CacheBuilder};

use crate::database::Database;
use crate::models::{Nation, TankType, Vehicle};
use crate::wargaming::WargamingApi;

/// Web application global state.
#[derive(Clone)]
pub struct State {
    pub api: WargamingApi,
    pub database: Arc<Mutex<Database>>,

    tankopedia_cache: Cache<i32, Arc<Vehicle>>,
}

impl State {
    pub fn new(api: WargamingApi, database: Database) -> Self {
        State {
            api,
            database: Arc::new(Mutex::new(database)),
            tankopedia_cache: CacheBuilder::new(1_000_000)
                .time_to_live(StdDuration::from_secs(86400))
                .build(),
        }
    }

    /// Retrieves cached vehicle information.
    pub async fn get_vehicle(&self, tank_id: i32) -> crate::Result<Arc<Vehicle>> {
        match self.tankopedia_cache.get(&tank_id) {
            Some(vehicle) => Ok(vehicle),
            None => {
                let vehicle = Arc::new(
                    self.database
                        .lock()
                        .await
                        .retrieve_vehicle(tank_id)?
                        .unwrap_or_else(|| Self::get_hardcoded_vehicle(tank_id)),
                );
                self.tankopedia_cache.insert(tank_id, vehicle.clone()).await;
                Ok(vehicle)
            }
        }
    }

    fn get_hardcoded_vehicle(tank_id: i32) -> Vehicle {
        match tank_id {
            23057 => Vehicle {
                tank_id,
                name: "Kunze Panzer".to_string(),
                tier: 7,
                is_premium: true,
                nation: Nation::Germany,
                type_: TankType::Light,
                imported_at: Utc::now(),
            },
            _ => Vehicle {
                tank_id,
                name: format!("#{}", tank_id),
                tier: 0,
                is_premium: false,
                type_: TankType::Other,
                imported_at: Utc::now(),
                nation: Nation::Other,
            },
        }
    }
}
