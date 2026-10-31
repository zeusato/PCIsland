CREATE TABLE lyrics_cache (
  track_key TEXT PRIMARY KEY,
  source TEXT NOT NULL,          -- lrclib | file | none
  synced INTEGER NOT NULL,
  content TEXT,
  offset_ms INTEGER NOT NULL DEFAULT 0,
  fetched_at INTEGER NOT NULL
);
