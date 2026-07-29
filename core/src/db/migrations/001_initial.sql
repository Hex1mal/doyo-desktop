CREATE TABLE nodes (
    id          TEXT PRIMARY KEY NOT NULL,
    parent_id   TEXT REFERENCES nodes(id) ON DELETE CASCADE,
    position    REAL NOT NULL DEFAULT 0.0,
    type        TEXT NOT NULL DEFAULT 'Task' CHECK (type IN ('Workspace', 'Group', 'Task', 'Note', 'Attachment', 'Comment')),
    title       TEXT NOT NULL DEFAULT '',
    body        TEXT NOT NULL DEFAULT '',
    properties  TEXT NOT NULL DEFAULT '{}',
    is_collapsed INTEGER NOT NULL DEFAULT 0,
    is_completed INTEGER NOT NULL DEFAULT 0,
    completed_at TEXT,
    deleted_at  TEXT,
    version     INTEGER NOT NULL DEFAULT 1,
    clock       TEXT NOT NULL DEFAULT '',
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now'))
);

CREATE INDEX idx_nodes_parent_id ON nodes(parent_id);
CREATE INDEX idx_nodes_type ON nodes(type);
CREATE INDEX idx_nodes_parent_position ON nodes(parent_id, position);
CREATE INDEX idx_nodes_deleted_at ON nodes(deleted_at);
CREATE INDEX idx_nodes_type_parent ON nodes(type, parent_id);

CREATE INDEX idx_nodes_task_due ON nodes(type, json_extract(properties, '$.due_date'))
    WHERE type = 'Task' AND json_extract(properties, '$.due_date') IS NOT NULL;
CREATE INDEX idx_nodes_task_priority ON nodes(type, json_extract(properties, '$.priority'))
    WHERE type = 'Task' AND json_extract(properties, '$.priority') IS NOT NULL;
CREATE INDEX idx_nodes_task_completed ON nodes(type, is_completed, completed_at)
    WHERE type = 'Task';

CREATE VIRTUAL TABLE nodes_fts USING fts5(
    node_id UNINDEXED,
    title,
    body,
    tags,
    tokenize='unicode61 remove_diacritics 2'
);

CREATE TABLE tags (
    id          TEXT PRIMARY KEY NOT NULL,
    name        TEXT NOT NULL UNIQUE COLLATE NOCASE,
    color       TEXT,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now'))
);

CREATE TABLE node_tags (
    node_id     TEXT NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    tag_id      TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (node_id, tag_id)
);

CREATE INDEX idx_node_tags_tag_id ON node_tags(tag_id);

CREATE TABLE attachments (
    id          TEXT PRIMARY KEY NOT NULL,
    node_id     TEXT NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    filename    TEXT NOT NULL,
    mime_type   TEXT NOT NULL,
    size_bytes  INTEGER NOT NULL,
    file_path   TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now'))
);

CREATE INDEX idx_attachments_node ON attachments(node_id);

CREATE TABLE activity_log (
    id          TEXT PRIMARY KEY NOT NULL,
    node_id     TEXT NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    action      TEXT NOT NULL,
    changes     TEXT NOT NULL,
    timestamp   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now'))
);

CREATE INDEX idx_activity_node ON activity_log(node_id);
CREATE INDEX idx_activity_timestamp ON activity_log(timestamp);

CREATE TABLE settings (
    key         TEXT PRIMARY KEY NOT NULL,
    value       TEXT NOT NULL,
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now'))
);

CREATE TABLE plugins (
    id              TEXT PRIMARY KEY NOT NULL,
    name            TEXT NOT NULL,
    version         TEXT NOT NULL,
    enabled         INTEGER NOT NULL DEFAULT 1,
    permissions     TEXT NOT NULL DEFAULT '[]',
    settings        TEXT NOT NULL DEFAULT '{}',
    installed_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now'))
);
