use std::fmt::Debug;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use prost::Message;
use serde_bytes::ByteBuf;
use zip::ZipArchive;

use crate::opts::TestReplayOpts;
use crate::prelude::*;

pub fn run(opts: TestReplayOpts) -> Result {
    let battle_results = read_battle_results(opts.path)?;
    let (_, battle_results): (u64, ByteBuf) =
        serde_pickle::from_slice(&battle_results, Default::default())?;
    let battle_results = BattleResults::decode(battle_results.as_ref())
        .context("failed to decode the battle results")?;

    info!(timestamp = ?battle_results.timestamp()?);
    for player in &battle_results.players {
        info!(player.account_id, player.info.nickname, player.info.team_number);
    }

    Ok(())
}

#[instrument(skip_all, fields(path = ?path))]
fn read_battle_results(path: impl AsRef<Path> + Debug) -> Result<Vec<u8>> {
    let replay_file = File::open(path).context("failed to open the replay file")?;
    let mut archive = ZipArchive::new(replay_file).context("failed to open the archive")?;
    let mut battle_results = archive
        .by_name("battle_results.dat")
        .context("failed to open the battle results")?;
    let mut buffer = Vec::new();
    battle_results
        .read_to_end(&mut buffer)
        .context("failed to read the battle results")?;
    Ok(buffer)
}

#[derive(Message)]
struct BattleResults {
    #[prost(int64, tag = "2")]
    pub timestamp: i64,

    #[prost(message, repeated, tag = "201")]
    pub players: Vec<Player>,
}

impl BattleResults {
    pub fn timestamp(&self) -> Result<DateTime> {
        Utc.timestamp_opt(self.timestamp, 0)
            .single()
            .ok_or_else(|| anyhow!("invalid timestamp"))
    }
}

#[derive(Message)]
struct Player {
    #[prost(uint32, tag = "1")]
    pub account_id: wargaming::AccountId,

    #[prost(message, required, tag = "2")]
    pub info: PlayerInfo,
}

#[derive(Message)]
struct PlayerInfo {
    #[prost(string, tag = "1")]
    pub nickname: String,

    #[prost(uint32, tag = "3")]
    pub team_number: u32,
}
