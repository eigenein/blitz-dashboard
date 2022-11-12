use futures::Stream;
use mongodb::bson::doc;
use mongodb::options::FindOptions;
use mongodb::Database;
use serde::Deserialize;

use crate::database::mongodb::traits::TypedDocument;
use crate::prelude::*;
use crate::wargaming;

#[derive(Deserialize, Copy, Clone)]
pub struct AccountIdProjection {
    #[serde(rename = "aid")]
    pub id: wargaming::AccountId,
}

impl TypedDocument for AccountIdProjection {
    const NAME: &'static str = "accounts";
}

impl AccountIdProjection {
    #[instrument(skip_all, level = "info", fields(realm = realm, limit = limit))]
    pub async fn retrieve_recently_active(
        from: &Database,
        realm: &str,
        limit: i64,
    ) -> Result<impl Stream<Item = Result<Self, mongodb::error::Error>>> {
        let filter = doc! { "rlm": realm };
        let options = FindOptions::builder()
            .limit(limit)
            .sort(doc! { "rlm": 1, "lbts": -1 })
            .projection(doc! { "_id": 0, "aid": 1 })
            .build();
        Ok(Self::collection(from).find(filter, options).await?)
    }

    #[instrument(skip_all, level = "info", fields(realm = ?realm, since = ?since))]
    pub async fn retrieve_active_since(
        from: &Database,
        realm: wargaming::Realm,
        since: DateTime,
    ) -> Result<impl Stream<Item = Result<Self, mongodb::error::Error>>> {
        let filter = doc! { "rlm": realm.to_str(), "lbts": { "$gte": since } };
        let options = FindOptions::builder()
            .projection(doc! { "_id": 0, "aid": 1 })
            .build();
        Ok(Self::collection(from).find(filter, options).await?)
    }
}
