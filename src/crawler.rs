use anyhow::Context;
use chrono::{DateTime, TimeZone, Utc};
use log::Level;
use rand::prelude::*;
use sentry::integrations::anyhow::capture_anyhow;
use sqlx::{PgConnection, PgPool};

use crate::database;
use crate::metrics::Stopwatch;
use crate::models::{AccountInfo, GeneralAccountInfo, Tank};
use crate::opts::CrawlerOpts;
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
    pub async fn run(api: WargamingApi, opts: CrawlerOpts) -> crate::Result {
        sentry::configure_scope(|scope| scope.set_tag("app", "crawler"));
        let database = crate::database::open(&opts.database).await?;
        let crawler = Self { api, database };
        loop {
            if let Err(error) = crawler.crawl_batch().await {
                let sentry_id = capture_anyhow(&error);
                log::error!("Failed to crawl a batch: {} (https://sentry.io/eigenein/blitz-dashboard/events/{})", error, sentry_id);
            }
        }
    }

    async fn crawl_batch(&self) -> crate::Result {
        let _stopwatch = Stopwatch::new("Batch crawled").level(Level::Info);

        let mut previous_infos = retrieve_batch(&self.database, 50, 50).await?;
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
        log::debug!("Nickname: {}.", current_info.general.nickname);
        current_info.general.crawled_at = Some(Utc::now());
        database::insert_account_or_replace(&mut *connection, &current_info.general).await?;

        match mode {
            CrawlMode::New => {
                database::insert_account_snapshot(&mut *connection, &current_info).await?;
                let tanks = self.api.get_merged_tanks(account_id).await?;
                database::insert_tank_snapshots(&mut *connection, &tanks).await?;
            }
            CrawlMode::LastBattleTime(last_battle_time)
                if current_info.general.last_battle_time != last_battle_time =>
            {
                database::insert_account_snapshot(&mut *connection, &current_info).await?;
                let tanks: Vec<Tank> = self
                    .api
                    .get_merged_tanks(account_id)
                    .await?
                    .into_iter()
                    .filter(|tank| tank.last_battle_time > last_battle_time)
                    .collect();
                database::insert_tank_snapshots(&mut *connection, &tanks).await?;
            }
            _ => log::info!("No new battles detected."),
        };

        Ok(())
    }
}

pub async fn retrieve_batch(
    database: &PgPool,
    n_least_recently_crawled: i32,
    n_most_recently_played: i32,
) -> crate::Result<Vec<GeneralAccountInfo>> {
    // language=SQL
    const QUERY: &str = r#"
        (SELECT * FROM accounts ORDER BY crawled_at NULLS FIRST LIMIT $1)
        UNION
        (
            SELECT * FROM accounts
            WHERE last_battle_time < NOW() - INTERVAL '1 minute'
            ORDER BY last_battle_time DESC
            LIMIT $2
        );
    "#;
    let accounts = sqlx::query_as(QUERY)
        .bind(n_least_recently_crawled)
        .bind(n_most_recently_played)
        .fetch_all(database)
        .await
        .context("failed to retrieve a batch")?;
    Ok(accounts)
}
