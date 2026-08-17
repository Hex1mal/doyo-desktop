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

## Sidebar And Navigation

The top sidebar area keeps secondary views compact so the Workspaces tree receives most of the available height. Tags and saved filters are available from compact expandable sections. Long workspace and group names are truncated in the tree and can be read from the row tooltip.

Use Favorites for shortcuts to important workspaces, groups, tasks, or subtasks. Opening Favorites clears the previous workspace view and shows only current non-deleted favorites. Selecting a favorite reveals the original item in its hierarchy.

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
- Use Change color in a row context menu to set a subtle color accent. Choose Default to remove the custom color.
- Drag workspaces up or down in the Workspaces tree, or use Move up and Move down from a workspace context menu.
- Drag groups, subgroups, tasks, and subtasks in the main tree to reorder siblings or move into valid parents. Cross-parent moves ask for confirmation and keep the existing descendants.
- Use the inspector to edit title, description, priority, schedule, tags, and task-specific metadata.
- Drag supported cards/items in Calendar, Kanban, Timeline, Matrix, and GTD views.

## Scheduling Tasks

Open Schedule from a task row or the inspector to set date, time, reminder, repeat, and estimated duration together.

- Pick a date from the calendar grid or use quick actions such as Today, Tomorrow, 3 Days Later, This Sunday, or No Date.
- Type time in 24-hour form such as `09:00`, `14:30`, or shorthand such as `930`.
- Reminder choices require a due date. Relative reminders require a task date/time that can be calculated.
- Repeat uses Doyo's recurrence model, including daily, weekly, and monthly rules.
- Estimated duration is a planned task estimate, such as `30m`, `1h`, or `1h 30m`. It is separate from completed Focus or Pomodoro session history.
- Cancel closes the modal without saving. Done validates and saves the scheduling fields atomically.

Today's date is shown with a visible ring and label in Calendar views and in the scheduling date picker. The selected date and today remain visually distinct.

## Keyboard Shortcuts

- `Ctrl+K`: Command palette
- `Ctrl+P`: Quick open
- `Ctrl++` or `Ctrl+=`: Zoom in
- `Ctrl+-`: Zoom out
- `Ctrl+0`: Reset zoom
- `Enter`: Create or confirm where supported
- `Space`: Select focused node
- `Shift+F10` or Menu key: Open the context menu for the selected row

Desktop shortcuts, such as `Super+T`, are configured through the operating system.

## Zoom

Use `Ctrl++` or `Ctrl+=` to zoom in, `Ctrl+-` to zoom out, and `Ctrl+0` to reset. The same controls are available in Settings -> Appearance. Zoom applies to the complete application interface and persists after restart.

## Completion Policies

Doyo supports three parent-task completion policies:

- **Individual**: Complete only the selected task.
- **Ask**: Confirm before completing incomplete descendants.
- **Cascade**: Complete the selected task and all descendant tasks.

Set the policy in Settings or the inspector controls where available.

## Backup And Restore

Open Settings -> Data and Backup.

- **Create Backup** writes a local SQLite backup.
- **Restore Backup** validates the selected file first, snapshots the current database to `doyo-pre-restore-*.db`, and reports that filename so the restore can be rolled back. The restored database is loaded immediately; no manual restart is needed.
- Safety backup before restore can be enabled in Settings.

## Import And Export

Open Settings -> Import and Export.

- JSON export/import uses a versioned transfer format for structured Doyo data.
- Full SQLite backup/restore is the exact recovery workflow.
- Markdown export creates readable task documents.

## Data Migration From TodoApp

On first Doyo launch, if the new Doyo database is missing, Doyo copies compatible data from the previous Doyo identifier or the older TodoApp identifier. Old directories remain untouched as recovery copies.
