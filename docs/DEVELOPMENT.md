# Development

## Setup

```bash
npm ci
```

## Common Commands

```bash
npm run tauri dev
npm run check
npm run test:unit
npm run build
cargo test
cargo check
npm run tauri build
```

## Code Layout

- `src/lib/api/client.ts`: frontend/backend command boundary and casing normalization
- `src/lib/stores/`: Svelte stores for app state
- `src/lib/utils/`: shared projection, calendar, statistics, and productivity logic
- `src/lib/components/`: UI components
- `src-tauri/src/lib.rs`: Tauri commands and app startup
- `core/src/`: Rust services and SQLite migrations
- `core/tests/`: Rust integration tests

## Database Changes

Use additive migrations only. Register new migrations in `core/src/db/migrations.rs` and add integration tests for data compatibility.

## Public Repository Hygiene

Do not commit:

- local SQLite databases
- backups
- screenshots containing private data
- logs
- generated build artifacts
- local absolute-path configuration
