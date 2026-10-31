CREATE TABLE reminders_items (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  note TEXT,
  due_at INTEGER NOT NULL,
  repeat_json TEXT NOT NULL,
  lead_min INTEGER NOT NULL DEFAULT 0,
  snoozed_until INTEGER,
  status TEXT NOT NULL,          -- pending | ringing | missed | done
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  completed_at INTEGER
);
CREATE INDEX reminders_items_status_due ON reminders_items (status, due_at);
