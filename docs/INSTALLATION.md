# Installation

## System Dependencies

On Debian-based Linux distributions:

```bash
sudo apt update
sudo apt install -y \
  build-essential \
  curl \
  libwebkit2gtk-4.1-dev \
  libgtk-3-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  patchelf
```

Install Node.js 22 and Rust using your preferred package manager or official installers.

## Build A Package

```bash
npm ci
npm run tauri build
```

Generated Linux packages are written under:

```text
target/release/bundle/
```

Doyo is a Cargo workspace, so build output goes to `target/` at the repository
root rather than inside `src-tauri/`.

## Install The Debian Package

```bash
sudo apt install ./target/release/bundle/deb/*.deb
```

## Desktop Entry

The package installs a desktop entry for Doyo. A user-local desktop entry can also be placed in:

```text
~/.local/share/applications/
```

It should use:

- `Name=Doyo`
- `Exec=doyo`
- `Icon=io.github.hex1mal.doyo`
- `Categories=Office;Utility;`

## Data Location

Linux app data:

```text
~/.local/share/io.github.hex1mal.doyo/
```

Old Doyo and TodoApp data directories are not removed automatically. Keep them until you have confirmed Doyo has migrated and loaded your data.
