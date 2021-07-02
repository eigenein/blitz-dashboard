use std::time::Duration as StdDuration;

use async_std::task::sleep;
use chrono::Utc;
use log::Level;

use crate::database::Database;
use crate::metrics::Stopwatch;
use crate::models::{AccountInfo, BasicAccountInfo};
use crate::opts::CrawlerOpts;
use crate::wargaming::WargamingApi;

pub struct Crawler {
    pub api: WargamingApi,
    pub database: Database,
    pub opts: CrawlerOpts,
}

impl Crawler {
    pub async fn run(&self) -> crate::Result {
        loop {
            self.crawl_batch().await?;
            if self.opts.once {
                break;
            }

            // FIXME: https://github.com/eigenein/blitz-dashboard/issues/15.
            sleep(StdDuration::from_secs(1)).await;
        }

        Ok(())
    }

    async fn crawl_batch(&self) -> crate::Result {
        let _stopwatch = Stopwatch::new("Batch crawled").level(Level::Info);
        let previous_infos = self.database.retrieve_oldest_accounts(100)?;
        let mut current_infos = self
            .api
            .get_account_info(previous_infos.iter().map(|account| account.id))
            .await?;
        let mut n_updated_accounts: usize = 0;
        for previous_info in previous_infos.iter() {
            let current_info = current_infos
                .remove(&previous_info.id.to_string())
                .flatten();
            if self.crawl_account(previous_info, current_info).await? {
                n_updated_accounts += 1;
            }
        }
        log::info!(
            "Updated {} of {} accounts.",
            n_updated_accounts,
            previous_infos.len()
        );
        Ok(())
    }

    async fn crawl_account(
        &self,
        previous_info: &BasicAccountInfo,
        current_info: Option<AccountInfo>,
    ) -> crate::Result<bool> {
        log::info!(
            "Account #{}, last crawled at {}.",
            previous_info.id,
            previous_info.crawled_at,
        );

        let tx = self.database.start_transaction()?;
        let is_updated = match current_info {
            Some(current_info) if !current_info.is_active() => {
                log::debug!("Nickname: {}.", current_info.nickname);
                log::warn!("The account is inactive. Deleting…");
                self.database.prune_account(previous_info.id)?;
                true
            }
            Some(mut current_info) => {
                log::debug!("Nickname: {}.", current_info.nickname);
                current_info.basic.crawled_at = Utc::now();
                self.database
                    .insert_account_or_replace(&current_info.basic)?;
                if self.opts.force
                    || current_info.basic.last_battle_time != previous_info.last_battle_time
                {
                    self.database.upsert_account_snapshot(&current_info)?;
                    self.database.upsert_tank_snapshots(
                        &self.api.get_merged_tanks(previous_info.id).await?,
                    )?;
                    true
                } else {
                    log::info!("No new battles.");
                    false
                }
            }
            None => {
                log::warn!("The account does not exist. Deleting…");
                self.database.prune_account(previous_info.id)?;
                true
            }
        };

        log::debug!("Committing…");
        tx.commit()?;
        Ok(is_updated)
    }
}
