CREATE TABLE IF NOT EXISTS focus_sessions (
    id                  TEXT PRIMARY KEY NOT NULL,
    task_id             TEXT REFERENCES nodes(id) ON DELETE SET NULL,
    task_title          TEXT NOT NULL DEFAULT '',
    method              TEXT NOT NULL CHECK (method IN ('pomodoro', 'stopwatch')),
    state               TEXT NOT NULL CHECK (state IN ('running', 'paused', 'completed', 'stopped')),
    pomodoro_phase      TEXT CHECK (pomodoro_phase IN ('focus', 'short_break', 'long_break')),
    pomodoro_cycle      INTEGER NOT NULL DEFAULT 1,
    planned_seconds     INTEGER NOT NULL DEFAULT 0,
    accumulated_seconds INTEGER NOT NULL DEFAULT 0,
    duration_seconds    INTEGER NOT NULL DEFAULT 0,
    interruptions       INTEGER NOT NULL DEFAULT 0,
    note                TEXT NOT NULL DEFAULT '',
    started_at          TEXT NOT NULL,
    last_started_at     TEXT,
    ended_at            TEXT,
    created_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now')),
    updated_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now'))
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_focus_one_active
ON focus_sessions((1))
WHERE state IN ('running', 'paused');

CREATE INDEX IF NOT EXISTS idx_focus_sessions_task_id ON focus_sessions(task_id);
CREATE INDEX IF NOT EXISTS idx_focus_sessions_started_at ON focus_sessions(started_at);
CREATE INDEX IF NOT EXISTS idx_focus_sessions_state ON focus_sessions(state);
