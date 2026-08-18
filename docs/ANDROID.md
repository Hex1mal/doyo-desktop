# Android

Status: **audit complete, target not initialized.**

This document records what an Android port of Doyo would share with the desktop
app, what has to change, and what is required to build it. No Android target has
been generated yet, because the build prerequisites are not installed on the
development machine — see [Prerequisites](#prerequisites).

Nothing here has been run on a device or emulator. Every statement about Android
runtime behaviour below is marked as unverified unless it was checked against
the current Tauri documentation, and even then it is a documentation claim, not
a test result.

## Why The Codebase Is A Good Candidate

The audit found a much smaller platform surface than expected:

- The Rust shell registers **one** Tauri plugin, `tauri-plugin-notification`.
  There is no `fs`, `dialog`, `shell`, `clipboard`, or `os` plugin to port.
- The frontend imports from `@tauri-apps` in exactly **two** files:
  `src/lib/api/client.ts` (`invoke`) and `src/lib/stores/ui.svelte.ts`
  (webview zoom). Everything else is ordinary web code.
- `run()` in `src-tauri/src/lib.rs` already carries
  `#[cfg_attr(mobile, tauri::mobile_entry_point)]`, so the entry point is
  already mobile-shaped.
- Storage is resolved through `app.path().app_data_dir()`, which on Android
  resolves inside the app's private storage. No path logic needs to change and
  no storage permission is required.
- Import and export move data through the clipboard and a textarea, not a file
  picker, so the one flow that usually forces a platform-specific file dialog
  does not need one.
- There is a single SvelteKit route (`src/routes/+page.svelte`); "navigation"
  is `uiStore.activeModule` state. There is no router to adapt.

The work is therefore almost entirely **interaction design for touch**, not
architecture. There is no case for a second implementation of the task logic.

## Compatibility Matrix

Verdict columns are mutually exclusive. "Reusable as-is" means the code compiles
and behaves correctly on Android with no change; it does not mean the result is
pleasant to use on a phone, which is covered by the UX section.

| Component / Module                  | Desktop behavior                                                                 | Android compatibility                                                | Reusable as-is | Needs adaptation | Desktop-only | Android replacement                            |
| ----------------------------------- | -------------------------------------------------------------------------------- | -------------------------------------------------------------------- | :------------: | :--------------: | :----------: | ---------------------------------------------- |
| Svelte UI components                | Mouse/keyboard, dense desktop layout                                             | Renders in Android WebView                                           |                |        ✓         |              | Adaptive layouts, touch target sizing          |
| Routing                             | Single route, module switching via `uiStore`                                     | No router involved                                                   |       ✓        |                  |              | —                                              |
| Stores (`src/lib/stores/`)          | Svelte 5 runes over `invoke`                                                     | Platform independent                                                 |       ✓        |                  |              | —                                              |
| Task projections (`src/lib/utils/`) | Pure functions                                                                   | Pure functions                                                       |       ✓        |                  |              | —                                              |
| Rust core (`core/`)                 | Services, models, validation                                                     | Compiles for `*-linux-android` (unverified)                          |       ✓        |                  |              | —                                              |
| SQLite (`rusqlite`, bundled)        | Bundled SQLite, WAL                                                              | Bundled build compiles for Android NDK (unverified)                  |       ✓        |                  |              | —                                              |
| Migrations (7, additive)            | Run at startup                                                                   | Same code path                                                       |       ✓        |                  |              | —                                              |
| Node repository / services          | Rust, behind `invoke`                                                            | Same                                                                 |       ✓        |                  |              | —                                              |
| Backup / restore                    | Files under `app_data_dir/backups`, names listed in UI                           | Stays inside app sandbox                                             |       ✓        |                  |              | —                                              |
| Import / export                     | Clipboard + textarea                                                             | `navigator.clipboard` needs a secure context and may need a fallback |                |        ✓         |              | Android share sheet or SAF, optional           |
| File access                         | Only Rust-side, inside `app_data_dir`                                            | Sandbox-private, no permission needed                                |       ✓        |                  |              | —                                              |
| Attachments                         | `core/attachment` exists but is not registered as a command and is not in the UI | Dormant                                                              |       ✓        |                  |              | — (not a shipped feature)                      |
| Notifications                       | `tauri-plugin-notification`, immediate                                           | Plugin documents Android support                                     |       ✓        |                  |              | —                                              |
| Reminders                           | `setInterval(checkDue, 60_000)` in the WebView                                   | Timer stops when the app is backgrounded or killed                   |                |        ✓         |              | `Schedule.at(...)` from the same plugin        |
| Dialogs                             | Custom Svelte modals, plus 9 `window.confirm` calls                              | Renders, but `window.confirm` is a system dialog                     |                |        ✓         |              | Themed modals (the restore one already exists) |
| Keyboard shortcuts                  | Global handler on `window`                                                       | No hardware keyboard on a phone                                      |                |                  |      ✓       | Every action must also have a touch path       |
| Context menus                       | `oncontextmenu` in 3 components                                                  | No right click on touch                                              |                |                  |      ✓       | Long-press, or an overflow menu                |
| Drag and drop                       | Pointer events in 5 components                                                   | Pointer events fire, but conflict with scrolling                     |                |        ✓         |              | Explicit move action; drag as an enhancement   |
| Window management                   | Sidebar/inspector widths, zoom                                                   | No windows on Android                                                |                |                  |      ✓       | Full-screen views and sheets                   |
| Filesystem APIs                     | Rust `std::fs` inside `app_data_dir`                                             | Same                                                                 |       ✓        |                  |              | —                                              |
| Shell APIs                          | None used                                                                        | —                                                                    |       ✓        |                  |              | —                                              |
| Clipboard                           | `navigator.clipboard.writeText`                                                  | Available in WebView, context-dependent                              |                |        ✓         |              | Fallback for the export flow                   |
| Settings                            | `settings` table + localStorage prefs                                            | Both available                                                       |       ✓        |                  |              | —                                              |
| Theme                               | CSS custom properties, `[data-theme]`                                            | Same                                                                 |       ✓        |                  |              | Optional: follow the system theme              |
| Focus timers                        | `setInterval` in the WebView                                                     | Drifts or stops when backgrounded                                    |                |        ✓         |              | Persist start timestamp, recompute on resume   |
| Habits                              | Rust + Svelte over the same DB                                                   | Platform independent                                                 |       ✓        |                  |              | —                                              |
| Countdowns                          | Rust + Svelte over the same DB                                                   | Platform independent                                                 |       ✓        |                  |              | —                                              |
| Statistics                          | Pure computation over queries                                                    | Platform independent                                                 |       ✓        |                  |              | —                                              |

Counted across 28 rows: 18 reusable as-is, 7 need adaptation, 3 desktop-only.

The three desktop-only rows are all _input modalities_, not features. No feature
is desktop-only; each of these needs a touch equivalent alongside the existing
desktop path, which stays unchanged.

## Target Architecture

One product codebase, with platform differences pushed to the edges:

```text
Doyo
├── Shared Svelte application/domain logic   (stores, utils, components)
├── Shared Rust core                          (core/: models, hierarchy rules,
│                                              task logic, migrations, services)
├── Desktop platform layer                    (src-tauri desktop config,
│                                              window sizing, global shortcuts)
└── Android platform layer                    (src-tauri/gen/android, lifecycle,
                                               permissions, mobile presentation)
```

The Rust core stays the source of truth. There must not be a
`desktop-task-service` and an `android-task-service`; if a behaviour differs by
platform it belongs behind a small capability check at the edge, not in a second
copy of the rules.

For components, prefer one component with an adaptive presentation over two
components with duplicated logic. The task inspector is the model case: the same
data and the same handlers, presented as a right-hand panel on desktop and as a
full-screen sheet on Android.

## Prerequisites

None of the Android toolchain is installed on this machine. Measured state:

| Requirement          | Status here                                                           |
| -------------------- | --------------------------------------------------------------------- |
| Android SDK          | missing (`ANDROID_HOME` unset, no `~/Android/Sdk`)                    |
| Android NDK          | missing (`NDK_HOME` unset)                                            |
| `sdkmanager`         | missing                                                               |
| Emulator             | missing                                                               |
| `adb`                | present (`/usr/bin/adb`)                                              |
| Gradle               | present (system `/usr/bin/gradle`)                                    |
| JDK                  | OpenJDK **25.0.3-ea** — newer than the Android Gradle Plugin supports |
| Rust Android targets | none (`x86_64-unknown-linux-gnu` only)                                |
| Tauri CLI            | 2.11.4, includes `tauri android`                                      |

Two things need attention before initialization:

1. The SDK and NDK are a multi-gigabyte install. This machine has **30 GB free
   (91% used)**, which is enough but not comfortable once Gradle caches and
   Android Rust build artifacts are added.
2. JDK 25 is an early-access build and is ahead of what the Android Gradle
   Plugin supports. A JDK 17 or 21 will almost certainly be needed, selected per
   project rather than by changing the system default.

### Setup

Install the command line tools and the SDK packages:

```bash
# Android command line tools -> $HOME/Android/Sdk/cmdline-tools/latest
export ANDROID_HOME="$HOME/Android/Sdk"
sdkmanager --install "platform-tools" "platforms;android-34" \
  "build-tools;34.0.0" "ndk;27.0.12077973"
```

Point the environment at them, per shell rather than system-wide:

```bash
export ANDROID_HOME="$HOME/Android/Sdk"
export NDK_HOME="$ANDROID_HOME/ndk/$(ls -1 $ANDROID_HOME/ndk | tail -1)"
export JAVA_HOME=/usr/lib/jvm/java-21-openjdk-amd64   # not the ea JDK 25
```

Add the Rust targets:

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi \
  i686-linux-android x86_64-linux-android
```

### Initialization

Once the above resolve, generate the Android project with Tauri rather than
hand-writing one:

```bash
npm run tauri android init
```

This writes `src-tauri/gen/android/`. Two things to check immediately
afterwards, because they are easy to break:

- `src-tauri/tauri.conf.json` must keep `identifier` and the desktop `bundle`
  configuration unchanged. Desktop `.deb` packaging must not move.
- `src-tauri/capabilities/default.json` currently points at
  `../gen/schemas/desktop-schema.json` and lists `"windows": ["main"]`. A mobile
  capability set is generated separately; the desktop file must keep working.

Every generated file should be recorded in this document when it is created.

### Dev And Device Workflow

```bash
npm run tauri android dev            # emulator or attached device
npm run tauri android build --debug  # development APK
adb devices                          # confirm the target is attached
adb install -r <path-to-apk>
```

## Mobile UX Requirements

Doyo Android must not be the desktop layout scaled down. The design language —
calm, minimal, structured — stays; the interaction model changes.

Hard requirements, from the audit above:

- No action may be reachable **only** through hover, right click, `Shift+F10`,
  or a keyboard shortcut. The three desktop-only rows in the matrix are exactly
  the places this is currently true.
- Touch targets need a minimum of 48dp. The navigation rail buttons are 32–36px
  and the calendar item controls are smaller still.
- The 9 components that reveal controls on hover need a persistent or
  long-press equivalent.
- Android system Back must map to the view stack: close a dialog, close the
  inspector sheet, leave a nested screen, then exit.
- Desktop behaviour must not change. Presentation differences belong behind a
  platform check, not a fork of the component.

A candidate information architecture, to be validated against real screen
density before being implemented:

```text
Today · Tasks · Plan · Focus · More

Plan  -> Calendar, Kanban, Timeline
Focus -> Pomodoro, Stopwatch, productivity methods
More  -> Habits, Countdown, Statistics, Settings
```

Twelve navigation destinations do not belong in a bottom bar; five is the
ceiling, with the rest grouped underneath.

## Known Unsupported / Unverified

Nothing in this list may be described as working until it has been run on a
device.

- **Everything.** No Android build has been produced.
- **Reminders while the app is closed.** The desktop app cannot do this and says
  so. On Android the plugin exposes `Schedule.at()`, `ScheduleEvery`,
  `allowWhileIdle`, `pending()` and `cancel()`, which map to Android's alarm
  scheduling, so it is likely achievable _without_ custom Kotlin. Doyo does not
  use that API today. Until it is implemented and observed firing with the app
  closed, the closed-app claim stays false on Android too.
- **Focus timers across background/resume.** The current timer is a WebView
  `setInterval`. It will drift or stop when the app is backgrounded. The fix is
  to persist the start timestamp and recompute on resume, which is also more
  correct on desktop.
- **`navigator.clipboard`** in the Android WebView, which the export flow
  depends on.
- **Attachments.** Not a shipped feature on desktop either.
- **Google Play.** Out of scope. No signing keys, no Play Console, no published
  artifacts.

## Next Milestones

In priority order:

1. Install the prerequisites and run `tauri android init`; record the generated
   files and confirm desktop packaging is unaffected.
2. Get a debug APK to launch, and prove the first-run database is created in
   app-private storage.
3. Prove the core task workflow end to end: workspace, group, task, subtask,
   edit, due date, priority, tag, complete — then kill the process, reopen, and
   confirm the data and `PRAGMA integrity_check` are intact.
4. Wire Android system Back to the view stack.
5. Replace the hover-only and right-click-only affordances with touch paths,
   starting with the tree row context menu.
6. Bottom navigation and the inspector-as-sheet presentation.
7. Timer correctness across background, resume, and process death.
8. Reminder scheduling through the notification plugin's `Schedule` API, then
   verify delivery with the app closed.
