# Android

Status: **foundation working.** Doyo builds, installs, and runs on Android, and
the core task workflow persists correctly. The mobile UX has not been built yet:
what runs on a phone today is the desktop layout.

Verified on a physical device — Samsung SM-S908U, arm64-v8a, Android 14 (SDK 34)
— not an emulator. Anything below that has not been run on a device is called
out as unverified.

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

Verified working combination:

| Component            | Version                                   |
| -------------------- | ----------------------------------------- |
| Android SDK Platform | android-34                                |
| Build tools          | 34.0.0                                    |
| NDK                  | 27.1.12297006 (r27b)                      |
| JDK                  | 21 (`/usr/lib/jvm/java-21-openjdk-amd64`) |
| Gradle               | 8.14.3 (wrapper, downloaded by the build) |
| Tauri CLI            | 2.11.4                                    |
| Rust target          | `aarch64-linux-android`                   |

The system default JDK on the build machine is an early-access JDK 25, which the
Android Gradle Plugin does not support. Do not change the system default: set
`JAVA_HOME` for the Android commands only.

### Setup

```bash
export ANDROID_HOME="$HOME/Android/Sdk"
sdkmanager --install "platform-tools" "platforms;android-34" \
  "build-tools;34.0.0" "ndk;27.1.12297006"

rustup target add aarch64-linux-android
```

Then, per shell, before any `tauri android` command:

```bash
export ANDROID_HOME="$HOME/Android/Sdk"
export NDK_HOME="$ANDROID_HOME/ndk/27.1.12297006"
export JAVA_HOME=/usr/lib/jvm/java-21-openjdk-amd64
export PATH="$JAVA_HOME/bin:$ANDROID_HOME/platform-tools:$PATH"
```

Pin `NDK_HOME` rather than globbing for the newest: installing the SDK can pull
in a second NDK as a dependency, and "latest" then silently changes toolchain.

Building all four ABIs is not necessary for device testing. `--target aarch64`
covers current phones and avoids three extra Rust toolchains.

### Initialization

```bash
npm run tauri android init
```

This generated `src-tauri/gen/android/` — 40 tracked files, plus Tauri's own
`.gitignore` entries for build output, `local.properties`, `tauri.properties`
and the `jniLibs` symlinks. None of the tracked files contain machine-specific
absolute paths. `tauri android init` also installs the other three Rust Android
targets whether or not they are needed.

Two things had to change in the repository, both additive:

- `src-tauri/Cargo.toml` gained a `[lib]` block with
  `crate-type = ["staticlib", "cdylib", "rlib"]`. Android loads the app as a
  shared library rather than executing a binary, and without this the build
  fails at packaging with `Library artifact not found ... libdoyo.so`. The lib
  name stays `doyo`, so `main.rs` still links the rlib and the desktop binary is
  byte-for-byte equivalent in behaviour. Desktop `.deb` packaging, the bundle
  identifier and `bundle.targets` are unchanged.
- `.prettierignore` gained `src-tauri/gen/android/`, because the generated
  `assets/tauri.conf.json` is not Prettier-formatted and would otherwise fail
  `npm run lint` for anyone who has run `android init`. CI never runs it, so CI
  was never affected.

`src-tauri/capabilities/default.json` still references the desktop schema and
`"windows": ["main"]`, and did not need changing for the app to run.

### Build And Install

```bash
npm run tauri android build --debug --target aarch64
adb install -r src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk
adb shell am start -n io.github.hex1mal.doyo/.MainActivity
```

The debug APK is ~157 MB because the unstripped `libdoyo.so` is ~150 MB of it.
A release build strips symbols; the debug size is not indicative.

## What Has Been Verified On Device

The first milestone — "Doyo launches on Android and the core task workflow works
correctly with local persistence" — is met. The whole target flow was performed
through the UI on the device:

```text
launch -> first-run database -> create workspace -> create group
      -> create task -> nested subtask -> rename -> due date -> priority
      -> tag -> complete -> force-stop -> reopen -> data intact
```

Confirmed:

- **First-run database** created at
  `/data/data/io.github.hex1mal.doyo/doyo.db`, inside app-private storage. No
  storage permission is requested or needed.
- **Migrations ran**: `schema_version` reached 6, the same as desktop.
- **WAL** is active, as on desktop.
- **Persistence across process death**: `adb shell am force-stop` then relaunch
  kept all 5 nodes, the tag, and the completion state.
- **Integrity**: `PRAGMA integrity_check` returns `ok` and
  `PRAGMA foreign_key_check` is clean, read from the device's own database file
  with its WAL.
- **Hierarchy**: `Workspace -> Group -> Task -> Subtask` parentage is correct.
- **Completion policy**: completing a parent with `Individual` left the subtask
  open, as on desktop.
- **Text entry** through the soft keyboard, including spaces, commits on blur.
- **Background and resume**: `HOME` then relaunch kept the same process and
  state, with no crash.
- **Notification permissions**: the plugin merged `POST_NOTIFICATIONS`,
  `WAKE_LOCK` and `RECEIVE_BOOT_COMPLETED` into the manifest. Permissions being
  present is not the same as reminders working; see below.

The Rust core, `doyo-core`, and `rusqlite` with bundled SQLite all cross-compile
for `aarch64-linux-android` without a single source change.

## Problems Found On Device

These are observed, not predicted:

- **System Back exits the app.** Pressing Back on the main view terminates Doyo
  and returns to the launcher. Nothing maps Back to the view stack, so it will
  also discard an open dialog or inspector rather than closing it. This is the
  most urgent mobile defect.
- **No safe-area handling.** The Doyo header renders underneath the system
  status bar, and the status bar at the bottom sits under the gesture bar.
- **The layout is wider than the screen.** The sidebar, tree and inspector are
  all laid out at desktop widths, so the shell scrolls horizontally and the
  inspector is cut off until it is scrolled to.
- **Enter in a text field triggers the desktop shortcut.** Committing a title
  with the soft keyboard's Enter also fired "create sibling" and produced an
  empty node. The global keyboard handler does not know it is on a phone.
- **Touch targets are desktop-sized.** The navigation rail buttons are 32-36px
  against a 48dp guideline.

None of these are architectural. They are all in the presentation layer.

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

- **The mobile UX itself.** What runs today is the desktop layout on a phone.
  See "Problems Found On Device".
- **Reminders while the app is closed.** The desktop app cannot do this and says
  so. On Android the notification plugin exposes `Schedule.at()`,
  `ScheduleEvery`, `allowWhileIdle`, `pending()` and `cancel()`, and the build
  already carries `WAKE_LOCK` and `RECEIVE_BOOT_COMPLETED`, so it looks
  achievable without custom Kotlin. Doyo does not use that API today. Until it
  is implemented and observed firing with the app closed, the closed-app claim
  stays false on Android too.
- **Focus timers across background and process death.** Not exercised. The
  current timer is a WebView `setInterval` and will drift or stop when
  backgrounded. Persisting the start timestamp and recomputing on resume is
  also more correct on desktop.
- **Notification delivery.** The permissions are in the manifest; no
  notification has been observed on the device.
- **`navigator.clipboard`** in the Android WebView, which the export flow
  depends on. Not exercised.
- **Backup and restore on Android.** Not exercised.
- **Rotation and configuration changes.** Not exercised.
- **Other ABIs.** Only `arm64-v8a` was built and run.
- **Release builds and signing.** Only an unsigned debug build exists.
- **Attachments.** Not a shipped feature on desktop either.
- **Google Play.** Out of scope. No signing keys, no Play Console, no published
  artifacts.

## Next Milestones

In priority order:

1. Map Android system Back to the view stack: dialog, then inspector, then
   nested screen, then exit. Today Back exits from anywhere.
2. Safe-area insets, so the header and status bar are not under the system UI.
3. Stop the shell scrolling horizontally: the sidebar and inspector need mobile
   presentations rather than desktop widths.
4. Gate the global keyboard handler so soft-keyboard Enter does not fire desktop
   shortcuts.
5. Touch targets and the hover-only affordances: 9 components reveal controls on
   hover, 3 depend on right click.
6. Bottom navigation, and the inspector as a full-screen sheet.
7. Timer correctness across background, resume and process death.
8. Reminder scheduling through the notification plugin's `Schedule` API, then
   verify delivery with the app closed.
9. Backup, restore, import and export on Android.
10. Release build, signing, and only then any store discussion.
