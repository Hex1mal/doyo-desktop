CREATE TABLE IF NOT EXISTS saved_filters (
    id          TEXT PRIMARY KEY NOT NULL,
    name        TEXT NOT NULL,
    definition  TEXT NOT NULL DEFAULT '{}',
    position    REAL NOT NULL DEFAULT 0.0,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now')),
    CHECK (length(trim(name)) > 0)
);

CREATE INDEX IF NOT EXISTS idx_saved_filters_position ON saved_filters(position);

CREATE TABLE IF NOT EXISTS habits (
    id            TEXT PRIMARY KEY NOT NULL,
    title         TEXT NOT NULL,
    icon          TEXT NOT NULL DEFAULT '',
    color         TEXT,
    frequency     TEXT NOT NULL DEFAULT 'daily' CHECK (frequency IN ('daily', 'weekly')),
    goal          REAL NOT NULL DEFAULT 1.0,
    goal_unit     TEXT NOT NULL DEFAULT 'count',
    start_date    TEXT NOT NULL,
    reminder_time TEXT,
    archived      INTEGER NOT NULL DEFAULT 0,
    position      REAL NOT NULL DEFAULT 0.0,
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now')),
    updated_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now')),
    CHECK (length(trim(title)) > 0),
    CHECK (goal > 0)
);

CREATE INDEX IF NOT EXISTS idx_habits_archived_position ON habits(archived, position);

CREATE TABLE IF NOT EXISTS habit_logs (
    id          TEXT PRIMARY KEY NOT NULL,
    habit_id    TEXT NOT NULL REFERENCES habits(id) ON DELETE CASCADE,
    log_date    TEXT NOT NULL,
    status      TEXT NOT NULL CHECK (status IN ('completed', 'skipped', 'partial')),
    value       REAL NOT NULL DEFAULT 1.0,
    note        TEXT NOT NULL DEFAULT '',
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now')),
    UNIQUE(habit_id, log_date)
);

CREATE INDEX IF NOT EXISTS idx_habit_logs_habit_date ON habit_logs(habit_id, log_date);
CREATE INDEX IF NOT EXISTS idx_habit_logs_date ON habit_logs(log_date);

CREATE TABLE IF NOT EXISTS countdowns (
    id            TEXT PRIMARY KEY NOT NULL,
    title         TEXT NOT NULL,
    target_date   TEXT NOT NULL,
    mode          TEXT NOT NULL DEFAULT 'countdown' CHECK (mode IN ('countdown', 'countup')),
    icon          TEXT NOT NULL DEFAULT '',
    color         TEXT,
    recurrence    TEXT,
    reminder_at   TEXT,
    archived      INTEGER NOT NULL DEFAULT 0,
    position      REAL NOT NULL DEFAULT 0.0,
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now')),
    updated_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now')),
    CHECK (length(trim(title)) > 0)
);

CREATE INDEX IF NOT EXISTS idx_countdowns_archived_position ON countdowns(archived, position);
CREATE INDEX IF NOT EXISTS idx_countdowns_target_date ON countdowns(target_date);
