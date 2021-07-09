use std::time::Duration as StdDuration;

use async_std::task::sleep;
use chrono::Utc;
use log::Level;
use sentry::integrations::anyhow::capture_anyhow;
use sqlx::PgPool;

use crate::database;
use crate::metrics::Stopwatch;
use crate::models::{AccountInfo, BasicAccountInfo, Tank};
use crate::opts::CrawlerOpts;
use crate::wargaming::WargamingApi;

pub struct Crawler {
    pub api: WargamingApi,
    pub database: PgPool,
    pub opts: CrawlerOpts,
}

impl Crawler {
    pub async fn run(&self) -> crate::Result {
        sentry::configure_scope(|scope| scope.set_tag("app", "crawler"));

        loop {
            if let Err(error) = self.crawl_batch().await {
                let sentry_id = capture_anyhow(&error);
                log::error!("Failed to crawl a batch: {} (https://sentry.io/eigenein/blitz-dashboard/events/{})", error, sentry_id);
            }
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
        let previous_infos = database::retrieve_oldest_accounts(&self.database, 100).await?;
        let mut current_infos = self
            .api
            .get_account_info(
                &previous_infos
                    .iter()
                    .map(|account| account.id)
                    .collect::<Vec<_>>(),
            )
            .await?;
        for previous_info in previous_infos.iter() {
            let current_info = current_infos
                .remove(&previous_info.id.to_string())
                .flatten();
            self.crawl_account(previous_info, current_info).await?;
        }
        Ok(())
    }

    async fn crawl_account(
        &self,
        previous_info: &BasicAccountInfo,
        current_info: Option<AccountInfo>,
    ) -> crate::Result {
        let _stopwatch =
            Stopwatch::new(format!("Account #{} crawled", previous_info.id)).level(Level::Info);
        log::info!(
            "Crawling account #{}, last crawled at {:?}…",
            previous_info.id,
            previous_info.crawled_at,
        );

        let mut transaction = self.database.begin().await?;
        match current_info {
            Some(mut current_info) => {
                log::debug!("Nickname: {}.", current_info.nickname);
                current_info.basic.crawled_at = Some(Utc::now());
                database::insert_account_or_replace(&mut transaction, &current_info.basic).await?;
                let force = self.opts.force || previous_info.crawled_at.is_none();
                if force || current_info.basic.last_battle_time != previous_info.last_battle_time {
                    database::insert_account_snapshot(&mut transaction, &current_info).await?;
                    let tanks: Vec<Tank> = self
                        .api
                        .get_merged_tanks(previous_info.id)
                        .await?
                        .into_iter()
                        .filter(|tank| {
                            force || tank.last_battle_time >= previous_info.last_battle_time
                        })
                        .collect();
                    database::insert_tank_snapshots(&mut transaction, &tanks).await?;
                } else {
                    log::info!("No new battles detected.");
                }
            }
            None => {
                log::warn!("The account does not exist anymore. Deleting…");
                database::delete_account(&self.database, previous_info.id).await?;
            }
        };
        log::debug!("Committing…");
        transaction.commit().await?;

        Ok(())
    }
}
