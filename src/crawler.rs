use std::time::{Duration as StdDuration, Instant};

use anyhow::anyhow;
use async_std::task::sleep;
use chrono::{Duration, Utc};

use crate::database::Database;
use crate::wargaming::WargamingApi;

const ACCOUNT_STALE_TIMEOUT_SECS: i64 = 60;

pub async fn run(api: WargamingApi, database: Database) -> crate::Result {
    loop {
        let account = database
            .retrieve_oldest_account()?
            .ok_or_else(|| anyhow!("the database is empty"))?;
        log::info!(
            "Account #{}, last crawled at {}.",
            account.id,
            account.crawled_at,
        );
        let age = Utc::now() - account.crawled_at;
        let sleep_duration = (Duration::seconds(ACCOUNT_STALE_TIMEOUT_SECS) - age).num_seconds();
        if sleep_duration > 0 {
            log::info!("Sleeping for {} secs…", sleep_duration);
            sleep(StdDuration::from_secs(sleep_duration.unsigned_abs())).await;
        }

        let start_instant = Instant::now();
        let account_info = api.get_account_info(account.id).await?;
        let tx = database.start_transaction()?;
        match account_info {
            Some(account_info) => {
                database.upsert_account(&account_info.basic)?;
                if account_info.basic.last_battle_time > account.crawled_at {
                    database.upsert_account_snapshot(&account_info)?;
                    database.upsert_tanks(&api.get_merged_tanks(account.id).await?)?;
                } else {
                    log::info!("No new battles.");
                }
            }
            None => {
                log::warn!("The account does not exist. Deleting…");
                database.delete_account(account.id)?;
            }
        }
        tx.commit()?;
        log::info!("Elapsed: {:?}.", Instant::now() - start_instant);
    }
}
