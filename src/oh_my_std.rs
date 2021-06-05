use std::collections::HashMap;
use std::hash::Hash;

pub fn inner_join<K: Hash + Eq, V1, V2, R, F: Fn(V1, V2) -> R>(
    left: &mut HashMap<K, V1>,
    right: &mut HashMap<K, V2>,
    join: F,
) -> HashMap<K, R> {
    left.drain()
        .filter_map(|(key, left_value)| {
            right
                .remove(&key)
                .map(|right_value| (key, join(left_value, right_value)))
        })
        .collect()
}
