# User Guide

## Hierarchy

Doyo organizes work as:

```text
Workspace
  Group
    Subgroup
      Task
        Subtask
```

There is no fixed nesting limit. Context menus show only valid actions for the selected item.

## Main Views

- **Today**: Active tasks due today.
- **Inbox**: Tasks without a specific workflow location.
- **Next 7 Days**: Upcoming and overdue dated tasks.
- **Completed**: Completed tasks with grouping and search.
- **Trash**: Soft-deleted records with restore and permanent-delete actions.
- **Search**: Full-text search across tasks.
- **Filters**: Temporary and saved task filters.
- **Calendar**: Month, Week, Day, and Agenda planning.
- **Kanban**: Columns by status, priority, tag, workspace, or group.
- **Timeline**: Date-range planning.
- **Productivity**: Focus sessions and task prioritization methods.
- **Habits**: Daily and weekly habit tracking.
- **Countdowns**: Countdown and count-up records.
- **Statistics**: Overview, Tasks, Focus, and Habits summaries.
- **Settings**: Preferences, backup, import, export, and data tools.

## Common Actions

- Right-click a row to open contextual actions.
- Use `Shift+F10` or the Menu key to open the selected row's context menu.
- Use the inspector to edit title, description, priority, due date, tags, and task-specific metadata.
- Drag supported cards/items in Calendar, Kanban, Timeline, Matrix, and GTD views.

## Completion Policies

Doyo supports three parent-task completion policies:

- **Individual**: Complete only the selected task.
- **Ask**: Confirm before completing incomplete descendants.
- **Cascade**: Complete the selected task and all descendant tasks.

Set the policy in Settings or the inspector controls where available.

## Backup And Restore

Open Settings -> Data and Backup.

- **Create Backup** writes a local SQLite backup.
- **Restore Backup** replaces the current database with the selected backup and should be followed by an app restart.
- Safety backup before restore can be enabled in Settings.

## Import And Export

Open Settings -> Import and Export.

- JSON export/import preserves Doyo's structured data.
- Markdown export creates readable task documents.

## Data Migration From TodoApp

On first Doyo launch, if the Doyo database is missing and an old TodoApp database exists, Doyo copies the old data into the new app data directory. The old TodoApp directory remains untouched as a recovery copy.
