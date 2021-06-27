use std::time::{Duration, Instant};

use anyhow::anyhow;
use async_std::task::sleep;
use chrono::Utc;

use crate::database::Database;
use crate::wargaming::WargamingApi;

pub async fn run(api: WargamingApi, database: Database, once: bool) -> crate::Result {
    loop {
        let account = database
            .retrieve_oldest_account()?
            .ok_or_else(|| anyhow!("the database is empty"))?;
        log::info!(
            "Account #{}, last crawled at {}.",
            account.id,
            account.crawled_at,
        );

        let start_instant = Instant::now();
        let account_info = api.get_account_info(account.id).await?;
        let tx = database.start_transaction()?;
        match account_info {
            Some(account_info) if !account_info.is_active() => {
                log::warn!("The account is inactive. Deleting…");
                database.prune_account(account.id)?;
            }
            Some(mut account_info) => {
                account_info.basic.crawled_at = Utc::now();
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
                database.prune_account(account.id)?;
            }
        }
        tx.commit()?;
        log::info!("Elapsed: {:?}.", Instant::now() - start_instant);

        if once {
            break;
        }

        // FIXME: https://github.com/eigenein/blitz-dashboard/issues/15.
        sleep(Duration::from_secs(1)).await;
    }

    Ok(())
}
