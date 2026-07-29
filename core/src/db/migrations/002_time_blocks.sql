CREATE TABLE IF NOT EXISTS time_blocks (
    id          TEXT PRIMARY KEY NOT NULL,
    task_id     TEXT REFERENCES nodes(id) ON DELETE SET NULL,
    title       TEXT NOT NULL DEFAULT '',
    start_time  TEXT NOT NULL,
    end_time    TEXT NOT NULL,
    all_day     INTEGER NOT NULL DEFAULT 0,
    notes       TEXT NOT NULL DEFAULT '',
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now')),
    CHECK (datetime(end_time) > datetime(start_time))
);

CREATE INDEX IF NOT EXISTS idx_time_blocks_start ON time_blocks(start_time);
CREATE INDEX IF NOT EXISTS idx_time_blocks_task ON time_blocks(task_id);
