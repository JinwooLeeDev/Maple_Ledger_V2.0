CREATE TABLE main_character (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    account_id TEXT NOT NULL CHECK (length(account_id) > 0),
    ocid TEXT NOT NULL CHECK (length(ocid) > 0),
    world_name TEXT NOT NULL CHECK (length(world_name) > 0),
    character_name TEXT,
    character_class TEXT,
    character_level INTEGER CHECK (character_level IS NULL OR character_level >= 0)
);
PRAGMA user_version = 1;
