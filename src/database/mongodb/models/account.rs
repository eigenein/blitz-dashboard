use anyhow::Error;
use futures::stream::{iter, try_unfold};
use futures::{Stream, TryStreamExt};
use mongodb::bson::{doc, Document};
use mongodb::options::*;
use mongodb::{bson, Database, IndexModel};
use serde::{Deserialize, Serialize};
use serde_with::TryFromInto;

pub use self::id_projection::*;
pub use self::partial_tank_stats::*;
pub use self::random::*;
pub use self::rating::*;
pub use self::tank_last_battle_time::*;
use crate::database::mongodb::traits::{Indexes, TypedDocument, Upsert};
use crate::prelude::*;
use crate::{format_elapsed, wargaming};

mod id_projection;
mod partial_tank_stats;
mod random;
mod rating;
mod tank_last_battle_time;

#[serde_with::serde_as]
#[derive(Serialize, Deserialize, Clone)]
pub struct Account {
    /// Wargaming.net account ID.
    #[serde_as(as = "TryFromInto<i32>")]
    #[serde(rename = "aid")]
    pub id: wargaming::AccountId,

    #[serde(rename = "rlm")]
    pub realm: wargaming::Realm,

    #[serde(rename = "lbts")]
    #[serde_as(as = "Option<bson::DateTime>")]
    pub last_battle_time: Option<DateTime>,

    /// Used to select random accounts from the database.
    pub random: f64,

    #[serde(default, rename = "pts")]
    pub partial_tank_stats: Vec<PartialTankStats>,
}

impl TypedDocument for Account {
    const NAME: &'static str = "accounts";
}

#[async_trait]
impl Indexes for Account {
    type I = [IndexModel; 2];

    fn indexes() -> Self::I {
        [
            // Ensures the single entry for each account.
            IndexModel::builder()
                .keys(doc! { "rlm": 1, "aid": 1 })
                .options(IndexOptions::builder().unique(true).build())
                .build(),
            // Optimizes the crawler's range query.
            IndexModel::builder()
                .keys(doc! { "rlm": 1, "random": 1 "lbts": -1, })
                .build(),
        ]
    }
}

impl Account {
    pub fn new(realm: wargaming::Realm, account_id: wargaming::AccountId) -> Self {
        Self {
            id: account_id,
            realm,
            last_battle_time: None,
            random: fastrand::f64(),
            partial_tank_stats: Vec::new(),
        }
    }
}

#[async_trait]
impl Upsert for Account {
    type Update = Document;

    #[inline]
    fn query(&self) -> Document {
        doc! { "rlm": self.realm.to_str(), "aid": self.id }
    }

    #[inline]
    fn update(&self) -> Result<Self::Update> {
        Ok(doc! { "$set": bson::to_bson(&self)? })
    }
}

impl Account {
    #[instrument(skip_all, level = "debug")]
    pub fn get_sampled_stream(
        database: Database,
        realm: wargaming::Realm,
        sample_size: u32,
        min_offset: Duration,
        max_offset: Duration,
    ) -> impl Stream<Item = Result<Self>> {
        info!(sample_size, %min_offset, %max_offset);
        try_unfold((1, database), move |(sample_number, database)| async move {
            debug!(sample_number, "retrieving a sample…");
            let sample =
                Account::retrieve_sample(&database, realm, sample_size, min_offset, max_offset)
                    .await?;
            debug!(sample_number, "retrieved");
            Ok::<_, Error>(Some((iter(sample.into_iter().map(Ok)), (sample_number + 1, database))))
        })
        .try_flatten()
    }

    /// Ensures that the account exists in the database.
    /// Does nothing if it exists, inserts – otherwise.
    #[instrument(skip_all, level = "debug", fields(realm = ?self.realm, account_id = self.id))]
    pub async fn ensure_exists(&self, in_: &Database) -> Result {
        let filter = doc! { "rlm": self.realm.to_str(), "aid": self.id };
        let update = doc! { "$setOnInsert": bson::to_bson(&self)? };
        let options = UpdateOptions::builder().upsert(true).build();
        Self::collection(in_)
            .update_one(filter, update, options)
            .await
            .with_context(|| format!("failed to ensure the account #{} existence", self.id))?;
        Ok(())
    }

    #[instrument(skip_all, level = "debug")]
    pub async fn retrieve_sample(
        from: &Database,
        realm: wargaming::Realm,
        sample_size: u32,
        min_offset: Duration,
        max_offset: Duration,
    ) -> Result<Vec<Account>> {
        let now = Utc::now();
        let filter = doc! {
            "rlm": realm.to_str(),
            "random": { "$gt": fastrand::f64() },
            "$or": [
                { "lbts": null },
                { "lbts": { "$gt": now - max_offset, "$lte": now - min_offset } },
            ],
        };
        let options = FindOptions::builder()
            .sort(doc! { "random": 1 })
            .limit(sample_size as i64)
            .build();

        let start_instant = Instant::now();
        debug!(sample_size, "retrieving a sample…");
        let accounts: Vec<Account> = Self::collection(from)
            .find(filter, options)
            .await
            .context("failed to query a sample of accounts")?
            .try_collect()
            .await?;

        debug!(
            n_accounts = accounts.len(),
            elapsed = format_elapsed(start_instant).as_str(),
            "done",
        );
        Ok(accounts)
    }

    pub async fn sample_account(from: &Database, realm: wargaming::Realm) -> Result<Account> {
        let filter = doc! {
            "rlm": realm.to_str(),
            "random": { "$gt": fastrand::f64() },
            "lbts": { "$gt": Utc::now() - Duration::days(1) },
        };
        let options = FindOneOptions::builder().sort(doc! { "random": 1 }).build();
        let start_instant = Instant::now();
        let account = Self::collection(from)
            .find_one(filter, options)
            .await?
            .ok_or_else(|| anyhow!("could not sample a random account"))?;
        debug!(elapsed = ?start_instant.elapsed());
        Ok(account)
    }
}
