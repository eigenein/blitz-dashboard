use mongodb::Database;

use crate::database;
use crate::database::mongodb::traits::*;
use crate::prelude::*;

pub struct CrawledData {
    pub account: database::Account,
    pub account_snapshot: database::AccountSnapshot,
    pub tank_snapshots: Vec<database::TankSnapshot>,
    pub rating_snapshot: Option<database::RatingSnapshot>,
}

impl CrawledData {
    #[instrument(
        skip_all,
        level = "debug",
        fields(
            realm = ?self.account.realm,
            account_id = self.account.id,
            rating_snapshot.is_some = self.rating_snapshot.is_some(),
            n_tank_snapshots = self.tank_snapshots.len(),
        )
    )]
    pub async fn upsert(&self, into: &Database) -> Result {
        let start_instant = Instant::now();
        database::TankSnapshot::upsert_many(into, &self.tank_snapshots).await?;
        self.account_snapshot.upsert(into).await?;
        if let Some(rating_snapshot) = &self.rating_snapshot {
            rating_snapshot.upsert(into).await?;
        }
        self.account.upsert(into).await?;
        debug!(elapsed = ?start_instant.elapsed());
        Ok(())
    }
}
