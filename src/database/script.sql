CREATE TABLE IF NOT EXISTS accounts (
    account_id INTEGER PRIMARY KEY,
    last_battle_time TIMESTAMP WITH TIME ZONE NOT NULL,
    crawled_at TIMESTAMP WITH TIME ZONE NOT NULL
);
DROP INDEX IF EXISTS accounts_crawled_at;
DROP INDEX IF EXISTS accounts_crawled_at_2;
DROP INDEX IF EXISTS accounts_crawled_at_3;
ALTER TABLE ONLY accounts ALTER COLUMN crawled_at SET DEFAULT now();
CREATE INDEX IF NOT EXISTS accounts_last_battle_time ON accounts(last_battle_time DESC);

CREATE TABLE IF NOT EXISTS account_snapshots (
    account_id INTEGER NOT NULL REFERENCES accounts (account_id) ON DELETE CASCADE,
    last_battle_time TIMESTAMP WITH TIME ZONE NOT NULL,
    crawled_at TIMESTAMP WITH TIME ZONE NOT NULL,
    battles INTEGER NOT NULL,
    wins INTEGER NOT NULL,
    survived_battles INTEGER NOT NULL,
    win_and_survived INTEGER NOT NULL,
    damage_dealt INTEGER NOT NULL,
    damage_received INTEGER NOT NULL,
    shots INTEGER NOT NULL,
    hits INTEGER NOT NULL,
    frags INTEGER NOT NULL,
    xp INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS account_snapshots_key
    ON account_snapshots(account_id ASC, last_battle_time DESC);

CREATE TABLE IF NOT EXISTS tank_snapshots (
    account_id INTEGER NOT NULL REFERENCES accounts (account_id) ON DELETE CASCADE,
    tank_id INTEGER NOT NULL,
    last_battle_time TIMESTAMP WITH TIME ZONE NOT NULL,
    battle_life_time BIGINT NOT NULL,
    battles INTEGER NOT NULL,
    wins INTEGER NOT NULL,
    survived_battles INTEGER NOT NULL,
    win_and_survived INTEGER NOT NULL,
    damage_dealt INTEGER NOT NULL,
    damage_received INTEGER NOT NULL,
    shots INTEGER NOT NULL,
    hits INTEGER NOT NULL,
    frags INTEGER NOT NULL,
    xp INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS tank_snapshots_key
    ON tank_snapshots(account_id ASC, tank_id ASC, last_battle_time DESC);
