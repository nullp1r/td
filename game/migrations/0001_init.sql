PRAGMA foreign_keys = ON;

CREATE TABLE accounts (
  id INTEGER PRIMARY KEY,
  telegram_user_id INTEGER NOT NULL UNIQUE
) STRICT;

CREATE TABLE characters (
  id INTEGER PRIMARY KEY,
  account_id INTEGER NOT NULL UNIQUE REFERENCES accounts(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  xp INTEGER NOT NULL DEFAULT 0 CHECK (xp >= 0),
  level INTEGER NOT NULL DEFAULT 1 CHECK (level >= 1),
  coins INTEGER NOT NULL DEFAULT 20 CHECK (coins >= 0),
  location_id INTEGER NOT NULL DEFAULT 1,
  selected_bait_id INTEGER NOT NULL DEFAULT 1,
  equipped_rod_id INTEGER NOT NULL DEFAULT 1
) STRICT;

CREATE TABLE inventory_stacks (
  character_id INTEGER NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
  item_def_id INTEGER NOT NULL,
  quantity INTEGER NOT NULL CHECK (quantity >= 0),
  PRIMARY KEY (character_id, item_def_id)
) WITHOUT ROWID, STRICT;

CREATE TABLE items (
  id INTEGER PRIMARY KEY,
  owner_character_id INTEGER NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
  item_def_id INTEGER NOT NULL,
  created_at_ms INTEGER NOT NULL
) STRICT;

CREATE TABLE catches (
  item_id INTEGER PRIMARY KEY REFERENCES items(id) ON DELETE CASCADE,
  species_id INTEGER NOT NULL,
  length_mm INTEGER NOT NULL CHECK (length_mm > 0),
  weight_g INTEGER NOT NULL CHECK (weight_g > 0),
  seed INTEGER NOT NULL,
  generator_version INTEGER NOT NULL,
  caught_at_ms INTEGER NOT NULL,
  location_id INTEGER NOT NULL,
  bait_id INTEGER NOT NULL,
  weather INTEGER NOT NULL,
  game_minute INTEGER NOT NULL CHECK (game_minute >= 0 AND game_minute < 1440)
) STRICT;

CREATE TABLE discoveries (
  character_id INTEGER NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
  kind INTEGER NOT NULL,
  subject_id INTEGER NOT NULL,
  discovered_at_ms INTEGER NOT NULL,
  PRIMARY KEY (character_id, kind, subject_id)
) WITHOUT ROWID, STRICT;

CREATE TABLE global_discoveries (
  kind INTEGER NOT NULL,
  subject_id INTEGER NOT NULL,
  character_id INTEGER NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
  discovered_at_ms INTEGER NOT NULL,
  PRIMARY KEY (kind, subject_id)
) WITHOUT ROWID, STRICT;

CREATE TABLE fishing_encounters (
  id INTEGER PRIMARY KEY,
  character_id INTEGER NOT NULL UNIQUE REFERENCES characters(id) ON DELETE CASCADE,
  chat_id INTEGER NOT NULL,
  message_id INTEGER NOT NULL,
  step INTEGER NOT NULL,
  phase INTEGER NOT NULL,
  species_id INTEGER NOT NULL,
  seed INTEGER NOT NULL,
  location_id INTEGER NOT NULL,
  bait_id INTEGER NOT NULL,
  rod_id INTEGER NOT NULL,
  weather INTEGER NOT NULL,
  game_minute INTEGER NOT NULL,
  opened_at_ms INTEGER,
  created_at_ms INTEGER NOT NULL
) STRICT;

CREATE TABLE timers (
  id INTEGER PRIMARY KEY,
  due_at_ms INTEGER NOT NULL,
  kind INTEGER NOT NULL,
  entity_id INTEGER NOT NULL,
  step INTEGER NOT NULL,
  UNIQUE (kind, entity_id, step)
) STRICT;

CREATE INDEX timers_due ON timers(due_at_ms, id);

PRAGMA user_version = 1;
