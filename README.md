# Doyo

Doyo is a local-first desktop task manager for Linux, built with Tauri, SvelteKit, Rust, and SQLite. It keeps your workspaces, tasks, tags, calendar planning, focus sessions, habits, countdowns, and settings on your own machine.

## Privacy

Doyo works offline and stores data locally. It does not require an account, subscription, analytics service, or cloud backend.

## Features

- Recursive hierarchy: Workspace -> Group/Subgroup -> Task/Subtask with unlimited depth.
- Smart views: Inbox, Today, Next 7 Days, Completed, Trash, Search, Tags, and Filters.
- Favorites view for pinned workspace, group, task, and subtask shortcuts.
- Per-node colors, persistent workspace ordering, and drag/drop hierarchy moves.
- Calendar: Month, Week, Day, and Agenda views with task scheduling and time blocks.
- Desktop scheduling modal for due date, typed time, reminder, repeat, and estimated duration.
- Kanban and Timeline views over the same task records.
- Productivity tools: Pomodoro, Stopwatch, Timeboxing, Eisenhower Matrix, Eat the Frog, Flowtime, GTD, and Pareto.
- Habits and Countdowns with reminders.
- Statistics for tasks, focus sessions, and habits.
- Local backup, restore, import, and export tools.

## Screenshots

Screenshots are not committed by default so the repository stays small. Add release screenshots under `docs/assets/` when publishing a public release.

## Technology Stack

- Tauri 2 desktop shell
- SvelteKit and Svelte 5 frontend
- Rust core services
- SQLite with additive migrations
- Vitest and Rust integration tests

## Installation

For Debian-based Linux distributions, install the generated package:

```bash
sudo apt install ./src-tauri/target/release/bundle/deb/*.deb
```

See [Installation](docs/INSTALLATION.md) for system dependencies, build artifacts, and desktop integration notes.

## Build From Source

```bash
npm ci
npm run check
npm run test:unit
cargo test
npm run build
npm run tauri build
```

Development server:

```bash
npm run tauri dev
```

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

## Data Location

Doyo uses the Tauri application data directory for identifier `io.github.hex1mal.doyo`.

Typical Linux path:

```text
~/.local/share/io.github.hex1mal.doyo/
```

Database file:

```text
doyo.db
```

When first launched after upgrading from an older identifier, Doyo copies data from `~/.local/share/io.github.sembee.doyo/` or `~/.local/share/com.todoapp.desktop/` if the new Doyo database does not already exist. Old directories are left in place as recovery copies.

## Backup And Restore

Use Settings -> Data and Backup to create or restore local SQLite database backups. Backups are stored in the app data directory under `backups/` and are not intended to be committed to Git.

JSON import/export is a structured transfer format for moving Doyo records between databases. It is not a byte-for-byte backup. Use SQLite backup/restore for exact recovery.

## Project Structure

```text
core/        Rust core library and SQLite services
src/         SvelteKit frontend
src-tauri/   Tauri shell and desktop packaging
docs/        Public documentation
```

## Roadmap

- Packaging polish and signed release artifacts
- More documentation screenshots
- Optional importers and exporters for common task formats
- Accessibility and keyboard workflow refinements

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Doyo is licensed under AGPL-3.0-only. See [LICENSE](LICENSE).
