use serde::{Deserialize, Serialize};

use crate::prelude::*;
use crate::wargaming::{AccountId, BasicStats, RatingStats};

/// Wargaming.net account information.
#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
pub struct AccountInfo {
    #[serde(rename = "account_id")]
    pub id: AccountId,

    #[serde(with = "chrono::serde::ts_seconds")]
    pub last_battle_time: DateTime,

    pub nickname: String,

    #[serde(with = "chrono::serde::ts_seconds")]
    pub created_at: DateTime,

    #[serde(rename = "statistics")]
    pub stats: AccountInfoStats,
}

impl AccountInfo {
    pub fn is_active(&self) -> bool {
        self.last_battle_time > (Utc::now() - Duration::days(365))
    }

    pub fn has_recently_played(&self) -> bool {
        self.last_battle_time > (Utc::now() - Duration::hours(1))
    }

    pub fn is_account_birthday(&self) -> bool {
        let today = Utc::today();
        today.day() == self.created_at.day() && today.month() == self.created_at.month()
    }

    pub fn is_prerelease_account(&self) -> bool {
        self.created_at.date() < Utc.ymd(2014, 6, 26)
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
pub struct AccountInfoStats {
    #[serde(rename = "all")]
    pub random: BasicStats,

    pub rating: RatingStats,
}

impl AccountInfoStats {
    #[inline]
    #[must_use]
    pub const fn n_total_battles(&self) -> i32 {
        self.random.n_battles + self.rating.basic.n_battles
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wargaming::models::ResultMap;

    #[test]
    fn account_info_map_ok() -> Result {
        let mut map = serde_json::from_str::<ResultMap<AccountInfo>>(
            // language=json
            r#"{"5589968":{"statistics":{"clan":{"spotted":0,"max_frags_tank_id":0,"hits":0,"frags":0,"max_xp":0,"max_xp_tank_id":0,"wins":0,"losses":0,"capture_points":0,"battles":0,"damage_dealt":0,"damage_received":0,"max_frags":0,"shots":0,"frags8p":0,"xp":0,"win_and_survived":0,"survived_battles":0,"dropped_capture_points":0},"rating":{"spotted":152,"calibration_battles_left":0,"hits":2070,"frags":226,"recalibration_start_time":1639417161,"mm_rating":54.9,"wins":142,"losses":148,"is_recalibration":false,"capture_points":164,"battles":292,"current_season":32,"damage_dealt":378035,"damage_received":338354,"shots":2600,"frags8p":164,"xp":221273,"win_and_survived":96,"survived_battles":99,"dropped_capture_points":199},"all":{"spotted":9100,"max_frags_tank_id":3697,"hits":72822,"frags":8197,"max_xp":2292,"max_xp_tank_id":22817,"wins":5318,"losses":4327,"capture_points":5447,"battles":9676,"damage_dealt":10427192,"damage_received":8611546,"max_frags":6,"shots":93254,"frags8p":2760,"xp":6743639,"win_and_survived":3772,"survived_battles":3908,"dropped_capture_points":5882},"frags":null},"account_id":5589968,"created_at":1415225091,"updated_at":1635246495,"private":null,"last_battle_time":1635269048,"nickname":"eigenein"}}"#,
        )?;
        let info = map.remove("5589968").flatten().unwrap();
        assert_ne!(info.stats.random.frags, 0);
        assert_ne!(info.stats.random.xp, 0);
        Ok(())
    }
}
