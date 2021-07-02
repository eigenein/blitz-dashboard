use anyhow::{anyhow, Context};
use serde::Deserialize;
use tide::Request;

use crate::models::AccountInfo;
use crate::web::partials::SEARCH_QUERY_LENGTH;
use crate::web::state::State;

pub struct ViewModel {
    pub query: String,
    pub accounts: Vec<AccountInfo>,
}

impl ViewModel {
    pub async fn new(request: &Request<State>) -> crate::Result<Self> {
        let query: Query = request
            .query()
            .map_err(|error| anyhow!(error))
            .context("failed to parse the query")?;
        query.validate()?;

        let mut accounts = request
            .state()
            .search_accounts(&query.query)
            .await?
            .to_vec();
        accounts.sort_unstable_by(|left, right| {
            right
                .basic
                .last_battle_time
                .cmp(&left.basic.last_battle_time)
        });
        Ok(Self {
            accounts,
            query: query.query,
        })
    }
}

#[derive(Deserialize)]
struct Query {
    query: String,
}

impl Query {
    fn validate(&self) -> crate::Result {
        if SEARCH_QUERY_LENGTH.contains(&self.query.len()) {
            Ok(())
        } else {
            Err(anyhow!("invalid search query length"))
        }
    }
}
