CREATE TABLE managed_character (
    position INTEGER PRIMARY KEY CHECK (position BETWEEN 0 AND 5),
    ocid TEXT NOT NULL UNIQUE CHECK (length(ocid) > 0),
    account_id TEXT NOT NULL CHECK (length(account_id) > 0),
    world_name TEXT NOT NULL CHECK (length(world_name) > 0),
    is_main INTEGER NOT NULL CHECK ((position = 0 AND is_main = 1) OR (position > 0 AND is_main = 0)),
    character_name TEXT,
    character_class TEXT,
    character_level INTEGER CHECK (character_level IS NULL OR character_level >= 0)
);
CREATE TABLE selection_state (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    revision INTEGER NOT NULL CHECK (revision >= 0)
);
INSERT INTO selection_state VALUES (1, 0);
INSERT INTO managed_character (position, ocid, account_id, world_name, is_main, character_name, character_class, character_level)
SELECT 0, ocid, account_id, world_name, 1, character_name, character_class, character_level FROM main_character WHERE singleton = 1;
PRAGMA user_version = 2;
