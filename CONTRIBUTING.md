# Contributing To Doyo

Thanks for considering a contribution.

## Development Setup

Install Node.js 22, Rust, Cargo, and the Linux system libraries listed in `docs/INSTALLATION.md`.

```bash
npm ci
npm run tauri dev
```

## Checks

Run these before opening a pull request:

```bash
npm run check
npm run test:unit
npm run build
cargo test
cargo check
```

## Guidelines

- Keep Doyo local-first and offline-capable.
- Do not add duplicate task storage systems.
- Preserve the recursive Workspace -> Group/Subgroup -> Task/Subtask model.
- Use additive database migrations only.
- Keep user data out of commits.
- Document user-visible changes.

## Pull Requests

Use the pull request template and include the checks you ran. For migrations, explain compatibility and rollback expectations.
