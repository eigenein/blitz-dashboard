use hashbrown::{HashMap, HashSet};
use lru::LruCache;

use crate::Vector;

pub struct Cache {
    pub vehicles: HashMap<i32, Vector>,
    pub accounts: LruCache<i32, Vector>,
    pub modified_account_ids: HashSet<i32>,
}
