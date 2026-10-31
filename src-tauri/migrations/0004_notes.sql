CREATE TABLE notes_items (
  id TEXT PRIMARY KEY,
  text TEXT NOT NULL,
  normalized TEXT NOT NULL,
  pinned INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  deleted_at INTEGER
);
