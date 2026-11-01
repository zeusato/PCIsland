CREATE TABLE note_tasks (
    id TEXT PRIMARY KEY,
    note_id TEXT NOT NULL REFERENCES notes_items(id) ON DELETE CASCADE,
    line_index INTEGER NOT NULL,
    label TEXT NOT NULL,
    checked INTEGER NOT NULL DEFAULT 0,
    completed_at INTEGER,
    alarm_mode TEXT,
    alarm_at INTEGER,
    delay_ms INTEGER,
    due_at INTEGER,
    fired_at INTEGER,
    predecessor_id TEXT,
    UNIQUE(note_id, line_index)
);
CREATE INDEX note_tasks_due ON note_tasks(due_at) WHERE checked = 0;
