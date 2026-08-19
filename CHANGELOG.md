# Changelog

## 1.0.1 - 2026-08-19

Maintenance release. No new features.

- Global shortcuts no longer act on the tree behind an open modal, dialog or
  menu. Delete could previously delete the selected node while a confirmation
  dialog was on screen. Open surfaces now register a layer and own the
  keyboard until they close.
- Ctrl+N, Ctrl+Z and Ctrl+Y no longer fire while typing in a text field, so
  Ctrl+Z undoes the text rather than the last node operation.
- Enter and Space are no longer taken from a focused button or link.
- Restoring a backup asks for confirmation in a themed dialog that names the
  selected backup and explains the pre-restore snapshot, replacing the system
  dialog. The restore sequence and its safety logic are unchanged.
- All twelve navigation rail modules fit at 800x600 rather than scrolling
  without an affordance; the rail fades whichever end still has content.
- Calendar time block actions moved into a menu, so month cells lead with what
  is scheduled instead of five buttons. No action was removed, and the 30
  minute adjustments are now reachable by keyboard.
- The sidebar row context menu can be dismissed with Escape.
- Secret scanning: the gitleaks allowlist is now effective under the scanner
  version CI runs, and that version is pinned.

## 1.0.0 - 2026-07-29

Initial Doyo release.

- Renamed the application to Doyo.
- Set application identifier to `io.github.hex1mal.doyo`.
- Preserved compatibility with existing local Doyo and TodoApp data through one-time migrations.
- Hardened JSON transfer, Markdown export filenames, backup restore path validation, Tauri permissions, and public repository metadata.
- Includes recursive task hierarchy, smart views, Calendar, Kanban, Timeline, productivity methods, habits, countdowns, statistics, settings, backup, restore, import, and export.
