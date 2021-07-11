use std::time::Duration as StdDuration;

use async_std::task::sleep;
use chrono::{DateTime, TimeZone, Utc};
use log::Level;
use rand::prelude::*;
use sentry::integrations::anyhow::capture_anyhow;
use sqlx::{PgConnection, PgPool};

use crate::database;
use crate::metrics::Stopwatch;
use crate::models::{AccountInfo, BasicAccountInfo, Tank};
use crate::wargaming::WargamingApi;

pub struct Crawler {
    api: WargamingApi,
    database: PgPool,
}

#[derive(Debug, PartialEq)]
enum CrawlMode {
    New,
    LastBattleTime(DateTime<Utc>),
}

impl Crawler {
    pub async fn run(api: WargamingApi, database: PgPool, run: bool) -> crate::Result {
        if !run {
            return Ok(());
        }

        let crawler = Self { api, database };
        loop {
            if let Err(error) = crawler.crawl_batch().await {
                let sentry_id = capture_anyhow(&error);
                log::error!("Failed to crawl a batch: {} (https://sentry.io/eigenein/blitz-dashboard/events/{})", error, sentry_id);
            }

            // FIXME: https://github.com/eigenein/blitz-dashboard/issues/15.
            sleep(StdDuration::from_secs(1)).await;
        }
    }

    async fn crawl_batch(&self) -> crate::Result {
        let _stopwatch = Stopwatch::new("Batch crawled").level(Level::Info);

        let mut previous_infos =
            database::retrieve_oldest_crawled_accounts(&self.database, 99).await?;
        previous_infos.push(BasicAccountInfo {
            id: (1..146458230).choose(&mut thread_rng()).unwrap(),
            last_battle_time: Utc.timestamp(0, 0),
            crawled_at: None,
        }); // FIXME
        let account_ids = previous_infos
            .iter()
            .map(|account| account.id)
            .collect::<Vec<_>>();
        let mut current_infos = self.api.get_account_info(&account_ids).await?;

        for previous_info in previous_infos.into_iter() {
            let current_info = current_infos
                .remove(&previous_info.id.to_string())
                .flatten();
            self.crawl_account(
                previous_info.id,
                match previous_info.crawled_at {
                    Some(_) => CrawlMode::LastBattleTime(previous_info.last_battle_time),
                    None => CrawlMode::New,
                },
                current_info,
            )
            .await?;
        }

        Ok(())
    }

    async fn crawl_account(
        &self,
        account_id: i32,
        mode: CrawlMode,
        current_info: Option<AccountInfo>,
    ) -> crate::Result {
        let _stopwatch =
            Stopwatch::new(format!("Account #{} crawled", account_id)).level(Level::Info);
        log::info!("Crawling account #{}, {:?}…", account_id, mode);

        let mut transaction = self.database.begin().await?;
        match current_info {
            Some(current_info) => {
                self.crawl_existing_account(&mut transaction, account_id, mode, current_info)
                    .await?;
            }
            None if mode != CrawlMode::New => {
                log::warn!("The account does not exist anymore. Deleting…");
                database::delete_account(&self.database, account_id).await?;
            }
            _ => {}
        };
        log::debug!("Committing…");
        transaction.commit().await?;

        Ok(())
    }

    async fn crawl_existing_account(
        &self,
        connection: &mut PgConnection,
        account_id: i32,
        mode: CrawlMode,
        mut current_info: AccountInfo,
    ) -> crate::Result {
        log::debug!("Nickname: {}.", current_info.nickname);
        current_info.basic.crawled_at = Some(Utc::now());
        database::insert_account_or_replace(&mut *connection, &current_info.basic).await?;

        match mode {
            CrawlMode::New => {
                database::insert_account_snapshot(&mut *connection, &current_info).await?;
                let tanks = self.api.get_merged_tanks(account_id).await?;
                database::insert_tank_snapshots(&mut *connection, &tanks).await?;
            }
            CrawlMode::LastBattleTime(last_battle_time)
                if current_info.basic.last_battle_time != last_battle_time =>
            {
                database::insert_account_snapshot(&mut *connection, &current_info).await?;
                let tanks: Vec<Tank> = self.api.get_merged_tanks(account_id).await?;
                let tanks: Vec<Tank> = tanks
                    .into_iter()
                    .filter(|tank| tank.last_battle_time >= last_battle_time)
                    .collect();
                database::insert_tank_snapshots(&mut *connection, &tanks).await?;
            }
            _ => log::info!("No new battles detected."),
        };

        Ok(())
    }
}
