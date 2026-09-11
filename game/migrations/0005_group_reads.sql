ALTER TABLE group_event_claims ADD COLUMN approach INTEGER NOT NULL DEFAULT 0 CHECK (approach BETWEEN 0 AND 2);
ALTER TABLE group_event_claims ADD COLUMN read_correct INTEGER NOT NULL DEFAULT 0 CHECK (read_correct IN (0, 1));

PRAGMA user_version = 5;
