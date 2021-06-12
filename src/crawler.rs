use async_std::task::sleep;
use chrono::{DateTime, Duration, Utc};

use crate::database::Database;
use crate::wargaming::WargamingApi;
use anyhow::anyhow;
use std::time::Instant;

const ACCOUNT_STALE_TIMEOUT_SECS: i64 = 60;

pub async fn run(api: WargamingApi, database: Database) -> crate::Result {
    loop {
        let account = database.get_oldest_account()?;
        let account = match account {
            None => {
                log::info!("No accounts in the database.");
                sleep(std::time::Duration::from_secs(60)).await;
                continue;
            }
            Some(account) => account,
        };
        let age = Utc::now() - Into::<DateTime<Utc>>::into(account.crawled_at);
        log::info!(
            "Selected account #{}, last crawled at {}.",
            account.id,
            account.crawled_at,
        );
        let sleep_duration = (Duration::seconds(ACCOUNT_STALE_TIMEOUT_SECS) - age).num_seconds();
        if sleep_duration > 0 {
            log::info!("Sleeping for {} secs…", sleep_duration);
            sleep(std::time::Duration::from_secs(
                sleep_duration.unsigned_abs(),
            ))
            .await;
        }

        let start_instant = Instant::now();

        let account_info = api
            .get_account_info(account.id)
            .await?
            .ok_or_else(|| anyhow!("account #{} not found", account.id))?;
        let tanks = api.get_merged_tanks(account.id).await?;

        let tx = database.transaction()?;
        database.upsert_account(&account_info.basic)?;
        database.upsert_account_snapshot(&account_info)?;
        database.upsert_tanks(&tanks)?;
        tx.commit()?;

        log::info!(
            "Account #{} crawled in {:?}.",
            account.id,
            Instant::now() - start_instant
        );
    }
}
