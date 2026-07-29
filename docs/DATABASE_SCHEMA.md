# Database Schema

Doyo stores data in SQLite at the Tauri app data directory for `io.github.hex1mal.doyo`.

Typical Linux path:

```text
~/.local/share/io.github.hex1mal.doyo/doyo.db
```

## Versioning

Schema versions are recorded in `schema_version`. The current release uses version `6`.

| Version | Purpose                                                                    |
| ------- | -------------------------------------------------------------------------- |
| 1       | Initial nodes, tags, attachments, activity log, settings, plugins, and FTS |
| 2       | Calendar time blocks                                                       |
| 3       | Focus sessions                                                             |
| 4       | Saved filters, habits, habit logs, and countdowns                          |
| 5       | Weekly habit days                                                          |
| 6       | Focus workflow metadata                                                    |

## Core Tables

| Table            | Purpose                                                                    |
| ---------------- | -------------------------------------------------------------------------- |
| `nodes`          | Unified workspaces, groups, tasks, notes, and related records              |
| `tags`           | Normalized tag definitions                                                 |
| `node_tags`      | Many-to-many task tag assignments                                          |
| `time_blocks`    | Calendar planning records, optionally linked to a task                     |
| `focus_sessions` | Pomodoro, Stopwatch, and Flowtime history                                  |
| `habits`         | Habit definitions                                                          |
| `habit_logs`     | Daily habit completions, skips, and partial logs                           |
| `countdowns`     | Countdown and count-up records                                             |
| `saved_filters`  | Saved filter definitions                                                   |
| `settings`       | SQLite-backed application preferences                                      |
| `attachments`    | Attachment metadata                                                        |
| `activity_log`   | Local activity records                                                     |
| `plugins`        | Legacy plugin registry table retained for additive migration compatibility |

## Hierarchy Rules

`nodes.parent_id` creates the recursive tree.

- Workspaces have no parent.
- Groups can live under workspaces or other groups.
- Tasks can live under workspaces, groups, or other tasks.
- A task under another task is displayed as a Subtask.

Backend validation prevents invalid parent/child relationships and cycles.

## Backups

Doyo backups are SQLite database copies stored under:

```text
~/.local/share/io.github.hex1mal.doyo/backups/
```

Backup filenames start with `doyo-backup-`.

JSON exports are transfer envelopes, not database backups. They recreate portable records through import logic. Use SQLite backups for exact restore.
