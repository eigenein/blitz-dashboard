use serde::Deserialize;
use std::sync::Arc;
use surf::Url;
use tide::StatusCode;

#[derive(Clone)]
pub struct WargamingApi {
    application_id: Arc<String>,
    client: surf::Client,
}

#[derive(Deserialize)]
pub struct Account {
    pub nickname: String,

    #[serde(alias = "account_id")]
    pub id: u32,
}

impl WargamingApi {
    pub fn new(application_id: String) -> WargamingApi {
        Self {
            application_id: Arc::new(application_id),
            client: surf::client(),
        }
    }

    /// See: <https://developers.wargaming.net/reference/all/wotb/account/list/>.
    pub async fn search_accounts(&self, query: &str) -> tide::Result<Vec<Account>> {
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
            .await?
            .body_json::<ApiResponse<Vec<Account>>>()
            .await?
            .into()
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ApiResponse<T> {
    Data {
        data: T,
    },

    /// See: <https://developers.wargaming.net/documentation/guide/getting-started/#common-errors>
    Error {
        error: ApiError,
    },
}

#[derive(Deserialize)]
struct ApiError {
    message: String,
}

impl<T> From<ApiResponse<T>> for tide::Result<T> {
    fn from(response: ApiResponse<T>) -> tide::Result<T> {
        match response {
            ApiResponse::Data { data } => Ok(data),
            ApiResponse::Error { error } => tide::Result::Err(tide::Error::from_str(
                StatusCode::InternalServerError,
                error.message,
            )),
        }
    }
}
