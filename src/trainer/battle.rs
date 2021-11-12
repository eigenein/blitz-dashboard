use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct Battle {
    pub account_id: i32,
    pub tank_id: i32,
    pub is_win: bool,
    pub is_test: bool,
}
