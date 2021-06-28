use std::time::{Duration, Instant};

use anyhow::anyhow;
use async_std::task::sleep;
use chrono::Utc;

use crate::database::Database;
use crate::models::{BasicAccountInfo, TankSnapshot};
use crate::wargaming::WargamingApi;

pub struct Crawler {
    pub api: WargamingApi,
    pub database: Database,
    pub once: bool,
}

impl Crawler {
    pub async fn run(&self) -> crate::Result {
        loop {
            self.make_step().await?;
            if self.once {
                break;
            }

            // FIXME: https://github.com/eigenein/blitz-dashboard/issues/15.
            sleep(Duration::from_secs(1)).await;
        }

        Ok(())
    }

    async fn make_step(&self) -> crate::Result {
        let account = self
            .database
            .retrieve_oldest_account()?
            .ok_or_else(|| anyhow!("the database is empty"))?;
        log::info!(
            "Account #{}, last crawled at {}.",
            account.id,
            account.crawled_at,
        );

        let start_instant = Instant::now();
        let account_info = self.api.get_account_info(account.id).await?;
        let tx = self.database.start_transaction()?;
        match account_info {
            Some(account_info) if !account_info.is_active() => {
                log::debug!("Nickname: {}.", account_info.nickname);
                log::warn!("The account is inactive. Deleting…");
                self.database.prune_account(account.id)?;
            }
            Some(mut account_info) => {
                log::debug!("Nickname: {}.", account_info.nickname);
                account_info.basic.crawled_at = Utc::now();
                self.database.upsert_account(&account_info.basic)?;
                if account_info.basic.last_battle_time >= account.crawled_at {
                    self.database.upsert_account_snapshot(&account_info)?;
                    let tank_snapshots = self.get_tank_snapshots(&account).await?;
                    self.database.upsert_tank_snapshots(&tank_snapshots)?;
                } else {
                    log::info!("No new battles.");
                }
            }
            None => {
                log::warn!("The account does not exist. Deleting…");
                self.database.prune_account(account.id)?;
            }
        }
        log::debug!("Committing…");
        tx.commit()?;
        log::info!("Elapsed: {:?}.", Instant::now() - start_instant);

        Ok(())
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
