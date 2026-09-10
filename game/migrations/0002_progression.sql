CREATE TABLE character_locations (
  character_id INTEGER NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
  location_id INTEGER NOT NULL,
  discovered_at_ms INTEGER NOT NULL,
  PRIMARY KEY (character_id, location_id)
) WITHOUT ROWID, STRICT;

CREATE TABLE character_rods (
  character_id INTEGER NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
  rod_id INTEGER NOT NULL,
  acquired_at_ms INTEGER NOT NULL,
  PRIMARY KEY (character_id, rod_id)
) WITHOUT ROWID, STRICT;

INSERT OR IGNORE INTO character_locations (character_id, location_id, discovered_at_ms)
SELECT id, 1, 0 FROM characters;
INSERT OR IGNORE INTO character_locations (character_id, location_id, discovered_at_ms)
SELECT id, location_id, 0 FROM characters;
INSERT OR IGNORE INTO character_rods (character_id, rod_id, acquired_at_ms)
SELECT id, 1, 0 FROM characters;
INSERT OR IGNORE INTO character_rods (character_id, rod_id, acquired_at_ms)
SELECT id, equipped_rod_id, 0 FROM characters;

ALTER TABLE fishing_encounters ADD COLUMN special_id INTEGER NOT NULL DEFAULT 0;
ALTER TABLE items ADD COLUMN sold_at_ms INTEGER;

PRAGMA user_version = 2;
