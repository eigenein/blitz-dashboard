use std::time::Duration;

use async_std::task::sleep;
use chrono::Utc;
use log::Level;

use crate::database::Database;
use crate::metrics::Stopwatch;
use crate::models::{AccountInfo, BasicAccountInfo, TankSnapshot};
use crate::wargaming::WargamingApi;

pub struct Crawler {
    pub api: WargamingApi,
    pub database: Database,
    pub once: bool,
}

impl Crawler {
    pub async fn run(&self) -> crate::Result {
        loop {
            self.crawl_batch().await?;
            if self.once {
                break;
            }

            // FIXME: https://github.com/eigenein/blitz-dashboard/issues/15.
            sleep(Duration::from_secs(1)).await;
        }

        Ok(())
    }

    async fn crawl_batch(&self) -> crate::Result {
        let _stopwatch = Stopwatch::new("Batch crawled").level(Level::Info);
        let stored_accounts = self.database.retrieve_oldest_accounts(100)?;
        let mut account_infos = self
            .api
            .get_account_info(stored_accounts.iter().map(|account| account.id))
            .await?;
        let mut n_snapshots: usize = 0;
        for stored_account in stored_accounts.iter() {
            let account_info = account_infos
                .remove(&stored_account.id.to_string())
                .flatten();
            n_snapshots += self.crawl_account(stored_account, account_info).await?;
        }
        log::info!(
            "Processed {} accounts, upserted {} tank snapshots.",
            stored_accounts.len(),
            n_snapshots,
        );
        Ok(())
    }

    async fn crawl_account(
        &self,
        stored_account: &BasicAccountInfo,
        account_info: Option<AccountInfo>,
    ) -> crate::Result<usize> {
        log::info!(
            "Account #{}, last crawled at {}.",
            stored_account.id,
            stored_account.crawled_at,
        );

        let tx = self.database.start_transaction()?;
        let n_snapshots = match account_info {
            Some(account_info) if !account_info.is_active() => {
                log::debug!("Nickname: {}.", account_info.nickname);
                log::warn!("The account is inactive. Deleting…");
                self.database.prune_account(stored_account.id)?;
                0
            }
            Some(mut account_info) => {
                log::debug!("Nickname: {}.", account_info.nickname);
                account_info.basic.crawled_at = Utc::now();
                self.database
                    .insert_account_or_replace(&account_info.basic)?;
                if account_info.basic.last_battle_time >= stored_account.crawled_at {
                    self.database.upsert_account_snapshot(&account_info)?;
                    let tank_snapshots = self.get_tank_snapshots(&stored_account).await?;
                    self.database.upsert_tank_snapshots(&tank_snapshots)?;
                    tank_snapshots.len()
                } else {
                    log::info!("No new battles.");
                    0
                }
            }
            None => {
                log::warn!("The account does not exist. Deleting…");
                self.database.prune_account(stored_account.id)?;
                0
            }
        };

        log::debug!("Committing…");
        tx.commit()?;
        Ok(n_snapshots)
    }

    async fn get_tank_snapshots(
        &self,
        account: &BasicAccountInfo,
    ) -> crate::Result<Vec<TankSnapshot>> {
        Ok(self
            .api
            .get_merged_tanks(account.id)
            .await?
            .into_iter()
            .filter(|snapshot| snapshot.last_battle_time >= account.crawled_at)
            .collect())
    }
}
