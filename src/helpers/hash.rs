use std::hash::{Hash, Hasher};

use ahash::AHasher;

pub fn hash_digest<T: Hash>(value: &T) -> String {
    let mut hasher = AHasher::default();
    value.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}
