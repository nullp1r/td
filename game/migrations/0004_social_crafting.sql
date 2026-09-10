ALTER TABLE characters ADD COLUMN title_id INTEGER NOT NULL DEFAULT 0 CHECK (title_id BETWEEN 0 AND 64);

CREATE TABLE craft_consumptions (
  item_id INTEGER PRIMARY KEY REFERENCES items(id),
  recipe_id INTEGER NOT NULL,
  consumed_at_ms INTEGER NOT NULL
) STRICT;

CREATE TABLE group_event_claims (
  chat_id INTEGER NOT NULL,
  cycle INTEGER NOT NULL,
  character_id INTEGER NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
  species_id INTEGER NOT NULL,
  item_id INTEGER NOT NULL REFERENCES items(id),
  claimed_at_ms INTEGER NOT NULL,
  PRIMARY KEY (chat_id, cycle, character_id)
) WITHOUT ROWID, STRICT;

CREATE INDEX group_event_claims_cycle ON group_event_claims(chat_id, cycle);

PRAGMA user_version = 4;
