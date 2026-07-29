# Architecture

Doyo is a local-first desktop application. The frontend, desktop shell, and database all run on the user's machine.

## Layers

| Layer | Path | Responsibility |
|---|---|---|
| Frontend | `src/` | SvelteKit UI, stores, projections, keyboard handling, and view components |
| Desktop shell | `src-tauri/` | Tauri window, commands, app data path, notifications, backup/restore entry points |
| Core library | `core/` | SQLite access, migrations, domain services, validation, and integration tests |

## Data Flow

1. Svelte components call typed helpers in `src/lib/api/client.ts`.
2. The API client normalizes camelCase frontend data to snake_case Tauri command payloads.
3. Tauri commands in `src-tauri/src/lib.rs` call Rust services in `core/`.
4. Rust services validate and persist data in SQLite.
5. Frontend stores reload shared source records so smart views stay consistent.

## Source Of Truth

Tasks, subtasks, groups, subgroups, and workspaces all come from the `nodes` table. Semantic labels are derived from node type and parent context:

- `Workspace`: root workspace
- `Group`: group directly under a workspace
- `Subgroup`: group under another group
- `Task`: task under a workspace or group/subgroup
- `Subtask`: task under another task

Productivity views, Calendar, Kanban, Timeline, tags, filters, and statistics use the same persisted task records rather than duplicating tasks.

## Migrations

Database migrations are additive and registered in `core/src/db/migrations.rs`. The current schema version is `6`.

## Local Data Migration

The Doyo Tauri identifier is `io.github.sembee.doyo`. On startup, if `doyo.db` does not exist in the new app data directory, Doyo checks for the older `com.todoapp.desktop` data directory and copies compatible data into the new location. Existing Doyo data is never overwritten silently, and the old directory is left untouched.
