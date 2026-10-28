CREATE TABLE boss_price_settings (
 variant_id TEXT PRIMARY KEY,
 price TEXT,
 reason TEXT NOT NULL,
 revision INTEGER NOT NULL CHECK(revision > 0),
 saved_at INTEGER NOT NULL
);
CREATE TABLE boss_api_observations (
 id INTEGER PRIMARY KEY,
 ocid TEXT NOT NULL,
 start_day INTEGER NOT NULL,
 query_date TEXT NOT NULL,
 observed_at INTEGER NOT NULL,
 selection_revision INTEGER NOT NULL,
 character_json TEXT NOT NULL,
 entries_json TEXT NOT NULL
);
CREATE TABLE boss_api_acceptances (
 ocid TEXT NOT NULL,
 start_day INTEGER NOT NULL,
 clear_group TEXT NOT NULL,
 observation_id INTEGER NOT NULL REFERENCES boss_api_observations(id),
 request_id TEXT NOT NULL UNIQUE REFERENCES boss_clear_revisions(request_id),
 PRIMARY KEY(ocid,start_day,clear_group)
);
PRAGMA user_version = 4;
