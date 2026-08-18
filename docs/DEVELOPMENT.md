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

## Roadmap

- Packaging polish and signed release artifacts
- Optional importers and exporters for common task formats
- Accessibility and keyboard workflow refinements

## Android

An Android port is under evaluation. `docs/ANDROID.md` records the compatibility
audit, the toolchain prerequisites, and what is not supported yet. No Android
target has been generated, and the public README does not advertise Android.

## Secret Scanning

CI runs `gitleaks/gitleaks-action@v2`, which executes `gitleaks detect` over the
commit range of the push or pull request. The gitleaks version is pinned with
`GITLEAKS_VERSION` in `.github/workflows/ci.yml` so the scanner CI runs is the
same one `.gitleaks.toml` is tested against.

Reproduce a CI scan locally with the pinned version:

```bash
gitleaks detect --source . --no-banner --redact -v
```

To check the exact CI build rather than a distribution package:

```bash
curl -sSL -o gitleaks.tar.gz \
  https://github.com/zricethezav/gitleaks/releases/download/v8.24.3/gitleaks_8.24.3_linux_x64.tar.gz
tar -xzf gitleaks.tar.gz gitleaks && ./gitleaks detect --source . --no-banner
```

### Why The Previous Allowlist Did Nothing

Doyo's localStorage keys (`doyo.uiPrefs.v1`, `doyo.sentReminderKeys.v1`) are
assigned to constants named `PREF_KEY` and `SENT_KEY`, so the default
`generic-api-key` rule reads the key name as a credential value.

The first attempt at suppressing this used a top-level `[[allowlists]]` array.
Support for that table is version dependent, measured against this repository:

| gitleaks | top-level `[[allowlists]]` | findings with the old config |
| -------- | -------------------------- | ---------------------------- |
| 8.24.3   | ignored                    | 2                            |
| 8.25.0   | honoured                   | 0                            |
| 8.25.1   | honoured                   | 0                            |
| 8.26.0   | honoured                   | 0                            |

8.24.3 is the version `gitleaks-action@v2` installs by default, and therefore
the version CI was actually running. It parses the array without error and then
ignores it, so the allowlist had no effect and the findings were suppressed only
by fingerprint entries in `.gitleaksignore`. The configuration appeared correct
when tested against a newer locally installed gitleaks, which is what hid the
problem.

Rule sensitivity moves between versions as well: the `ui-prefs.test.ts` finding
recorded in the old `.gitleaksignore` no longer reproduces under 8.24.3, but
does reproduce under 8.25.0. Pinning the version is what makes any of this
testable.

Two follow-on traps are worth knowing about:

- A top-level `[allowlist]` (singular) _is_ honoured by 8.24.3, but `condition`
  is not. A singular allowlist combining `paths` and `regexes` therefore falls
  back to OR, and the `paths` entry alone exempts those whole files from _every_
  rule. Testing this variant showed a planted AWS key and GitHub PAT going
  undetected.
- Allowlist `regexes` are matched against the detected secret, not the source
  line, so the pattern has to describe the captured value.

The current configuration attaches the allowlist to the `generic-api-key` rule
only and matches the captured secret against Doyo's localStorage key naming
convention. No path or file type is exempted, and redefining the rule id merges
the allowlist into the default rule rather than replacing it. Verified against
8.24.3, 8.25.0, 8.25.1 and 8.26.0: the repository scans clean with no
`.gitleaksignore` present, while planted AWS keys, GitHub PATs, and
high-entropy generic keys are still reported inside the same two store files.

When changing `.gitleaks.toml`, test against the pinned version with a fixture
that contains both the false positives and known-detectable secrets. A config
that reports zero findings is not evidence that it works.

## Public Repository Hygiene

Do not commit:

- local SQLite databases
- backups
- screenshots containing private data
- logs
- generated build artifacts
- local absolute-path configuration
