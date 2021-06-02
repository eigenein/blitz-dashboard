use async_std::task::sleep;
use chrono::{DateTime, Duration, Utc};

use crate::database::Database;
use crate::wargaming::WargamingApi;

const ACCOUNT_STALE_TIMEOUT_SECS: i64 = 60;

pub async fn run(api: WargamingApi, database: Database) -> crate::Result {
    loop {
        let account = database.get_oldest_account().await?;
        let account = match account {
            None => {
                log::info!("No accounts in the database.");
                sleep(std::time::Duration::from_secs(60)).await;
                continue;
            }
            Some(account) => account,
        };
        let age = Utc::now() - Into::<DateTime<Utc>>::into(account.updated_at);
        log::info!(
            "Selected account #{} updated at {}.",
            account.id,
            account.updated_at,
        );
        let sleep_duration = (Duration::seconds(ACCOUNT_STALE_TIMEOUT_SECS) - age).num_seconds();
        if sleep_duration > 0 {
            log::info!("Sleeping for {} secs…", sleep_duration);
            sleep(std::time::Duration::from_secs(
                sleep_duration.unsigned_abs(),
            ))
            .await;
        }
        let full_info = api.get_full_account_info(account.id).await?;
        database.upsert_full_info(&full_info).await?;
    }
}
