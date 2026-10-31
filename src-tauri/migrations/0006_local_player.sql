CREATE TABLE local_player_folders (
  id TEXT PRIMARY KEY,
  path TEXT NOT NULL UNIQUE,
  added_at INTEGER NOT NULL,
  last_scan_at INTEGER
);
CREATE TABLE local_player_tracks (
  id TEXT PRIMARY KEY,
  path TEXT NOT NULL UNIQUE,
  folder_id TEXT REFERENCES local_player_folders(id) ON DELETE CASCADE,
  title TEXT NOT NULL,
  artist TEXT,
  album TEXT,
  track_no INTEGER,
  duration_ms INTEGER,
  has_artwork INTEGER NOT NULL DEFAULT 0,
  size INTEGER NOT NULL,
  mtime INTEGER NOT NULL,
  normalized TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
CREATE INDEX local_player_tracks_folder ON local_player_tracks (folder_id);
CREATE TABLE local_player_playlists (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
CREATE TABLE local_player_playlist_items (
  playlist_id TEXT NOT NULL REFERENCES local_player_playlists(id) ON DELETE CASCADE,
  position INTEGER NOT NULL,
  track_id TEXT NOT NULL REFERENCES local_player_tracks(id) ON DELETE CASCADE,
  PRIMARY KEY (playlist_id, position)
);
