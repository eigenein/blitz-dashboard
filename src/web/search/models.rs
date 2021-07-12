use crate::logging::clear_user;
use crate::models::AccountInfo;
use crate::web::state::State;

pub struct ViewModel {
    pub query: String,
    pub accounts: Vec<AccountInfo>,
}

impl ViewModel {
    pub async fn new(state: &State, query: String) -> crate::Result<Self> {
        clear_user();

        let mut accounts = state.search_accounts(&query).await?.to_vec();
        accounts.sort_unstable_by(|left, right| {
            right
                .basic
                .last_battle_time
                .cmp(&left.basic.last_battle_time)
        });
        Ok(Self { query, accounts })
    }
}
