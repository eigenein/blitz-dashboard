use std::any::type_name;

use mongodb::bson::doc;
use serde::Deserialize;
use tide::Request;

use crate::wargaming::models::Account;
use crate::web::components::SEARCH_QUERY_LENGTH;
use crate::web::State;

// TODO: `pub mod index`;
pub mod player;

/// User search query.
#[derive(Deserialize)]
pub struct IndexQueryString {
    #[serde(default = "Option::default")]
    search: Option<String>,
}

pub struct IndexViewModel {
    pub accounts: Option<Vec<Account>>,
}

impl IndexViewModel {
    pub async fn new(request: Request<State>) -> crate::Result<Self> {
        let query: IndexQueryString = request.query().map_err(surf::Error::into_inner)?;
        log::debug!("{} {:?}…", type_name::<Self>(), query.search);
        if let Some(query) = query.search {
            if SEARCH_QUERY_LENGTH.contains(&query.len()) {
                return Ok(IndexViewModel {
                    accounts: Some(request.state().api.search_accounts(&query).await?),
                });
            }
        }
        Ok(Self { accounts: None })
    }
}
