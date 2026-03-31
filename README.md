# Transmission Remote — Slint

A lightweight native desktop GUI for **Transmission daemon** built with **Rust + Slint**.  
No GTK, no Qt — pure Rust rendering via Skia/OpenGL or Vulkan.

> **Developed with Qwen 3.5 Plus (Alibaba).**

---

## What's New in v0.5

### 🌐 Internationalization
- **5 languages:** English, Deutsch, Русский, 中文, Español
- Language selection via Settings Dialog (⚙ button in toolbar)
- Persistent language preference between sessions
- Tray menu also translated

### 🖥️ New UI
- **Modern toolbar:**
  - ➕ Open, 🧲 Magnet, 📀 Create, 🔄 Rehash buttons on left
  - Search field centered
  - Settings button (⚙) on right showing current language
- **Left panel:**
  - Status filters: All, Downloading, Seeding, Completed, Stopped, Active, Inactive, Checking, Error
  - Real-time torrent count for each status
  - Click to filter torrents by status
- **Settings Dialog:**
  - Language selection with flags
  - Restart notification

### 🔧 System Tray
- **Resume All / Pause All** buttons for batch control
- Visual status indication (buttons show current state)

### 🛠️ Other Improvements
- Magnet links via input dialog + xdg-open
- .torrent file picker + xdg-open integration
- Fixed double-Cancel bug in CreateTorrentDialog

---

## Features

- **Torrent list** — name, status, progress, ↓/↑ speed, inline error messages
- **Per-torrent actions** — Start / Pause / Recheck / Open folder / Remove / Delete with files
- **Bulk actions** — Start All / Stop All with confirmation dialog
- **Status filters** — filter torrents by status (All, Downloading, Seeding, etc.)
- **Instant search** — filter by torrent name without waiting for RPC
- **System tray** — StatusNotifierItem via D-Bus (native zbus 4, no ksni/GTK)
- **Desktop notifications** — download complete, recheck done, torrent errors
- **Single instance** — second launch focuses the window or adds a `.torrent` file
- **Auto-detect Transmission** — reads daemon status, connects automatically
- **`.torrent` file handler** — open from file manager or pass as argument
- **i18n** — 5 languages, configurable via Settings Dialog
- **App icon** — embedded in binary, installed to hicolor theme via PKGBUILD
- **Autostart** — optional `.desktop` entry in `~/.config/autostart/`
- **Render backend** — auto-selects Vulkan → OpenGL → Software

---

## Installation

### AUR (Arch Linux) — build from source

```bash
paru -S transmission-remote-slint
# or manually:
git clone https://aur.archlinux.org/transmission-remote-slint.git
cd transmission-remote-slint
makepkg -si
```

### AUR — prebuilt binary

```bash
paru -S transmission-remote-slint-bin
```

### Manual installation

```bash
git clone https://github.com/guglovich/Transmission-Remote-Slint.git
cd Transmission-Remote-Slint
cargo build --release
sudo cp target/release/transmission-remote-slint /usr/local/bin/
```

---

## Configuration

Configuration file location: `~/.config/transmission-remote-slint/config.toml`

```toml
# Transmission daemon URL
url = "http://localhost:9091"

# Authentication (optional)
# username = "transmission"
# password = "transmission"

# Language (en, de, ru, zh, es)
language = "en"

# Refresh interval in seconds
refresh_interval_secs = 2

# Start minimized to tray
start_minimized = false

# Suspend on hide (save resources when minimized)
suspend_on_hide = false

# Delete .torrent file after adding
delete_torrent_after_add = true

# Autostart on login
autostart = false
```

---

## Usage

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Ctrl+O` | Open .torrent file |
| `Ctrl+M` | Add magnet link |
| `Ctrl+S` | Open settings |
| `Escape` | Close dialog |

### System Tray

- **Left click** — Show/Hide window
- **Right click** — Context menu:
  - Show / Hide
  - Resume All
  - Pause All
  - Quit

---

## Requirements

### Runtime

- `transmission-daemon` — running Transmission daemon
- `libxcb` — X11 protocol library
- `libxkbcommon` — XKB keymap library
- `fontconfig` — font configuration
- `freetype2` — font rendering
- `dbus` — desktop bus for notifications and tray

### Optional

- `zenity` — file picker dialogs (GNOME/X11)
- `kdialog` — file picker dialogs (KDE)
- `yad` — file picker dialogs (alternative)
- `libnotify` — desktop notifications
- `snixembed` / `xfce4-statusnotifier-plugin` — system tray support

### Build

- `rust` — Rust compiler
- `cargo` — Rust package manager
- `pkg-config` — build configuration helper

---

## Building from Source

```bash
# Clone repository
git clone https://github.com/guglovich/Transmission-Remote-Slint.git
cd Transmission-Remote-Slint

# Build release
cargo build --release

# Run
./target/release/transmission-remote-slint
```

### Build with specific renderer

```bash
# OpenGL (default)
cargo build --release --features backend-winit,renderer-winit-skia-opengl

# Vulkan
cargo build --release --features backend-winit,renderer-winit-skia-vulkan

# Software (fallback)
cargo build --release --features backend-winit,renderer-winit-software
```

---

## Troubleshooting

### System tray not showing

Install system tray support:
- **XFCE:** `xfce4-statusnotifier-plugin`
- **Openbox:** `snixembed`
- **GNOME:** `gnome-shell-extension-appindicator`

### Notifications not working

Install libnotify:
```bash
sudo pacman -S libnotify
```

### File picker not opening

Install a dialog utility:
```bash
# GNOME/X11
sudo pacman -S zenity

# KDE
sudo pacman -S kdialog

# Alternative
sudo pacman -S yad
```

### Can't connect to daemon

1. Ensure Transmission daemon is running:
   ```bash
   systemctl --user start transmission
   ```

2. Check daemon URL in config:
   ```toml
   url = "http://localhost:9091"
   ```

3. Check authentication credentials if required.

---

## Comparison

| Feature | **transmission-remote-slint v0.5** | transmission-remote-gtk | transmission-qt | Transmission GTK 4.x |
|---|---|---|---|---|
| Type | Remote only | Remote only | Standalone + Remote | Standalone |
| Toolkit | Slint (Rust) | GTK 3 | Qt 5/6 | GTK 4 |
| System tray | ✅ Works (SNI/D-Bus) | ✅ Works | ✅ Works | ⚠️ Broken in GTK 4¹ |
| Desktop notifications | ✅ | ✅ | ✅ | ✅ |
| RAM (idle) | ~50 MB | ~80 MB | ~90 MB | ~150 MB |
| Languages | 5 | 1 | Multiple | Multiple |
| License | GPL-2.0-or-later | GPL-2.0-or-later | GPL-2.0-or-later | GPL-2.0-or-later |

> ¹ GTK 4 dropped tray support. The fix is in development but not yet merged as of early 2026.  
> RAM figures are approximate, measured on Arch Linux with ~50 torrents.

---

## Screenshots

### v0.5 Main Window

![Main Window](screenshots/main-v0.5.png)

### Settings Dialog

![Settings](screenshots/settings-v0.5.png)

### System Tray Menu

![Tray Menu](screenshots/tray-v0.5.png)

---

## Roadmap

### v0.6 (planned)
- [ ] Disk filter bar (re-add with improved UI)
- [ ] Torrent properties dialog
- [ ] Speed limit controls
- [ ] Queue management
- [ ] Dark/Light theme toggle

### Future
- [ ] Torrent creation wizard
- [ ] Peer list view
- [ ] File priority controls
- [ ] RSS feed support
- [ ] More translations

---

## License

GPL-2.0-or-later — see [LICENSE](LICENSE) for details.

---

## Acknowledgments

- **Transmission** — BitTorrent daemon
- **Slint** — UI toolkit
- **Qwen 3.5 Plus** — AI development assistant
- **AUR maintainers** — package maintenance
