use serde::Deserialize;

use crate::wargaming;

#[derive(Deserialize)]
pub struct PathSegments {
    pub realm: wargaming::Realm,
    pub account_id: wargaming::AccountId,
}
