ALTER TABLE items RENAME COLUMN sold_at_ms TO removed_at_ms;
ALTER TABLE items ADD COLUMN removal_kind INTEGER CHECK (removal_kind IN (1, 2));

ALTER TABLE character_rods ADD COLUMN condition INTEGER NOT NULL DEFAULT 100 CHECK (condition BETWEEN 1 AND 100);

CREATE TABLE objective_claims (
  character_id INTEGER NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
  objective_id INTEGER NOT NULL,
  claimed_at_ms INTEGER NOT NULL,
  PRIMARY KEY (character_id, objective_id)
) WITHOUT ROWID, STRICT;

CREATE TABLE contract_claims (
  character_id INTEGER NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
  cycle INTEGER NOT NULL,
  species_id INTEGER NOT NULL,
  item_id INTEGER NOT NULL REFERENCES items(id),
  claimed_at_ms INTEGER NOT NULL,
  PRIMARY KEY (character_id, cycle)
) WITHOUT ROWID, STRICT;

PRAGMA user_version = 3;
