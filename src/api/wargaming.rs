pub mod models;

use crate::api::wargaming::models::{AccountId, AccountInfos, Accounts, ApiResponse};
use surf::Url;

#[derive(Clone)]
pub struct WargamingApi {
    application_id: String,
    client: surf::Client,
}

impl WargamingApi {
    pub fn new(application_id: &str) -> WargamingApi {
        Self {
            application_id: application_id.to_string(),
            client: surf::client(),
        }
    }

    /// See: <https://developers.wargaming.net/reference/all/wotb/account/list/>.
    pub async fn search_accounts(&self, query: &str) -> crate::Result<Accounts> {
        log::debug!("Search: {}", query);
        self.client
            .get(Url::parse_with_params(
                "https://api.wotblitz.ru/wotb/account/list/",
                &[
                    ("application_id", self.application_id.as_str()),
                    ("limit", "20"),
                    ("search", query),
                ],
            )?)
            .await
            .map_err(surf::Error::into_inner)?
            .body_json::<ApiResponse<Accounts>>()
            .await
            .map_err(surf::Error::into_inner)?
            .into()
    }

    /// See <https://developers.wargaming.net/reference/all/wotb/account/info/>.
    pub async fn get_account_info(&self, account_id: AccountId) -> crate::Result<AccountInfos> {
        log::debug!("Get account info: {}", account_id);
        self.client
            .get(Url::parse_with_params(
                "https://api.wotblitz.ru/wotb/account/info/",
                &[
                    ("application_id", self.application_id.as_str()),
                    ("account_id", account_id.to_string().as_str()),
                ],
            )?)
            .await
            .map_err(surf::Error::into_inner)?
            .body_json::<ApiResponse<AccountInfos>>()
            .await
            .map_err(surf::Error::into_inner)?
            .into()
    }
}
