use anyhow::{anyhow, Context};
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use tide::Request;

use crate::statistics::ConfidenceInterval;
use crate::web::state::State;

pub struct PlayerViewModel {
    pub account_id: i32,
    pub nickname: String,
    pub created_at: DateTime<Utc>,
    pub last_battle_time: DateTime<Utc>,
    pub has_recently_played: bool,
    pub is_inactive: bool,
    pub total_battles: i32,
    pub total_tanks: usize,
    pub since: Since,
    pub period_battles: i32,
    pub period_wins: Option<ConfidenceInterval>,
    pub period_survival: Option<ConfidenceInterval>,
    pub period_hits: Option<ConfidenceInterval>,
}

impl PlayerViewModel {
    pub async fn new(request: &Request<State>) -> crate::Result<PlayerViewModel> {
        let account_id = Self::parse_account_id(&request)?;
        let query = request
            .query::<Query>()
            .map_err(|error| anyhow!(error))
            .context("failed to parse the query")?;
        let since = Utc::now() - Duration::from(&query.since);
        log::info!("Retrieving player #{} since {:?}.", account_id, since);

        let state = request.state();
        let account_info = state.get_account_info(account_id).await?;
        let tanks = state.get_tanks(account_id).await?;
        let older_account_info = state
            .retrieve_latest_account_snapshot(account_id, since)
            .await?;
        let older_account_info = older_account_info.as_ref();

        let period_battles = account_info.statistics.all.battles
            - older_account_info.map_or(0, |info| info.statistics.all.battles);
        let period_wins = ConfidenceInterval::from_proportion(
            period_battles,
            account_info.statistics.all.wins
                - older_account_info.map_or(0, |info| info.statistics.all.wins),
            None,
        );
        let period_survival = ConfidenceInterval::from_proportion(
            period_battles,
            account_info.statistics.all.survived_battles
                - older_account_info.map_or(0, |info| info.statistics.all.survived_battles),
            None,
        );
        let period_hits = ConfidenceInterval::from_proportion(
            account_info.statistics.all.shots
                - older_account_info.map_or(0, |info| info.statistics.all.shots),
            account_info.statistics.all.hits
                - older_account_info.map_or(0, |info| info.statistics.all.hits),
            None,
        );

        Ok(Self {
            account_id: account_info.basic.id,
            nickname: account_info.nickname.clone(),
            created_at: account_info.created_at,
            last_battle_time: account_info.basic.last_battle_time,
            total_battles: account_info.statistics.all.battles,
            has_recently_played: account_info.basic.last_battle_time
                > (Utc::now() - Duration::hours(1)),
            is_inactive: account_info.basic.last_battle_time < (Utc::now() - Duration::days(365)),
            total_tanks: tanks.len(),
            since: query.since,
            period_battles,
            period_wins,
            period_survival,
            period_hits,
        })
    }

    fn parse_account_id(request: &Request<State>) -> crate::Result<i32> {
        Ok(request
            .param("account_id")
            .map_err(surf::Error::into_inner)
            .context("missing account ID")?
            .parse()
            .context("invalid account ID")?)
    }
}

#[derive(Deserialize)]
struct Query {
    #[serde(default)]
    since: Since,
}

#[derive(Deserialize, PartialEq)]
pub enum Since {
    #[serde(rename = "1h")]
    Hour,

    #[serde(rename = "1d")]
    Day,

    #[serde(rename = "1w")]
    Week,

    #[serde(rename = "1m")]
    Month,

    #[serde(rename = "1y")]
    Year,
}

impl Default for Since {
    fn default() -> Self {
        Self::Day
    }
}

impl From<&Since> for Duration {
    fn from(this: &Since) -> Self {
        match this {
            Since::Hour => Self::hours(1),
            Since::Day => Self::days(1),
            Since::Week => Self::weeks(1),
            Since::Month => Self::days(30),
            Since::Year => Self::days(365),
        }
    }
}
