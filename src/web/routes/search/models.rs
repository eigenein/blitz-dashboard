use crate::logging::clear_user;
use crate::models::AccountInfo;
use crate::wargaming::AccountSearchCache;

pub struct ViewModel {
    pub query: String,
    pub accounts: Vec<AccountInfo>,
}

impl ViewModel {
    pub async fn new(
        query: String,
        account_search_cache: &AccountSearchCache,
    ) -> crate::Result<Self> {
        clear_user();

        let mut accounts = account_search_cache.get(&query).await?.to_vec();
        accounts.sort_unstable_by(|left, right| {
            right
                .general
                .last_battle_time
                .cmp(&left.general.last_battle_time)
        });
        Ok(Self { query, accounts })
    }
}
