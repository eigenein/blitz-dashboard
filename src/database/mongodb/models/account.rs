use futures::stream::{iter, try_unfold};
use futures::{Stream, TryStreamExt};
use mongodb::bson::{doc, Document};
use mongodb::options::*;
use mongodb::{bson, Database, IndexModel};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use serde_with::TryFromInto;
use statrs::distribution::Exp;

pub use self::id_projection::*;
pub use self::random::*;
pub use self::rating::*;
pub use self::tank_last_battle_time::*;
use crate::database::mongodb::traits::*;
use crate::prelude::*;

mod id_projection;
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
}

impl TypedDocument for Account {
    const NAME: &'static str = "accounts";
}

#[async_trait]
impl Indexes for Account {
    type I = [IndexModel; 2];

    fn indexes() -> Self::I {
        [
            IndexModel::builder()
                .keys(doc! { "rlm": 1, "lbts": -1 })
                .build(),
            IndexModel::builder()
                .keys(doc! { "rlm": 1, "aid": 1 })
                .options(IndexOptions::builder().unique(true).build())
                .build(),
        ]
    }
}

impl Account {
    pub const fn new(realm: wargaming::Realm, account_id: wargaming::AccountId) -> Self {
        Self {
            id: account_id,
            realm,
            last_battle_time: None,
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
        sample_size: usize,
        min_offset: Duration,
        offset_scale: time::Duration,
    ) -> Result<impl Stream<Item = Result<Self>>> {
        info!(sample_size, %min_offset, ?offset_scale);
        let offset_scale_secs = offset_scale.as_secs_f64();
        let exp1 = Exp::new(1.0)?;
        let stream = try_unfold((1, database), move |(sample_number, database)| async move {
            let offset =
                Duration::seconds((thread_rng().sample::<f64, _>(exp1) * offset_scale_secs) as i64);
            let before = Utc::now() - min_offset - offset;
            debug!(sample_number, ?before, "retrieving a sample…");
            let sample = Account::retrieve_sample(&database, realm, before, sample_size).await?;
            debug!(sample_number, "retrieved");
            Ok::<_, Error>(Some((iter(sample.into_iter().map(Ok)), (sample_number + 1, database))))
        })
        .try_flatten();
        Ok(stream)
    }

    /// Ensures that the account exists in the database.
    /// Does nothing if it exists, inserts – otherwise.
    #[instrument(skip_all, level = "debug", fields(realm = ?realm, account_id = account_id))]
    pub async fn ensure_exists(
        in_: &Database,
        realm: wargaming::Realm,
        account_id: wargaming::AccountId,
    ) -> Result {
        let filter = doc! { "rlm": realm.to_str(), "aid": account_id };
        let update = doc! { "$setOnInsert": { "lbts": null } };
        let options = UpdateOptions::builder().upsert(true).build();
        Self::collection(in_)
            .update_one(filter, update, options)
            .await
            .with_context(|| format!("failed to ensure the account #{} existence", account_id))?;
        Ok(())
    }

    #[instrument(skip_all, level = "debug")]
    pub async fn retrieve_sample(
        from: &Database,
        realm: wargaming::Realm,
        before: DateTime,
        sample_size: usize,
    ) -> Result<Vec<Account>> {
        debug!(sample_size, "retrieving…");
        let start_instant = Instant::now();

        // Retrieve new accounts:
        let mut accounts = {
            debug!("querying new accounts…");
            let filter = doc! { "rlm": realm.to_str(), "lbts": null };
            let options = FindOptions::builder().limit(sample_size as i64).build();
            let new_accounts = Self::find_vec(from, filter, options).await?;
            debug!(
                n_new_accounts = new_accounts.len(),
                elapsed = ?start_instant.elapsed(),
            );
            new_accounts
        };

        // Retrieve random selection of accounts:
        if accounts.len() != sample_size {
            debug!("querying random accounts…");
            let filter = doc! {
                "rlm": realm.to_str(),
                "$and": [ { "lbts": { "$ne": null } }, { "lbts": { "$lte": before } } ],
            };
            let options = FindOptions::builder()
                .sort(doc! { "lbts": -1 })
                .limit((sample_size - accounts.len()) as i64)
                .build();
            let random_accounts = Self::find_vec(from, filter, options).await?;
            debug!(
                n_random_accounts = random_accounts.len(),
                elapsed = ?start_instant.elapsed(),
            );
            accounts.extend(random_accounts);
        };

        Ok(accounts)
    }

    pub async fn sample_account(from: &Database, realm: wargaming::Realm) -> Result<Account> {
        let filter = doc! {
            "rlm": realm.to_str(),
            "lbts": { "$lte": now() - Duration::seconds(thread_rng().gen_range(0..14400)) },
        };
        let options = FindOneOptions::builder().sort(doc! { "lbts": -1 }).build();
        let start_instant = Instant::now();
        let account = Self::collection(from)
            .find_one(filter, options)
            .await?
            .ok_or_else(|| anyhow!("could not sample a random account"))?;
        debug!(elapsed = ?start_instant.elapsed());
        Ok(account)
    }
}
