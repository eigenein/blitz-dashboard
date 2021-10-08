CREATE TABLE IF NOT EXISTS accounts (
    account_id INTEGER PRIMARY KEY,
    last_battle_time TIMESTAMP WITH TIME ZONE NOT NULL
);
CREATE INDEX IF NOT EXISTS accounts_last_battle_time ON accounts(last_battle_time DESC);

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

-- 0.38.0

CREATE TABLE IF NOT EXISTS vehicles (
    tank_id INTEGER PRIMARY KEY
);

-- 0.54.0

ALTER TABLE accounts
    ADD COLUMN IF NOT EXISTS bias double precision NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS factors double precision ARRAY NOT NULL DEFAULT '{}';

-- 0.59.1

ALTER TABLE tank_snapshots
    ALTER CONSTRAINT tank_snapshots_account_id_fkey
    DEFERRABLE INITIALLY DEFERRED;

-- 0.71.0

ALTER TABLE accounts DROP COLUMN IF EXISTS bias;

-- 0.84.8

ALTER TABLE accounts SET (FILLFACTOR = 90);

-- 0.84.9

ALTER TABLE tank_snapshots DROP CONSTRAINT IF EXISTS tank_snapshots_account_id_fkey;
