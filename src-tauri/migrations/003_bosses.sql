CREATE TABLE record_characters (
    ocid TEXT PRIMARY KEY NOT NULL,
    account_id TEXT NOT NULL,
    world_name TEXT NOT NULL,
    character_json TEXT NOT NULL CHECK(json_valid(character_json))
);
INSERT INTO record_characters SELECT ocid, account_id, world_name,
    json_object('identity',json_object('ocid',ocid,'account_id',account_id,'world_name',world_name),
        'character_name',character_name,'character_class',character_class,'character_level',character_level)
    FROM managed_character;
CREATE TABLE character_boss_plans (
    ocid TEXT NOT NULL REFERENCES record_characters(ocid),
    boss_id TEXT NOT NULL,
    variant_id TEXT NOT NULL,
    enabled INTEGER NOT NULL CHECK(enabled IN (0,1)),
    revision INTEGER NOT NULL CHECK(revision>0),
    PRIMARY KEY(ocid,boss_id)
);
CREATE TABLE boss_periods (
    start_day INTEGER PRIMARY KEY,
    end_day INTEGER NOT NULL CHECK(end_day=start_day+7),
    timezone TEXT NOT NULL CHECK(timezone='Asia/Seoul'),
    rule_version TEXT NOT NULL
);
CREATE TABLE boss_clear_records (
    id INTEGER PRIMARY KEY,
    ocid TEXT NOT NULL REFERENCES record_characters(ocid),
    start_day INTEGER NOT NULL REFERENCES boss_periods(start_day),
    clear_group TEXT NOT NULL,
    current_revision INTEGER NOT NULL CHECK(current_revision>0),
    UNIQUE(ocid,start_day,clear_group),
    FOREIGN KEY(id,current_revision) REFERENCES boss_clear_revisions(record_id,revision)
        DEFERRABLE INITIALLY DEFERRED
);
CREATE TABLE boss_clear_revisions (
    record_id INTEGER NOT NULL REFERENCES boss_clear_records(id),
    revision INTEGER NOT NULL CHECK(revision>0),
    request_id TEXT NOT NULL UNIQUE,
    request_json TEXT NOT NULL CHECK(json_valid(request_json)),
    snapshot_json TEXT NOT NULL CHECK(json_valid(snapshot_json)),
    PRIMARY KEY(record_id,revision)
);
CREATE INDEX boss_clear_week ON boss_clear_records(start_day,ocid);
PRAGMA user_version=3;
