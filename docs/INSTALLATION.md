# Installation

## Install The Published Release

This is the path for normal use. No build tools are required.

1. Download `Doyo_1.0.1_amd64.deb` from the [latest release](https://github.com/Hex1mal/doyo-desktop/releases/latest).
2. Install it from the directory you downloaded it into:

   ```bash
   sudo apt install ./Doyo_1.0.1_amd64.deb
   ```

To verify the download first, fetch `SHA256SUMS` from the same release and run `sha256sum -c SHA256SUMS` next to the package.

Doyo publishes a Debian/Ubuntu `.deb` for Linux on x86-64 only.

## Build From Source

The sections below are for developers building the repository locally.

### System Dependencies

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

### Build A Package

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

### Install The Package You Built

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
