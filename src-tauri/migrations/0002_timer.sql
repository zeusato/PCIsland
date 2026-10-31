CREATE TABLE timer_items (
  id TEXT PRIMARY KEY,
  kind TEXT NOT NULL,            -- countdown | stopwatch | pomodoro
  label TEXT,
  state_json TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
CREATE TABLE timer_pomodoro_sessions (
  id TEXT PRIMARY KEY,
  phase TEXT NOT NULL,           -- work | short_break | long_break
  planned_ms INTEGER NOT NULL,
  started_at INTEGER NOT NULL,
  ended_at INTEGER NOT NULL,
  completed INTEGER NOT NULL
);
CREATE INDEX timer_pomodoro_sessions_started ON timer_pomodoro_sessions (started_at);
