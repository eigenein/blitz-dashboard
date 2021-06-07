use chrono::Utc;

use crate::database;
use crate::wargaming;

impl From<&wargaming::models::AccountInfo> for database::models::Account {
    fn from(account_info: &wargaming::models::AccountInfo) -> Self {
        Self {
            id: account_info.id,
            last_battle_time: account_info.last_battle_time.into(),
            updated_at: Utc::now().into(),
        }
    }
}

impl From<&wargaming::models::AccountInfo> for database::models::AccountSnapshot {
    fn from(account_info: &wargaming::models::AccountInfo) -> Self {
        Self {
            account_id: account_info.id,
            last_battle_time: account_info.last_battle_time.into(),
            statistics: (&account_info.statistics.all).into(),
        }
    }
}

impl From<&wargaming::models::AllStatistics> for database::models::StatisticsSnapshot {
    fn from(statistics: &wargaming::models::AllStatistics) -> Self {
        Self {
            battles: statistics.battles,
            survived_battles: statistics.survived_battles,
            wins: statistics.wins,
            win_and_survived: statistics.win_and_survived,
            damage_dealt: statistics.damage_dealt,
            damage_received: statistics.damage_received,
        }
    }
}

/// Convert tank statistics & achievements into a tank snapshot in the database.
pub fn to_tank_snapshot(
    account_id: i32,
    statistics: &wargaming::models::TankStatistics,
    achievements: &wargaming::models::TankAchievements,
) -> database::models::TankSnapshot {
    database::models::TankSnapshot {
        account_id,
        tank_id: statistics.tank_id,
        last_battle_time: statistics.last_battle_time.into(),
        battle_life_time: statistics.battle_life_time.num_seconds(),
        statistics: (&statistics.all).into(),
        achievements: achievements
            .achievements
            .iter()
            .map(|(key, value)| (encode_key(key), *value))
            .collect(),
        max_series: achievements
            .max_series
            .iter()
            .map(|(key, value)| (encode_key(key), *value))
            .collect(),
    }
}

/// Encodes the key with hexadecimal representation of its CRC32.
fn encode_key(key: &str) -> String {
    format!("{:x}", crc32(key.as_bytes()))
}

fn crc32(buf: &[u8]) -> u32 {
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(buf);
    hasher.finalize()
}
